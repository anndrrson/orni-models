use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::services::inference::InferenceService;
use crate::state::AppState;
use orni_models_types::{
    ContentSource, CreatorModelDetail, CreatorStats, FineTuneJob, Model, ModelStatus,
};

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
) -> AppResult<Json<CreatorStats>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    let stats = sqlx::query_as::<_, CreatorStats>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM models WHERE creator_id = $1) as total_models,
            (SELECT COALESCE(SUM(total_queries), 0) FROM models WHERE creator_id = $1) as total_queries,
            (SELECT COALESCE(SUM(total_revenue), 0) FROM models WHERE creator_id = $1) as total_revenue,
            (SELECT COALESCE(SUM(creator_share), 0) FROM payments p
             JOIN models m ON m.id = p.model_id WHERE m.creator_id = $1) as pending_earnings
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(stats))
}

pub async fn get_models(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
) -> AppResult<Json<Vec<Model>>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    let models = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE creator_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(models))
}

pub async fn get_model_detail(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CreatorModelDetail>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    let model = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE id = $1 AND creator_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Model not found".into()))?;

    let content_sources = sqlx::query_as::<_, ContentSource>(
        "SELECT * FROM content_sources WHERE model_id = $1 ORDER BY created_at DESC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let fine_tune_jobs = sqlx::query_as::<_, FineTuneJob>(
        "SELECT * FROM fine_tune_jobs WHERE model_id = $1 ORDER BY created_at DESC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let recent_queries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chat_messages cm JOIN chat_sessions cs ON cs.id = cm.session_id WHERE cs.model_id = $1 AND cm.created_at > NOW() - INTERVAL '7 days' AND cm.role = 'user'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let recent_revenue: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM payments WHERE model_id = $1 AND created_at > NOW() - INTERVAL '7 days'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(CreatorModelDetail {
        model,
        content_sources,
        fine_tune_jobs,
        recent_queries,
        recent_revenue,
    }))
}

pub async fn start_fine_tune(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Path(model_id): Path<Uuid>,
) -> AppResult<Json<FineTuneJob>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    let model = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE id = $1 AND creator_id = $2",
    )
    .bind(model_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Model not found".into()))?;

    // Check for existing training dataset
    let dataset = sqlx::query_scalar::<_, String>(
        "SELECT file_key FROM training_datasets WHERE model_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(model_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::BadRequest("No training dataset available. Add content first.".into()))?;

    // Submit to Together.ai
    let inference = InferenceService::new(&state.config, &state.http_client);
    let suffix = format!("orni-{}", model.slug);
    let job_id = inference
        .create_fine_tune(&model.base_model, &dataset, &suffix)
        .await?;

    // Save job
    let job = sqlx::query_as::<_, FineTuneJob>(
        r#"
        INSERT INTO fine_tune_jobs (id, model_id, provider_job_id, status)
        VALUES ($1, $2, $3, 'running')
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(model_id)
    .bind(&job_id)
    .fetch_one(&state.db)
    .await?;

    // Update model status
    sqlx::query("UPDATE models SET status = 'training', updated_at = NOW() WHERE id = $1")
        .bind(model_id)
        .execute(&state.db)
        .await?;

    Ok(Json(job))
}

pub async fn publish_model(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Path(model_id): Path<Uuid>,
) -> AppResult<Json<Model>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    let model = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE id = $1 AND creator_id = $2",
    )
    .bind(model_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Model not found".into()))?;

    // Must have a provider model ID (completed fine-tune)
    if model.provider_model_id.is_none() {
        return Err(AppError::BadRequest("Model training not complete".into()));
    }

    let updated = sqlx::query_as::<_, Model>(
        "UPDATE models SET status = 'live', updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(model_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated))
}
