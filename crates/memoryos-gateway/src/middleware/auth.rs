use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;

use crate::AppState;

pub async fn admin_only(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    if !state.config.auth.enabled {
        return Ok(next.run(request).await);
    }

    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else {
        auth_header
    };

    let is_admin = state.config.auth.admin_keys.contains(&token.to_string());

    if is_admin {
        Ok(next.run(request).await)
    } else {
        Err((
            StatusCode::FORBIDDEN,
            axum::Json(json!({
                "error": {
                    "code": "forbidden",
                    "message": "Admin access required"
                }
            })),
        )
            .into_response())
    }
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // 如果未启用认证，直接通过
    if !state.config.auth.enabled {
        return Ok(next.run(request).await);
    }

    // 检查 Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // 支持两种格式：
    // 1. Bearer <token>
    // 2. <token>
    let token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else {
        auth_header
    };

    // 验证 token
    let is_valid = if let Some(store) = &state.api_key_store {
        // 使用 Redis 存储验证
        store.validate_key(token).await.unwrap_or(false)
    } else {
        // 使用静态配置验证
        state.config.auth.api_keys.contains(&token.to_string())
    };

    if is_valid {
        Ok(next.run(request).await)
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({
                "error": {
                    "code": "unauthorized",
                    "message": "Invalid or missing API key"
                }
            })),
        )
            .into_response())
    }
}
