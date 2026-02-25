//! IP 防御中间件
//!
//! Global rate-limiting middleware using IpDefenseSystem.
//! Requires `ConnectInfo<SocketAddr>` which needs `into_make_service_with_connect_info()`.
//! Automatically enabled when IpDefenseSystem is configured in AppState.

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use memoryos_core::security::defense::{AttackType, IpDefenseSystem};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;

/// IP defense middleware — wired when IpDefenseSystem is enabled.
/// Defense management API is available at /v1/admin/defense/*.
///
/// Supports reverse proxy deployments: checks X-Forwarded-For and X-Real-IP
/// headers before falling back to ConnectInfo (direct connection IP).
pub async fn ip_defense_middleware(
    State(defense): State<Arc<IpDefenseSystem>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Prefer X-Real-IP > first entry of X-Forwarded-For > ConnectInfo
    let ip = request
        .headers()
        .get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
        .or_else(|| {
            request
                .headers()
                .get("X-Forwarded-For")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
        })
        .unwrap_or_else(|| addr.ip());

    // 检查是否被封禁并限流
    match defense.check_rate_limit(ip, AttackType::RateLimit).await {
        Ok(()) => Ok(next.run(request).await),
        Err(e) => Err((
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(json!({
                "error": {
                    "code": "too_many_requests",
                    "message": format!("Request blocked: {}", e)
                }
            })),
        )
            .into_response()),
    }
}
