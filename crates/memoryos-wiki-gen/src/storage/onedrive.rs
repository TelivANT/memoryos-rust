use async_trait::async_trait;
use reqwest::Client;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, StorageConnector};

/// Microsoft OneDrive connector (Graph API)
pub struct OneDriveConnector {
    client: Client,
    access_token: String,
    drive_id: Option<String>,
}

impl OneDriveConnector {
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
            drive_id: None,
        }
    }

    pub fn with_drive_id(mut self, drive_id: String) -> Self {
        self.drive_id = Some(drive_id);
        self
    }

    fn api_base(&self) -> String {
        match &self.drive_id {
            Some(id) => format!("https://graph.microsoft.com/v1.0/drives/{}", id),
            None => "https://graph.microsoft.com/v1.0/me/drive".to_string(),
        }
    }
}

#[async_trait]
impl StorageConnector for OneDriveConnector {
    async fn connect(&mut self) -> Result<()> {
        let url = format!("{}/root", self.api_base());
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("OneDrive connect failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "OneDrive auth failed: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let url = if path.is_empty() || path == "/" {
            format!("{}/root/children", self.api_base())
        } else {
            let encoded = path.trim_matches('/');
            format!("{}/root:/{}:/children", self.api_base(), encoded)
        };
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("OneDrive list failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "OneDrive list failed: {}",
                resp.status()
            )));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("OneDrive parse failed: {}", e)))?;
        let mut entries = Vec::new();
        if let Some(items) = body["value"].as_array() {
            for item in items {
                let name = item["name"].as_str().unwrap_or("").to_string();
                let is_dir = item.get("folder").is_some();
                let size = item["size"].as_u64().unwrap_or(0);
                entries.push(FileEntry {
                    path: if path.is_empty() || path == "/" {
                        name
                    } else {
                        format!("{}/{}", path.trim_matches('/'), name)
                    },
                    is_dir,
                    size,
                });
            }
        }
        Ok(entries)
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let encoded = path.trim_matches('/');
        let url = format!("{}/root:/{}:/content", self.api_base(), encoded);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("OneDrive get failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "OneDrive get failed: {}",
                resp.status()
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| WikiGenError::Storage(format!("OneDrive read failed: {}", e)))?;
        Ok(bytes.to_vec())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let encoded = path.trim_matches('/');
        let url = format!("{}/root:/{}", self.api_base(), encoded);
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
        let encoded = path.trim_matches('/');
        let url = format!("{}/root:/{}", self.api_base(), encoded);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("OneDrive metadata failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "OneDrive metadata failed: {}",
                resp.status()
            )));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("OneDrive parse failed: {}", e)))?;
        Ok(FileMetadata {
            size: body["size"].as_u64().unwrap_or(0),
            modified: None,
            is_dir: body.get("folder").is_some(),
        })
    }

    fn name(&self) -> &str {
        "onedrive"
    }
}
