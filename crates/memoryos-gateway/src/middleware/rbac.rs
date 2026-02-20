use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use memoryos_core::rbac::Permission;
use serde_json::json;
use std::sync::Arc;

use crate::AppState;

fn extract_token(headers: &HeaderMap) -> &str {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if let Some(stripped) = auth_header.strip_prefix("Bearer ") {
        stripped
    } else {
        auth_header
    }
}

fn extract_tenant_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-Tenant-ID")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

pub async fn rbac_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    if !state.config.auth.enabled {
        return Ok(next.run(request).await);
    }

    let rbac = match &state.rbac_manager {
        Some(rbac) => rbac,
        None => return Ok(next.run(request).await),
    };

    let token = extract_token(&headers);
    if token.is_empty() {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path().to_string();
    let method = request.method().clone();

    let required_permission = match (method.as_str(), path.as_str()) {
        ("GET", p) if p.starts_with("/v1/memory") => Some(Permission::ReadMemory),
        ("POST", "/v1/memory/retrieve") => Some(Permission::ReadMemory),
        ("POST", "/v1/memory/add") => Some(Permission::WriteMemory),
        ("POST", "/v1/chat/completions") => Some(Permission::ReadMemory),
        ("GET", p) if p.starts_with("/v1/graph") => Some(Permission::ReadMemory),
        ("POST", p) if p.starts_with("/v1/graph") => Some(Permission::WriteMemory),
        ("GET", p) if p.starts_with("/v1/security/audit") => Some(Permission::ViewAudit),
        ("GET", p) if p.starts_with("/v1/admin") => Some(Permission::ManageUsers),
        ("POST", p) if p.starts_with("/v1/admin") => Some(Permission::ManageUsers),
        ("DELETE", p) if p.starts_with("/v1/admin") => Some(Permission::ManageUsers),
        _ => None,
    };

    if let Some(permission) = required_permission {
        let is_admin_key = state.config.auth.admin_keys.contains(&token.to_string());
        if is_admin_key {
            return Ok(next.run(request).await);
        }

        let has_permission = rbac.check_permission(token, permission).await;
        if !has_permission {
            let user_exists = rbac.get_user(token).await.is_some();
            if user_exists {
                return Err((
                    StatusCode::FORBIDDEN,
                    axum::Json(json!({
                        "error": {
                            "code": "forbidden",
                            "message": format!("Permission '{}' required", permission)
                        }
                    })),
                )
                    .into_response());
            }
            return Err((
                StatusCode::FORBIDDEN,
                axum::Json(json!({
                    "error": {
                        "code": "forbidden",
                        "message": "Unknown user. Register via admin service first."
                    }
                })),
            )
                .into_response());
        }
    }

    if let Some(tenant_id) = extract_tenant_id(&headers) {
        if let Some(tenant_mgr) = &state.tenant_manager {
            if !tenant_mgr.is_tenant_enabled(&tenant_id).await {
                let tenant_exists = tenant_mgr.get_tenant(&tenant_id).await.is_some();
                if tenant_exists {
                    return Err((
                        StatusCode::FORBIDDEN,
                        axum::Json(json!({
                            "error": {
                                "code": "tenant_disabled",
                                "message": "Tenant is disabled"
                            }
                        })),
                    )
                        .into_response());
                }
            }
        }
    }

    Ok(next.run(request).await)
}
