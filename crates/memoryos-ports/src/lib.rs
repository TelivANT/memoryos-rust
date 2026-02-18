pub mod history;
pub mod llm;
pub mod memory;
pub mod wiki;

use async_trait::async_trait;
use memoryos_core::{AppError, HealthStatus};
use serde::{Deserialize, Serialize};

pub use history::HistoryStorage;
pub use llm::{
    ChatDelta, ChatMessage, ChatRequest, ChatResponse, ChatStreamChoice, ChatStreamChunk,
    LlmAdapter,
};
pub use memory::{ConcurrencyControl, MemoryManager, ShortTermStorage, VectorStorage};
pub use wiki::{WikiAdapter, WikiDocument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedRequest {
    pub model: String,
    pub body: serde_json::Value,
    pub stream: bool,
    pub request_id: String,
    pub trace_id: String,
    pub event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedResponse {
    pub status_code: u16,
    pub body: serde_json::Value,
    pub adapter_warnings: Vec<String>,
}

#[async_trait]
pub trait UpstreamClient: Send + Sync {
    async fn send_request(
        &self,
        request: NormalizedRequest,
    ) -> Result<NormalizedResponse, AppError>;
    async fn stream_response(
        &self,
        request: NormalizedRequest,
    ) -> Result<Vec<NormalizedResponse>, AppError>;
}

#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish_chat_log(
        &self,
        event_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn current_status(&self) -> Result<HealthStatus, AppError>;
}
