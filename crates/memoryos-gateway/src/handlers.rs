use crate::state::AppState;
use axum::{
    extract::{Json, State},
    http::HeaderMap,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
};
use futures::stream::{self, StreamExt};
use memoryos_core::{
    llm::{RouteDecision, RouteTier, RouterContext},
    security::ComplianceResult,
    AppError,
};
use memoryos_ports::llm::ChatChoice;
use memoryos_ports::{ChatMessage, ChatRequest, ChatResponse};
use std::convert::Infallible;
use tracing::{info, warn};

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<ChatRequest>,
) -> Result<Response, AppError> {
    // Check if streaming is requested
    if request.stream {
        return Ok(chat_completions_stream(state, headers, request)
            .await?
            .into_response());
    }

    let user_id = headers
        .get("X-User-ID")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("default_user");

    // Basic validation: user_id must be alphanumeric + hyphens/underscores, max 128 chars
    if user_id.len() > 128
        || (!user_id.is_empty()
            && user_id != "default_user"
            && !user_id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.'))
    {
        return Err(AppError::BadRequest(
            "Invalid X-User-ID: must be alphanumeric with hyphens/underscores, max 128 chars"
                .to_string(),
        ));
    }

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
        })
        .into_response());
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
    // Estimate token count: CJK characters are ~1-2 tokens each (avg 1.5),
    // ASCII text is roughly len/4. We count CJK chars separately for accuracy.
    let cjk_chars = last_msg
        .chars()
        .filter(|c| {
            matches!(*c as u32,
                0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF |
                0x3000..=0x303F | 0x3040..=0x309F | 0x30A0..=0x30FF |
                0xAC00..=0xD7AF
            )
        })
        .count();
    let ascii_bytes = last_msg.len().saturating_sub(cjk_chars * 3); // CJK chars are ~3 bytes in UTF-8
    let estimated_tokens = (cjk_chars * 3 / 2) + (ascii_bytes / 4); // CJK: ~1.5 tok/char, ASCII: ~4 bytes/tok
    let router_ctx = RouterContext {
        query: last_msg.clone(),
        token_count: estimated_tokens.max(1),
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
        })
        .into_response());
    }

    // 4. Upstream Call
    let target_provider = match decision.tier {
        RouteTier::Local => state
            .config
            .router
            .local_backends
            .first()
            .map(|s| s.as_str())
            .unwrap_or(&state.config.llm.default_provider),
        RouteTier::Cloud => &state.config.llm.default_provider,
        _ => return Err(AppError::Internal("Invalid route tier".to_string())),
    };

    let adapter = state
        .get_adapter(target_provider)
        .ok_or_else(|| AppError::Config(format!("Provider '{}' not found", target_provider)))?;

    // Use retry with circuit breaker for LLM calls
    let retry_config = memoryos_core::retry::RetryConfig {
        max_retries: 2,
        initial_backoff_ms: 200,
        max_backoff_ms: 3000,
        backoff_multiplier: 2.0,
    };
    let adapter_ref = adapter.clone();
    let request_clone = request.clone();
    let breaker = state.circuit_breaker.clone();

    let response = match crate::middleware::circuit_breaker::with_circuit_breaker(
        &breaker,
        memoryos_core::retry::retry_with_backoff(&retry_config, "llm_chat", {
            let adapter_ref = adapter_ref.clone();
            let req = request_clone;
            move || {
                let a = adapter_ref.clone();
                let r = req.clone();
                async move { a.chat(r).await }
            }
        }),
    )
    .await
    {
        Some(result) => result?,
        None => {
            return Err(AppError::ServiceUnavailable(
                "LLM service circuit breaker is open — too many recent failures".to_string(),
            ));
        }
    };

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

    Ok(Json(response).into_response())
}

/// Streaming chat completions handler
async fn chat_completions_stream(
    state: AppState,
    headers: HeaderMap,
    mut request: ChatRequest,
) -> Result<impl IntoResponse, AppError> {
    let user_id = headers
        .get("X-User-ID")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("default_user")
        .to_string();

    let last_msg = request
        .messages
        .last()
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // Compliance check (same as non-streaming path)
    let compliance = state.shield.check_compliance(&last_msg);
    match &compliance {
        ComplianceResult::Blocked(reason) => {
            warn!(
                user_id = user_id.as_str(),
                reason = reason.as_str(),
                "Streaming request blocked by Security Shield"
            );
            return Err(AppError::BadRequest(reason.clone()));
        }
        ComplianceResult::RequiresLocal => {
            info!(
                user_id = user_id.as_str(),
                "Streaming: Compliance enforced, routing to Local LLM"
            );
        }
        ComplianceResult::Safe => {}
    }

    // PII sanitization (same as non-streaming path)
    if let Some(msg) = request.messages.last_mut() {
        msg.content = state.shield.sanitize_pii(&msg.content);
    }

    // Route decision (same logic as non-streaming path)
    let has_sensitive = matches!(compliance, ComplianceResult::RequiresLocal);
    let target_provider = if has_sensitive {
        // Sensitive content must go to local backend
        state
            .config
            .router
            .local_backends
            .first()
            .cloned()
            .unwrap_or_else(|| state.config.llm.default_provider.clone())
    } else {
        state.config.llm.default_provider.clone()
    };

    let adapter = state
        .get_adapter(&target_provider)
        .ok_or_else(|| AppError::Config(format!("Provider '{}' not found", target_provider)))?;

    // Call streaming API
    let chunks = adapter.chat_stream(request).await?;

    // Convert to SSE stream
    let stream = stream::iter(chunks).map(move |chunk| {
        let data = serde_json::to_string(&chunk).unwrap_or_default();
        Ok::<_, Infallible>(Event::default().data(data))
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
