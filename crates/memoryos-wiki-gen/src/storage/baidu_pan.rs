use async_trait::async_trait;
use reqwest::Client;
use std::path::PathBuf;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, StorageConnector};

/// Baidu Pan (百度网盘) connector (Open Platform API)
pub struct BaiduPanConnector {
    client: Client,
    access_token: String,
    app_path: String,
}

impl BaiduPanConnector {
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
            app_path: "/apps/memoryos".to_string(),
        }
    }

    pub fn with_app_path(mut self, app_path: String) -> Self {
        self.app_path = app_path;
        self
    }

    fn full_path(&self, path: &str) -> String {
        if path.is_empty() || path == "/" {
            self.app_path.clone()
        } else {
            format!("{}/{}", self.app_path, path.trim_matches('/'))
        }
    }
}

#[async_trait]
impl StorageConnector for BaiduPanConnector {
    async fn connect(&mut self) -> Result<()> {
        let url = format!(
            "https://pan.baidu.com/rest/2.0/xpan/nas?method=uinfo&access_token={}",
            self.access_token
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan connect failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Baidu Pan auth failed: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let dir = self.full_path(path);
        let url = format!(
            "https://pan.baidu.com/rest/2.0/xpan/file?method=list&access_token={}&dir={}",
            self.access_token,
            urlencoding::encode(&dir)
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan list failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Baidu Pan list failed: {}",
                resp.status()
            )));
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan parse failed: {}", e)))?;
        let mut entries = Vec::new();
        if let Some(list) = data["list"].as_array() {
            for item in list {
                let server_filename = item["server_filename"].as_str().unwrap_or("").to_string();
                let is_dir = item["isdir"].as_u64().unwrap_or(0) == 1;
                let size = item["size"].as_u64().unwrap_or(0);
                entries.push(FileEntry {
                    path: server_filename,
                    is_dir,
                    size,
                });
            }
        }
        Ok(entries)
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        // Baidu Pan requires fs_id for download; use filemetas to get dlink
        let full = self.full_path(path);
        // Step 1: get fs_id
        let meta_url = format!(
            "https://pan.baidu.com/rest/2.0/xpan/file?method=list&access_token={}&dir={}",
            self.access_token,
            urlencoding::encode(
                &full
                    .rsplit_once('/')
                    .map(|(d, _)| d.to_string())
                    .unwrap_or_default()
            )
        );
        let resp = self
            .client
            .get(&meta_url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan meta failed: {}", e)))?;
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan parse failed: {}", e)))?;
        let filename = full.rsplit_once('/').map(|(_, f)| f).unwrap_or(&full);
        let fs_id = data["list"]
            .as_array()
            .and_then(|list| {
                list.iter()
                    .find(|item| item["server_filename"].as_str().unwrap_or("") == filename)
            })
            .and_then(|item| item["fs_id"].as_u64())
            .ok_or_else(|| WikiGenError::Storage("File not found on Baidu Pan".to_string()))?;

        // Step 2: get dlink
        let dlink_url = format!(
            "https://pan.baidu.com/rest/2.0/xpan/multimedia?method=filemetas&access_token={}&fsids=[{}]&dlink=1",
            self.access_token, fs_id
        );
        let resp = self
            .client
            .get(&dlink_url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan dlink failed: {}", e)))?;
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan parse failed: {}", e)))?;
        let dlink = data["list"]
            .as_array()
            .and_then(|l| l.first())
            .and_then(|item| item["dlink"].as_str())
            .ok_or_else(|| WikiGenError::Storage("No download link".to_string()))?;

        // Step 3: download
        let download_url = format!("{}&access_token={}", dlink, self.access_token);
        let resp = self
            .client
            .get(&download_url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan download failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Baidu Pan download failed: {}",
                resp.status()
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan read failed: {}", e)))?;
        Ok(bytes.to_vec())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let full = self.full_path(path);
        let parent = full
            .rsplit_once('/')
            .map(|(d, _)| d.to_string())
            .unwrap_or_default();
        let filename = full.rsplit_once('/').map(|(_, f)| f).unwrap_or(&full);
        let url = format!(
            "https://pan.baidu.com/rest/2.0/xpan/file?method=list&access_token={}&dir={}",
            self.access_token,
            urlencoding::encode(&parent)
        );
        let resp = self.client.get(&url).send().await;
        match resp {
            Ok(r) if r.status().is_success() => {
                let data: serde_json::Value = r.json().await.unwrap_or_default();
                Ok(data["list"]
                    .as_array()
                    .map(|list| {
                        list.iter()
                            .any(|item| item["server_filename"].as_str().unwrap_or("") == filename)
                    })
                    .unwrap_or(false))
            }
            _ => Ok(false),
        }
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        // Use list on parent to get file info
        let full = self.full_path(path);
        let parent = full
            .rsplit_once('/')
            .map(|(d, _)| d.to_string())
            .unwrap_or_default();
        let filename = full.rsplit_once('/').map(|(_, f)| f).unwrap_or(&full);
        let url = format!(
            "https://pan.baidu.com/rest/2.0/xpan/file?method=list&access_token={}&dir={}",
            self.access_token,
            urlencoding::encode(&parent)
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan metadata failed: {}", e)))?;
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Baidu Pan parse failed: {}", e)))?;
        let item = data["list"]
            .as_array()
            .and_then(|list| {
                list.iter()
                    .find(|item| item["server_filename"].as_str().unwrap_or("") == filename)
            })
            .ok_or_else(|| WikiGenError::Storage("File not found".to_string()))?;
        Ok(FileMetadata {
            size: item["size"].as_u64().unwrap_or(0),
            modified: None,
            is_dir: item["isdir"].as_u64().unwrap_or(0) == 1,
        })
    }

    async fn clone_to_temp(&self) -> Result<PathBuf> {
        Err(WikiGenError::Storage(
            "Baidu Pan does not support clone_to_temp".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "baidu_pan"
    }
}
