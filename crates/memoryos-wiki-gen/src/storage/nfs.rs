use async_trait::async_trait;
use std::path::PathBuf;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, LocalConnector, StorageConnector};

/// NFS connector via local mount point.
///
/// Expects the NFS share to be mounted at `mount_path` before use.
/// Example: `mount -t nfs server:/export/path /mnt/nfs`
pub struct NfsConnector {
    inner: LocalConnector,
    mount_path: PathBuf,
    server: String,
    export_path: String,
}

impl NfsConnector {
    pub fn new(server: String, export_path: String, mount_path: PathBuf) -> Self {
        let inner = LocalConnector::new(mount_path.clone());
        Self {
            inner,
            mount_path,
            server,
            export_path,
        }
    }
}

#[async_trait]
impl StorageConnector for NfsConnector {
    async fn connect(&mut self) -> Result<()> {
        if !self.mount_path.exists() {
            return Err(WikiGenError::Storage(format!(
                "NFS mount path does not exist: {}. Mount {}:{} first.",
                self.mount_path.display(),
                self.server,
                self.export_path
            )));
        }
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
        "nfs"
    }
}
