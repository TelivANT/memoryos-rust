//! 防御系统管理 API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use memoryos_core::security::defense::IpDefenseSystem;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct DefenseStats {
    pub total_bans: usize,
    pub temp_bans: usize,
    pub permanent_bans: usize,
}

#[derive(Debug, Deserialize)]
pub struct WhitelistRequest {
    pub ip: String,
}

pub fn create_defense_routes(defense: Arc<IpDefenseSystem>) -> Router {
    Router::new()
        .route("/stats", get(get_stats))
        .route("/whitelist", post(add_whitelist))
        .route("/unban/:ip", delete(unban_ip))
        .with_state(defense)
}

async fn get_stats(State(_defense): State<Arc<IpDefenseSystem>>) -> impl IntoResponse {
    // TODO: 实现统计
    Json(DefenseStats {
        total_bans: 0,
        temp_bans: 0,
        permanent_bans: 0,
    })
}

async fn add_whitelist(
    State(defense): State<Arc<IpDefenseSystem>>,
    Json(req): Json<WhitelistRequest>,
) -> impl IntoResponse {
    let ip: std::net::IpAddr = match req.ip.parse() {
        Ok(ip) => ip,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid IP address"})),
            )
        }
    };

    match defense.add_whitelist(ip).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"success": true}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

async fn unban_ip(
    State(_defense): State<Arc<IpDefenseSystem>>,
    Path(_ip): Path<String>,
) -> impl IntoResponse {
    // TODO: 实现解封
    (StatusCode::OK, Json(serde_json::json!({"success": true})))
}
