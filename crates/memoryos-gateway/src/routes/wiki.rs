use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use memoryos_ports::LlmAdapter;
use memoryos_wiki_gen::config::WikiGenConfig;
use memoryos_wiki_gen::llm_adapter as wiki_llm;
use memoryos_wiki_gen::WikiGenerator;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

struct PortsLlmBridge {
    inner: Arc<dyn LlmAdapter>,
}

#[async_trait::async_trait]
impl wiki_llm::WikiLlmAdapter for PortsLlmBridge {
    async fn chat(
        &self,
        request: wiki_llm::ChatRequest,
    ) -> Result<wiki_llm::ChatResponse, wiki_llm::WikiLlmError> {
        let ports_request = memoryos_ports::ChatRequest {
            model: request.model,
            messages: request
                .messages
                .into_iter()
                .map(|m| memoryos_ports::ChatMessage {
                    role: m.role,
                    content: m.content,
                })
                .collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: request.stream,
            extra: request.extra,
        };

        match self.inner.chat(ports_request).await {
            Ok(resp) => Ok(wiki_llm::ChatResponse {
                id: resp.id,
                object: resp.object,
                model: resp.model,
                choices: resp
                    .choices
                    .into_iter()
                    .map(|c| wiki_llm::ChatChoice {
                        index: c.index,
                        message: wiki_llm::ChatMessage {
                            role: c.message.role,
                            content: c.message.content,
                        },
                        finish_reason: c.finish_reason,
                    })
                    .collect(),
            }),
            Err(e) => Err(wiki_llm::WikiLlmError::RequestFailed(e.to_string())),
        }
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[derive(Clone)]
pub struct WikiState {
    pub llm_adapter: Option<Arc<dyn LlmAdapter>>,
    pub jobs: Arc<RwLock<Vec<WikiJob>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WikiJob {
    pub id: String,
    pub repo_path: String,
    pub status: JobStatus,
    pub pages_generated: usize,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    pub repo_path: String,
    #[serde(default)]
    pub config: Option<WikiGenConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ParseRequest {
    pub repo_path: String,
    #[serde(default)]
    pub config: Option<WikiGenConfig>,
}

#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    pub job_id: String,
    pub status: JobStatus,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ParseResponse {
    pub files: usize,
    pub symbols: usize,
    pub references: usize,
    pub endpoints: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub jobs: Vec<WikiJob>,
}

#[derive(Debug, Serialize)]
pub struct PageResponse {
    pub path: String,
    pub content: String,
}

pub fn create_wiki_routes(wiki_state: WikiState) -> Router {
    use super::wiki_connector;

    Router::new()
        .route("/generate", post(generate_wiki))
        .route("/parse", post(parse_repo))
        .route("/status", get(get_status))
        .route("/jobs/:job_id", get(get_job_status))
        .merge(wiki_connector::create_connector_routes())
        .with_state(wiki_state)
}

async fn generate_wiki(
    State(state): State<WikiState>,
    Json(req): Json<GenerateRequest>,
) -> impl IntoResponse {
    let repo_path = PathBuf::from(&req.repo_path);
    if !repo_path.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenerateResponse {
                job_id: String::new(),
                status: JobStatus::Failed,
                message: format!("Repository path does not exist: {}", req.repo_path),
            }),
        );
    }

    let job_id = format!("wiki-{}", chrono::Utc::now().timestamp_millis());
    let job = WikiJob {
        id: job_id.clone(),
        repo_path: req.repo_path.clone(),
        status: JobStatus::Pending,
        pages_generated: 0,
        error: None,
        started_at: chrono::Utc::now().to_rfc3339(),
        completed_at: None,
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.push(job);
    }

    let config = req.config.unwrap_or_default();
    let jobs = state.jobs.clone();
    let adapter = state.llm_adapter.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        {
            let mut jobs_lock = jobs.write().await;
            if let Some(job) = jobs_lock.iter_mut().find(|j| j.id == job_id_clone) {
                job.status = JobStatus::Running;
            }
        }

        let generator = match adapter {
            Some(a) => {
                let bridge: Arc<dyn wiki_llm::WikiLlmAdapter> =
                    Arc::new(PortsLlmBridge { inner: a });
                WikiGenerator::with_llm_adapter(config, bridge)
            }
            None => WikiGenerator::new(config),
        };

        match generator.generate(&repo_path).await {
            Ok(()) => {
                info!("Wiki generation completed for {}", repo_path.display());
                let mut jobs_lock = jobs.write().await;
                if let Some(job) = jobs_lock.iter_mut().find(|j| j.id == job_id_clone) {
                    job.status = JobStatus::Completed;
                    job.completed_at = Some(chrono::Utc::now().to_rfc3339());
                }
            }
            Err(e) => {
                warn!("Wiki generation failed: {}", e);
                let mut jobs_lock = jobs.write().await;
                if let Some(job) = jobs_lock.iter_mut().find(|j| j.id == job_id_clone) {
                    job.status = JobStatus::Failed;
                    job.error = Some(e.to_string());
                    job.completed_at = Some(chrono::Utc::now().to_rfc3339());
                }
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(GenerateResponse {
            job_id,
            status: JobStatus::Pending,
            message: "Wiki generation started".to_string(),
        }),
    )
}

async fn parse_repo(
    State(_state): State<WikiState>,
    Json(req): Json<ParseRequest>,
) -> impl IntoResponse {
    let repo_path = PathBuf::from(&req.repo_path);
    if !repo_path.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Repository path does not exist: {}", req.repo_path)
            })),
        );
    }

    let config = req.config.unwrap_or_default();
    let generator = WikiGenerator::new(config);

    match generator.parse_only(&repo_path) {
        Ok(ir) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "files": ir.files.len(),
                "symbols": ir.symbols.len(),
                "references": ir.references.len(),
                "endpoints": ir.endpoints.len(),
                "diagnostics": ir.diagnostics.len(),
                "manifests": ir.manifests.len(),
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Parse failed: {}", e)
            })),
        ),
    }
}

async fn get_status(State(state): State<WikiState>) -> impl IntoResponse {
    let jobs = state.jobs.read().await;
    Json(StatusResponse { jobs: jobs.clone() })
}

async fn get_job_status(
    State(state): State<WikiState>,
    AxumPath(job_id): AxumPath<String>,
) -> impl IntoResponse {
    let jobs = state.jobs.read().await;
    match jobs.iter().find(|j| j.id == job_id) {
        Some(job) => (StatusCode::OK, Json(serde_json::json!(job))),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Job {} not found", job_id)
            })),
        ),
    }
}
