//! Memory history routes

use axum::{
    extract::{Path, State},
    Json,
};
use memoryos_core::{AppError, MemoryHistoryEntry};

use crate::state::AppState;

/// GET /v1/memory/{memory_id}/history
pub async fn get_memory_history(
    Path(memory_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<MemoryHistoryEntry>>, AppError> {
    if let Some(history_storage) = &state.history_storage {
        let history = history_storage.get_history(&memory_id).await?;
        Ok(Json(history))
    } else {
        Err(AppError::NotFound("History storage not configured".into()))
    }
}
