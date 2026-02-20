use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::debug;

use crate::error::{WikiGenError, WikiGenResult};
use crate::evidence::LlmDocResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStore {
    pub version: u32,
    pub entries: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub prompt_hash: String,
    pub content_hash: String,
    pub result: LlmDocResult,
    pub generated_at: String,
}

impl Default for CacheStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheStore {
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: HashMap::new(),
        }
    }

    pub fn load(cache_dir: &Path) -> WikiGenResult<Self> {
        let cache_file = cache_dir.join("cache.json");
        if !cache_file.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&cache_file)
            .map_err(|e| WikiGenError::Cache(format!("Failed to read cache: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| WikiGenError::Cache(format!("Failed to parse cache: {}", e)))
    }

    pub fn save(&self, cache_dir: &Path) -> WikiGenResult<()> {
        std::fs::create_dir_all(cache_dir)?;
        let cache_file = cache_dir.join("cache.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&cache_file, content)?;
        debug!("Cache saved to {}", cache_file.display());
        Ok(())
    }

    pub fn lookup(&self, symbol_id_hash: &str, prompt_hash: &str) -> Option<&LlmDocResult> {
        self.entries.get(symbol_id_hash).and_then(|entry| {
            if entry.prompt_hash == prompt_hash {
                Some(&entry.result)
            } else {
                None
            }
        })
    }

    pub fn insert(
        &mut self,
        symbol_id_hash: String,
        prompt_hash: String,
        content_hash: String,
        result: LlmDocResult,
    ) {
        let now = chrono::Utc::now().to_rfc3339();
        self.entries.insert(
            symbol_id_hash,
            CacheEntry {
                prompt_hash,
                content_hash,
                result,
                generated_at: now,
            },
        );
    }
}

pub fn compute_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn compute_file_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}
