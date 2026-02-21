use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// File entry metadata
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

/// File metadata
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub is_dir: bool,
}

/// Storage connector trait
#[async_trait]
pub trait StorageConnector: Send + Sync {
    /// Connect to storage
    async fn connect(&mut self) -> Result<()>;

    /// List files in path
    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>>;

    /// Read file content
    async fn read_file(&self, path: &str) -> Result<Vec<u8>>;

    /// Check if path exists
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Get file metadata
    async fn metadata(&self, path: &str) -> Result<FileMetadata>;

    /// Clone to temp directory (optional)
    async fn clone_to_temp(&self) -> Result<PathBuf>;

    /// Connector name
    fn name(&self) -> &str;
}

mod cos;
mod git;
mod local;
mod obs;
mod oss;
mod s3;
mod sftp;
mod webdav;

#[cfg(test)]
mod tests;

pub use cos::CosConnector;
pub use git::GitConnector;
pub use local::LocalConnector;
pub use obs::ObsConnector;
pub use oss::OssConnector;
pub use s3::S3Connector;
pub use sftp::SftpConnector;
pub use webdav::WebDavConnector;
