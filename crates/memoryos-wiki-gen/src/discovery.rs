use std::path::Path;

use ignore::WalkBuilder;
use tracing::debug;

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

        let walker = WalkBuilder::new(repo_root)
            .hidden(true)
            .git_global(true)
            .git_ignore(true)
            .git_exclude(true)
            .follow_links(false)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let relative = path.strip_prefix(repo_root).unwrap_or(path);
            let rel_str = relative.to_string_lossy();

            if is_extra_excluded(&rel_str, exclude) {
                continue;
            }

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

fn is_extra_excluded(rel_str: &str, patterns: &[String]) -> bool {
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
