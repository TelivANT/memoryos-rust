use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
#[derive(Debug, Serialize)]
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

pub fn create_connector_routes() -> Router<super::wiki::WikiState> {
    Router::new()
        .route("/connectors", get(list_connectors))
        .route("/connectors/test", post(test_connection))
        .route("/connectors/browse", post(browse_directory))
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
    State(_state): State<super::wiki::WikiState>,
    Json(_req): Json<TestConnectionRequest>,
) -> impl IntoResponse {
    // TODO: Implement actual connection test
    let connector_id = Uuid::now_v7().to_string();

    Json(TestConnectionResponse {
        success: true,
        message: Some("Connection successful".to_string()),
        connector_id: Some(connector_id),
        metadata: Some(HashMap::new()),
        error: None,
        error_code: None,
    })
}

async fn browse_directory(
    State(_state): State<super::wiki::WikiState>,
    Json(req): Json<BrowseDirectoryRequest>,
) -> impl IntoResponse {
    // TODO: Implement actual directory browsing
    Json(BrowseDirectoryResponse {
        path: req.path.clone(),
        entries: vec![],
        total: 0,
    })
}
