use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use memoryos_core::{AuditLogger, GdprManager};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct SecurityState {
    pub audit_logger: Arc<AuditLogger>,
    pub gdpr_manager: Arc<RwLock<GdprManager>>,
}

#[derive(Deserialize)]
pub struct AuditQueryRequest {
    pub user_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct ConsentRequest {
    pub user_id: String,
    pub purpose: String,
    pub granted: bool,
}

#[derive(Deserialize)]
pub struct GdprUserRequest {
    pub user_id: String,
}

fn default_limit() -> usize {
    50
}

pub fn create_security_routes(state: SecurityState) -> Router {
    Router::new()
        .route("/audit/logs", post(get_audit_logs))
        .route("/audit/stats", get(get_audit_stats))
        .route("/gdpr/export", post(gdpr_export))
        .route("/gdpr/delete", post(gdpr_delete_request))
        .route("/gdpr/consent", post(record_consent))
        .route("/gdpr/consent/check", post(check_consent))
        .with_state(state)
}

async fn get_audit_logs(
    State(state): State<SecurityState>,
    Json(req): Json<AuditQueryRequest>,
) -> impl IntoResponse {
    let events = if let Some(user_id) = &req.user_id {
        state.audit_logger.get_by_user(user_id, req.limit)
    } else {
        state.audit_logger.get_recent(req.limit)
    };

    Json(serde_json::json!({
        "status": "ok",
        "count": events.len(),
        "events": events,
    }))
}

async fn get_audit_stats(State(state): State<SecurityState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "total_events": state.audit_logger.event_count(),
    }))
}

async fn gdpr_export(
    State(state): State<SecurityState>,
    Json(req): Json<GdprUserRequest>,
) -> impl IntoResponse {
    let gdpr = state.gdpr_manager.read().await;
    let export = gdpr.prepare_data_export(&req.user_id);

    state
        .audit_logger
        .log_data_access(&req.user_id, "gdpr", "data_export");

    Json(serde_json::json!({
        "status": "ok",
        "export": export,
    }))
}

async fn gdpr_delete_request(
    State(state): State<SecurityState>,
    Json(req): Json<GdprUserRequest>,
) -> impl IntoResponse {
    let mut gdpr = state.gdpr_manager.write().await;
    match gdpr.request_deletion(&req.user_id) {
        Ok(deletion_req) => {
            state
                .audit_logger
                .log_data_deletion(&req.user_id, "all_data");

            Json(serde_json::json!({
                "status": "ok",
                "deletion_request": deletion_req,
            }))
        }
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e.to_string(),
        })),
    }
}

async fn record_consent(
    State(state): State<SecurityState>,
    Json(req): Json<ConsentRequest>,
) -> impl IntoResponse {
    let mut gdpr = state.gdpr_manager.write().await;
    gdpr.record_consent(&req.user_id, &req.purpose, req.granted);

    state.audit_logger.log_data_modification(
        &req.user_id,
        "consent",
        if req.granted { "grant" } else { "revoke" },
    );

    Json(serde_json::json!({
        "status": "ok",
        "message": "Consent recorded",
    }))
}

async fn check_consent(
    State(state): State<SecurityState>,
    Json(req): Json<ConsentRequest>,
) -> impl IntoResponse {
    let gdpr = state.gdpr_manager.read().await;
    let has_consent = gdpr.has_consent(&req.user_id, &req.purpose);

    Json(serde_json::json!({
        "status": "ok",
        "user_id": req.user_id,
        "purpose": req.purpose,
        "has_consent": has_consent,
    }))
}
