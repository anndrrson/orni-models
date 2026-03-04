use std::sync::Arc;

use sqlx::PgPool;

use crate::auth::NonceStore;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub http_client: reqwest::Client,
    pub nonce_store: Arc<NonceStore>,
}
