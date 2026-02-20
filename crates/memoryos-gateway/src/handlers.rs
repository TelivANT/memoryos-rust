use crate::state::AppState;
use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use memoryos_core::{
    llm::{RouteDecision, RouteTier, RouterContext},
    security::ComplianceResult,
    AppError,
};
use memoryos_ports::llm::ChatChoice;
use memoryos_ports::{ChatMessage, ChatRequest, ChatResponse};
use serde_json::json;
use tracing::{info, warn};

pub async fn chat_completions(
    State(state): State<AppState>,
    _headers: HeaderMap,
    Json(mut request): Json<ChatRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = "default_user"; // TODO: Extract from Auth Header

    // 1. Security Shield: Input Validation & PII
    let last_msg = request
        .messages
        .last()
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // Check Compliance
    match state.shield.check_compliance(&last_msg) {
        ComplianceResult::Blocked(reason) => {
            warn!(user_id, reason, "Request blocked by Security Shield");
            return Err(AppError::BadRequest(reason));
        }
        ComplianceResult::RequiresLocal => {
            info!(user_id, "Compliance enforced: Routing to Local LLM");
            // Force router decision later or mark context
        }
        ComplianceResult::Safe => {}
    }

    // Sanitize PII (Modify request in place)
    if let Some(msg) = request.messages.last_mut() {
        msg.content = state.shield.sanitize_pii(&msg.content);
    }

    // 2. FAQ Tier 0: Search for FAQ matches before routing
    let memory_mgr = state.memory_manager.read().await.clone();
    let (is_faq_match, global_similarity, faq_answer) =
        match memory_mgr.retrieve_context(user_id, &last_msg).await {
            Ok(ctx) => {
                let faq_match = ctx
                    .mid_term
                    .iter()
                    .enumerate()
                    .find(|(_, seg)| seg.memory_type == memoryos_core::MemoryType::Faq);
                match faq_match {
                    Some((pos, seg)) => {
                        let estimated_similarity = match pos {
                            0 => 0.98_f32,
                            1 => 0.93,
                            _ => 0.80,
                        };
                        (true, estimated_similarity, Some(seg.summary.clone()))
                    }
                    None => (false, 0.0, None),
                }
            }
            Err(e) => {
                warn!(user_id, error = %e, "FAQ lookup failed, proceeding without FAQ match");
                (false, 0.0, None)
            }
        };

    let router_ctx = RouterContext {
        query: last_msg.clone(),
        token_count: last_msg.len() / 4,
        global_similarity,
        is_faq_match,
        has_sensitive_keywords: matches!(
            state.shield.check_compliance(&last_msg),
            ComplianceResult::RequiresLocal
        ),
        faq_answer,
    };

    let decision = state.router.route(&router_ctx).await?;
    info!(user_id, decision = ?decision, "Router decision made");

    // 3. Handle Direct Hit (Tier 0)
    if let RouteDecision {
        tier: RouteTier::DirectHit,
        direct_response: Some(content),
        ..
    } = &decision
    {
        return Ok(Json(ChatResponse {
            id: format!("direct-{}", uuid::Uuid::now_v7()),
            object: "chat.completion".to_string(),
            model: "memoryos-direct".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: content.clone(),
                },
                finish_reason: "stop".to_string(),
            }],
        }));
    }

    // 4. Upstream Call
    let target_provider = match decision.tier {
        RouteTier::Local => "local", // TODO: Should come from config
        RouteTier::Cloud => &state.config.llm.default_provider,
        _ => return Err(AppError::Internal("Invalid route tier".to_string())),
    };

    let adapter = state
        .get_adapter(target_provider)
        .ok_or_else(|| AppError::Config(format!("Provider '{}' not found", target_provider)))?;

    let response = adapter.chat(request).await?;

    // 5. Async: Emit Event (TODO)
    // state.event_bus.emit(ChatLog { ... });

    Ok(Json(response))
}

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub async fn health_status(State(state): State<AppState>) -> impl IntoResponse {
    let worker = state.current_worker_monitor().await;
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "async_memory_pipeline": worker.async_memory_enabled,
            "worker_stream": worker.stream_key,
            "worker_group": worker.group,
            "worker_consumers": worker.worker_consumers,
            "worker_last_check_at": worker.last_check_at,
            "worker_last_error": worker.last_error,
        })),
    )
}
