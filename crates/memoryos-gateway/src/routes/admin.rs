use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{auth::ApiKeyMetadata, AppState};

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub api_key: String,
    pub user_id: String,
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub api_key: String,
    pub message: String,
}

/// POST /admin/keys - 创建新的 API Key（需要管理员权限）
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, (StatusCode, Json<serde_json::Value>)> {
    let store = state.api_key_store.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "API Key store not configured"})),
        )
    })?;

    let metadata = ApiKeyMetadata {
        user_id: req.user_id,
        description: req.description,
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: None,
        permissions: req.permissions,
        is_active: true,
    };

    store
        .create_key(&req.api_key, metadata)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    Ok(Json(CreateKeyResponse {
        api_key: req.api_key,
        message: "API Key created successfully".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct DeleteKeyRequest {
    pub api_key: String,
}

/// DELETE /admin/keys - 删除 API Key
pub async fn delete_api_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteKeyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let store = state.api_key_store.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "API Key store not configured"})),
        )
    })?;

    store.delete_key(&req.api_key).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(json!({"message": "API Key deleted successfully"})))
}
