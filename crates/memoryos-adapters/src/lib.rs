pub mod eventbus;
pub mod history;
pub mod llm;
pub mod memory;
pub mod wiki;

use async_trait::async_trait;
use memoryos_core::{AppError, DependencyState, HealthMode, HealthStatus};
use memoryos_ports::{HealthProbe, NormalizedRequest, NormalizedResponse, UpstreamClient};

pub use eventbus::RedisStreamEventBus;
pub use history::QdrantHistoryStorage;
pub use llm::{
    AzureOpenAiAdapter, ClaudeAdapter, CohereAdapter, DeepSeekAdapter, GeminiAdapter, GroqAdapter,
    MistralAdapter, OllamaAdapter, OpenAiAdapter, OpenRouterAdapter,
};
pub use memory::{
    ChromaStorage, DefaultMemoryManager, DegradedMemoryManager, NoopMemoryManager, PineconeStorage,
    QdrantStorage, RedisStorage,
};
pub use wiki::OpenDALAdapter;

#[derive(Debug, Default)]
pub struct StubUpstreamClient;

#[async_trait]
impl UpstreamClient for StubUpstreamClient {
    async fn send_request(
        &self,
        request: NormalizedRequest,
    ) -> Result<NormalizedResponse, AppError> {
        let body = serde_json::json!({
            "id": format!("chatcmpl-{}", request.request_id),
            "object": "chat.completion",
            "model": request.model,
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "stub response"},
                "finish_reason": "stop"
            }]
        });

        Ok(NormalizedResponse {
            status_code: 200,
            body,
            adapter_warnings: Vec::new(),
        })
    }

    async fn stream_response(
        &self,
        request: NormalizedRequest,
    ) -> Result<Vec<NormalizedResponse>, AppError> {
        let one = self.send_request(request).await?;
        Ok(vec![one])
    }
}

#[derive(Debug, Clone)]
pub struct StaticHealthProbe {
    pub status: HealthStatus,
}

impl Default for StaticHealthProbe {
    fn default() -> Self {
        Self {
            status: HealthStatus {
                mode: HealthMode::Ready,
                redis: DependencyState::Up,
                qdrant: DependencyState::Up,
                upstream: DependencyState::Up,
                auth_cache: DependencyState::Up,
            },
        }
    }
}

#[async_trait]
impl HealthProbe for StaticHealthProbe {
    async fn current_status(&self) -> Result<HealthStatus, AppError> {
        Ok(self.status.clone())
    }
}
