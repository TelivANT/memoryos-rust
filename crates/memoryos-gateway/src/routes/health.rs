use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::routes::apply_degraded_header;
use crate::AppState;

/// Legacy health endpoint required by Phase 2 acceptance.
pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

/// Liveness probe - returns 200 if process is running
pub async fn liveness() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

/// Readiness probe - returns 200 if service can handle requests
pub async fn readiness(State(state): State<AppState>) -> Response {
    let health = state.current_health().await;
    let status = match health.mode.as_str() {
        "ready" | "degraded_ready" => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };

    let mut response = (
        status,
        Json(json!({
            "status": health.mode,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
        .into_response();
    apply_degraded_header(&mut response, state.degraded_mode().await);
    response
}

/// Detailed dependency status endpoint.
pub async fn status(State(state): State<AppState>) -> Response {
    let health = state.current_health().await;
    let mut response = (
        StatusCode::OK,
        Json(json!({
            "mode": health.mode,
            "redis": health.redis,
            "qdrant": health.qdrant,
            "upstream": health.upstream,
            "auth_cache": health.auth_cache,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
        .into_response();
    apply_degraded_header(&mut response, state.degraded_mode().await);
    response
}

// TODO: Gateway 测试需要完整的集成环境，暂时禁用
// 使用 integration-tests feature 启用
#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod tests {
    #[test]
    fn placeholder() {
        // 占位测试
    }
}
