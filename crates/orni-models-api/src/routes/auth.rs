use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use uuid::Uuid;

use crate::auth::{issue_jwt, verify_siws};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use orni_models_types::{AuthResponse, NonceRequest, NonceResponse, User, VerifyRequest};

pub async fn get_nonce(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NonceRequest>,
) -> AppResult<Json<NonceResponse>> {
    let nonce = state.nonce_store.generate(&req.wallet_address);
    let message = format!(
        "Sign in to Orni Models\nWallet: {}\nNonce: {}",
        req.wallet_address, nonce
    );

    Ok(Json(NonceResponse { nonce, message }))
}

pub async fn verify(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> AppResult<Json<AuthResponse>> {
    // Validate nonce
    if !state.nonce_store.validate_and_remove(&req.nonce, &req.wallet_address) {
        return Err(AppError::Unauthorized("Invalid or expired nonce".into()));
    }

    // Verify signature
    let message = format!(
        "Sign in to Orni Models\nWallet: {}\nNonce: {}",
        req.wallet_address, req.nonce
    );
    verify_siws(&req.wallet_address, message.as_bytes(), &req.signature)?;

    // Upsert user
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, wallet_address)
        VALUES ($1, $2)
        ON CONFLICT (wallet_address)
        DO UPDATE SET updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&req.wallet_address)
    .fetch_one(&state.db)
    .await?;

    let token = issue_jwt(&user.id, &user.wallet_address, &state.config.jwt_secret)?;

    Ok(Json(AuthResponse { token, user }))
}
