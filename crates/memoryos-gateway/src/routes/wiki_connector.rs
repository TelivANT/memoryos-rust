use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use memoryos_wiki_gen::storage::{GitConnector, LocalConnector, StorageConnector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Connector field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,
    pub description: String,
}

/// Connector metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorMetadata {
    #[serde(rename = "type")]
    pub connector_type: String,
    pub name: String,
    pub description: String,
    pub auth_required: bool,
    pub fields: Vec<ConnectorField>,
}

/// List connectors response
#[derive(Debug, Serialize)]
pub struct ListConnectorsResponse {
    pub connectors: Vec<ConnectorMetadata>,
}

/// Test connection request
#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    #[serde(rename = "type")]
    pub connector_type: String,
    pub config: HashMap<String, serde_json::Value>,
}

/// Test connection response
#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

/// Browse directory request
#[derive(Debug, Deserialize)]
pub struct BrowseDirectoryRequest {
    pub connector_id: String,
    pub path: String,
}

/// Directory entry
#[derive(Debug, Clone, Serialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children_count: Option<usize>,
}

/// Browse directory response
#[derive(Debug, Serialize)]
pub struct BrowseDirectoryResponse {
    pub path: String,
    pub entries: Vec<DirectoryEntry>,
    pub total: usize,
}

/// Generate wiki request
#[derive(Debug, Deserialize)]
pub struct GenerateWithConnectorRequest {
    pub connector_id: String,
    pub path: String,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

/// Generate wiki response
#[derive(Debug, Serialize)]
pub struct GenerateWithConnectorResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
}

pub fn create_connector_routes() -> Router<super::wiki::WikiState> {
    Router::new()
        .route("/connectors", get(list_connectors))
        .route("/connectors/test", post(test_connection))
        .route("/connectors/browse", post(browse_directory))
        .route("/connectors/generate", post(generate_with_connector))
}

async fn list_connectors(State(_state): State<super::wiki::WikiState>) -> impl IntoResponse {
    let connectors = vec![
        ConnectorMetadata {
            connector_type: "local".to_string(),
            name: "Local Filesystem".to_string(),
            description: "Read from local filesystem".to_string(),
            auth_required: false,
            fields: vec![ConnectorField {
                name: "path".to_string(),
                field_type: "string".to_string(),
                required: true,
                default: None,
                options: None,
                sensitive: None,
                description: "Local directory path".to_string(),
            }],
        },
        ConnectorMetadata {
            connector_type: "git".to_string(),
            name: "Git Repository".to_string(),
            description: "Clone from Git repository".to_string(),
            auth_required: true,
            fields: vec![
                ConnectorField {
                    name: "url".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    default: None,
                    options: None,
                    sensitive: None,
                    description: "Git repository URL".to_string(),
                },
                ConnectorField {
                    name: "branch".to_string(),
                    field_type: "string".to_string(),
                    required: false,
                    default: Some("main".to_string()),
                    options: None,
                    sensitive: None,
                    description: "Branch name".to_string(),
                },
                ConnectorField {
                    name: "auth_type".to_string(),
                    field_type: "enum".to_string(),
                    required: true,
                    default: None,
                    options: Some(vec![
                        "token".to_string(),
                        "ssh_key".to_string(),
                        "none".to_string(),
                    ]),
                    sensitive: None,
                    description: "Authentication type".to_string(),
                },
                ConnectorField {
                    name: "token".to_string(),
                    field_type: "string".to_string(),
                    required: false,
                    default: None,
                    options: None,
                    sensitive: Some(true),
                    description: "Personal access token (if auth_type=token)".to_string(),
                },
            ],
        },
        ConnectorMetadata {
            connector_type: "s3".to_string(),
            name: "AWS S3".to_string(),
            description: "Read from S3-compatible storage".to_string(),
            auth_required: true,
            fields: vec![
                ConnectorField {
                    name: "region".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    default: None,
                    options: None,
                    sensitive: None,
                    description: "AWS region".to_string(),
                },
                ConnectorField {
                    name: "bucket".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    default: None,
                    options: None,
                    sensitive: None,
                    description: "S3 bucket name".to_string(),
                },
                ConnectorField {
                    name: "access_key_id".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    default: None,
                    options: None,
                    sensitive: Some(true),
                    description: "AWS access key ID".to_string(),
                },
                ConnectorField {
                    name: "secret_access_key".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    default: None,
                    options: None,
                    sensitive: Some(true),
                    description: "AWS secret access key".to_string(),
                },
            ],
        },
    ];

    Json(ListConnectorsResponse { connectors })
}

async fn test_connection(
    State(state): State<super::wiki::WikiState>,
    Json(req): Json<TestConnectionRequest>,
) -> impl IntoResponse {
    // Create connector based on type
    let connector: Box<dyn StorageConnector> = match req.connector_type.as_str() {
        "local" => {
            let path = req
                .config
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'path' field");
            match path {
                Ok(p) => Box::new(LocalConnector::new(PathBuf::from(p))),
                Err(e) => {
                    return Json(TestConnectionResponse {
                        success: false,
                        message: None,
                        connector_id: None,
                        metadata: None,
                        error: Some(e.to_string()),
                        error_code: Some("INVALID_CONFIG".to_string()),
                    })
                }
            }
        }
        "git" => {
            let url = req
                .config
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'url' field");
            match url {
                Ok(u) => {
                    let mut connector = GitConnector::new(u.to_string());
                    if let Some(token) = req.config.get("token").and_then(|v| v.as_str()) {
                        connector = connector.with_token(token.to_string());
                    }
                    Box::new(connector)
                }
                Err(e) => {
                    return Json(TestConnectionResponse {
                        success: false,
                        message: None,
                        connector_id: None,
                        metadata: None,
                        error: Some(e.to_string()),
                        error_code: Some("INVALID_CONFIG".to_string()),
                    })
                }
            }
        }
        _ => {
            return Json(TestConnectionResponse {
                success: false,
                message: None,
                connector_id: None,
                metadata: None,
                error: Some(format!(
                    "Unsupported connector type: {}",
                    req.connector_type
                )),
                error_code: Some("UNSUPPORTED_TYPE".to_string()),
            })
        }
    };

    // Test connection
    let mut connector = connector;
    match connector.connect().await {
        Ok(_) => {
            let connector_id = Uuid::now_v7().to_string();

            // Store in session
            state
                .connector_sessions
                .write()
                .await
                .insert(connector_id.clone(), Arc::new(RwLock::new(connector)));

            Json(TestConnectionResponse {
                success: true,
                message: Some("Connection successful".to_string()),
                connector_id: Some(connector_id),
                metadata: Some(HashMap::new()),
                error: None,
                error_code: None,
            })
        }
        Err(e) => Json(TestConnectionResponse {
            success: false,
            message: None,
            connector_id: None,
            metadata: None,
            error: Some(format!("{}", e)),
            error_code: Some("CONNECTION_FAILED".to_string()),
        }),
    }
}

async fn browse_directory(
    State(state): State<super::wiki::WikiState>,
    Json(req): Json<BrowseDirectoryRequest>,
) -> impl IntoResponse {
    // Get connector from session
    let sessions = state.connector_sessions.read().await;
    let connector = match sessions.get(&req.connector_id) {
        Some(c) => c.clone(),
        None => {
            return Json(BrowseDirectoryResponse {
                path: req.path.clone(),
                entries: vec![],
                total: 0,
            })
        }
    };
    drop(sessions);

    // List files
    let connector = connector.read().await;
    match connector.list_files(&req.path).await {
        Ok(entries) => {
            let dir_entries: Vec<DirectoryEntry> = entries
                .into_iter()
                .map(|e| {
                    let name = std::path::Path::new(&e.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&e.path)
                        .to_string();

                    DirectoryEntry {
                        name,
                        path: e.path,
                        entry_type: if e.is_dir { "directory" } else { "file" }.to_string(),
                        size: Some(e.size),
                        modified: None,
                        children_count: None,
                    }
                })
                .collect();

            Json(BrowseDirectoryResponse {
                path: req.path,
                entries: dir_entries.clone(),
                total: dir_entries.len(),
            })
        }
        Err(_) => Json(BrowseDirectoryResponse {
            path: req.path,
            entries: vec![],
            total: 0,
        }),
    }
}

async fn generate_with_connector(
    State(state): State<super::wiki::WikiState>,
    Json(req): Json<GenerateWithConnectorRequest>,
) -> impl IntoResponse {
    // Get connector from session
    let sessions = state.connector_sessions.read().await;
    let connector = match sessions.get(&req.connector_id) {
        Some(c) => c.clone(),
        None => {
            return Json(GenerateWithConnectorResponse {
                job_id: String::new(),
                status: "failed".to_string(),
                message: "Connector not found or session expired".to_string(),
            })
        }
    };
    drop(sessions);

    // Clone to temp directory
    let connector = connector.read().await;
    let temp_path = match connector.clone_to_temp().await {
        Ok(p) => p,
        Err(e) => {
            return Json(GenerateWithConnectorResponse {
                job_id: String::new(),
                status: "failed".to_string(),
                message: format!("Failed to clone: {}", e),
            })
        }
    };

    // Create job
    let job_id = format!("wiki-{}", uuid::Uuid::now_v7());
    let job = super::wiki::WikiJob {
        id: job_id.clone(),
        repo_path: temp_path.display().to_string(),
        status: super::wiki::JobStatus::Pending,
        pages_generated: 0,
        error: None,
        started_at: chrono::Utc::now().to_rfc3339(),
        completed_at: None,
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.push(job);
    }

    // Start generation in background
    let jobs = state.jobs.clone();
    let job_id_clone = job_id.clone();
    let adapter = state.llm_adapter.clone();

    tokio::spawn(async move {
        use memoryos_wiki_gen::llm_adapter as wiki_llm;
        use memoryos_wiki_gen::WikiGenerator;

        {
            let mut jobs_lock = jobs.write().await;
            if let Some(job) = jobs_lock.iter_mut().find(|j| j.id == job_id_clone) {
                job.status = super::wiki::JobStatus::Running;
            }
        }

        let config = memoryos_wiki_gen::config::WikiGenConfig::default();
        let generator = match adapter {
            Some(a) => {
                let bridge: Arc<dyn wiki_llm::WikiLlmAdapter> =
                    Arc::new(super::wiki::PortsLlmBridge { inner: a });
                WikiGenerator::with_llm_adapter(config, bridge)
            }
            None => WikiGenerator::new(config),
        };

        match generator.generate(&temp_path).await {
            Ok(()) => {
                let mut jobs_lock = jobs.write().await;
                if let Some(job) = jobs_lock.iter_mut().find(|j| j.id == job_id_clone) {
                    job.status = super::wiki::JobStatus::Completed;
                    job.completed_at = Some(chrono::Utc::now().to_rfc3339());
                }
            }
            Err(e) => {
                let mut jobs_lock = jobs.write().await;
                if let Some(job) = jobs_lock.iter_mut().find(|j| j.id == job_id_clone) {
                    job.status = super::wiki::JobStatus::Failed;
                    job.error = Some(format!("{}", e));
                    job.completed_at = Some(chrono::Utc::now().to_rfc3339());
                }
            }
        }
    });

    Json(GenerateWithConnectorResponse {
        job_id,
        status: "pending".to_string(),
        message: "Wiki generation started".to_string(),
    })
}
