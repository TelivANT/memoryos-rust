//! FAQ 管理 API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use memoryos_core::{AutoPromoter, HeatTracker, PromotionRecord};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// FAQ 管理状态
#[allow(dead_code)]
#[derive(Clone)]
pub struct FaqState {
    pub heat_tracker: Arc<HeatTracker>,
    pub auto_promoter: Arc<AutoPromoter>,
}

/// FAQ 候选响应
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct FaqCandidateResponse {
    pub candidates: Vec<FaqCandidate>,
    pub total: usize,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct FaqCandidate {
    pub id: Uuid,
    pub user_id: String,
    pub summary: String,
    pub access_count: u32,
    pub heat_score: f32,
    pub memory_type: String,
    pub created_at: String,
    pub last_accessed: Option<String>,
}

/// 手动提升请求
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PromoteRequest {
    pub memory_id: Uuid,
    pub reason: Option<String>,
}

/// 提升响应
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct PromoteResponse {
    pub success: bool,
    pub message: String,
    pub record: Option<PromotionRecord>,
}

/// 创建 FAQ 路由
#[allow(dead_code)]
pub fn create_faq_routes(faq_state: FaqState) -> Router {
    Router::new()
        .route("/candidates", get(get_candidates))
        .route("/promote", post(promote_to_faq))
        .route("/:id", delete(delete_faq))
        .route("/history", get(get_promotion_history))
        .route("/stats", get(get_stats))
        .with_state(faq_state)
}

/// GET /admin/faq/candidates - 获取候选 FAQ
#[allow(dead_code)]
async fn get_candidates(State(_state): State<FaqState>) -> impl IntoResponse {
    // TODO: 从 Qdrant 获取实际数据
    let candidates = vec![];

    Json(FaqCandidateResponse {
        total: candidates.len(),
        candidates,
    })
}

/// POST /admin/faq/promote - 手动提升为 FAQ
#[allow(dead_code)]
async fn promote_to_faq(
    State(_state): State<FaqState>,
    Json(req): Json<PromoteRequest>,
) -> impl IntoResponse {
    // TODO: 实现手动提升逻辑
    Json(PromoteResponse {
        success: true,
        message: format!("Memory {} promoted to FAQ", req.memory_id),
        record: None,
    })
}

/// DELETE /admin/faq/:id - 删除 FAQ
#[allow(dead_code)]
async fn delete_faq(State(_state): State<FaqState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    // TODO: 实现删除逻辑
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": format!("FAQ {} deleted", id)
        })),
    )
}

/// GET /admin/faq/history - 获取提升历史
#[allow(dead_code)]
async fn get_promotion_history(State(state): State<FaqState>) -> impl IntoResponse {
    let history = state.auto_promoter.get_history(50).await;
    Json(serde_json::json!({
        "history": history,
        "total": history.len()
    }))
}

/// GET /admin/faq/stats - 获取统计信息
#[allow(dead_code)]
async fn get_stats(State(state): State<FaqState>) -> impl IntoResponse {
    let stats = state.auto_promoter.get_stats().await;
    Json(stats)
}
