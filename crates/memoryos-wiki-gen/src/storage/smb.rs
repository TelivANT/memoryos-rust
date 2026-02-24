use async_trait::async_trait;
use std::path::PathBuf;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, LocalConnector, StorageConnector};

/// SMB/CIFS connector via local mount point.
///
/// Expects the SMB share to be mounted at `mount_path` before use.
/// On Linux: `mount -t cifs //server/share /mnt/smb -o username=user,password=pass`
/// On macOS: `mount_smbfs //user:pass@server/share /mnt/smb`
pub struct SmbConnector {
    inner: LocalConnector,
    mount_path: PathBuf,
    server: String,
    share: String,
}

impl SmbConnector {
    pub fn new(server: String, share: String, mount_path: PathBuf) -> Self {
        let inner = LocalConnector::new(mount_path.clone());
        Self {
            inner,
            mount_path,
            server,
            share,
        }
    }
}

#[async_trait]
impl StorageConnector for SmbConnector {
    async fn connect(&mut self) -> Result<()> {
        if !self.mount_path.exists() {
            return Err(WikiGenError::Storage(format!(
                "SMB mount path does not exist: {}. Mount //{}/{} first.",
                self.mount_path.display(),
                self.server,
                self.share
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
        "smb"
    }
}
