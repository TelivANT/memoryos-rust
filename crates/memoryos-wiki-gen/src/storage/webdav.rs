use async_trait::async_trait;
use reqwest::Client;
use std::path::PathBuf;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, StorageConnector};

/// WebDAV connector
pub struct WebDavConnector {
    client: Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
}

impl WebDavConnector {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            username: None,
            password: None,
        }
    }

    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }

    fn build_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/{}", self.base_url, path)
    }
}

#[async_trait]
impl StorageConnector for WebDavConnector {
    async fn connect(&mut self) -> Result<()> {
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let url = self.build_url(path);
        let mut req = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url);

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            req = req.basic_auth(user, Some(pass));
        }

        req = req.header("Depth", "1");

        let resp = req
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("WebDAV PROPFIND failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "WebDAV PROPFIND failed: {}",
                resp.status()
            )));
        }

        // Simple parsing - in production, use proper XML parser
        let body = resp
            .text()
            .await
            .map_err(|e| WikiGenError::Storage(format!("WebDAV read failed: {}", e)))?;

        let mut entries = Vec::new();
        // Basic XML parsing (simplified)
        for line in body.lines() {
            if line.contains("<d:href>") {
                if let Some(href) = line
                    .split("<d:href>")
                    .nth(1)
                    .and_then(|s| s.split("</d:href>").next())
                {
                    let path = href.trim_start_matches('/').to_string();
                    if !path.is_empty() && path != path.trim_end_matches('/') {
                        entries.push(FileEntry {
                            path,
                            is_dir: href.ends_with('/'),
                            size: 0,
                        });
                    }
                }
            }
        }

        Ok(entries)
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let url = self.build_url(path);
        let mut req = self.client.get(&url);

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            req = req.basic_auth(user, Some(pass));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("WebDAV GET failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "WebDAV GET failed: {}",
                resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| WikiGenError::Storage(format!("WebDAV read failed: {}", e)))?;

        Ok(bytes.to_vec())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let url = self.build_url(path);
        let mut req = self.client.head(&url);

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            req = req.basic_auth(user, Some(pass));
        }

        match req.send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let url = self.build_url(path);
        let mut req = self.client.head(&url);

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            req = req.basic_auth(user, Some(pass));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("WebDAV HEAD failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "WebDAV HEAD failed: {}",
                resp.status()
            )));
        }

        let size = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Ok(FileMetadata {
            size,
            modified: None,
            is_dir: false,
        })
    }

    async fn clone_to_temp(&self) -> Result<PathBuf> {
        Err(WikiGenError::Storage(
            "WebDAV does not support clone_to_temp".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "webdav"
    }
}
