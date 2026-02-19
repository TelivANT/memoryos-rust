use axum::{
    extract::State,
    response::{IntoResponse, Json, Response},
};
use memoryos_core::{AppError, Message};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::routes::apply_degraded_header;
use crate::AppState;

#[derive(Deserialize)]
pub struct AddMessageRequest {
    pub user_id: String,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub event_id: Option<String>,
}

#[derive(Serialize)]
pub struct AddMessageResponse {
    pub status: String,
}

#[derive(Deserialize)]
pub struct RetrieveContextRequest {
    pub user_id: String,
    pub query: String,
}

/// POST /v1/memory/add
pub async fn add_message(
    State(state): State<AppState>,
    Json(request): Json<AddMessageRequest>,
) -> Result<Response, AppError> {
    info!("Adding message for user: {}", request.user_id);

    let event_id = request
        .event_id
        .clone()
        .unwrap_or_else(|| format!("evt-{}", uuid::Uuid::now_v7()));

    let timestamp = chrono::Utc::now();
    let message = Message {
        role: request.role.clone(),
        content: request.content.clone(),
        timestamp,
        embedding: None,
    };

    // 直接同步写入
    let manager = state.memory_manager.read().await;
    manager
        .add_message_with_event(&request.user_id, message, Some(&event_id))
        .await?;

    let mut response = Json(AddMessageResponse {
        status: "ok".to_string(),
    })
    .into_response();
    apply_degraded_header(&mut response, state.degraded_mode().await);
    Ok(response)
}

/// POST /v1/memory/retrieve
pub async fn retrieve_context(
    State(state): State<AppState>,
    Json(request): Json<RetrieveContextRequest>,
) -> Result<Response, AppError> {
    info!("Retrieving context for user: {}", request.user_id);

    let manager = state.memory_manager.read().await;
    let context = manager
        .retrieve_context(&request.user_id, &request.query)
        .await?;

    let value = serde_json::to_value(context)
        .map_err(|e| AppError::Internal(format!("Failed to serialize memory context: {}", e)))?;
    let mut response = Json(value).into_response();
    apply_degraded_header(&mut response, state.degraded_mode().await);
    Ok(response)
}

// TODO: Memory API 测试需要重构以适配新架构
#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::{body::Body, http::Request};
    use memoryos_adapters::NoopMemoryManager;
    use memoryos_core::{DependencyState, HealthMode, HealthStatus, MemoryContext};
    use memoryos_ports::{
        llm::ChatChoice, ChatDelta, ChatMessage, ChatRequest, ChatResponse, ChatStreamChoice,
        ChatStreamChunk, EventBus, LlmAdapter, MemoryManager,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Mutex;
    use tower::util::ServiceExt;

    use crate::{router::LlmRouter, routes, AppState};

    struct TestLlmAdapter;

    #[async_trait]
    impl LlmAdapter for TestLlmAdapter {
        async fn chat(
            &self,
            request: ChatRequest,
        ) -> Result<ChatResponse, memoryos_core::AppError> {
            Ok(ChatResponse {
                id: "resp_test".to_string(),
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
            })
        }

        async fn chat_stream(
            &self,
            request: ChatRequest,
        ) -> Result<Vec<ChatStreamChunk>, memoryos_core::AppError> {
            Ok(vec![ChatStreamChunk {
                id: "chunk_test".to_string(),
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
            "test"
        }
    }

    struct CountingMemoryManager {
        add_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl MemoryManager for CountingMemoryManager {
        async fn add_message(
            &self,
            _user_id: &str,
            _message: Message,
        ) -> Result<(), memoryos_core::AppError> {
            self.add_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn retrieve_context(
            &self,
            _user_id: &str,
            _query: &str,
        ) -> Result<MemoryContext, memoryos_core::AppError> {
            Ok(MemoryContext {
                short_term: vec![],
                mid_term: vec![],
                long_term: None,
            })
        }
    }

    struct ControlledEventBus {
        should_fail: bool,
        publish_count: Arc<AtomicUsize>,
        last_event_id: Arc<Mutex<Option<String>>>,
    }

    #[async_trait]
    impl EventBus for ControlledEventBus {
        async fn publish_chat_log(
            &self,
            event_id: &str,
            _payload: serde_json::Value,
        ) -> Result<(), memoryos_core::AppError> {
            self.publish_count.fetch_add(1, Ordering::SeqCst);
            *self.last_event_id.lock().await = Some(event_id.to_string());
            if self.should_fail {
                Err(memoryos_core::AppError::ExternalService(
                    "mock publish failure".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }

    fn degraded_state() -> Arc<AppState> {
        let adapter: Arc<dyn LlmAdapter> = Arc::new(TestLlmAdapter);
        let router = LlmRouter::new(adapter.clone(), adapter.clone(), adapter);
        let memory_manager: Arc<dyn MemoryManager> = Arc::new(NoopMemoryManager);
        Arc::new(AppState {
            router,
            memory_manager: Arc::new(tokio::sync::RwLock::new(memory_manager)),
            health_status: Arc::new(tokio::sync::RwLock::new(HealthStatus {
                mode: HealthMode::DegradedReady,
                redis: DependencyState::Bypassed,
                qdrant: DependencyState::Down,
                upstream: DependencyState::Up,
                auth_cache: DependencyState::Up,
            })),
            event_bus: None,
            async_memory_pipeline: false,
        })
    }

    fn async_state(
        async_pipeline: bool,
        event_bus: Option<Arc<dyn EventBus>>,
        memory_manager: Arc<dyn MemoryManager>,
    ) -> Arc<AppState> {
        let adapter: Arc<dyn LlmAdapter> = Arc::new(TestLlmAdapter);
        let router = LlmRouter::new(adapter.clone(), adapter.clone(), adapter);
        Arc::new(AppState {
            router,
            memory_manager: Arc::new(tokio::sync::RwLock::new(memory_manager)),
            health_status: Arc::new(tokio::sync::RwLock::new(HealthStatus {
                mode: HealthMode::Ready,
                redis: DependencyState::Up,
                qdrant: DependencyState::Up,
                upstream: DependencyState::Up,
                auth_cache: DependencyState::Up,
            })),
            event_bus,
            async_memory_pipeline: async_pipeline,
        })
    }

    #[tokio::test]
    async fn retrieve_context_includes_degraded_header() {
        let app = routes::memory_routes().with_state(degraded_state());
        let request = Request::builder()
            .method("POST")
            .uri("/memory/retrieve")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"user_id":"u1","query":"what did I say last time?"}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response.headers().get(routes::DEGRADED_HEADER).unwrap(),
            routes::DEGRADED_VALUE
        );
    }

    #[tokio::test]
    async fn add_message_async_publish_success_skips_sync_write() {
        let publish_count = Arc::new(AtomicUsize::new(0));
        let add_count = Arc::new(AtomicUsize::new(0));
        let event_bus: Arc<dyn EventBus> = Arc::new(ControlledEventBus {
            should_fail: false,
            publish_count: publish_count.clone(),
            last_event_id: Arc::new(Mutex::new(None)),
        });
        let memory_manager: Arc<dyn MemoryManager> = Arc::new(CountingMemoryManager {
            add_count: add_count.clone(),
        });

        let app =
            routes::memory_routes().with_state(async_state(true, Some(event_bus), memory_manager));
        let request = Request::builder()
            .method("POST")
            .uri("/memory/add")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"user_id":"u1","role":"user","content":"hello","event_id":"evt-test-1"}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(publish_count.load(Ordering::SeqCst), 1);
        assert_eq!(add_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn add_message_async_publish_failure_falls_back_sync_write() {
        let publish_count = Arc::new(AtomicUsize::new(0));
        let add_count = Arc::new(AtomicUsize::new(0));
        let event_bus: Arc<dyn EventBus> = Arc::new(ControlledEventBus {
            should_fail: true,
            publish_count: publish_count.clone(),
            last_event_id: Arc::new(Mutex::new(None)),
        });
        let memory_manager: Arc<dyn MemoryManager> = Arc::new(CountingMemoryManager {
            add_count: add_count.clone(),
        });

        let app =
            routes::memory_routes().with_state(async_state(true, Some(event_bus), memory_manager));
        let request = Request::builder()
            .method("POST")
            .uri("/memory/add")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"user_id":"u1","role":"user","content":"hello","event_id":"evt-test-2"}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(publish_count.load(Ordering::SeqCst), 1);
        assert_eq!(add_count.load(Ordering::SeqCst), 1);
    }
}
