use async_trait::async_trait;
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{Result, WikiGenError};

use super::{FileEntry, FileMetadata, StorageConnector};

/// SFTP connector
pub struct SftpConnector {
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    key_path: Option<PathBuf>,
    session: Option<Arc<Session>>,
}

impl SftpConnector {
    pub fn new(host: String, username: String) -> Self {
        Self {
            host,
            port: 22,
            username,
            password: None,
            key_path: None,
            session: None,
        }
    }

    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    pub fn with_key(mut self, key_path: PathBuf) -> Self {
        self.key_path = Some(key_path);
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

#[async_trait]
impl StorageConnector for SftpConnector {
    async fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let username = self.username.clone();
        let password = self.password.clone();
        let key_path = self.key_path.clone();

        let sess =
            tokio::task::spawn_blocking(move || -> std::result::Result<Session, WikiGenError> {
                let tcp = TcpStream::connect(&addr)
                    .map_err(|e| WikiGenError::Storage(format!("SFTP connect failed: {}", e)))?;

                let mut sess = Session::new()
                    .map_err(|e| WikiGenError::Storage(format!("SFTP session failed: {}", e)))?;
                sess.set_tcp_stream(tcp);
                sess.handshake()
                    .map_err(|e| WikiGenError::Storage(format!("SFTP handshake failed: {}", e)))?;

                if let Some(password) = &password {
                    sess.userauth_password(&username, password)
                        .map_err(|e| WikiGenError::Storage(format!("SFTP auth failed: {}", e)))?;
                } else if let Some(key_path) = &key_path {
                    sess.userauth_pubkey_file(&username, None, key_path, None)
                        .map_err(|e| {
                            WikiGenError::Storage(format!("SFTP key auth failed: {}", e))
                        })?;
                }

                Ok(sess)
            })
            .await
            .map_err(|e| WikiGenError::Storage(format!("SFTP spawn_blocking failed: {}", e)))??;

        self.session = Some(Arc::new(sess));
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let sess = self
            .session
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let sftp = sess
            .sftp()
            .map_err(|e| WikiGenError::Storage(format!("SFTP init failed: {}", e)))?;

        let entries = sftp
            .readdir(std::path::Path::new(path))
            .map_err(|e| WikiGenError::Storage(format!("SFTP readdir failed: {}", e)))?;

        Ok(entries
            .into_iter()
            .map(|(path, stat)| FileEntry {
                path: path.to_string_lossy().to_string(),
                is_dir: stat.is_dir(),
                size: stat.size.unwrap_or(0),
            })
            .collect())
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let sess = self
            .session
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let sftp = sess
            .sftp()
            .map_err(|e| WikiGenError::Storage(format!("SFTP init failed: {}", e)))?;

        let mut file = sftp
            .open(std::path::Path::new(path))
            .map_err(|e| WikiGenError::Storage(format!("SFTP open failed: {}", e)))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| WikiGenError::Storage(format!("SFTP read failed: {}", e)))?;

        Ok(contents)
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let sess = self
            .session
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let sftp = sess
            .sftp()
            .map_err(|e| WikiGenError::Storage(format!("SFTP init failed: {}", e)))?;

        Ok(sftp.stat(std::path::Path::new(path)).is_ok())
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let sess = self
            .session
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let sftp = sess
            .sftp()
            .map_err(|e| WikiGenError::Storage(format!("SFTP init failed: {}", e)))?;

        let stat = sftp
            .stat(std::path::Path::new(path))
            .map_err(|e| WikiGenError::Storage(format!("SFTP stat failed: {}", e)))?;

        Ok(FileMetadata {
            size: stat.size.unwrap_or(0),
            modified: stat
                .mtime
                .map(|t| std::time::UNIX_EPOCH + std::time::Duration::from_secs(t)),
            is_dir: stat.is_dir(),
        })
    }

    async fn clone_to_temp(&self) -> Result<PathBuf> {
        Err(WikiGenError::Storage(
            "SFTP does not support clone_to_temp".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "sftp"
    }
}
