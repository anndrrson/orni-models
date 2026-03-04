use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::sse::{Event, Sse};
use axum::Json;
use futures::stream::Stream;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::services::inference::InferenceService;
use crate::state::AppState;
use orni_models_types::{
    ChatMessage, ChatRequest, ChatRole, ChatSession, ChatStartResponse,
    InferenceChatMessage, Model, ModelStatus,
};

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Path(slug): Path<String>,
    Json(req): Json<ChatRequest>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, axum::Error>>>> {
    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    // Get model
    let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE slug = $1 AND status = $2")
        .bind(&slug)
        .bind(ModelStatus::Live)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Model not found or not live".into()))?;

    let provider_model_id = model
        .provider_model_id
        .as_ref()
        .ok_or_else(|| AppError::Internal("Model has no provider model ID".into()))?
        .clone();

    // Check balance
    let balance: i64 =
        sqlx::query_scalar("SELECT usdc_balance FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;

    if balance < model.price_per_query {
        return Err(AppError::InsufficientBalance);
    }

    // Get or create session
    let session_id = if let Some(sid) = req.session_id {
        // Verify session belongs to user
        let session = sqlx::query_as::<_, ChatSession>(
            "SELECT * FROM chat_sessions WHERE id = $1 AND user_id = $2",
        )
        .bind(sid)
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Session not found".into()))?;
        session.id
    } else {
        let session = sqlx::query_as::<_, ChatSession>(
            "INSERT INTO chat_sessions (id, user_id, model_id) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(model.id)
        .fetch_one(&state.db)
        .await?;
        session.id
    };

    // Save user message
    sqlx::query(
        "INSERT INTO chat_messages (id, session_id, role, content) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(session_id)
    .bind(ChatRole::User)
    .bind(&req.message)
    .execute(&state.db)
    .await?;

    // Deduct balance
    sqlx::query("UPDATE users SET usdc_balance = usdc_balance - $1 WHERE id = $2")
        .bind(model.price_per_query)
        .execute(&state.db)
        .await?;

    // Record payment with split
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

    // Credit creator
    sqlx::query("UPDATE users SET usdc_balance = usdc_balance + $1 WHERE id = $2")
        .bind(creator_share)
        .execute(&state.db)
        .await?;

    // Update model stats
    sqlx::query(
        "UPDATE models SET total_queries = total_queries + 1, total_revenue = total_revenue + $1 WHERE id = $2",
    )
    .bind(model.price_per_query)
    .bind(model.id)
    .execute(&state.db)
    .await?;

    // Build message history
    let history = sqlx::query_as::<_, ChatMessage>(
        "SELECT * FROM chat_messages WHERE session_id = $1 ORDER BY created_at ASC LIMIT 50",
    )
    .bind(session_id)
    .fetch_all(&state.db)
    .await?;

    let mut messages = vec![InferenceChatMessage {
        role: "system".into(),
        content: model.system_prompt.clone(),
    }];

    for msg in &history {
        messages.push(InferenceChatMessage {
            role: match msg.role {
                ChatRole::System => "system".into(),
                ChatRole::User => "user".into(),
                ChatRole::Assistant => "assistant".into(),
            },
            content: msg.content.clone(),
        });
    }

    // Stream inference response
    let (tx, rx) = mpsc::channel::<String>(32);
    let inference = InferenceService::new(&state.config, &state.http_client);
    let db = state.db.clone();

    // Spawn inference task
    tokio::spawn(async move {
        let mut full_response = String::new();

        // Collect full response for saving
        let (content_tx, mut content_rx) = mpsc::channel::<String>(32);

        let inference_handle = tokio::spawn(async move {
            inference
                .chat_stream(&provider_model_id, messages, content_tx)
                .await
        });

        while let Some(chunk) = content_rx.recv().await {
            full_response.push_str(&chunk);
            let _ = tx.send(chunk).await;
        }

        // Wait for inference to complete
        let _ = inference_handle.await;

        // Save assistant response
        if !full_response.is_empty() {
            let _ = sqlx::query(
                "INSERT INTO chat_messages (id, session_id, role, content) VALUES ($1, $2, $3, $4)",
            )
            .bind(Uuid::new_v4())
            .bind(session_id)
            .bind(ChatRole::Assistant)
            .bind(&full_response)
            .execute(&db)
            .await;
        }
    });

    // Convert to SSE stream
    let sid = session_id;
    let stream = ReceiverStream::new(rx).map(move |content| {
        Ok(Event::default()
            .event("message")
            .json_data(serde_json::json!({
                "session_id": sid,
                "content": content,
            }))
            .unwrap())
    });

    Ok(Sse::new(stream))
}
