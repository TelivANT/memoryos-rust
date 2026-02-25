#![cfg(feature = "s3")]

use async_trait::async_trait;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::Client;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, StorageConnector};

/// S3-compatible storage connector
pub struct S3Connector {
    client: Option<Client>,
    bucket: String,
    prefix: String,
    region: String,
    endpoint: Option<String>,
    access_key: String,
    secret_key: String,
}

impl S3Connector {
    pub fn new(bucket: String, region: String, access_key: String, secret_key: String) -> Self {
        Self {
            client: None,
            bucket,
            prefix: String::new(),
            region,
            endpoint: None,
            access_key,
            secret_key,
        }
    }

    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = prefix;
        self
    }

    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }
}

#[async_trait]
impl StorageConnector for S3Connector {
    async fn connect(&mut self) -> Result<()> {
        let creds = Credentials::new(
            &self.access_key,
            &self.secret_key,
            None,
            None,
            "s3-connector",
        );

        let region = Region::new(self.region.clone());
        let mut config_builder = aws_sdk_s3::Config::builder()
            .credentials_provider(creds)
            .region(region);

        if let Some(endpoint) = &self.endpoint {
            config_builder = config_builder.endpoint_url(endpoint);
        }

        let config = config_builder.build();
        self.client = Some(Client::from_conf(config));
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let prefix = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.prefix, path)
        };

        let resp = client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("S3 list failed: {}", e)))?;

        let mut entries = Vec::new();
        for obj in resp.contents() {
            if let Some(key) = obj.key() {
                let path = key.strip_prefix(&self.prefix).unwrap_or(key).to_string();
                entries.push(FileEntry {
                    path,
                    is_dir: key.ends_with('/'),
                    size: obj.size().unwrap_or(0) as u64,
                });
            }
        }

        Ok(entries)
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let key = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.prefix, path)
        };

        let resp = client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("S3 get failed: {}", e)))?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| WikiGenError::Storage(format!("S3 read failed: {}", e)))?;

        Ok(data.into_bytes().to_vec())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let key = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.prefix, path)
        };

        match client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let key = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.prefix, path)
        };

        let resp = client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| WikiGenError::Storage(format!("S3 head failed: {}", e)))?;

        Ok(FileMetadata {
            size: resp.content_length().unwrap_or(0) as u64,
            modified: resp.last_modified().and_then(|dt| {
                std::time::SystemTime::UNIX_EPOCH
                    .checked_add(std::time::Duration::from_secs(dt.secs() as u64))
            }),
            is_dir: false,
        })
    }

    fn name(&self) -> &str {
        "s3"
    }
}
