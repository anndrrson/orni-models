use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, Method};
use axum::middleware;
use axum::routing::{delete, get, post, put};
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod auth;
mod config;
mod error;
mod routes;
mod schema;
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

    // Fail fast if JWT secret is the dev default in production
    if config.jwt_secret == "dev-secret-change-me" && config.bind_addr.contains("0.0.0.0") {
        tracing::warn!("JWT_SECRET is the dev default — set a strong secret in production!");
    }

    // Append search_path to DATABASE_URL so all connections use the orni schema
    let db_url = if config.database_url.contains('?') {
        format!(
            "{}&options=-csearch_path%3Dorni%2Cpublic",
            config.database_url
        )
    } else {
        format!(
            "{}?options=-csearch_path%3Dorni%2Cpublic",
            config.database_url
        )
    };

    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&db_url)
        .await?;

    tracing::info!("Connected to database (pool: max 4 connections)");

    crate::schema::ensure_schema(&db).await?;
    tracing::info!("Schema ready");

    // Build HTTP client with timeouts (prevents SSRF hangs)
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let state = Arc::new(AppState {
        db,
        config: config.clone(),
        http_client,
        nonce_store: Arc::new(NonceStore::new()),
        guest_rate_limits: Arc::new(Default::default()),
        auth_rate_limiter: Arc::new(state::AuthRateLimiter::new()),
    });

    // CORS — locked to allowed origins only
    let allowed_origins: Vec<_> = config
        .frontend_url
        .split(',')
        .chain(std::iter::once("https://ghola.xyz"))
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
        ])
        .allow_credentials(true);

    // Public routes
    let public = Router::new()
        .route("/auth/nonce", post(routes::auth::get_nonce))
        .route("/auth/verify", post(routes::auth::verify))
        .route("/models", get(routes::marketplace::browse))
        .route("/models/featured", get(routes::marketplace::get_featured))
        .route("/models/{slug}", get(routes::models::get_model))
        .route("/models/{slug}/reviews", get(routes::models::get_reviews))
        .route("/ai", get(routes::ai::get_ai_key))
        .route(
            "/creator/{did}/profile",
            get(routes::identity::get_creator_profile),
        )
        .route(
            "/creators/{slug}",
            get(routes::creators::get_creator_by_slug),
        )
        .route("/auth/register", post(routes::auth::register_email))
        .route("/auth/login", post(routes::auth::login_email))
        .route("/payments/webhook", post(routes::payments::stripe_webhook));

    // Protected routes
    let protected = Router::new()
        // Chat
        .route("/chat/{slug}/message", post(routes::chat::send_message))
        .route("/chat/{slug}/usage", get(routes::chat::get_usage))
        .route("/chat/sessions", get(routes::chat::list_sessions))
        .route(
            "/chat/sessions/{id}/messages",
            get(routes::chat::get_session_messages),
        )
        // Creator
        .route("/creator/stats", get(routes::creator::get_stats))
        .route("/creator/models", get(routes::creator::get_models))
        .route(
            "/creator/models/{id}",
            get(routes::creator::get_model_detail),
        )
        .route(
            "/creator/models/{id}/fine-tune",
            post(routes::creator::start_fine_tune),
        )
        .route(
            "/creator/models/{id}/publish",
            post(routes::creator::publish_model),
        )
        .route(
            "/creator/models/{id}/status",
            put(routes::creator::toggle_status),
        )
        .route("/creator/earnings", get(routes::creator::get_earnings))
        // Models
        .route("/models/create", post(routes::models::create_model))
        .route(
            "/models/quick-list",
            post(routes::models::quick_list_model),
        )
        .route("/models/id/{id}", put(routes::models::update_model))
        .route(
            "/models/id/{id}/content",
            post(routes::models::add_content),
        )
        .route(
            "/models/{slug}/review",
            post(routes::models::create_review),
        )
        // Payments
        .route("/balance", get(routes::payments::get_balance))
        .route("/deposits", post(routes::payments::submit_deposit))
        .route("/withdraw", post(routes::payments::request_withdraw))
        .route("/checkout", post(routes::payments::create_checkout))
        // Identity
        .route("/identity/link", post(routes::identity::link_did))
        // API Keys
        .route("/keys", post(routes::api_keys::create_api_key))
        .route("/keys", get(routes::api_keys::list_api_keys))
        .route("/keys/{id}", delete(routes::api_keys::revoke_api_key))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    // Root-level routes (not under /api)
    let root_routes = Router::new()
        .route("/agents.txt", get(routes::discovery::agents_txt))
        .route(
            "/.well-known/said.json",
            get(routes::discovery::well_known_said),
        )
        .route(
            "/v1/chat/completions",
            post(routes::openai_compat::chat_completions)
                .get(routes::openai_compat::x402_discovery),
        );

    let app = Router::new()
        .merge(root_routes)
        .nest("/api", public.merge(protected))
        .with_state(state)
        // 256KB max request body
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(cors)
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
