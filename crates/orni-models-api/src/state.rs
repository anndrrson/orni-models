use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::auth::NonceStore;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub http_client: reqwest::Client,
    pub nonce_store: Arc<NonceStore>,
    pub guest_rate_limits: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
}
