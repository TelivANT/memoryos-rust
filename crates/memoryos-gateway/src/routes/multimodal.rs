use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use memoryos_core::{AppError, MultiModalContent, MultiModalMessage};
use memoryos_ports::MultiModalStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct MultiModalState {
    pub storage: Arc<dyn MultiModalStorage>,
}

#[derive(Deserialize)]
pub struct StoreMultiModalRequest {
    pub user_id: String,
    pub role: String,
    pub contents: Vec<MultiModalContent>,
}

#[derive(Deserialize)]
pub struct SearchMultiModalRequest {
    pub user_id: String,
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct SearchByEmbeddingRequest {
    pub user_id: String,
    pub embedding: Vec<f32>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub modality: String,
}

#[derive(Deserialize)]
pub struct RecentMultiModalRequest {
    pub user_id: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

#[derive(Serialize)]
pub struct MultiModalResponse {
    pub status: String,
    pub count: usize,
    pub messages: Vec<MultiModalMessage>,
}

pub fn create_multimodal_routes(state: MultiModalState) -> Router {
    Router::new()
        .route("/store", post(store_message))
        .route("/search", post(search_by_text))
        .route("/search/embedding", post(search_by_embedding))
        .route("/recent", post(get_recent))
        .with_state(state)
}

async fn store_message(
    State(state): State<MultiModalState>,
    Json(req): Json<StoreMultiModalRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Storing multimodal message for user: {} ({} contents)",
        req.user_id,
        req.contents.len()
    );

    let message = MultiModalMessage {
        role: req.role,
        contents: req.contents,
        timestamp: chrono::Utc::now(),
    };

    state
        .storage
        .store_multimodal_message(&req.user_id, message)
        .await?;

    Ok(Json(serde_json::json!({
        "status": "ok",
    })))
}

async fn search_by_text(
    State(state): State<MultiModalState>,
    Json(req): Json<SearchMultiModalRequest>,
) -> Result<impl IntoResponse, AppError> {
    let messages = state
        .storage
        .search_by_text(&req.user_id, &req.query, req.limit)
        .await?;

    Ok(Json(MultiModalResponse {
        status: "ok".to_string(),
        count: messages.len(),
        messages,
    }))
}

async fn search_by_embedding(
    State(state): State<MultiModalState>,
    Json(req): Json<SearchByEmbeddingRequest>,
) -> Result<impl IntoResponse, AppError> {
    let messages = if req.modality == "audio" {
        state
            .storage
            .search_by_audio(&req.user_id, req.embedding, req.limit)
            .await?
    } else {
        state
            .storage
            .search_by_image(&req.user_id, req.embedding, req.limit)
            .await?
    };

    Ok(Json(MultiModalResponse {
        status: "ok".to_string(),
        count: messages.len(),
        messages,
    }))
}

async fn get_recent(
    State(state): State<MultiModalState>,
    Json(req): Json<RecentMultiModalRequest>,
) -> Result<impl IntoResponse, AppError> {
    let messages = state
        .storage
        .get_recent_multimodal(&req.user_id, req.limit)
        .await?;

    Ok(Json(MultiModalResponse {
        status: "ok".to_string(),
        count: messages.len(),
        messages,
    }))
}
