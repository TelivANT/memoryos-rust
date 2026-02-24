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

mod aliyun_drive;
mod azure_blob;
mod baidu_pan;
mod dropbox;
mod gcs;
mod google_drive;
mod nfs;
mod onedrive;
mod smb;

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

pub use aliyun_drive::AliyunDriveConnector;
pub use azure_blob::AzureBlobConnector;
pub use baidu_pan::BaiduPanConnector;
pub use dropbox::DropboxConnector;
pub use gcs::GcsConnector;
pub use google_drive::GoogleDriveConnector;
pub use nfs::NfsConnector;
pub use onedrive::OneDriveConnector;
pub use smb::SmbConnector;
