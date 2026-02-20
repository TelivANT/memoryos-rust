//! Wiki exporter - delegates to the FAQ WikiExporter for actual export logic.
//!
//! This module bridges the adapter-layer WikiAdapter trait with the core FAQ
//! wiki export pipeline. For the full export implementation (filtering,
//! categorization, markdown generation, S3/Confluence backends), see
//! `memoryos_core::faq::wiki_exporter`.

use crate::faq::{WikiExportConfig, WikiExporter as FaqWikiExporter};
use crate::memory::MidTermSegment;
use crate::AppError;
use tracing::info;

pub struct WikiExporter {
    inner: FaqWikiExporter,
}

impl WikiExporter {
    pub fn new(config: WikiExportConfig) -> Self {
        Self {
            inner: FaqWikiExporter::new(config),
        }
    }

    pub async fn run_export(&self, segments: &[MidTermSegment]) -> Result<usize, AppError> {
        info!("Starting Wiki Export job...");

        let exportable = self.inner.filter_exportable(segments);

        if exportable.is_empty() {
            info!("No exportable FAQ segments found");
            return Ok(0);
        }

        let categories = self.inner.categorize(exportable);
        let markdown = self.inner.generate_markdown(categories);

        match self.inner.export(markdown).await {
            Ok(result) => {
                info!(
                    "Wiki Export complete: {} items to {}",
                    result.exported_count, result.target
                );
                Ok(result.exported_count)
            }
            Err(e) => Err(AppError::ExternalService(format!(
                "Wiki export failed: {}",
                e
            ))),
        }
    }
}
