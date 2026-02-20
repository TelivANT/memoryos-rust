//! FAQ 功能模块

pub mod auto_promoter;
pub mod heat_tracker;
pub mod wiki_exporter;

pub use auto_promoter::{
    AutoPromoter, AutoPromotionConfig, PromotionRecord, PromotionResult, PromotionStats,
};
pub use heat_tracker::{HeatConfig, HeatStats, HeatTracker};
pub use wiki_exporter::{
    ExportResult, ExportTarget, WikiExportBackend, WikiExportConfig, WikiExporter,
};
