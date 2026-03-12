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
/// Auth via Bearer API key (orn_...)
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<OpenAIChatRequest>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, axum::Error>>>> {
    // Extract API key from Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

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

    let (key_id, user_id, model_id) = key_record;

    // Update last_used_at
    sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
        .bind(key_id)
        .execute(&state.db)
        .await?;

    // Get model
    let model = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE id = $1 AND status = $2",
    )
    .bind(model_id)
    .bind(ModelStatus::Live)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Model not found or not live".into()))?;

    let self_hosted_endpoint = model.self_hosted_endpoint.clone();
    let base_model = model.base_model.clone();

    let provider_model_id = match model.provider_model_id.as_ref() {
        Some(id) => id.clone(),
        None if self_hosted_endpoint.is_some() => base_model.clone(),
        None => return Err(AppError::Internal("Model has no provider model ID".into())),
    };

    // Check free tier
    let mut is_free_query = false;
    if model.free_queries_per_day > 0 {
        let usage: Option<i32> = sqlx::query_scalar(
            "SELECT query_count FROM free_query_usage WHERE user_id = $1 AND model_id = $2 AND query_date = CURRENT_DATE",
        )
        .bind(user_id)
        .bind(model.id)
        .fetch_optional(&state.db)
        .await?;

        let used = usage.unwrap_or(0);
        if used < model.free_queries_per_day {
            is_free_query = true;
        }
    }

    // Check balance (skip for free queries)
    if !is_free_query {
        let balance: i64 =
            sqlx::query_scalar("SELECT usdc_balance FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&state.db)
                .await?;

        if balance < model.price_per_query {
            return Err(AppError::InsufficientBalance);
        }
    }

    // Charge user
    if is_free_query {
        sqlx::query(
            r#"INSERT INTO free_query_usage (user_id, model_id, query_date, query_count)
            VALUES ($1, $2, CURRENT_DATE, 1)
            ON CONFLICT (user_id, model_id, query_date) DO UPDATE SET query_count = free_query_usage.query_count + 1"#,
        )
        .bind(user_id)
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
        .bind(user_id)
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

    // Build messages: inject system prompt at the beginning
    let mut messages = vec![InferenceChatMessage {
        role: "system".into(),
        content: model.system_prompt.clone(),
    }];
    for msg in &req.messages {
        if msg.role != "system" {
            messages.push(msg.clone());
        }
    }

    // Stream inference response in OpenAI format
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
                    .try_connect_self_hosted(&node.endpoint_url, &provider_model_id, &inference_messages)
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

    // Stream in OpenAI SSE format
    let stream = ReceiverStream::new(rx).map(move |content| {
        Ok(Event::default()
            .data(serde_json::to_string(&serde_json::json!({
                "id": format!("chatcmpl-{}", Uuid::new_v4()),
                "object": "chat.completion.chunk",
                "model": model_id_str,
                "choices": [{
                    "index": 0,
                    "delta": { "content": content },
                    "finish_reason": null
                }]
            }))
            .unwrap_or_default()))
    });

    Ok(Sse::new(stream))
}
