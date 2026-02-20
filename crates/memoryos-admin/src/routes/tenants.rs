use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::AdminState;
use memoryos_core::tenant::Tenant;

#[derive(Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_max_users")]
    pub max_users: u32,
    #[serde(default = "default_storage_quota")]
    pub storage_quota_mb: u64,
    #[serde(default = "default_rate_limit")]
    pub api_rate_limit: u32,
}

fn default_max_users() -> u32 {
    100
}
fn default_storage_quota() -> u64 {
    1024
}
fn default_rate_limit() -> u32 {
    1000
}

#[derive(Deserialize)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub max_users: Option<u32>,
    pub storage_quota_mb: Option<u64>,
    pub api_rate_limit: Option<u32>,
    pub enabled: Option<bool>,
}

pub async fn list_tenants(State(state): State<AdminState>) -> Json<serde_json::Value> {
    let tenants = state.tenant_manager.list_tenants().await;
    Json(json!({ "tenants": tenants, "total": tenants.len() }))
}

pub async fn create_tenant(
    State(state): State<AdminState>,
    Json(req): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let now = chrono::Utc::now().to_rfc3339();
    let tenant = Tenant {
        id: uuid::Uuid::now_v7().to_string(),
        name: req.name,
        description: req.description,
        max_users: req.max_users,
        storage_quota_mb: req.storage_quota_mb,
        api_rate_limit: req.api_rate_limit,
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };

    let tenant_clone = tenant.clone();
    state
        .tenant_manager
        .create_tenant(tenant)
        .await
        .map_err(|e| (StatusCode::CONFLICT, Json(json!({"error": e}))))?;

    Ok((StatusCode::CREATED, Json(json!({ "tenant": tenant_clone }))))
}

pub async fn get_tenant(
    State(state): State<AdminState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tenant = state
        .tenant_manager
        .get_tenant(&tenant_id)
        .await
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Tenant not found"})),
            )
        })?;

    Ok(Json(json!({ "tenant": tenant })))
}

pub async fn update_tenant(
    State(state): State<AdminState>,
    Path(tenant_id): Path<String>,
    Json(req): Json<UpdateTenantRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let updated = state
        .tenant_manager
        .update_tenant(
            &tenant_id,
            req.name,
            req.description,
            req.max_users,
            req.storage_quota_mb,
            req.api_rate_limit,
            req.enabled,
        )
        .await;

    if updated {
        let tenant = state.tenant_manager.get_tenant(&tenant_id).await;
        Ok(Json(json!({ "tenant": tenant })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Tenant not found"})),
        ))
    }
}

pub async fn delete_tenant(
    State(state): State<AdminState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if state.tenant_manager.delete_tenant(&tenant_id).await {
        Ok(Json(json!({"message": "Tenant deleted"})))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Tenant not found"})),
        ))
    }
}
