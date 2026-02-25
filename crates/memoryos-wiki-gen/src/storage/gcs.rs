#![cfg(feature = "s3")]

use async_trait::async_trait;
use std::path::PathBuf;

use crate::error::Result;

use super::{FileEntry, FileMetadata, S3Connector, StorageConnector};

/// Google Cloud Storage connector (S3-compatible interop)
pub struct GcsConnector {
    inner: S3Connector,
}

impl GcsConnector {
    pub fn new(bucket: String, access_key: String, secret_key: String) -> Self {
        let endpoint = "https://storage.googleapis.com".to_string();
        let inner = S3Connector::new(bucket, "auto".to_string(), access_key, secret_key)
            .with_endpoint(endpoint);
        Self { inner }
    }

    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.inner = self.inner.with_prefix(prefix);
        self
    }
}

#[async_trait]
impl StorageConnector for GcsConnector {
    async fn connect(&mut self) -> Result<()> {
        self.inner.connect().await
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        self.inner.list_files(path).await
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        self.inner.read_file(path).await
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        self.inner.exists(path).await
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        self.inner.metadata(path).await
    }

    async fn clone_to_temp(&self) -> Result<PathBuf> {
        self.inner.clone_to_temp().await
    }

    fn name(&self) -> &str {
        "gcs"
    }
}
