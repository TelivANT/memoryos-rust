use super::{FileEntry, FileMetadata, StorageConnector};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Local filesystem connector
pub struct LocalConnector {
    root: PathBuf,
}

impl LocalConnector {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Validate that the resolved path stays within the root directory (path traversal prevention).
    fn validate_path(&self, path: &str) -> Result<PathBuf> {
        let full_path = self.root.join(path);
        let canonical = full_path
            .canonicalize()
            .map_err(crate::error::WikiGenError::Io)?;
        let root_canonical = self
            .root
            .canonicalize()
            .map_err(crate::error::WikiGenError::Io)?;
        if !canonical.starts_with(&root_canonical) {
            return Err(crate::error::WikiGenError::Storage(
                "Path traversal denied: path escapes root directory".to_string(),
            ));
        }
        Ok(canonical)
    }
}

#[async_trait]
impl StorageConnector for LocalConnector {
    async fn connect(&mut self) -> Result<()> {
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let canonical = self.validate_path(path)?;
        let mut entries = Vec::new();

        let mut read_dir = tokio::fs::read_dir(&canonical).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            let path = entry
                .path()
                .strip_prefix(&self.root)
                .unwrap_or(&entry.path())
                .to_string_lossy()
                .to_string();

            entries.push(FileEntry {
                path,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
            });
        }

        Ok(entries)
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let canonical = self.validate_path(path)?;
        Ok(tokio::fs::read(canonical).await?)
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let full_path = self.root.join(path);
        // For exists check, canonicalize may fail if path doesn't exist.
        // Reject obvious traversal attempts, then delegate to try_exists.
        if path.contains("..") {
            return Err(crate::error::WikiGenError::Storage(
                "Path traversal denied".to_string(),
            ));
        }
        Ok(tokio::fs::try_exists(full_path).await?)
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let canonical = self.validate_path(path)?;
        let meta = tokio::fs::metadata(canonical).await?;

        Ok(FileMetadata {
            size: meta.len(),
            modified: meta.modified().ok(),
            is_dir: meta.is_dir(),
        })
    }

    async fn clone_to_temp(&self) -> Result<PathBuf> {
        Ok(self.root.clone())
    }

    fn name(&self) -> &str {
        "local"
    }
}
