use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;
use subtle::ConstantTimeEq;

use crate::AppState;

pub(crate) fn extract_bearer_token(headers: &HeaderMap) -> &str {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .strip_prefix("Bearer ")
        .unwrap_or("")
}

pub(crate) fn constant_time_contains(haystack: &[String], needle: &str) -> bool {
    let needle_bytes = needle.as_bytes();
    let mut found = false;
    for candidate in haystack {
        if candidate.len() == needle.len() && bool::from(candidate.as_bytes().ct_eq(needle_bytes)) {
            found = true;
        }
    }
    found
}

#[allow(dead_code)]
pub async fn admin_only(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    if !state.config.auth.enabled {
        return Ok(next.run(request).await);
    }

    let token = extract_bearer_token(&headers);

    if constant_time_contains(&state.config.auth.admin_keys, token) {
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
    if !state.config.auth.enabled {
        return Ok(next.run(request).await);
    }

    let token = extract_bearer_token(&headers);

    let is_valid = if let Some(store) = &state.api_key_store {
        store.validate_key(token).await.unwrap_or(false)
    } else {
        constant_time_contains(&state.config.auth.api_keys, token)
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
