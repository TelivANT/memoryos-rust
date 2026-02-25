use async_trait::async_trait;
use reqwest::Client;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, StorageConnector};

/// Aliyun Drive (阿里云盘) connector (Open API)
pub struct AliyunDriveConnector {
    client: Client,
    access_token: String,
    drive_id: String,
    root_file_id: String,
}

impl AliyunDriveConnector {
    pub fn new(access_token: String, drive_id: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
            drive_id,
            root_file_id: "root".to_string(),
        }
    }

    pub fn with_root_folder(mut self, file_id: String) -> Self {
        self.root_file_id = file_id;
        self
    }
}

#[async_trait]
impl StorageConnector for AliyunDriveConnector {
    async fn connect(&mut self) -> Result<()> {
        let resp = self
            .client
            .post("https://openapi.alipan.com/adrive/v1.0/user/getDriveInfo")
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Aliyun Drive connect failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Aliyun Drive auth failed: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let parent_file_id = if path.is_empty() || path == "/" {
            self.root_file_id.clone()
        } else {
            path.to_string()
        };
        let body = serde_json::json!({
            "drive_id": self.drive_id,
            "parent_file_id": parent_file_id,
            "limit": 200
        });
        let resp = self
            .client
            .post("https://openapi.alipan.com/adrive/v1.0/openFile/list")
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Aliyun Drive list failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Aliyun Drive list failed: {}",
                resp.status()
            )));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Aliyun Drive parse failed: {}", e)))?;
        let mut entries = Vec::new();
        if let Some(items) = data["items"].as_array() {
            for item in items {
                let name = item["name"].as_str().unwrap_or("").to_string();
                let file_type = item["type"].as_str().unwrap_or("");
                let is_dir = file_type == "folder";
                let size = item["size"].as_u64().unwrap_or(0);
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
        // path is file_id for Aliyun Drive
        let body = serde_json::json!({
            "drive_id": self.drive_id,
            "file_id": path
        });
        let resp = self
            .client
            .post("https://openapi.alipan.com/adrive/v1.0/openFile/getDownloadUrl")
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Aliyun Drive url failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Aliyun Drive url failed: {}",
                resp.status()
            )));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Aliyun Drive parse failed: {}", e)))?;
        let url = data["url"]
            .as_str()
            .ok_or_else(|| WikiGenError::Storage("No download URL".to_string()))?;
        let resp =
            self.client.get(url).send().await.map_err(|e| {
                WikiGenError::Storage(format!("Aliyun Drive download failed: {}", e))
            })?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Aliyun Drive download failed: {}",
                resp.status()
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Aliyun Drive read failed: {}", e)))?;
        Ok(bytes.to_vec())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let body = serde_json::json!({
            "drive_id": self.drive_id,
            "file_id": path
        });
        match self
            .client
            .post("https://openapi.alipan.com/adrive/v1.0/openFile/get")
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
        let body = serde_json::json!({
            "drive_id": self.drive_id,
            "file_id": path
        });
        let resp = self
            .client
            .post("https://openapi.alipan.com/adrive/v1.0/openFile/get")
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Aliyun Drive metadata failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Aliyun Drive metadata failed: {}",
                resp.status()
            )));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Aliyun Drive parse failed: {}", e)))?;
        let file_type = data["type"].as_str().unwrap_or("");
        Ok(FileMetadata {
            size: data["size"].as_u64().unwrap_or(0),
            modified: None,
            is_dir: file_type == "folder",
        })
    }

    fn name(&self) -> &str {
        "aliyun_drive"
    }
}
