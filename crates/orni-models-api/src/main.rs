use std::sync::Arc;

use axum::middleware;
use axum::routing::{get, post, put};
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod auth;
mod config;
mod error;
mod routes;
mod services;
mod state;

use auth::NonceStore;
use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("orni_models_api=debug,tower_http=debug")
        }))
        .init();

    let config = Config::from_env();

    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await?;

    tracing::info!("Connected to database");

    sqlx::migrate!("../../migrations").run(&db).await?;
    tracing::info!("Migrations applied");

    let state = Arc::new(AppState {
        db,
        config: config.clone(),
        http_client: reqwest::Client::new(),
        nonce_store: Arc::new(NonceStore::new()),
    });

    // Public routes
    let public = Router::new()
        .route("/auth/nonce", post(routes::auth::get_nonce))
        .route("/auth/verify", post(routes::auth::verify))
        .route("/models", get(routes::marketplace::browse))
        .route("/models/{slug}", get(routes::models::get_model));

    // Protected routes
    let protected = Router::new()
        .route("/chat/{slug}/message", post(routes::chat::send_message))
        .route("/creator/stats", get(routes::creator::get_stats))
        .route("/creator/models", get(routes::creator::get_models))
        .route("/creator/models/{id}", get(routes::creator::get_model_detail))
        .route("/creator/models/{id}/fine-tune", post(routes::creator::start_fine_tune))
        .route("/creator/models/{id}/publish", post(routes::creator::publish_model))
        .route("/models/create", post(routes::models::create_model))
        .route("/models/id/{id}", put(routes::models::update_model))
        .route("/models/id/{id}/content", post(routes::models::add_content))
        .route("/balance", get(routes::payments::get_balance))
        .route("/deposits", post(routes::payments::submit_deposit))
        .route("/withdraw", post(routes::payments::request_withdraw))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    let app = Router::new()
        .nest("/api", public.merge(protected))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let bind_addr = config.bind_addr.parse::<std::net::SocketAddr>()?;
    tracing::info!("Starting server on {bind_addr}");

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl+c");
    tracing::info!("Shutdown signal received");
}
