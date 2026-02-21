pub mod admin;
pub mod chat;
pub mod defense;
pub mod faq;
pub mod graph;
pub mod health;
pub mod history;
pub mod memory;
pub mod memory_manage;
pub mod metrics;
pub mod multimodal;
pub mod security;
pub mod wiki;
pub mod wiki_connector;

use axum::{
    http::HeaderMap, http::HeaderValue, response::Response, routing::get, routing::post, Router,
};
use memoryos_core::{tenant::TenantManager, AppError};
use tracing::warn;

pub async fn extract_validated_tenant_id(
    headers: &HeaderMap,
    tenant_manager: &Option<TenantManager>,
) -> Result<Option<String>, AppError> {
    let raw = headers
        .get("X-Tenant-ID")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let tid = match raw {
        Some(t) => t,
        None => return Ok(None),
    };
    if let Some(mgr) = tenant_manager {
        if !mgr.is_tenant_enabled(&tid).await {
            warn!("Rejected request for unknown/disabled tenant: {}", tid);
            return Err(AppError::BadRequest(format!(
                "Tenant '{}' does not exist or is disabled",
                tid
            )));
        }
    }
    Ok(Some(tid))
}

pub const DEGRADED_HEADER: &str = "X-MemoryOS-Status";
pub const DEGRADED_VALUE: &str = "degraded";

pub fn apply_degraded_header(response: &mut Response, degraded_mode: bool) {
    if degraded_mode {
        response
            .headers_mut()
            .insert(DEGRADED_HEADER, HeaderValue::from_static(DEGRADED_VALUE));
    }
}

#[allow(dead_code)]
pub fn health_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/health", get(health::health))
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        .route("/health/status", get(health::status))
        .route("/metrics", get(metrics::metrics_handler))
}

#[allow(dead_code)]
pub fn chat_routes() -> Router<crate::AppState> {
    Router::new().route("/chat/completions", post(chat::chat_completions))
}

#[allow(dead_code)]
pub fn memory_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/memory/add", post(memory::add_message))
        .route("/memory/retrieve", post(memory::retrieve_context))
}
