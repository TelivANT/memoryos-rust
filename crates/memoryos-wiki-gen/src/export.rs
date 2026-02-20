use std::path::Path;

use tracing::{debug, info};

use crate::cache::compute_hash;
use crate::error::WikiGenResult;
use crate::page_builder::GeneratedPage;
use crate::wiki_index::WikiIndex;

pub struct ExportOrchestrator;

impl ExportOrchestrator {
    pub fn export_local(
        pages: &[GeneratedPage],
        wiki_index: &WikiIndex,
        output_dir: &Path,
    ) -> WikiGenResult<()> {
        std::fs::create_dir_all(output_dir)?;

        let modules_dir = output_dir.join("modules");
        std::fs::create_dir_all(&modules_dir)?;

        let faq_dir = output_dir.join("faq");
        std::fs::create_dir_all(&faq_dir)?;

        for page in pages {
            let page_path = output_dir.join(&page.path);

            if let Some(parent) = page_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&page_path, &page.content)?;
            debug!("Exported: {}", page_path.display());
        }

        wiki_index.save(output_dir)?;

        info!("Exported {} pages to {}", pages.len(), output_dir.display());

        Ok(())
    }

    pub fn should_skip_page(page: &GeneratedPage, output_dir: &Path) -> bool {
        let page_path = output_dir.join(&page.path);
        if !page_path.exists() {
            return false;
        }

        if let Ok(existing) = std::fs::read_to_string(&page_path) {
            let existing_hash = compute_hash(&existing);
            let new_hash = compute_hash(&page.content);
            existing_hash == new_hash
        } else {
            false
        }
    }

    pub fn export_incremental(
        pages: &[GeneratedPage],
        wiki_index: &WikiIndex,
        output_dir: &Path,
    ) -> WikiGenResult<usize> {
        std::fs::create_dir_all(output_dir)?;

        let modules_dir = output_dir.join("modules");
        std::fs::create_dir_all(&modules_dir)?;

        let mut exported_count = 0;

        for page in pages {
            if Self::should_skip_page(page, output_dir) {
                debug!("Skipping unchanged: {}", page.path);
                continue;
            }

            let page_path = output_dir.join(&page.path);
            if let Some(parent) = page_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&page_path, &page.content)?;
            debug!("Exported (updated): {}", page_path.display());
            exported_count += 1;
        }

        wiki_index.save(output_dir)?;

        info!(
            "Incremental export: {}/{} pages updated in {}",
            exported_count,
            pages.len(),
            output_dir.display()
        );

        Ok(exported_count)
    }
}
