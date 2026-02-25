pub mod admin;
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

use axum::{http::HeaderMap, http::HeaderValue, response::Response};
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
