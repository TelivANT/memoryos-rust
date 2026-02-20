use std::path::Path;

use tracing::debug;
use walkdir::WalkDir;

use crate::config::WikiGenConfig;
use crate::error::WikiGenResult;
use crate::lang::DiscoveredFile;

pub struct FileDiscovery {
    config: WikiGenConfig,
}

impl FileDiscovery {
    pub fn new(config: WikiGenConfig) -> Self {
        Self { config }
    }

    pub fn discover(&self, repo_root: &Path) -> WikiGenResult<Vec<DiscoveredFile>> {
        let mut files = Vec::new();
        let max_size = self.config.parse.max_file_size_kb * 1024;
        let exclude = &self.config.repo.exclude_patterns;

        for entry in WalkDir::new(repo_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_hidden(e) && !is_excluded_dir(e, repo_root, exclude))
            .flatten()
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let relative = path.strip_prefix(repo_root).unwrap_or(path);

            if let Ok(meta) = path.metadata() {
                if meta.len() as usize > max_size {
                    debug!(
                        "Skipping large file: {} ({} bytes)",
                        relative.display(),
                        meta.len()
                    );
                    continue;
                }
            }

            if let Some(discovered) = crate::lang::detect_file(relative, &self.config.parse) {
                files.push(DiscoveredFile {
                    absolute_path: path.to_path_buf(),
                    relative_path: relative.to_path_buf(),
                    file_type: discovered.file_type,
                });
            }
        }

        debug!(
            "Discovered {} parseable files in {}",
            files.len(),
            repo_root.display()
        );
        Ok(files)
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.') && s != ".")
        .unwrap_or(false)
}

fn is_excluded_dir(entry: &walkdir::DirEntry, repo_root: &Path, patterns: &[String]) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    let name = entry.file_name().to_string_lossy();

    let always_skip = [
        "node_modules",
        "target",
        "__pycache__",
        ".git",
        "vendor",
        "dist",
        "build",
        ".venv",
        "venv",
    ];
    if always_skip.contains(&name.as_ref()) {
        return true;
    }

    let relative = entry.path().strip_prefix(repo_root).unwrap_or(entry.path());
    let rel_str = relative.to_string_lossy();

    for pattern in patterns {
        let trimmed = pattern
            .trim_start_matches("**/")
            .trim_end_matches("/**")
            .trim_end_matches("/*");
        if rel_str.contains(trimmed) {
            return true;
        }
    }

    false
}
