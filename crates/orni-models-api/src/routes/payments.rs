use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::services::solana;
use crate::state::AppState;
use orni_models_types::{BalanceResponse, Deposit, DepositRequest, WithdrawRequest};

pub async fn get_balance(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
) -> AppResult<Json<BalanceResponse>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    let balance: i64 =
        sqlx::query_scalar("SELECT usdc_balance FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;

    let pending_earnings: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(p.creator_share), 0)
        FROM payments p
        JOIN models m ON m.id = p.model_id
        WHERE m.creator_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(BalanceResponse {
        balance,
        pending_earnings,
    }))
}

pub async fn submit_deposit(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Json(req): Json<DepositRequest>,
) -> AppResult<Json<Deposit>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    // Check for duplicate
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deposits WHERE tx_signature = $1",
    )
    .bind(&req.tx_signature)
    .fetch_one(&state.db)
    .await?;

    if existing > 0 {
        return Err(AppError::Conflict("Deposit already processed".into()));
    }

    // Verify on-chain
    let wallet: String =
        sqlx::query_scalar("SELECT wallet_address FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;

    let verified = solana::verify_deposit(
        &state.http_client,
        &state.config,
        &req.tx_signature,
        req.amount as u64,
        &wallet,
    )
    .await?;

    if !verified {
        return Err(AppError::BadRequest(
            "Could not verify deposit transaction".into(),
        ));
    }

    // Record deposit and credit balance
    let deposit = sqlx::query_as::<_, Deposit>(
        r#"
        INSERT INTO deposits (id, user_id, amount, tx_signature, verified)
        VALUES ($1, $2, $3, $4, true)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(req.amount)
    .bind(&req.tx_signature)
    .fetch_one(&state.db)
    .await?;

    sqlx::query("UPDATE users SET usdc_balance = usdc_balance + $1 WHERE id = $2")
        .bind(req.amount)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(deposit))
}

pub async fn request_withdraw(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Json(req): Json<WithdrawRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    let balance: i64 =
        sqlx::query_scalar("SELECT usdc_balance FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;

    if balance < req.amount {
        return Err(AppError::InsufficientBalance);
    }

    // Deduct balance
    sqlx::query("UPDATE users SET usdc_balance = usdc_balance - $1 WHERE id = $2")
        .bind(req.amount)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    // TODO: Actually send USDC on-chain via escrow wallet
    // For MVP, just record the withdrawal request
    tracing::info!(
        user_id = %user_id,
        amount = req.amount,
        destination = %req.destination_wallet,
        "Withdrawal requested — manual payout required"
    );

    Ok(Json(serde_json::json!({
        "status": "pending",
        "amount": req.amount,
        "destination": req.destination_wallet,
        "message": "Withdrawal submitted. USDC will be sent within 24 hours."
    })))
}
