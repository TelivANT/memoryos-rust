use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::AdminState;
use memoryos_core::rbac::{Role, UserRecord};

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub tenant_id: String,
    pub display_name: String,
    pub email: String,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "user".to_string()
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize)]
pub struct AssignRoleRequest {
    pub role: String,
}

pub async fn list_users(State(state): State<AdminState>) -> Json<serde_json::Value> {
    let users = state.rbac_manager.list_users().await;
    Json(json!({ "users": users, "total": users.len() }))
}

pub async fn create_user(
    State(state): State<AdminState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let role = Role::parse_role(&req.role).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid role: {}", req.role)})),
        )
    })?;

    let tenant_exists = state
        .tenant_manager
        .get_tenant(&req.tenant_id)
        .await
        .is_some();
    if !tenant_exists {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Tenant '{}' not found", req.tenant_id)})),
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let user = UserRecord {
        user_id: uuid::Uuid::now_v7().to_string(),
        tenant_id: req.tenant_id,
        role,
        display_name: req.display_name,
        email: req.email,
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    };

    let user_clone = user.clone();
    state.rbac_manager.add_user(user).await;

    Ok((StatusCode::CREATED, Json(json!({ "user": user_clone }))))
}

pub async fn get_user(
    State(state): State<AdminState>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user = state.rbac_manager.get_user(&user_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        )
    })?;

    Ok(Json(json!({ "user": user })))
}

pub async fn update_user(
    State(state): State<AdminState>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let updated = state
        .rbac_manager
        .update_user(&user_id, req.display_name, req.email, req.is_active)
        .await;

    if updated {
        let user = state.rbac_manager.get_user(&user_id).await;
        Ok(Json(json!({ "user": user })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        ))
    }
}

pub async fn delete_user(
    State(state): State<AdminState>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if state.rbac_manager.remove_user(&user_id).await {
        Ok(Json(json!({"message": "User deleted"})))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        ))
    }
}

pub async fn assign_role(
    State(state): State<AdminState>,
    Path(user_id): Path<String>,
    Json(req): Json<AssignRoleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let role = Role::parse_role(&req.role).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid role: {}. Valid roles: super_admin, admin, user, read_only", req.role)})),
        )
    })?;

    if state.rbac_manager.assign_role(&user_id, role).await {
        let user = state.rbac_manager.get_user(&user_id).await;
        Ok(Json(
            json!({ "user": user, "message": format!("Role updated to {}", role) }),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "User not found"})),
        ))
    }
}
