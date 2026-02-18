use axum::{
    extract::State,
    response::{IntoResponse, Json, Response},
};
use memoryos_core::{AppError, llm::RouterContext};
use memoryos_ports::ChatRequest;
use std::sync::Arc;
use tracing::info;
use serde::{Serialize, Deserialize};

use crate::routes::apply_degraded_header;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// POST /v1/chat/completions
/// 支持透传模式：保留所有未知字段
pub async fn chat_completions(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Result<Response, AppError> {
    info!(
        "Received chat request: model={}, messages={}, stream={}",
        request.model,
        request.messages.len(),
        request.stream
    );

    // 简化：构造路由上下文
    let query = request.messages.last()
        .map(|m| m.content.clone())
        .unwrap_or_default();
    
    let decision = state.router.route(&RouterContext {
        query: query.clone(),
        token_count: query.len() / 4, // 粗略估算
        global_similarity: 0.0,
        is_faq_match: false,
        has_sensitive_keywords: false,
    }).await?;

    let response = ChatCompletionResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::now_v7()),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: decision.model.clone(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: decision.direct_response.unwrap_or_else(|| "OK".to_string()),
            },
            finish_reason: Some("stop".to_string()),
        }],
    };

    let mut http_response: Response = Json(response).into_response();
    apply_degraded_header(&mut http_response, state.degraded_mode().await);
    Ok(http_response)
}
