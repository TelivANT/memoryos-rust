use axum::{extract::State, http::HeaderMap, response::IntoResponse, routing::post, Json, Router};
use memoryos_core::{AppError, MemoryType, MidTermSegment};
use memoryos_ports::VectorStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

fn extract_tenant_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-Tenant-ID")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

#[derive(Clone)]
pub struct MemoryManageState {
    pub vector_store: Arc<dyn VectorStorage>,
}

#[derive(Deserialize)]
pub struct TagRequest {
    pub user_id: String,
    pub segment_id: String,
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
pub struct SearchByTagRequest {
    pub user_id: String,
    pub tags: Vec<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct ExportRequest {
    pub user_id: String,
    #[serde(default)]
    pub format: ExportFormat,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[default]
    Json,
    Markdown,
}

#[derive(Deserialize)]
pub struct ImportRequest {
    pub user_id: String,
    pub segments: Vec<ImportSegment>,
}

#[derive(Deserialize)]
pub struct ImportSegment {
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub memory_type: Option<String>,
}

#[derive(Deserialize)]
pub struct VersionHistoryRequest {
    pub user_id: String,
    pub segment_id: String,
}

fn default_limit() -> usize {
    50
}

#[derive(Serialize)]
pub struct MemoryExport {
    pub user_id: String,
    pub exported_at: String,
    pub segment_count: usize,
    pub segments: Vec<ExportedSegment>,
}

#[derive(Serialize)]
pub struct ExportedSegment {
    pub id: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub memory_type: String,
    pub version: u32,
    pub created_at: String,
    pub heat_score: f32,
}

pub fn create_memory_manage_routes(state: MemoryManageState) -> Router {
    Router::new()
        .route("/tags", post(add_tags))
        .route("/search/tags", post(search_by_tags))
        .route("/export", post(export_memories))
        .route("/import", post(import_memories))
        .route("/versions", post(get_version_history))
        .with_state(state)
}

async fn add_tags(
    State(state): State<MemoryManageState>,
    headers: HeaderMap,
    Json(req): Json<TagRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Adding tags {:?} to segment {} for user {}",
        req.tags, req.segment_id, req.user_id
    );

    let dummy_embedding = vec![0.0_f32; 1536];
    let tenant_id = extract_tenant_id(&headers);
    let segments = if let Some(ref tid) = tenant_id {
        state
            .vector_store
            .search_segments_for_tenant(&req.user_id, tid, dummy_embedding, 100)
            .await?
    } else {
        state
            .vector_store
            .search_segments(&req.user_id, dummy_embedding, 100)
            .await?
    };

    let segment_id: Uuid = req
        .segment_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid segment ID".to_string()))?;

    let segment = segments
        .iter()
        .find(|s| s.id == segment_id)
        .ok_or_else(|| AppError::NotFound("Segment not found".to_string()))?;

    let mut updated = segment.clone();
    for tag in &req.tags {
        if !updated.tags.contains(tag) {
            updated.tags.push(tag.clone());
        }
    }
    updated.version += 1;
    updated.updated_at = Some(chrono::Utc::now());
    updated.previous_version_id = Some(segment.id);
    updated.id = Uuid::now_v7();

    state.vector_store.store_segment(updated).await?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Tags added successfully",
    })))
}

async fn search_by_tags(
    State(state): State<MemoryManageState>,
    headers: HeaderMap,
    Json(req): Json<SearchByTagRequest>,
) -> Result<impl IntoResponse, AppError> {
    let tenant_id = extract_tenant_id(&headers);
    let segments = if let Some(ref tid) = tenant_id {
        state
            .vector_store
            .search_segments_by_tags_for_tenant(&req.user_id, tid, &req.tags, req.limit)
            .await?
    } else {
        state
            .vector_store
            .search_segments_by_tags(&req.user_id, &req.tags, req.limit)
            .await?
    };

    let results: Vec<serde_json::Value> = segments
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id.to_string(),
                "summary": s.summary,
                "tags": s.tags,
                "memory_type": format!("{:?}", s.memory_type),
                "heat_score": s.heat_score,
                "version": s.version,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "status": "ok",
        "count": results.len(),
        "segments": results,
    })))
}

async fn export_memories(
    State(state): State<MemoryManageState>,
    headers: HeaderMap,
    Json(req): Json<ExportRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("Exporting memories for user: {}", req.user_id);

    let dummy_embedding = vec![0.0_f32; 1536];
    let tenant_id = extract_tenant_id(&headers);
    let segments = if let Some(ref tid) = tenant_id {
        state
            .vector_store
            .search_segments_for_tenant(&req.user_id, tid, dummy_embedding, 1000)
            .await?
    } else {
        state
            .vector_store
            .search_segments(&req.user_id, dummy_embedding, 1000)
            .await?
    };

    let exported: Vec<ExportedSegment> = segments
        .iter()
        .map(|s| ExportedSegment {
            id: s.id.to_string(),
            summary: s.summary.clone(),
            tags: s.tags.clone(),
            memory_type: format!("{:?}", s.memory_type),
            version: s.version,
            created_at: s.created_at.to_rfc3339(),
            heat_score: s.heat_score,
        })
        .collect();

    let export = MemoryExport {
        user_id: req.user_id.clone(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        segment_count: exported.len(),
        segments: exported,
    };

    match req.format {
        ExportFormat::Markdown => {
            let mut md = format!(
                "# Memory Export: {}\n\nExported: {}\nSegments: {}\n\n",
                export.user_id, export.exported_at, export.segment_count
            );
            for seg in &export.segments {
                md.push_str(&format!(
                    "## {} (v{})\n\n{}\n\nTags: {}\nType: {}\nHeat: {:.2}\n\n---\n\n",
                    seg.id,
                    seg.version,
                    seg.summary,
                    seg.tags.join(", "),
                    seg.memory_type,
                    seg.heat_score,
                ));
            }
            Ok(Json(serde_json::json!({
                "status": "ok",
                "format": "markdown",
                "content": md,
            })))
        }
        ExportFormat::Json => Ok(Json(serde_json::json!({
            "status": "ok",
            "format": "json",
            "data": export,
        }))),
    }
}

async fn import_memories(
    State(state): State<MemoryManageState>,
    Json(req): Json<ImportRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Importing {} segments for user: {}",
        req.segments.len(),
        req.user_id
    );

    let mut imported = 0;
    for seg in &req.segments {
        let memory_type = match seg.memory_type.as_deref() {
            Some("faq") => MemoryType::Faq,
            Some("faq_candidate") => MemoryType::FaqCandidate,
            _ => MemoryType::QA,
        };

        let segment = MidTermSegment {
            id: Uuid::now_v7(),
            user_id: req.user_id.clone(),
            summary: seg.summary.clone(),
            embedding: vec![0.0; 1536],
            heat: 0.0,
            created_at: chrono::Utc::now(),
            tenant_id: None,
            access_count: 0,
            heat_score: 0.0,
            last_accessed: None,
            memory_type,
            version: 1,
            tags: seg.tags.clone(),
            updated_at: None,
            previous_version_id: None,
        };

        state.vector_store.store_segment(segment).await?;
        imported += 1;
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "imported": imported,
    })))
}

async fn get_version_history(
    State(state): State<MemoryManageState>,
    headers: HeaderMap,
    Json(req): Json<VersionHistoryRequest>,
) -> Result<impl IntoResponse, AppError> {
    let dummy_embedding = vec![0.0_f32; 1536];
    let tenant_id = extract_tenant_id(&headers);
    let segments = if let Some(ref tid) = tenant_id {
        state
            .vector_store
            .search_segments_for_tenant(&req.user_id, tid, dummy_embedding, 200)
            .await?
    } else {
        state
            .vector_store
            .search_segments(&req.user_id, dummy_embedding, 200)
            .await?
    };

    let segment_id: Uuid = req
        .segment_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid segment ID".to_string()))?;

    let mut history = Vec::new();
    let mut current_id = Some(segment_id);

    while let Some(id) = current_id {
        if let Some(seg) = segments.iter().find(|s| s.id == id) {
            history.push(serde_json::json!({
                "id": seg.id.to_string(),
                "version": seg.version,
                "summary": seg.summary,
                "tags": seg.tags,
                "created_at": seg.created_at.to_rfc3339(),
                "updated_at": seg.updated_at.map(|t| t.to_rfc3339()),
            }));
            current_id = seg.previous_version_id;
        } else {
            break;
        }
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "segment_id": req.segment_id,
        "version_count": history.len(),
        "history": history,
    })))
}
