//! FAQ 管理 API

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use memoryos_core::{
    tenant::TenantManager, AutoPromoter, FaqClassification, HeatTracker, LlmClassifierConfig,
    MemoryType, MidTermSegment, PromotionRecord,
};
use memoryos_ports::VectorStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use memoryos_core::AppError;

async fn extract_validated_tenant_id(
    headers: &HeaderMap,
    tenant_manager: &Option<TenantManager>,
) -> Result<Option<String>, AppError> {
    let raw = headers
        .get("X-Tenant-ID")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let tid = match raw {
        Some(t) => t,
        None => return Ok(None),
    };
    if let Some(mgr) = tenant_manager {
        if !mgr.is_tenant_enabled(&tid).await {
            warn!("Rejected FAQ request for unknown/disabled tenant: {}", tid);
            return Err(AppError::BadRequest(format!(
                "Tenant '{}' does not exist or is disabled",
                tid
            )));
        }
    }
    Ok(Some(tid))
}

/// FAQ 管理状态
#[derive(Clone)]
pub struct FaqState {
    pub heat_tracker: Arc<HeatTracker>,
    pub auto_promoter: Arc<AutoPromoter>,
    pub vector_store: Arc<dyn VectorStorage>,
    pub tenant_manager: Option<TenantManager>,
}

/// FAQ 候选响应
#[derive(Debug, Serialize)]
pub struct FaqCandidateResponse {
    pub candidates: Vec<FaqCandidate>,
    pub total: usize,
}

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
#[derive(Debug, Deserialize)]
pub struct PromoteRequest {
    pub memory_id: Uuid,
    pub reason: Option<String>,
}

/// 提升响应
#[derive(Debug, Serialize)]
pub struct PromoteResponse {
    pub success: bool,
    pub message: String,
    pub record: Option<PromotionRecord>,
}

fn segment_to_candidate(seg: &MidTermSegment) -> FaqCandidate {
    let type_str = match seg.memory_type {
        MemoryType::QA => "qa",
        MemoryType::FaqCandidate => "faq_candidate",
        MemoryType::Faq => "faq",
    };
    FaqCandidate {
        id: seg.id,
        user_id: seg.user_id.clone(),
        summary: seg.summary.clone(),
        access_count: seg.access_count,
        heat_score: seg.heat_score,
        memory_type: type_str.to_string(),
        created_at: seg.created_at.to_rfc3339(),
        last_accessed: seg.last_accessed.map(|dt| dt.to_rfc3339()),
    }
}

/// 创建 FAQ 路由
pub fn create_faq_routes(faq_state: FaqState) -> Router {
    Router::new()
        .route("/candidates", get(get_candidates))
        .route("/promote", post(promote_to_faq))
        .route("/classify", post(classify_faq))
        .route("/:id", delete(delete_faq))
        .route("/history", get(get_promotion_history))
        .route("/stats", get(get_stats))
        .with_state(faq_state)
}

/// GET /admin/faq/candidates - 获取候选 FAQ
async fn get_candidates(State(state): State<FaqState>, headers: HeaderMap) -> impl IntoResponse {
    let dummy_embedding = vec![0.0_f32; 1536];
    let tenant_id = extract_validated_tenant_id(&headers, &state.tenant_manager)
        .await
        .ok()
        .flatten();
    let segments = match if let Some(ref tid) = tenant_id {
        state
            .vector_store
            .search_segments_for_tenant("__global__", tid, dummy_embedding, 50)
            .await
    } else {
        state
            .vector_store
            .search_segments("__global__", dummy_embedding, 50)
            .await
    } {
        Ok(segs) => segs,
        Err(e) => {
            warn!("Failed to fetch FAQ candidates: {}", e);
            vec![]
        }
    };

    let candidates: Vec<FaqCandidate> = segments
        .iter()
        .filter(|s| s.memory_type == MemoryType::FaqCandidate || s.memory_type == MemoryType::Faq)
        .map(segment_to_candidate)
        .collect();

    Json(FaqCandidateResponse {
        total: candidates.len(),
        candidates,
    })
}

/// POST /admin/faq/promote - 手动提升为 FAQ
async fn promote_to_faq(
    State(state): State<FaqState>,
    headers: HeaderMap,
    Json(req): Json<PromoteRequest>,
) -> impl IntoResponse {
    info!("Promoting memory {} to FAQ", req.memory_id);

    let dummy_embedding = vec![0.0_f32; 1536];
    let tenant_id = extract_validated_tenant_id(&headers, &state.tenant_manager)
        .await
        .ok()
        .flatten();
    let segments = match if let Some(ref tid) = tenant_id {
        state
            .vector_store
            .search_segments_for_tenant("__global__", tid, dummy_embedding, 100)
            .await
    } else {
        state
            .vector_store
            .search_segments("__global__", dummy_embedding, 100)
            .await
    } {
        Ok(segs) => segs,
        Err(e) => {
            warn!("Failed to search segments for promotion: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PromoteResponse {
                    success: false,
                    message: format!("Failed to search segments: {}", e),
                    record: None,
                }),
            );
        }
    };

    let target = segments.iter().find(|s| s.id == req.memory_id);
    match target {
        Some(seg) => {
            let mut promoted = seg.clone();
            state.heat_tracker.promote_to_faq(&mut promoted);

            if let Err(e) = state.vector_store.store_segment(promoted).await {
                warn!("Failed to store promoted FAQ: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(PromoteResponse {
                        success: false,
                        message: format!("Failed to store promoted FAQ: {}", e),
                        record: None,
                    }),
                );
            }

            let record = PromotionRecord {
                id: Uuid::new_v4(),
                memory_id: req.memory_id,
                from_type: seg.memory_type.clone(),
                to_type: MemoryType::Faq,
                reason: req.reason.unwrap_or_else(|| "Manual promotion".to_string()),
                heat_score: seg.heat_score,
                access_count: seg.access_count,
                promoted_at: chrono::Utc::now(),
            };

            info!("Successfully promoted memory {} to FAQ", req.memory_id);
            (
                StatusCode::OK,
                Json(PromoteResponse {
                    success: true,
                    message: format!("Memory {} promoted to FAQ", req.memory_id),
                    record: Some(record),
                }),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(PromoteResponse {
                success: false,
                message: format!("Memory {} not found", req.memory_id),
                record: None,
            }),
        ),
    }
}

/// DELETE /admin/faq/:id - 删除 FAQ (demote back to QA)
async fn delete_faq(
    State(state): State<FaqState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    info!("Demoting FAQ {} back to QA", id);

    let dummy_embedding = vec![0.0_f32; 1536];
    let tenant_id = extract_validated_tenant_id(&headers, &state.tenant_manager)
        .await
        .ok()
        .flatten();
    let segments = match if let Some(ref tid) = tenant_id {
        state
            .vector_store
            .search_segments_for_tenant("__global__", tid, dummy_embedding, 100)
            .await
    } else {
        state
            .vector_store
            .search_segments("__global__", dummy_embedding, 100)
            .await
    } {
        Ok(segs) => segs,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "message": format!("Failed to search segments: {}", e)
                })),
            );
        }
    };

    let target = segments.iter().find(|s| s.id == id);
    match target {
        Some(seg) => {
            let mut demoted = seg.clone();
            demoted.memory_type = MemoryType::QA;

            if let Err(e) = state.vector_store.store_segment(demoted).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "message": format!("Failed to store demoted segment: {}", e)
                    })),
                );
            }

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "message": format!("FAQ {} demoted to QA", id)
                })),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "message": format!("FAQ {} not found", id)
            })),
        ),
    }
}

/// GET /admin/faq/history - 获取提升历史
async fn get_promotion_history(State(state): State<FaqState>) -> impl IntoResponse {
    let history = state.auto_promoter.get_history(50).await;
    Json(serde_json::json!({
        "history": history,
        "total": history.len()
    }))
}

/// GET /admin/faq/stats - 获取统计信息
async fn get_stats(State(state): State<FaqState>) -> impl IntoResponse {
    let stats = state.auto_promoter.get_stats().await;
    Json(stats)
}

/// POST /admin/faq/classify - LLM FAQ classification (offline / dry-run)
///
/// Accepts a question + answer pair and returns the classification result.
/// This does NOT call an actual LLM — it builds the prompt and returns it
/// together with the classifier config so callers can feed it to any LLM.
/// If a `response_text` field is provided (i.e. caller already obtained an
/// LLM response), it will be parsed into a structured classification.
#[derive(Debug, Deserialize)]
struct ClassifyRequest {
    question: String,
    answer: String,
    response_text: Option<String>,
}

#[derive(Debug, Serialize)]
struct ClassifyResponse {
    classification: Option<FaqClassification>,
    prompt: Option<Vec<memoryos_core::faq::PromptMessage>>,
    error: Option<String>,
}

async fn classify_faq(Json(req): Json<ClassifyRequest>) -> impl IntoResponse {
    let config = LlmClassifierConfig::default();

    if let Some(ref response_text) = req.response_text {
        match memoryos_core::faq::parse_classification_response(response_text, &config) {
            Ok(classification) => {
                info!(
                    question = %req.question,
                    category = %classification.category,
                    confidence = %classification.confidence,
                    "FAQ classified"
                );
                (
                    StatusCode::OK,
                    Json(ClassifyResponse {
                        classification: Some(classification),
                        prompt: None,
                        error: None,
                    }),
                )
            }
            Err(e) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ClassifyResponse {
                    classification: None,
                    prompt: None,
                    error: Some(e),
                }),
            ),
        }
    } else {
        let prompt =
            memoryos_core::faq::build_classification_prompt(&config, &req.question, &req.answer);
        (
            StatusCode::OK,
            Json(ClassifyResponse {
                classification: None,
                prompt: Some(prompt),
                error: None,
            }),
        )
    }
}
