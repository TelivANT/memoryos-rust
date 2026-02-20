use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use memoryos_core::{ExtractedTriple, GraphManager, GraphQueryResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone)]
pub struct GraphState {
    pub graph_manager: Arc<RwLock<GraphManager>>,
}

#[derive(Deserialize)]
pub struct ExtractRequest {
    pub text: String,
}

#[derive(Serialize)]
pub struct ExtractResponse {
    pub entities_added: usize,
    pub triples: Vec<ExtractedTriple>,
}

#[derive(Deserialize)]
pub struct QueryEntityRequest {
    pub query: String,
}

#[derive(Deserialize)]
pub struct QueryPathRequest {
    pub from: String,
    pub to: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

fn default_max_depth() -> usize {
    3
}

#[derive(Serialize)]
pub struct GraphStatsResponse {
    pub entity_count: usize,
    pub relation_count: usize,
}

pub fn create_graph_routes(state: GraphState) -> Router {
    Router::new()
        .route("/extract", post(extract_entities))
        .route("/extract/llm-prompt", post(build_llm_prompt))
        .route("/extract/llm-parse", post(parse_llm_response))
        .route("/query", post(query_entities))
        .route("/path", post(query_path))
        .route("/triples", get(get_all_triples))
        .route("/stats", get(get_stats))
        .with_state(state)
}

async fn extract_entities(
    State(state): State<GraphState>,
    Json(req): Json<ExtractRequest>,
) -> impl IntoResponse {
    info!("Extracting entities from text ({} chars)", req.text.len());
    let mut gm = state.graph_manager.write().await;
    let triples = gm.extract_and_merge(&req.text);
    let entities_added = gm.entity_count();
    Json(ExtractResponse {
        entities_added,
        triples,
    })
}

#[derive(Deserialize)]
pub struct LlmParseRequest {
    pub response: String,
}

async fn build_llm_prompt(Json(req): Json<ExtractRequest>) -> impl IntoResponse {
    let prompt = GraphManager::build_llm_extraction_prompt(&req.text);
    Json(serde_json::json!({
        "prompt": prompt,
    }))
}

async fn parse_llm_response(
    State(state): State<GraphState>,
    Json(req): Json<LlmParseRequest>,
) -> impl IntoResponse {
    let mut gm = state.graph_manager.write().await;
    let (entities, triples) = gm.parse_llm_extraction_response(&req.response);
    Json(serde_json::json!({
        "entities_added": entities.len(),
        "triples": triples,
        "total_entities": gm.entity_count(),
        "total_relations": gm.relation_count(),
    }))
}

async fn query_entities(
    State(state): State<GraphState>,
    Json(req): Json<QueryEntityRequest>,
) -> impl IntoResponse {
    let gm = state.graph_manager.read().await;
    let entities = gm.query_by_label(&req.query);
    let mut triples = Vec::new();
    for entity in &entities {
        let id = &entity.id;
        triples.extend(gm.query_relations(id));
    }
    Json(GraphQueryResult {
        entities: entities.into_iter().cloned().collect(),
        triples,
    })
}

async fn query_path(
    State(state): State<GraphState>,
    Json(req): Json<QueryPathRequest>,
) -> impl IntoResponse {
    let gm = state.graph_manager.read().await;
    let from_id = req.from.to_lowercase().replace(' ', "_");
    let to_id = req.to.to_lowercase().replace(' ', "_");
    let paths = gm.query_path(&from_id, &to_id, req.max_depth);
    Json(serde_json::json!({
        "paths": paths,
        "count": paths.len(),
    }))
}

async fn get_all_triples(State(state): State<GraphState>) -> impl IntoResponse {
    let gm = state.graph_manager.read().await;
    let triples = gm.get_all_triples();
    Json(serde_json::json!({
        "triples": triples,
        "count": triples.len(),
    }))
}

async fn get_stats(State(state): State<GraphState>) -> impl IntoResponse {
    let gm = state.graph_manager.read().await;
    Json(GraphStatsResponse {
        entity_count: gm.entity_count(),
        relation_count: gm.relation_count(),
    })
}
