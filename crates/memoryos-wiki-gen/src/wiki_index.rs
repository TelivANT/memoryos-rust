use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::cache::compute_hash;
use crate::error::WikiGenResult;
use crate::evidence::EvidenceRef;
use crate::page_builder::GeneratedPage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiIndex {
    pub version: u32,
    pub generated_at: String,
    pub source_commit: String,
    pub pages: Vec<PageIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageIndex {
    pub path: String,
    pub content_hash: String,
    pub symbols_referenced: Vec<String>,
    pub evidence: Vec<EvidenceRef>,
}

impl WikiIndex {
    pub fn build(pages: &[GeneratedPage], source_commit: &str) -> Self {
        let page_indices: Vec<PageIndex> = pages
            .iter()
            .map(|page| PageIndex {
                path: page.path.clone(),
                content_hash: compute_hash(&page.content),
                symbols_referenced: page.symbols_referenced.clone(),
                evidence: Vec::new(),
            })
            .collect();

        Self {
            version: 1,
            generated_at: chrono::Utc::now().to_rfc3339(),
            source_commit: source_commit.to_string(),
            pages: page_indices,
        }
    }

    pub fn save(&self, output_dir: &Path) -> WikiGenResult<()> {
        let index_path = output_dir.join("wiki_index.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&index_path, content)?;
        debug!("Wiki index saved to {}", index_path.display());
        Ok(())
    }
}
