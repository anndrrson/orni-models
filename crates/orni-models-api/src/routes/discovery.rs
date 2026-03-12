use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::http::{header, StatusCode};

use crate::error::AppResult;
use crate::state::AppState;

/// Minimal model info for agents.txt
#[derive(Debug, sqlx::FromRow)]
struct LiveModel {
    slug: String,
    name: String,
    description: Option<String>,
    category: Option<String>,
    price_per_query: i64,
    free_queries_per_day: i32,
}

/// GET /agents.txt — Dynamic agents.txt listing all live models
pub async fn agents_txt(
    State(state): State<Arc<AppState>>,
) -> AppResult<Response> {
    let models = sqlx::query_as::<_, LiveModel>(
        "SELECT slug, name, description, category, price_per_query, free_queries_per_day FROM models WHERE status = 'live' ORDER BY total_queries DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let base_url = &state.config.frontend_url;
    let platform_did = &state.config.platform_did;

    let mut txt = String::new();
    txt.push_str("# Orni Models - AI Model Marketplace\n");
    txt.push_str(&format!("# {}\n", base_url));
    txt.push_str(&format!("# DID: {}\n", platform_did));
    txt.push_str("#\n");
    txt.push_str("# Each model is an AI agent available via OpenAI-compatible API\n");
    txt.push_str("# Authentication: Bearer API key (obtain at /account)\n\n");

    for model in &models {
        txt.push_str(&format!("## {}\n", model.name));
        if let Some(ref desc) = model.description {
            txt.push_str(&format!("# {}\n", desc.replace('\n', " ")));
        }
        txt.push_str(&format!("Agent: {}\n", model.slug));
        txt.push_str(&format!("Endpoint: {}/v1/chat/completions\n", base_url.trim_end_matches('/')));
        txt.push_str(&format!("Model: {}\n", model.slug));
        txt.push_str("Protocol: openai\n");
        if let Some(ref cat) = model.category {
            txt.push_str(&format!("Category: {}\n", cat));
        }
        let price_usd = model.price_per_query as f64 / 1_000_000.0;
        txt.push_str(&format!("Price: ${:.4}/query\n", price_usd));
        if model.free_queries_per_day > 0 {
            txt.push_str(&format!("FreeTier: {}/day\n", model.free_queries_per_day));
        }
        txt.push('\n');
    }

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        txt,
    ).into_response())
}

/// GET /.well-known/said.json — Machine-readable service definitions
pub async fn well_known_said(
    State(state): State<Arc<AppState>>,
) -> AppResult<axum::Json<serde_json::Value>> {
    let models = sqlx::query_as::<_, LiveModel>(
        "SELECT slug, name, description, category, price_per_query, free_queries_per_day FROM models WHERE status = 'live' ORDER BY total_queries DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let base_url = &state.config.frontend_url;

    let services: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            let price_usd = m.price_per_query as f64 / 1_000_000.0;
            serde_json::json!({
                "id": m.slug,
                "name": m.name,
                "description": m.description,
                "category": m.category,
                "endpoint": format!("{}/v1/chat/completions", base_url.trim_end_matches('/')),
                "protocol": "openai",
                "model": m.slug,
                "pricing": {
                    "per_query_usd": price_usd,
                    "free_tier_per_day": m.free_queries_per_day,
                },
                "auth": {
                    "type": "bearer",
                    "description": "API key obtained from Orni Models account",
                }
            })
        })
        .collect();

    Ok(axum::Json(serde_json::json!({
        "did": state.config.platform_did,
        "name": "Orni Models",
        "description": "Creator AI model marketplace",
        "url": base_url,
        "services": services,
    })))
}
