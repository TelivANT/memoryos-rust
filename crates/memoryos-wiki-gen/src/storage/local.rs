use async_trait::async_trait;
use std::path::PathBuf;
use crate::error::Result;
use super::{StorageConnector, FileEntry, FileMetadata};

/// Local filesystem connector
pub struct LocalConnector {
    root: PathBuf,
}

impl LocalConnector {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl StorageConnector for LocalConnector {
    async fn connect(&mut self) -> Result<()> {
        Ok(())
    }
    
    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let full_path = self.root.join(path);
        let mut entries = Vec::new();
        
        let mut read_dir = tokio::fs::read_dir(&full_path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            let path = entry.path().strip_prefix(&self.root)
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
        let full_path = self.root.join(path);
        Ok(tokio::fs::read(full_path).await?)
    }
    
    async fn exists(&self, path: &str) -> Result<bool> {
        let full_path = self.root.join(path);
        Ok(tokio::fs::try_exists(full_path).await?)
    }
    
    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let full_path = self.root.join(path);
        let meta = tokio::fs::metadata(full_path).await?;
        
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
