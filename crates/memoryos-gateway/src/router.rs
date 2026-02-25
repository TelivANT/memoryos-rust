//! LLM Router - 3-tier 路由逻辑
//!
//! 根据请求的复杂度路由到不同的模型层级：
//! - Tier 1: 简单任务（小模型，低成本）
//! - Tier 2: 中等任务（中等模型）
//! - Tier 3: 复杂任务（大模型，高成本）

use memoryos_core::AppError;
use memoryos_ports::{ChatRequest, ChatResponse, ChatStreamChunk, LlmAdapter};
use std::sync::Arc;
use tracing::info;

pub struct LlmRouter {
    tier1: Arc<dyn LlmAdapter>,
    tier2: Arc<dyn LlmAdapter>,
    tier3: Arc<dyn LlmAdapter>,
}

impl LlmRouter {
    pub fn new(
        tier1: Arc<dyn LlmAdapter>,
        tier2: Arc<dyn LlmAdapter>,
        tier3: Arc<dyn LlmAdapter>,
    ) -> Self {
        Self { tier1, tier2, tier3 }
    }

    /// 路由请求到合适的 tier
    pub async fn route(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        let tier = self.classify_tier(&request);
        
        info!(
            "Routing request to tier {}, adapter: {}",
            tier,
            self.get_adapter(tier).name()
        );

        self.get_adapter(tier).chat(request).await
    }

    /// 路由流式请求到合适的 tier
    pub async fn route_stream(&self, request: ChatRequest) -> Result<Vec<ChatStreamChunk>, AppError> {
        let tier = self.classify_tier(&request);
        
        info!(
            "Routing stream request to tier {}, adapter: {}",
            tier,
            self.get_adapter(tier).name()
        );

        self.get_adapter(tier).chat_stream(request).await
    }

    /// 分类请求到对应的 tier（简单启发式规则）
    fn classify_tier(&self, request: &ChatRequest) -> u8 {
        let total_tokens = request
            .messages
            .iter()
            .map(|m| m.content.len())
            .sum::<usize>();

        // 简单规则：根据输入长度分类
        if total_tokens < 500 {
            1 // 简单任务
        } else if total_tokens < 2000 {
            2 // 中等任务
        } else {
            3 // 复杂任务
        }
    }

    fn get_adapter(&self, tier: u8) -> &Arc<dyn LlmAdapter> {
        match tier {
            1 => &self.tier1,
            2 => &self.tier2,
            _ => &self.tier3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use memoryos_ports::{ChatDelta, ChatMessage, ChatResponse, ChatStreamChoice};
    use memoryos_ports::llm::ChatChoice;
    use tokio::sync::Mutex;

    struct CaptureAdapter {
        seen_models: Arc<Mutex<Vec<String>>>,
    }

    impl CaptureAdapter {
        fn new(seen_models: Arc<Mutex<Vec<String>>>) -> Self {
            Self { seen_models }
        }
    }

    #[async_trait]
    impl LlmAdapter for CaptureAdapter {
        async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
            self.seen_models.lock().await.push(request.model.clone());
            Ok(ChatResponse {
                id: "resp_1".to_string(),
                object: "chat.completion".to_string(),
                model: request.model,
                choices: vec![ChatChoice {
                    index: 0,
                    message: ChatMessage {
                        role: "assistant".to_string(),
                        content: "ok".to_string(),
                    },
                    finish_reason: "stop".to_string(),
                }],
                usage: None,
            })
        }

        async fn chat_stream(&self, request: ChatRequest) -> Result<Vec<ChatStreamChunk>, AppError> {
            self.seen_models.lock().await.push(request.model.clone());
            Ok(vec![ChatStreamChunk {
                id: "chunk_1".to_string(),
                object: "chat.completion.chunk".to_string(),
                model: request.model,
                choices: vec![ChatStreamChoice {
                    index: 0,
                    delta: ChatDelta {
                        role: Some("assistant".to_string()),
                        content: Some("ok".to_string()),
                    },
                    finish_reason: Some("stop".to_string()),
                }],
            }])
        }

        fn name(&self) -> &str {
            "capture"
        }
    }

    fn sample_request(model: &str, stream: bool) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }],
            temperature: None,
            max_tokens: None,
            stream,
            extra: Default::default(),
        }
    }

    #[tokio::test]
    async fn route_keeps_request_model() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let adapter: Arc<dyn LlmAdapter> = Arc::new(CaptureAdapter::new(seen.clone()));
        let router = LlmRouter::new(adapter.clone(), adapter.clone(), adapter);

        let req = sample_request("gpt-oss:20b", false);
        let _ = router.route(req).await.unwrap();

        let models = seen.lock().await;
        assert_eq!(models[0], "gpt-oss:20b");
    }

    #[tokio::test]
    async fn route_stream_keeps_request_model() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let adapter: Arc<dyn LlmAdapter> = Arc::new(CaptureAdapter::new(seen.clone()));
        let router = LlmRouter::new(adapter.clone(), adapter.clone(), adapter);

        let req = sample_request("llama3.2:3b", true);
        let _ = router.route_stream(req).await.unwrap();

        let models = seen.lock().await;
        assert_eq!(models[0], "llama3.2:3b");
    }
}
