use crate::state::AppState;
use axum::{
    extract::{Json, State},
    http::HeaderMap,
    response::IntoResponse,
};
use memoryos_core::{
    llm::{RouteDecision, RouteTier, RouterContext},
    security::ComplianceResult,
    AppError,
};
use memoryos_ports::llm::ChatChoice;
use memoryos_ports::{ChatMessage, ChatRequest, ChatResponse};
use tracing::{info, warn};

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<ChatRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = headers
        .get("X-User-ID")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("default_user");

    let last_msg = request
        .messages
        .last()
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let compliance = state.shield.check_compliance(&last_msg);
    match &compliance {
        ComplianceResult::Blocked(reason) => {
            warn!(
                user_id,
                reason = reason.as_str(),
                "Request blocked by Security Shield"
            );
            return Err(AppError::BadRequest(reason.clone()));
        }
        ComplianceResult::RequiresLocal => {
            info!(user_id, "Compliance enforced: Routing to Local LLM");
        }
        ComplianceResult::Safe => {}
    }

    if let Some(msg) = request.messages.last_mut() {
        msg.content = state.shield.sanitize_pii(&msg.content);
    }

    if let Some(answer) = state.faq_matcher.read().await.match_faq(&last_msg).await {
        return Ok(Json(ChatResponse {
            id: format!("faq-bloom-{}", uuid::Uuid::now_v7()),
            object: "chat.completion".to_string(),
            model: "memoryos-faq-cache".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: answer,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: None,
        }));
    }

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
                    Some((_pos, seg)) => {
                        let similarity = seg.score.unwrap_or(0.85);
                        (true, similarity, Some(seg.summary.clone()))
                    }
                    None => (false, 0.0, None),
                }
            }
            Err(e) => {
                warn!(user_id, error = %e, "FAQ lookup failed, proceeding without FAQ match");
                (false, 0.0, None)
            }
        };

    let has_sensitive = matches!(compliance, ComplianceResult::RequiresLocal);
    let router_ctx = RouterContext {
        query: last_msg.clone(),
        token_count: last_msg.len() / 4,
        global_similarity,
        is_faq_match,
        has_sensitive_keywords: has_sensitive,
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
            usage: None,
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

    if let Some(ref event_bus) = state.event_bus {
        let event_id = uuid::Uuid::now_v7().to_string();
        let payload = serde_json::json!({
            "user_id": user_id,
            "query": last_msg,
            "provider": target_provider,
            "tier": format!("{:?}", decision.tier),
        });
        let bus = event_bus.clone();
        tokio::spawn(async move {
            if let Err(e) = bus.publish_chat_log(&event_id, payload).await {
                tracing::warn!(error = %e, "Failed to publish chat event");
            }
        });
    }

    Ok(Json(response))
}
