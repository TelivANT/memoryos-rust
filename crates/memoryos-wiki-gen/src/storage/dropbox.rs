use async_trait::async_trait;
use reqwest::Client;
use std::path::PathBuf;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, StorageConnector};

/// Dropbox connector (Dropbox API v2)
pub struct DropboxConnector {
    client: Client,
    access_token: String,
}

impl DropboxConnector {
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
        }
    }
}

#[async_trait]
impl StorageConnector for DropboxConnector {
    async fn connect(&mut self) -> Result<()> {
        let resp = self
            .client
            .post("https://api.dropboxapi.com/2/users/get_current_account")
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Dropbox connect failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Dropbox auth failed: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let folder_path = if path.is_empty() || path == "/" {
            "".to_string()
        } else {
            let p = path.trim_matches('/');
            format!("/{}", p)
        };
        let body = serde_json::json!({ "path": folder_path });
        let resp = self
            .client
            .post("https://api.dropboxapi.com/2/files/list_folder")
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Dropbox list failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Dropbox list failed: {}",
                resp.status()
            )));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Dropbox parse failed: {}", e)))?;
        let mut entries = Vec::new();
        if let Some(items) = data["entries"].as_array() {
            for item in items {
                let tag = item[".tag"].as_str().unwrap_or("");
                let name = item["name"].as_str().unwrap_or("").to_string();
                let is_dir = tag == "folder";
                let size = item["size"].as_u64().unwrap_or(0);
                let entry_path = item["path_display"]
                    .as_str()
                    .unwrap_or(&name)
                    .trim_start_matches('/')
                    .to_string();
                entries.push(FileEntry {
                    path: entry_path,
                    is_dir,
                    size,
                });
            }
        }
        Ok(entries)
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let file_path = format!("/{}", path.trim_matches('/'));
        let arg = serde_json::json!({ "path": file_path });
        let resp = self
            .client
            .post("https://content.dropboxapi.com/2/files/download")
            .bearer_auth(&self.access_token)
            .header("Dropbox-API-Arg", arg.to_string())
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Dropbox download failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Dropbox download failed: {}",
                resp.status()
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Dropbox read failed: {}", e)))?;
        Ok(bytes.to_vec())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let file_path = format!("/{}", path.trim_matches('/'));
        let body = serde_json::json!({ "path": file_path });
        match self
            .client
            .post("https://api.dropboxapi.com/2/files/get_metadata")
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let file_path = format!("/{}", path.trim_matches('/'));
        let body = serde_json::json!({ "path": file_path });
        let resp = self
            .client
            .post("https://api.dropboxapi.com/2/files/get_metadata")
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Dropbox metadata failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Dropbox metadata failed: {}",
                resp.status()
            )));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Dropbox parse failed: {}", e)))?;
        let tag = data[".tag"].as_str().unwrap_or("");
        Ok(FileMetadata {
            size: data["size"].as_u64().unwrap_or(0),
            modified: None,
            is_dir: tag == "folder",
        })
    }

    async fn clone_to_temp(&self) -> Result<PathBuf> {
        Err(WikiGenError::Storage(
            "Dropbox does not support clone_to_temp".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "dropbox"
    }
}
