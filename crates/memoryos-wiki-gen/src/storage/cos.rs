use async_trait::async_trait;
use std::path::PathBuf;

use crate::error::Result;

use super::{FileEntry, FileMetadata, S3Connector, StorageConnector};

/// Tencent Cloud COS connector (S3-compatible)
pub struct CosConnector {
    inner: S3Connector,
}

impl CosConnector {
    pub fn new(region: String, bucket: String, secret_id: String, secret_key: String) -> Self {
        let endpoint = format!("https://{}.cos.{}.myqcloud.com", bucket, region);
        let inner = S3Connector::new(bucket, region, secret_id, secret_key).with_endpoint(endpoint);
        Self { inner }
    }

    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.inner = self.inner.with_prefix(prefix);
        self
    }
}

#[async_trait]
impl StorageConnector for CosConnector {
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
        "cos"
    }
}
