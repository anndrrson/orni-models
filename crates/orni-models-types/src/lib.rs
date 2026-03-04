use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Database Models ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub wallet_address: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_creator: bool,
    pub usdc_balance: i64, // in micro-USDC (6 decimals)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "model_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Draft,
    Training,
    Live,
    Paused,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Model {
    pub id: Uuid,
    pub creator_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub system_prompt: String,
    pub base_model: String,
    pub provider_model_id: Option<String>,
    pub status: ModelStatus,
    pub price_per_query: i64, // micro-USDC
    pub total_queries: i64,
    pub total_revenue: i64,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "source_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Text,
    Pdf,
    Youtube,
    Blog,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "content_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ContentStatus {
    Pending,
    Processing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContentSource {
    pub id: Uuid,
    pub model_id: Uuid,
    pub source_type: SourceType,
    pub source_url: Option<String>,
    pub content_text: Option<String>,
    pub status: ContentStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrainingDataset {
    pub id: Uuid,
    pub model_id: Uuid,
    pub file_key: String,
    pub num_examples: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "fine_tune_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FineTuneStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FineTuneJob {
    pub id: Uuid,
    pub model_id: Uuid,
    pub provider_job_id: Option<String>,
    pub status: FineTuneStatus,
    pub result_model_id: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub model_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "chat_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: ChatRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub model_id: Uuid,
    pub amount: i64,
    pub creator_share: i64,
    pub platform_share: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Deposit {
    pub id: Uuid,
    pub user_id: Uuid,
    pub amount: i64,
    pub tx_signature: String,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
}

// ── API DTOs ──

#[derive(Debug, Deserialize)]
pub struct NonceRequest {
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub wallet_address: String,
    pub signature: String,
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub base_model: Option<String>,
    pub price_per_query: Option<i64>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub price_per_query: Option<i64>,
    pub status: Option<ModelStatus>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ModelCard {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub creator_name: Option<String>,
    pub creator_wallet: String,
    pub status: ModelStatus,
    pub price_per_query: i64,
    pub total_queries: i64,
    pub category: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct ChatStartResponse {
    pub session_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct AddContentRequest {
    pub source_type: SourceType,
    pub source_url: Option<String>,
    pub content_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    pub tx_signature: String,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub balance: i64,
    pub pending_earnings: i64,
}

#[derive(Debug, Deserialize)]
pub struct WithdrawRequest {
    pub amount: i64,
    pub destination_wallet: String,
}

#[derive(Debug, Deserialize)]
pub struct MarketplaceQuery {
    pub search: Option<String>,
    pub category: Option<String>,
    pub sort: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MarketplaceResponse {
    pub models: Vec<ModelCard>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

// ── Inference Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct InferenceRequest {
    pub model: String,
    pub messages: Vec<InferenceChatMessage>,
    pub stream: bool,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct InferenceChoice {
    pub delta: Option<InferenceDelta>,
    pub message: Option<InferenceDelta>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InferenceDelta {
    pub role: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InferenceChunk {
    pub choices: Vec<InferenceChoice>,
}

#[derive(Debug, Serialize)]
pub struct FineTuneRequest {
    pub training_file: String,
    pub model: String,
    pub suffix: String,
    pub n_epochs: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct FineTuneResponse {
    pub id: String,
    pub status: String,
    pub fine_tuned_model: Option<String>,
}

// ── Creator Dashboard Types ──

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CreatorStats {
    pub total_models: i64,
    pub total_queries: i64,
    pub total_revenue: i64,
    pub pending_earnings: i64,
}

#[derive(Debug, Serialize)]
pub struct CreatorModelDetail {
    pub model: Model,
    pub content_sources: Vec<ContentSource>,
    pub fine_tune_jobs: Vec<FineTuneJob>,
    pub recent_queries: i64,
    pub recent_revenue: i64,
}
