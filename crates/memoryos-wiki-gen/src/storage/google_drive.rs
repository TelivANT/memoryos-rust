use async_trait::async_trait;
use reqwest::Client;
use std::path::PathBuf;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, StorageConnector};

/// Google Drive connector (Drive API v3)
pub struct GoogleDriveConnector {
    client: Client,
    access_token: String,
    root_folder_id: String,
}

impl GoogleDriveConnector {
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
            root_folder_id: "root".to_string(),
        }
    }

    pub fn with_folder_id(mut self, folder_id: String) -> Self {
        self.root_folder_id = folder_id;
        self
    }
}

#[async_trait]
impl StorageConnector for GoogleDriveConnector {
    async fn connect(&mut self) -> Result<()> {
        let url = "https://www.googleapis.com/drive/v3/about?fields=user";
        let resp = self
            .client
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Google Drive connect failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Google Drive auth failed: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        // For Google Drive, path is treated as folder ID or "root"
        let folder_id = if path.is_empty() || path == "/" {
            &self.root_folder_id
        } else {
            path
        };
        let query = format!("'{}' in parents and trashed = false", folder_id);
        let url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,mimeType,size)",
            urlencoding::encode(&query)
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Google Drive list failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Google Drive list failed: {}",
                resp.status()
            )));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Google Drive parse failed: {}", e)))?;
        let mut entries = Vec::new();
        if let Some(files) = body["files"].as_array() {
            for file in files {
                let name = file["name"].as_str().unwrap_or("").to_string();
                let mime = file["mimeType"].as_str().unwrap_or("");
                let is_dir = mime == "application/vnd.google-apps.folder";
                let size = file["size"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0u64);
                entries.push(FileEntry {
                    path: name,
                    is_dir,
                    size,
                });
            }
        }
        Ok(entries)
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        // path is file ID for Google Drive
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?alt=media",
            path
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Google Drive get failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Google Drive get failed: {}",
                resp.status()
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Google Drive read failed: {}", e)))?;
        Ok(bytes.to_vec())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=id",
            path
        );
        match self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
        {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=id,name,mimeType,size,modifiedTime",
            path
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Google Drive metadata failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Google Drive metadata failed: {}",
                resp.status()
            )));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Google Drive parse failed: {}", e)))?;
        let mime = body["mimeType"].as_str().unwrap_or("");
        let size = body["size"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0u64);
        Ok(FileMetadata {
            size,
            modified: None,
            is_dir: mime == "application/vnd.google-apps.folder",
        })
    }

    async fn clone_to_temp(&self) -> Result<PathBuf> {
        Err(WikiGenError::Storage(
            "Google Drive does not support clone_to_temp".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "google_drive"
    }
}
