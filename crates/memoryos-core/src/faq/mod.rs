//! FAQ 功能模块

pub mod auto_promoter;
pub mod heat_tracker;

pub use auto_promoter::{
    AutoPromotionConfig, AutoPromoter, PromotionRecord, PromotionResult, PromotionStats,
};
pub use heat_tracker::{HeatConfig, HeatStats, HeatTracker};
