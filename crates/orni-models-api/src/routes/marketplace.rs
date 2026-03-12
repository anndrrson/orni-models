use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;

use crate::error::AppResult;
use crate::state::AppState;
use orni_models_types::{MarketplaceQuery, MarketplaceResponse, ModelCard, ModelStatus};

pub async fn browse(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MarketplaceQuery>,
) -> AppResult<Json<MarketplaceResponse>> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    let sort_clause = match params.sort.as_deref() {
        Some("popular") => "m.total_queries DESC",
        Some("revenue") => "m.total_revenue DESC",
        Some("newest") => "m.created_at DESC",
        Some("price_low") => "m.price_per_query ASC",
        Some("price_high") => "m.price_per_query DESC",
        Some("top_rated") => "m.avg_rating DESC, m.review_count DESC",
        _ => "m.total_queries DESC",
    };

    // Build dynamic query
    let mut conditions = vec!["m.status = 'live'".to_string()];
    let mut bind_idx = 1;

    if let Some(ref search) = params.search {
        if !search.is_empty() {
            conditions.push(format!(
                "(m.name ILIKE ${bind_idx} OR m.description ILIKE ${bind_idx})"
            ));
            bind_idx += 1;
        }
    }

    if let Some(ref category) = params.category {
        if !category.is_empty() {
            conditions.push(format!("m.category = ${bind_idx}"));
            bind_idx += 1;
        }
    }

    let where_clause = conditions.join(" AND ");

    let query_str = format!(
        r#"
        SELECT
            m.id, m.slug, m.name, m.description, m.avatar_url,
            u.display_name as creator_name, u.wallet_address as creator_wallet,
            m.status, m.price_per_query, m.total_queries, m.category, m.tags
        FROM models m
        JOIN users u ON u.id = m.creator_id
        WHERE {where_clause}
        ORDER BY {sort_clause}
        LIMIT {limit} OFFSET {offset}
        "#
    );

    let count_str = format!(
        "SELECT COUNT(*) FROM models m WHERE {where_clause}"
    );

    // Use a simpler approach with optional filters
    let models = sqlx::query_as::<_, ModelCard>(
        r#"
        SELECT
            m.id, m.slug, m.name, m.description, m.avatar_url,
            u.display_name as creator_name, u.wallet_address as creator_wallet,
            m.status, m.price_per_query, m.total_queries, m.category, m.tags,
            u.did as creator_did, COALESCE(u.said_verified, false) as creator_verified,
            m.is_featured, m.free_queries_per_day
        FROM models m
        JOIN users u ON u.id = m.creator_id
        WHERE m.status = 'live'
            AND ($1::text IS NULL OR m.name ILIKE '%' || $1 || '%' OR m.description ILIKE '%' || $1 || '%')
            AND ($2::text IS NULL OR m.category = $2)
        ORDER BY m.total_queries DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(params.search.as_deref())
    .bind(params.category.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM models m
        WHERE m.status = 'live'
            AND ($1::text IS NULL OR m.name ILIKE '%' || $1 || '%' OR m.description ILIKE '%' || $1 || '%')
            AND ($2::text IS NULL OR m.category = $2)
        "#,
    )
    .bind(params.search.as_deref())
    .bind(params.category.as_deref())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(MarketplaceResponse {
        models,
        total,
        page,
        limit,
    }))
}

/// GET /api/models/featured — returns top 6 featured live models
pub async fn get_featured(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<ModelCard>>> {
    let models = sqlx::query_as::<_, ModelCard>(
        r#"
        SELECT
            m.id, m.slug, m.name, m.description, m.avatar_url,
            u.display_name as creator_name, u.wallet_address as creator_wallet,
            m.status, m.price_per_query, m.total_queries, m.category, m.tags,
            u.did as creator_did, COALESCE(u.said_verified, false) as creator_verified,
            m.is_featured, m.free_queries_per_day
        FROM models m
        JOIN users u ON u.id = m.creator_id
        WHERE m.status = 'live' AND m.is_featured = true
        ORDER BY m.total_queries DESC
        LIMIT 6
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(models))
}
