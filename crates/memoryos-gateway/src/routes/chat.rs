use axum::{
    extract::State,
    response::{IntoResponse, Json, Response},
};
use memoryos_core::{llm::RouterContext, AppError};
use memoryos_ports::ChatRequest;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::routes::apply_degraded_header;
use crate::AppState;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// POST /v1/chat/completions
/// 支持透传模式：保留所有未知字段
#[allow(dead_code)]
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

    let query = request
        .messages
        .last()
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // FAQ Tier 0: Search for FAQ matches before routing
    let memory_mgr = state.memory_manager.read().await.clone();
    let (is_faq_match, global_similarity, faq_answer) =
        match memory_mgr.retrieve_context("default_user", &query).await {
            Ok(ctx) => {
                let faq_match = ctx
                    .mid_term
                    .iter()
                    .enumerate()
                    .find(|(_, seg)| seg.memory_type == memoryos_core::MemoryType::Faq);
                match faq_match {
                    Some((_pos, seg)) => {
                        let similarity = seg.score.unwrap_or(seg.heat_score);
                        (true, similarity, Some(seg.summary.clone()))
                    }
                    None => (false, 0.0, None),
                }
            }
            Err(e) => {
                warn!(error = %e, "FAQ lookup failed, proceeding without FAQ match");
                (false, 0.0, None)
            }
        };

    let decision = state
        .router
        .route(&RouterContext {
            query: query.clone(),
            token_count: query.len() / 4,
            global_similarity,
            is_faq_match,
            has_sensitive_keywords: false,
            faq_answer,
        })
        .await?;

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
