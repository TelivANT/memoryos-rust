use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::AdminState;

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit: Option<usize>,
    pub event_type: Option<String>,
    pub user_id: Option<String>,
}

pub async fn list_audit_logs(
    State(state): State<AdminState>,
    Query(query): Query<AuditQuery>,
) -> Json<serde_json::Value> {
    let all_events = state.audit_logger.get_recent(query.limit.unwrap_or(100));

    let filtered: Vec<_> = all_events
        .into_iter()
        .filter(|e| {
            if let Some(ref et) = query.event_type {
                if format!("{:?}", e.event_type) != *et {
                    return false;
                }
            }
            if let Some(ref uid) = query.user_id {
                if e.user_id.as_deref() != Some(uid.as_str()) {
                    return false;
                }
            }
            true
        })
        .collect();

    let total = filtered.len();
    Json(json!({ "logs": filtered, "total": total }))
}
