use async_trait::async_trait;
use reqwest::Client;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, StorageConnector};

/// Azure Blob Storage connector (REST API)
pub struct AzureBlobConnector {
    client: Client,
    account: String,
    container: String,
    sas_token: String,
    prefix: String,
}

impl AzureBlobConnector {
    pub fn new(account: String, container: String, sas_token: String) -> Self {
        Self {
            client: Client::new(),
            account,
            container,
            sas_token,
            prefix: String::new(),
        }
    }

    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = prefix;
        self
    }

    fn base_url(&self) -> String {
        format!(
            "https://{}.blob.core.windows.net/{}",
            self.account, self.container
        )
    }

    fn build_url(&self, path: &str) -> String {
        let key = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.prefix, path)
        };
        format!("{}{}?{}", self.base_url(), key, self.sas_token)
    }
}

#[async_trait]
impl StorageConnector for AzureBlobConnector {
    async fn connect(&mut self) -> Result<()> {
        // Validate SAS token by listing (comp=list returns 200 if valid)
        let url = format!(
            "{}?restype=container&comp=list&maxresults=1&{}",
            self.base_url(),
            self.sas_token
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Azure connect failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Azure auth failed: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let prefix = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.prefix, path)
        };
        let url = format!(
            "{}?restype=container&comp=list&prefix={}&{}",
            self.base_url(),
            prefix,
            self.sas_token
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Azure list failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Azure list failed: {}",
                resp.status()
            )));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Azure read failed: {}", e)))?;

        // Parse XML response for blob names
        let mut entries = Vec::new();
        let mut reader = quick_xml::Reader::from_str(&body);
        let mut in_name = false;
        let mut in_size = false;
        let mut name_buf = String::new();
        let mut size_buf = String::new();

        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(e)) => {
                    let local = e.local_name();
                    if local.as_ref() == b"Name" {
                        in_name = true;
                        name_buf.clear();
                    } else if local.as_ref() == b"Content-Length" {
                        in_size = true;
                        size_buf.clear();
                    }
                }
                Ok(quick_xml::events::Event::Text(e)) => {
                    if let Ok(text) = e.unescape() {
                        if in_name {
                            name_buf.push_str(&text);
                        } else if in_size {
                            size_buf.push_str(&text);
                        }
                    }
                }
                Ok(quick_xml::events::Event::End(e)) => {
                    let local = e.local_name();
                    if local.as_ref() == b"Name" && in_name {
                        in_name = false;
                        let path = name_buf
                            .strip_prefix(&self.prefix)
                            .unwrap_or(&name_buf)
                            .trim_start_matches('/')
                            .to_string();
                        let size = size_buf.parse().unwrap_or(0);
                        if !path.is_empty() {
                            entries.push(FileEntry {
                                is_dir: path.ends_with('/'),
                                path,
                                size,
                            });
                        }
                        size_buf.clear();
                    } else if local.as_ref() == b"Content-Length" {
                        in_size = false;
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => {
                    return Err(WikiGenError::Storage(format!(
                        "Azure XML parse error: {}",
                        e
                    )));
                }
                _ => {}
            }
        }
        Ok(entries)
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let url = self.build_url(path);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Azure get failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Azure get failed: {}",
                resp.status()
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Azure read failed: {}", e)))?;
        Ok(bytes.to_vec())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let url = self.build_url(path);
        match self.client.head(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let url = self.build_url(path);
        let resp = self
            .client
            .head(&url)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("Azure head failed: {}", e)))?;
        if !resp.status().is_success() {
            return Err(WikiGenError::Storage(format!(
                "Azure head failed: {}",
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

    fn name(&self) -> &str {
        "azure_blob"
    }
}
