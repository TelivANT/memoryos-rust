pub mod admin;
pub mod chat;
pub mod health;
pub mod history;
pub mod memory;
pub mod metrics;

use axum::{http::HeaderValue, response::Response, routing::get, routing::post, Router};

pub const DEGRADED_HEADER: &str = "X-MemoryOS-Status";
pub const DEGRADED_VALUE: &str = "degraded";

pub fn apply_degraded_header(response: &mut Response, degraded_mode: bool) {
    if degraded_mode {
        response
            .headers_mut()
            .insert(DEGRADED_HEADER, HeaderValue::from_static(DEGRADED_VALUE));
    }
}

pub fn health_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/health", get(health::health))
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        .route("/health/status", get(health::status))
        .route("/metrics", get(metrics::metrics_handler))
}

pub fn chat_routes() -> Router<crate::AppState> {
    Router::new().route("/chat/completions", post(chat::chat_completions))
}

pub fn memory_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/memory/add", post(memory::add_message))
        .route("/memory/retrieve", post(memory::retrieve_context))
}
