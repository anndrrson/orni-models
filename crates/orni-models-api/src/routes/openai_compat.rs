use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use axum::Json;
use futures::stream::Stream;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::inference::InferenceService;
use crate::state::AppState;
use orni_models_types::{InferenceChatMessage, Model, ModelStatus, OpenAIChatRequest};

/// POST /v1/chat/completions — OpenAI-compatible chat endpoint
///
/// Auth methods (in priority order):
/// 1. x402 payment proof via `X-Payment` header (agent-to-agent, no account needed)
/// 2. Bearer API key (orn_...) for registered users
///
/// If neither is provided and the model requires payment, returns HTTP 402
/// with x402-compliant `payment-required` header containing USDC payment instructions.
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<OpenAIChatRequest>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, axum::Error>>>> {
    // ── Resolve the model first (needed for pricing in 402 response) ──
    let model = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE (slug = $1 OR base_model = $1 OR provider_model_id = $1) AND status = $2",
    )
    .bind(&req.model)
    .bind(ModelStatus::Live)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Model '{}' not found or not live", req.model)))?;

    // ── Check for x402 payment proof first ──
    let x402_paid = if let Some(payment_header) = headers.get("x-payment").and_then(|v| v.to_str().ok()) {
        // x402 payment proof: base64-encoded Solana transaction signature
        // For now, we accept the proof and log it. Full on-chain verification is TODO.
        // This allows agents to pay per-request without an account.
        tracing::info!(
            model = %model.slug,
            payment_proof = %&payment_header[..payment_header.len().min(20)],
            "x402 payment received"
        );
        true
    } else {
        false
    };

    // ── Standard API key auth (if no x402 payment) ──
    let (user_id, model_id, is_free_query) = if x402_paid {
        // x402 agents don't need accounts — skip auth and balance checks
        (None, model.id, false)
    } else {
        // Require API key
        let auth_header = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                // No auth at all — return x402 payment instructions
                AppError::X402PaymentRequired {
                    pay_to: state.config.escrow_wallet_address.clone(),
                    amount_micro_usdc: model.price_per_query,
                    model_slug: model.slug.clone(),
                    model_name: model.name.clone(),
                }
            })?;

        let api_key = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("Invalid Authorization format".into()))?;

        // Hash the key and look it up
        use sha2::{Digest, Sha256};
        let key_hash = hex::encode(Sha256::digest(api_key.as_bytes()));

        let key_record = sqlx::query_as::<_, (Uuid, Uuid, Uuid)>(
            "SELECT id, user_id, model_id FROM api_keys WHERE key_hash = $1 AND is_active = true",
        )
        .bind(&key_hash)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid API key".into()))?;

        let (key_id, uid, _key_model_id) = key_record;

        // Update last_used_at
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(key_id)
            .execute(&state.db)
            .await?;

        // Check free tier
        let mut is_free = false;
        if model.free_queries_per_day > 0 {
            let usage: Option<i32> = sqlx::query_scalar(
                "SELECT query_count FROM free_query_usage WHERE user_id = $1 AND model_id = $2 AND query_date = CURRENT_DATE",
            )
            .bind(uid)
            .bind(model.id)
            .fetch_optional(&state.db)
            .await?;

            if usage.unwrap_or(0) < model.free_queries_per_day {
                is_free = true;
            }
        }

        // Check balance (skip for free queries)
        if !is_free {
            let balance: i64 =
                sqlx::query_scalar("SELECT usdc_balance FROM users WHERE id = $1")
                    .bind(uid)
                    .fetch_one(&state.db)
                    .await?;

            if balance < model.price_per_query {
                // Return x402 payment instructions instead of generic 402
                return Err(AppError::X402PaymentRequired {
                    pay_to: state.config.escrow_wallet_address.clone(),
                    amount_micro_usdc: model.price_per_query,
                    model_slug: model.slug.clone(),
                    model_name: model.name.clone(),
                });
            }
        }

        (Some(uid), model.id, is_free)
    };

    // ── Charge user (skip for x402 — they already paid on-chain) ──
    if let Some(uid) = user_id {
        if is_free_query {
            sqlx::query(
                r#"INSERT INTO free_query_usage (user_id, model_id, query_date, query_count)
                VALUES ($1, $2, CURRENT_DATE, 1)
                ON CONFLICT (user_id, model_id, query_date) DO UPDATE SET query_count = free_query_usage.query_count + 1"#,
            )
            .bind(uid)
            .bind(model.id)
            .execute(&state.db)
            .await?;

            sqlx::query("UPDATE models SET total_queries = total_queries + 1 WHERE id = $1")
                .bind(model.id)
                .execute(&state.db)
                .await?;
        } else {
            sqlx::query("UPDATE users SET usdc_balance = usdc_balance - $1 WHERE id = $2")
                .bind(model.price_per_query)
                .execute(&state.db)
                .await?;

            let platform_share =
                model.price_per_query * state.config.platform_share_bps as i64 / 10_000;
            let creator_share = model.price_per_query - platform_share;

            sqlx::query(
                "INSERT INTO payments (id, user_id, model_id, amount, creator_share, platform_share) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(uid)
            .bind(model.id)
            .bind(model.price_per_query)
            .bind(creator_share)
            .bind(platform_share)
            .execute(&state.db)
            .await?;

            sqlx::query("UPDATE users SET usdc_balance = usdc_balance + $1 WHERE id = $2")
                .bind(creator_share)
                .execute(&state.db)
                .await?;

            sqlx::query(
                "UPDATE models SET total_queries = total_queries + 1, total_revenue = total_revenue + $1 WHERE id = $2",
            )
            .bind(model.price_per_query)
            .bind(model.id)
            .execute(&state.db)
            .await?;
        }
    } else {
        // x402 payment — just count the query
        sqlx::query("UPDATE models SET total_queries = total_queries + 1, total_revenue = total_revenue + $1 WHERE id = $2")
            .bind(model.price_per_query)
            .bind(model.id)
            .execute(&state.db)
            .await?;
    }

    // ── Build messages ──
    let self_hosted_endpoint = model.self_hosted_endpoint.clone();
    let base_model = model.base_model.clone();
    let provider_model_id = match model.provider_model_id.as_ref() {
        Some(id) => id.clone(),
        None if self_hosted_endpoint.is_some() => base_model.clone(),
        None => return Err(AppError::Internal("Model has no provider model ID".into())),
    };

    let mut messages = vec![InferenceChatMessage {
        role: "system".into(),
        content: model.system_prompt.clone(),
    }];
    for msg in &req.messages {
        if msg.role != "system" {
            messages.push(msg.clone());
        }
    }

    // ── Stream inference ──
    let (tx, rx) = mpsc::channel::<String>(32);
    let inference = InferenceService::new(&state.config, &state.http_client);

    let said_cloud_url = state.config.said_cloud_url.clone();
    let http_for_resolve = state.http_client.clone();
    let model_id_str = format!("{}", req.model);

    tokio::spawn(async move {
        let (content_tx, mut content_rx) = mpsc::channel::<String>(32);

        let inference_messages = messages.clone();
        let _inference_handle = tokio::spawn(async move {
            let resolver = crate::services::node_resolver::NodeResolver::new(
                &http_for_resolve,
                &said_cloud_url,
            );
            let nodes = resolver.resolve(&provider_model_id).await;

            for node in &nodes {
                match inference
                    .try_connect_self_hosted(
                        &node.endpoint_url,
                        &provider_model_id,
                        &inference_messages,
                    )
                    .await
                {
                    Ok(response) => {
                        return InferenceService::stream_response(response, content_tx).await;
                    }
                    Err(_) => continue,
                }
            }

            if let Some(ref endpoint) = self_hosted_endpoint {
                match inference
                    .try_connect_self_hosted(endpoint, &provider_model_id, &inference_messages)
                    .await
                {
                    Ok(response) => {
                        return InferenceService::stream_response(response, content_tx).await;
                    }
                    Err(_) => {}
                }
            }

            inference
                .chat_stream(&provider_model_id, inference_messages, content_tx)
                .await
        });

        while let Some(chunk) = content_rx.recv().await {
            let _ = tx.send(chunk).await;
        }
    });

    // ── Stream in OpenAI SSE format with x402 pricing headers ──
    let stream = ReceiverStream::new(rx).map(move |content| {
        Ok(Event::default().data(
            serde_json::to_string(&serde_json::json!({
                "id": format!("chatcmpl-{}", Uuid::new_v4()),
                "object": "chat.completion.chunk",
                "model": model_id_str,
                "choices": [{
                    "index": 0,
                    "delta": { "content": content },
                    "finish_reason": null
                }]
            }))
            .unwrap_or_default(),
        ))
    });

    Ok(Sse::new(stream))
}

/// GET /v1/chat/completions — x402 discovery endpoint
///
/// Returns HTTP 402 with payment-required header so x402scan and other
/// discovery services can index this endpoint as an x402-compatible resource.
pub async fn x402_discovery(
    State(state): State<Arc<AppState>>,
) -> Response {
    use axum::response::IntoResponse;
    use base64::{engine::general_purpose::STANDARD, Engine};

    let pay_to = &state.config.escrow_wallet_address;

    let x402_payload = serde_json::json!({
        "x402Version": 1,
        "accepts": [{
            "scheme": "exact",
            "network": "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
            "maxAmountRequired": "50000",
            "payTo": pay_to,
            "resource": "/v1/chat/completions",
            "description": "AI model inference on ghola.xyz — chat with any open-source model",
            "mimeType": "text/event-stream",
            "extra": {
                "currency": "USDC",
                "platform": "ghola.xyz",
                "models": ["llama-3-8b", "llama-3-70b", "qwen-32b", "deepseek-r1-120b", "kimi-k2"],
                "priceRange": {"min": 50000, "max": 200000},
            }
        }]
    });

    let encoded = STANDARD.encode(serde_json::to_vec(&x402_payload).unwrap_or_default());

    let body = axum::Json(serde_json::json!({
        "error": "Payment required",
        "description": "AI inference endpoint — use POST with x402 payment or API key",
        "x402": {
            "version": 1,
            "payTo": pay_to,
            "currency": "USDC",
            "network": "solana",
            "models_available": 10,
            "docs": "https://ghola.xyz/models",
        }
    }));

    (
        StatusCode::PAYMENT_REQUIRED,
        [
            ("payment-required", encoded.as_str()),
            ("x-price-micro-usdc", "50000"),
            ("x-currency", "USDC"),
            ("x-payment-address", pay_to.as_str()),
        ],
        body,
    )
        .into_response()
}

use axum::response::Response;
use axum::http::StatusCode;
