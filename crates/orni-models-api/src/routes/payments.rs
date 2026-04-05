use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use uuid::Uuid;

use axum::http::StatusCode;
use orni_models_types::CheckoutRequest;

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
    let wallet: Option<String> =
        sqlx::query_scalar("SELECT wallet_address FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;

    let wallet = wallet.ok_or_else(|| AppError::BadRequest(
        "No wallet address linked. USDC deposits require a connected wallet.".into(),
    ))?;

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

/// POST /api/checkout — create a Stripe checkout session for credit pack purchase
pub async fn create_checkout(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Json(req): Json<CheckoutRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    let pack = crate::services::stripe::get_pack(&req.pack)
        .ok_or_else(|| AppError::BadRequest("Invalid credit pack. Use 5, 10, 25, or 50.".into()))?;

    let (session_id, checkout_url) = crate::services::stripe::create_checkout_session(
        &state.http_client,
        &state.config,
        pack,
        &user_id.to_string(),
    )
    .await?;

    // Record pending purchase
    sqlx::query(
        r#"INSERT INTO credit_purchases (id, user_id, amount_micro_usdc, amount_usd_cents, stripe_session_id, status)
        VALUES ($1, $2, $3, $4, $5, 'pending')"#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(pack.amount_micro_usdc)
    .bind(pack.amount_usd_cents)
    .bind(&session_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "checkout_url": checkout_url,
        "session_id": session_id,
    })))
}

/// POST /api/payments/webhook — Stripe webhook handler with signature verification
pub async fn stripe_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> AppResult<StatusCode> {
    // Verify Stripe webhook signature
    if let Some(ref secret) = state.config.stripe_webhook_secret {
        let sig_header = headers
            .get("stripe-signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Stripe-Signature header".into()))?;

        // Parse t= and v1= from signature header
        let mut timestamp = "";
        let mut signature = "";
        for part in sig_header.split(',') {
            let part = part.trim();
            if let Some(t) = part.strip_prefix("t=") {
                timestamp = t;
            } else if let Some(v) = part.strip_prefix("v1=") {
                signature = v;
            }
        }

        if timestamp.is_empty() || signature.is_empty() {
            return Err(AppError::Unauthorized("Invalid Stripe-Signature format".into()));
        }

        // Compute expected signature: HMAC-SHA256(secret, "timestamp.body")
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let signed_payload = format!("{}.{}", timestamp, body);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|_| AppError::Internal("HMAC init failed".into()))?;
        mac.update(signed_payload.as_bytes());
        let expected = hex::encode(mac.finalize().into_bytes());

        if expected != signature {
            tracing::warn!("Stripe webhook signature mismatch");
            return Err(AppError::Unauthorized("Invalid webhook signature".into()));
        }

        // Check timestamp is within 5 minutes (replay protection)
        if let Ok(ts) = timestamp.parse::<i64>() {
            let now = chrono::Utc::now().timestamp();
            if (now - ts).abs() > 300 {
                return Err(AppError::Unauthorized("Webhook timestamp too old".into()));
            }
        }
    } else {
        tracing::warn!("STRIPE_WEBHOOK_SECRET not set — webhook signature NOT verified");
    }

    let event: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| AppError::BadRequest("Invalid webhook payload".into()))?;

    let event_type = event["type"].as_str().unwrap_or("");

    if event_type == "checkout.session.completed" {
        let session = &event["data"]["object"];
        let session_id = session["id"].as_str().unwrap_or("");
        let user_id_str = session["metadata"]["user_id"].as_str().unwrap_or("");
        let micro_usdc_str = session["metadata"]["amount_micro_usdc"].as_str().unwrap_or("0");

        let user_id: Uuid = user_id_str.parse()
            .map_err(|_| AppError::BadRequest("Invalid user_id in metadata".into()))?;
        let amount_micro_usdc: i64 = micro_usdc_str.parse().unwrap_or(0);

        if amount_micro_usdc > 0 {
            // Update purchase status
            sqlx::query(
                "UPDATE credit_purchases SET status = 'completed' WHERE stripe_session_id = $1",
            )
            .bind(session_id)
            .execute(&state.db)
            .await?;

            // Credit user balance
            sqlx::query("UPDATE users SET usdc_balance = usdc_balance + $1 WHERE id = $2")
                .bind(amount_micro_usdc)
                .bind(user_id)
                .execute(&state.db)
                .await?;

            tracing::info!(
                user_id = %user_id,
                amount = amount_micro_usdc,
                "Stripe checkout completed, credits added"
            );
        }
    }

    Ok(StatusCode::OK)
}
