use axum::{extract::State, response::Json};
use serde_json::json;

use crate::AdminState;

pub async fn system_stats(State(state): State<AdminState>) -> Json<serde_json::Value> {
    let tenant_count = state.tenant_manager.tenant_count().await;
    let user_count = state.rbac_manager.user_count().await;
    let tenants = state.tenant_manager.list_tenants().await;

    let mut tenant_stats = Vec::new();
    for tenant in &tenants {
        let users = state.rbac_manager.user_count_by_tenant(&tenant.id).await;
        tenant_stats.push(json!({
            "tenant_id": tenant.id,
            "tenant_name": tenant.name,
            "enabled": tenant.enabled,
            "user_count": users,
            "max_users": tenant.max_users,
            "storage_quota_mb": tenant.storage_quota_mb,
            "api_rate_limit": tenant.api_rate_limit,
        }));
    }

    Json(json!({
        "system": {
            "version": "0.12.0",
            "service": "memoryos-admin",
            "uptime": "running",
        },
        "totals": {
            "tenants": tenant_count,
            "users": user_count,
        },
        "tenant_details": tenant_stats,
    }))
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "memoryos-admin",
        "version": "0.12.0",
    }))
}
