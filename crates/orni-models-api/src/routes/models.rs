use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use orni_models_types::{
    AddContentRequest, ContentSource, CreateModelRequest, Model, UpdateModelRequest,
};

pub async fn create_model(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Json(req): Json<CreateModelRequest>,
) -> AppResult<Json<Model>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    // Verify slug is valid
    if req.slug.is_empty() || req.slug.len() > 64 || !req.slug.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(AppError::BadRequest("Invalid slug: use lowercase alphanumeric and hyphens".into()));
    }

    let base_model = req.base_model.unwrap_or_else(|| state.config.default_base_model.clone());
    let price = req.price_per_query.unwrap_or(100_000); // $0.10 default
    let tags = req.tags.unwrap_or_default();

    // Mark user as creator
    sqlx::query("UPDATE users SET is_creator = true WHERE id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    let model = sqlx::query_as::<_, Model>(
        r#"
        INSERT INTO models (id, creator_id, slug, name, description, system_prompt, base_model, price_per_query, category, tags)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&req.slug.to_lowercase())
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.system_prompt)
    .bind(&base_model)
    .bind(price)
    .bind(&req.category)
    .bind(&tags)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("models_slug_key") {
                return AppError::Conflict("Slug already taken".into());
            }
        }
        AppError::from(e)
    })?;

    Ok(Json(model))
}

pub async fn get_model(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> AppResult<Json<Model>> {
    let model = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE slug = $1",
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Model not found".into()))?;

    Ok(Json(model))
}

pub async fn update_model(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateModelRequest>,
) -> AppResult<Json<Model>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    // Verify ownership
    let existing = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Model not found".into()))?;

    if existing.creator_id != user_id {
        return Err(AppError::Unauthorized("Not your model".into()));
    }

    let model = sqlx::query_as::<_, Model>(
        r#"
        UPDATE models SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            system_prompt = COALESCE($4, system_prompt),
            price_per_query = COALESCE($5, price_per_query),
            category = COALESCE($6, category),
            tags = COALESCE($7, tags),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.system_prompt)
    .bind(req.price_per_query)
    .bind(&req.category)
    .bind(&req.tags)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(model))
}

pub async fn add_content(
    State(state): State<Arc<AppState>>,
    claims: axum::Extension<Claims>,
    Path(model_id): Path<Uuid>,
    Json(req): Json<AddContentRequest>,
) -> AppResult<Json<ContentSource>> {
    let user_id: Uuid = claims.sub.parse().map_err(|_| AppError::Unauthorized("Invalid token".into()))?;

    // Verify ownership
    let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(model_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Model not found".into()))?;

    if model.creator_id != user_id {
        return Err(AppError::Unauthorized("Not your model".into()));
    }

    let source = sqlx::query_as::<_, ContentSource>(
        r#"
        INSERT INTO content_sources (id, model_id, source_type, source_url, content_text)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(model_id)
    .bind(&req.source_type)
    .bind(&req.source_url)
    .bind(&req.content_text)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(source))
}
