use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use memoryos_wiki_gen::storage::{
    CosConnector, GitConnector, LocalConnector, ObsConnector, OssConnector, S3Connector,
    SftpConnector, StorageConnector, WebDavConnector,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

const SESSION_TTL_SECS: u64 = 3600;

const SENSITIVE_FIELDS: &[&str] = &[
    "token",
    "password",
    "secret_access_key",
    "access_key_secret",
    "secret_key",
    "ssh_key_path",
];

fn connectors_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".memoryos")
        .join("connectors")
}

fn obfuscate_key() -> [u8; 32] {
    let seed = std::env::var("MEMORYOS_CONNECTOR_SECRET")
        .unwrap_or_else(|_| "memoryos-default-connector-key-change-me".to_string());
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    hasher.finalize().into()
}

fn xor_encrypt(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % 32])
        .collect()
}

fn mask_sensitive(
    config: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    config
        .iter()
        .map(|(k, v)| {
            if SENSITIVE_FIELDS.contains(&k.as_str()) {
                if let Some(s) = v.as_str() {
                    if s.len() > 4 {
                        let masked = format!("{}***", &s[..4]);
                        return (k.clone(), serde_json::Value::String(masked));
                    }
                }
                (k.clone(), serde_json::Value::String("***".to_string()))
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}

fn encrypt_config(
    config: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let key = obfuscate_key();
    config
        .iter()
        .map(|(k, v)| {
            if SENSITIVE_FIELDS.contains(&k.as_str()) {
                if let Some(s) = v.as_str() {
                    let encrypted = xor_encrypt(s.as_bytes(), &key);
                    let encoded = base64_encode(&encrypted);
                    return (k.clone(), serde_json::json!({"__encrypted": encoded}));
                }
            }
            (k.clone(), v.clone())
        })
        .collect()
}

fn decrypt_config(
    config: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let key = obfuscate_key();
    config
        .iter()
        .map(|(k, v)| {
            if let Some(obj) = v.as_object() {
                if let Some(enc) = obj.get("__encrypted").and_then(|e| e.as_str()) {
                    if let Some(decoded) = base64_decode(enc) {
                        let decrypted = xor_encrypt(&decoded, &key);
                        if let Ok(s) = String::from_utf8(decrypted) {
                            return (k.clone(), serde_json::Value::String(s));
                        }
                    }
                }
            }
            (k.clone(), v.clone())
        })
        .collect()
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const DECODE: [u8; 128] = {
        let mut table = [255u8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[chars[i] as usize] = i as u8;
            i += 1;
        }
        table
    };
    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();
    let mut result = Vec::new();
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let vals: Vec<u8> = chunk
            .iter()
            .filter_map(|&b| {
                if (b as usize) < 128 && DECODE[b as usize] != 255 {
                    Some(DECODE[b as usize])
                } else {
                    None
                }
            })
            .collect();
        if vals.len() < 2 {
            return None;
        }
        result.push((vals[0] << 2) | (vals[1] >> 4));
        if vals.len() > 2 {
            result.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if vals.len() > 3 {
            result.push((vals[2] << 6) | vals[3]);
        }
    }
    Some(result)
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorMetadata {
    #[serde(rename = "type")]
    pub connector_type: String,
    pub name: String,
    pub description: String,
    pub auth_required: bool,
    pub fields: Vec<ConnectorField>,
}

#[derive(Debug, Serialize)]
pub struct ListConnectorsResponse {
    pub connectors: Vec<ConnectorMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    #[serde(rename = "type")]
    pub connector_type: String,
    pub config: HashMap<String, serde_json::Value>,
}

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

#[derive(Debug, Deserialize)]
pub struct BrowseDirectoryRequest {
    pub connector_id: String,
    pub path: String,
}

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

#[derive(Debug, Serialize)]
pub struct BrowseDirectoryResponse {
    pub success: bool,
    pub path: String,
    pub entries: Vec<DirectoryEntry>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateWithConnectorRequest {
    pub connector_id: String,
}

#[derive(Debug, Serialize)]
pub struct GenerateWithConnectorResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveConnectorRequest {
    pub name: String,
    pub connector_type: String,
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct SaveConnectorResponse {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedConnector {
    pub id: String,
    pub name: String,
    pub connector_type: String,
    pub config: HashMap<String, serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SavedConnectorView {
    pub id: String,
    pub name: String,
    pub connector_type: String,
    pub config: HashMap<String, serde_json::Value>,
    pub created_at: String,
}

pub struct ConnectorSession {
    pub connector: Arc<RwLock<Box<dyn StorageConnector>>>,
    pub created_at: std::time::Instant,
}

pub fn create_connector_routes() -> Router<super::wiki::WikiState> {
    Router::new()
        .route("/connectors", get(list_connectors).post(save_connector))
        .route("/connectors/saved", get(list_saved_connectors))
        .route("/connectors/test", post(test_connection))
        .route("/connectors/browse", post(browse_directory))
        .route("/connectors/generate", post(generate_with_connector))
}

fn field(name: &str, ft: &str, required: bool, desc: &str) -> ConnectorField {
    ConnectorField {
        name: name.to_string(),
        field_type: ft.to_string(),
        required,
        default: None,
        options: None,
        sensitive: None,
        description: desc.to_string(),
    }
}

fn sensitive_field(name: &str, ft: &str, required: bool, desc: &str) -> ConnectorField {
    ConnectorField {
        sensitive: Some(true),
        ..field(name, ft, required, desc)
    }
}

fn field_with_default(
    name: &str,
    ft: &str,
    required: bool,
    default: &str,
    desc: &str,
) -> ConnectorField {
    ConnectorField {
        default: Some(default.to_string()),
        ..field(name, ft, required, desc)
    }
}

fn get_str(config: &HashMap<String, serde_json::Value>, key: &str) -> String {
    config
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn get_u16(config: &HashMap<String, serde_json::Value>, key: &str, default: u16) -> u16 {
    config
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u16)
        .unwrap_or(default)
}

async fn cleanup_expired_sessions(sessions: &Arc<RwLock<HashMap<String, ConnectorSession>>>) {
    let mut guard = sessions.write().await;
    let now = std::time::Instant::now();
    guard.retain(|_, s| now.duration_since(s.created_at).as_secs() < SESSION_TTL_SECS);
}

fn build_connector(
    connector_type: &str,
    config: &HashMap<String, serde_json::Value>,
) -> Result<Box<dyn StorageConnector>, String> {
    match connector_type {
        "local" => {
            let path = get_str(config, "path");
            if path.is_empty() {
                return Err("path is required".to_string());
            }
            Ok(Box::new(LocalConnector::new(PathBuf::from(path))))
        }
        "git" => {
            let url = get_str(config, "url");
            if url.is_empty() {
                return Err("url is required".to_string());
            }
            let mut conn = GitConnector::new(url);
            let branch = get_str(config, "branch");
            if !branch.is_empty() {
                conn = conn.with_branch(branch);
            }
            let token = get_str(config, "token");
            if !token.is_empty() {
                conn = conn.with_token(token);
            }
            Ok(Box::new(conn))
        }
        "s3" => {
            let bucket = get_str(config, "bucket");
            let region = get_str(config, "region");
            let access_key = get_str(config, "access_key");
            let secret_key = get_str(config, "secret_key");
            if bucket.is_empty()
                || region.is_empty()
                || access_key.is_empty()
                || secret_key.is_empty()
            {
                return Err("bucket, region, access_key, secret_key are required".to_string());
            }
            let mut conn = S3Connector::new(bucket, region, access_key, secret_key);
            let endpoint = get_str(config, "endpoint");
            if !endpoint.is_empty() {
                conn = conn.with_endpoint(endpoint);
            }
            let prefix = get_str(config, "prefix");
            if !prefix.is_empty() {
                conn = conn.with_prefix(prefix);
            }
            Ok(Box::new(conn))
        }
        "webdav" => {
            let url = get_str(config, "url");
            if url.is_empty() {
                return Err("url is required".to_string());
            }
            let mut conn = WebDavConnector::new(url);
            let username = get_str(config, "username");
            let password = get_str(config, "password");
            if !username.is_empty() && !password.is_empty() {
                conn = conn.with_auth(username, password);
            }
            Ok(Box::new(conn))
        }
        "sftp" => {
            let host = get_str(config, "host");
            let username = get_str(config, "username");
            if host.is_empty() || username.is_empty() {
                return Err("host and username are required".to_string());
            }
            let mut conn = SftpConnector::new(host, username);
            let port = get_u16(config, "port", 22);
            if port != 22 {
                conn = conn.with_port(port);
            }
            let password = get_str(config, "password");
            if !password.is_empty() {
                conn = conn.with_password(password);
            }
            let key_path = get_str(config, "ssh_key_path");
            if !key_path.is_empty() {
                conn = conn.with_key(PathBuf::from(key_path));
            }
            Ok(Box::new(conn))
        }
        "oss" => {
            let endpoint = get_str(config, "endpoint");
            let bucket = get_str(config, "bucket");
            let access_key_id = get_str(config, "access_key_id");
            let access_key_secret = get_str(config, "access_key_secret");
            if endpoint.is_empty()
                || bucket.is_empty()
                || access_key_id.is_empty()
                || access_key_secret.is_empty()
            {
                return Err(
                    "endpoint, bucket, access_key_id, access_key_secret are required".to_string(),
                );
            }
            let mut conn = OssConnector::new(endpoint, bucket, access_key_id, access_key_secret);
            let prefix = get_str(config, "prefix");
            if !prefix.is_empty() {
                conn = conn.with_prefix(prefix);
            }
            Ok(Box::new(conn))
        }
        "cos" => {
            let region = get_str(config, "region");
            let bucket = get_str(config, "bucket");
            let secret_id = get_str(config, "secret_id");
            let secret_key = get_str(config, "secret_key");
            if region.is_empty()
                || bucket.is_empty()
                || secret_id.is_empty()
                || secret_key.is_empty()
            {
                return Err("region, bucket, secret_id, secret_key are required".to_string());
            }
            let mut conn = CosConnector::new(region, bucket, secret_id, secret_key);
            let prefix = get_str(config, "prefix");
            if !prefix.is_empty() {
                conn = conn.with_prefix(prefix);
            }
            Ok(Box::new(conn))
        }
        "obs" => {
            let endpoint = get_str(config, "endpoint");
            let bucket = get_str(config, "bucket");
            let access_key_id = get_str(config, "access_key_id");
            let secret_access_key = get_str(config, "secret_access_key");
            if endpoint.is_empty()
                || bucket.is_empty()
                || access_key_id.is_empty()
                || secret_access_key.is_empty()
            {
                return Err(
                    "endpoint, bucket, access_key_id, secret_access_key are required".to_string(),
                );
            }
            let mut conn = ObsConnector::new(endpoint, bucket, access_key_id, secret_access_key);
            let prefix = get_str(config, "prefix");
            if !prefix.is_empty() {
                conn = conn.with_prefix(prefix);
            }
            Ok(Box::new(conn))
        }
        _ => Err(format!("Unknown connector type: {}", connector_type)),
    }
}

async fn list_connectors() -> impl IntoResponse {
    let connectors = vec![
        ConnectorMetadata {
            connector_type: "local".to_string(),
            name: "Local Filesystem".to_string(),
            description: "Browse local directories".to_string(),
            auth_required: false,
            fields: vec![field("path", "string", true, "Local directory path")],
        },
        ConnectorMetadata {
            connector_type: "git".to_string(),
            name: "Git Repository".to_string(),
            description: "Clone and browse a Git repository".to_string(),
            auth_required: false,
            fields: vec![
                field("url", "string", true, "Git repository URL"),
                field_with_default("branch", "string", false, "main", "Branch to checkout"),
                sensitive_field("token", "string", false, "Personal access token"),
            ],
        },
        ConnectorMetadata {
            connector_type: "s3".to_string(),
            name: "Amazon S3".to_string(),
            description: "Connect to Amazon S3 bucket".to_string(),
            auth_required: true,
            fields: vec![
                field("bucket", "string", true, "S3 bucket name"),
                field("region", "string", true, "AWS region"),
                field("access_key", "string", true, "AWS access key ID"),
                sensitive_field("secret_key", "string", true, "AWS secret access key"),
                field("endpoint", "string", false, "Custom S3 endpoint URL"),
                field("prefix", "string", false, "Key prefix filter"),
            ],
        },
        ConnectorMetadata {
            connector_type: "webdav".to_string(),
            name: "WebDAV".to_string(),
            description: "Connect to a WebDAV server".to_string(),
            auth_required: false,
            fields: vec![
                field("url", "string", true, "WebDAV server URL"),
                field("username", "string", false, "Username"),
                sensitive_field("password", "string", false, "Password"),
            ],
        },
        ConnectorMetadata {
            connector_type: "sftp".to_string(),
            name: "SFTP".to_string(),
            description: "Connect to an SFTP server".to_string(),
            auth_required: true,
            fields: vec![
                field("host", "string", true, "SFTP host"),
                field_with_default("port", "number", false, "22", "SFTP port"),
                field("username", "string", true, "Username"),
                sensitive_field("password", "string", false, "Password"),
                sensitive_field("ssh_key_path", "string", false, "Path to SSH private key"),
            ],
        },
        ConnectorMetadata {
            connector_type: "oss".to_string(),
            name: "Alibaba Cloud OSS".to_string(),
            description: "Connect to Alibaba Cloud Object Storage Service".to_string(),
            auth_required: true,
            fields: vec![
                field("endpoint", "string", true, "OSS endpoint URL"),
                field("bucket", "string", true, "Bucket name"),
                field("access_key_id", "string", true, "Access key ID"),
                sensitive_field("access_key_secret", "string", true, "Access key secret"),
                field("prefix", "string", false, "Key prefix filter"),
            ],
        },
        ConnectorMetadata {
            connector_type: "cos".to_string(),
            name: "Tencent Cloud COS".to_string(),
            description: "Connect to Tencent Cloud Object Storage".to_string(),
            auth_required: true,
            fields: vec![
                field("region", "string", true, "COS region"),
                field("bucket", "string", true, "Bucket name"),
                field("secret_id", "string", true, "Secret ID"),
                sensitive_field("secret_key", "string", true, "Secret key"),
                field("prefix", "string", false, "Key prefix filter"),
            ],
        },
        ConnectorMetadata {
            connector_type: "obs".to_string(),
            name: "Huawei Cloud OBS".to_string(),
            description: "Connect to Huawei Cloud Object Storage".to_string(),
            auth_required: true,
            fields: vec![
                field("endpoint", "string", true, "OBS endpoint URL"),
                field("bucket", "string", true, "Bucket name"),
                field("access_key_id", "string", true, "Access key ID"),
                sensitive_field("secret_access_key", "string", true, "Secret access key"),
                field("prefix", "string", false, "Key prefix filter"),
            ],
        },
    ];

    Json(ListConnectorsResponse { connectors })
}

async fn test_connection(
    State(state): State<super::wiki::WikiState>,
    Json(req): Json<TestConnectionRequest>,
) -> impl IntoResponse {
    let mut connector = match build_connector(&req.connector_type, &req.config) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TestConnectionResponse {
                    success: false,
                    message: None,
                    connector_id: None,
                    metadata: None,
                    error: Some(e),
                    error_code: Some("INVALID_CONFIG".to_string()),
                }),
            );
        }
    };

    if let Err(e) = connector.connect().await {
        return (
            StatusCode::BAD_REQUEST,
            Json(TestConnectionResponse {
                success: false,
                message: None,
                connector_id: None,
                metadata: None,
                error: Some(format!("{}", e)),
                error_code: Some("CONNECTION_FAILED".to_string()),
            }),
        );
    }

    cleanup_expired_sessions(&state.connector_sessions).await;

    let connector_id = Uuid::now_v7().to_string();
    let session = ConnectorSession {
        connector: Arc::new(RwLock::new(connector)),
        created_at: std::time::Instant::now(),
    };

    {
        let mut sessions = state.connector_sessions.write().await;
        sessions.insert(connector_id.clone(), session);
    }

    (
        StatusCode::OK,
        Json(TestConnectionResponse {
            success: true,
            message: Some("Connection successful".to_string()),
            connector_id: Some(connector_id),
            metadata: None,
            error: None,
            error_code: None,
        }),
    )
}

async fn browse_directory(
    State(state): State<super::wiki::WikiState>,
    Json(req): Json<BrowseDirectoryRequest>,
) -> impl IntoResponse {
    cleanup_expired_sessions(&state.connector_sessions).await;

    let sessions = state.connector_sessions.read().await;
    let session = match sessions.get(&req.connector_id) {
        Some(s) => s,
        None => {
            return (
                StatusCode::GONE,
                Json(BrowseDirectoryResponse {
                    success: false,
                    path: req.path,
                    entries: vec![],
                    total: 0,
                    error: Some("Session expired or not found".to_string()),
                }),
            );
        }
    };

    if std::time::Instant::now()
        .duration_since(session.created_at)
        .as_secs()
        >= SESSION_TTL_SECS
    {
        return (
            StatusCode::GONE,
            Json(BrowseDirectoryResponse {
                success: false,
                path: req.path,
                entries: vec![],
                total: 0,
                error: Some("Session expired".to_string()),
            }),
        );
    }

    let connector = session.connector.read().await;
    match connector.list_files(&req.path).await {
        Ok(files) => {
            let dir_entries: Vec<DirectoryEntry> = files
                .into_iter()
                .map(|f| {
                    let name = f.path.rsplit('/').next().unwrap_or(&f.path).to_string();
                    DirectoryEntry {
                        name,
                        path: f.path.clone(),
                        entry_type: if f.is_dir {
                            "directory".to_string()
                        } else {
                            "file".to_string()
                        },
                        size: if f.is_dir { None } else { Some(f.size) },
                        modified: None,
                        children_count: None,
                    }
                })
                .collect();
            let total = dir_entries.len();
            (
                StatusCode::OK,
                Json(BrowseDirectoryResponse {
                    success: true,
                    path: req.path,
                    entries: dir_entries,
                    total,
                    error: None,
                }),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(BrowseDirectoryResponse {
                success: false,
                path: req.path,
                entries: vec![],
                total: 0,
                error: Some(format!("{}", e)),
            }),
        ),
    }
}

async fn generate_with_connector(
    State(state): State<super::wiki::WikiState>,
    Json(req): Json<GenerateWithConnectorRequest>,
) -> impl IntoResponse {
    cleanup_expired_sessions(&state.connector_sessions).await;

    let sessions = state.connector_sessions.read().await;
    let session = match sessions.get(&req.connector_id) {
        Some(s) => s,
        None => {
            return (
                StatusCode::GONE,
                Json(GenerateWithConnectorResponse {
                    job_id: String::new(),
                    status: "failed".to_string(),
                    message: "Connector not found or session expired".to_string(),
                }),
            );
        }
    };

    if std::time::Instant::now()
        .duration_since(session.created_at)
        .as_secs()
        >= SESSION_TTL_SECS
    {
        return (
            StatusCode::GONE,
            Json(GenerateWithConnectorResponse {
                job_id: String::new(),
                status: "failed".to_string(),
                message: "Session expired".to_string(),
            }),
        );
    }

    let connector = session.connector.clone();
    drop(sessions);

    let connector_guard = connector.read().await;
    let temp_path = match connector_guard.clone_to_temp().await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenerateWithConnectorResponse {
                    job_id: String::new(),
                    status: "failed".to_string(),
                    message: format!("Failed to clone: {}", e),
                }),
            );
        }
    };
    drop(connector_guard);

    let job_id = format!("wiki-{}", Uuid::now_v7());
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

    (
        StatusCode::ACCEPTED,
        Json(GenerateWithConnectorResponse {
            job_id,
            status: "pending".to_string(),
            message: "Wiki generation started".to_string(),
        }),
    )
}

async fn save_connector(
    State(_state): State<super::wiki::WikiState>,
    Json(req): Json<SaveConnectorRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7().to_string();
    let encrypted = encrypt_config(&req.config);
    let saved = SavedConnector {
        id: id.clone(),
        name: req.name,
        connector_type: req.connector_type,
        config: encrypted,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let dir = connectors_dir();
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SaveConnectorResponse {
                id: String::new(),
                message: format!("Failed to create directory: {}", e),
            }),
        );
    }

    let file_path = dir.join(format!("{}.json", id));
    let json = match serde_json::to_string_pretty(&saved) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SaveConnectorResponse {
                    id: String::new(),
                    message: format!("Failed to serialize: {}", e),
                }),
            );
        }
    };

    match tokio::fs::write(&file_path, json).await {
        Ok(_) => (
            StatusCode::OK,
            Json(SaveConnectorResponse {
                id,
                message: "Connector saved successfully".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SaveConnectorResponse {
                id: String::new(),
                message: format!("Failed to save connector: {}", e),
            }),
        ),
    }
}

async fn list_saved_connectors(State(_state): State<super::wiki::WikiState>) -> impl IntoResponse {
    let dir = connectors_dir();
    let mut connectors = Vec::new();

    if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                if let Ok(saved) = serde_json::from_str::<SavedConnector>(&content) {
                    let decrypted = decrypt_config(&saved.config);
                    let masked = mask_sensitive(&decrypted);
                    connectors.push(SavedConnectorView {
                        id: saved.id,
                        name: saved.name,
                        connector_type: saved.connector_type,
                        config: masked,
                        created_at: saved.created_at,
                    });
                }
            }
        }
    }

    Json(connectors)
}
