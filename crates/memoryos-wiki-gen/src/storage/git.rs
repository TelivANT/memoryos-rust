use super::{FileEntry, FileMetadata, StorageConnector};
use crate::error::{Result, WikiGenError};
use async_trait::async_trait;
use git2::{Cred, FetchOptions, RemoteCallbacks};
use std::path::PathBuf;

/// Git authentication
pub enum GitAuth {
    None,
    Token(String),
    SshKey {
        private_key: PathBuf,
        passphrase: Option<String>,
    },
}

/// Git connector
pub struct GitConnector {
    url: String,
    branch: String,
    auth: GitAuth,
    repo_path: Option<PathBuf>,
}

impl GitConnector {
    pub fn new(url: String) -> Self {
        Self {
            url,
            branch: "main".to_string(),
            auth: GitAuth::None,
            repo_path: None,
        }
    }

    pub fn with_branch(mut self, branch: String) -> Self {
        self.branch = branch;
        self
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.auth = GitAuth::Token(token);
        self
    }

    pub fn with_ssh_key(mut self, private_key: PathBuf, passphrase: Option<String>) -> Self {
        self.auth = GitAuth::SshKey {
            private_key,
            passphrase,
        };
        self
    }
}

#[async_trait]
impl StorageConnector for GitConnector {
    async fn connect(&mut self) -> Result<()> {
        let temp_dir = std::env::temp_dir().join(format!("wiki-gen-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await?;

        let url = self.url.clone();
        let branch = self.branch.clone();
        let auth = match &self.auth {
            GitAuth::None => GitAuth::None,
            GitAuth::Token(t) => GitAuth::Token(t.clone()),
            GitAuth::SshKey {
                private_key,
                passphrase,
            } => GitAuth::SshKey {
                private_key: private_key.clone(),
                passphrase: passphrase.clone(),
            },
        };

        let repo_path = tokio::task::spawn_blocking(move || -> Result<PathBuf> {
            let mut callbacks = RemoteCallbacks::new();

            callbacks.credentials(move |_url, username, _allowed| match &auth {
                GitAuth::Token(token) => Cred::userpass_plaintext("git", token),
                GitAuth::SshKey {
                    private_key,
                    passphrase,
                } => Cred::ssh_key(
                    username.unwrap_or("git"),
                    None,
                    private_key,
                    passphrase.as_deref(),
                ),
                GitAuth::None => Cred::default(),
            });

            let mut fetch_opts = FetchOptions::new();
            fetch_opts.remote_callbacks(callbacks);

            let mut builder = git2::build::RepoBuilder::new();
            builder.fetch_options(fetch_opts);
            builder.branch(&branch);

            builder
                .clone(&url, &temp_dir)
                .map_err(|e| WikiGenError::Storage(format!("Git clone failed: {}", e)))?;

            Ok(temp_dir)
        })
        .await
        .map_err(|e| WikiGenError::Storage(format!("Task join error: {}", e)))??;

        self.repo_path = Some(repo_path);
        Ok(())
    }

    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>> {
        let repo_path = self
            .repo_path
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let full_path = repo_path.join(path);
        let mut entries = Vec::new();

        let mut read_dir = tokio::fs::read_dir(&full_path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            let path = entry
                .path()
                .strip_prefix(repo_path)
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
        let repo_path = self
            .repo_path
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let full_path = repo_path.join(path);
        Ok(tokio::fs::read(full_path).await?)
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let repo_path = self
            .repo_path
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let full_path = repo_path.join(path);
        Ok(tokio::fs::try_exists(full_path).await?)
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let repo_path = self
            .repo_path
            .as_ref()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()))?;

        let full_path = repo_path.join(path);
        let meta = tokio::fs::metadata(full_path).await?;

        Ok(FileMetadata {
            size: meta.len(),
            modified: meta.modified().ok(),
            is_dir: meta.is_dir(),
        })
    }

    async fn clone_to_temp(&self) -> Result<PathBuf> {
        self.repo_path
            .clone()
            .ok_or_else(|| WikiGenError::Storage("Not connected".to_string()).into())
    }

    fn name(&self) -> &str {
        "git"
    }
}
