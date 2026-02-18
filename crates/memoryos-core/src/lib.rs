pub mod config;
pub mod error;
pub mod faq;
pub mod health;
pub mod history;
pub mod identity;
pub mod llm;
pub mod memory;
pub mod security;
pub mod wiki;

pub use config::{AppConfig, ConfigManager};
pub use error::AppError;
pub use faq::{
    AutoPromotionConfig, AutoPromoter, HeatConfig, HeatStats, HeatTracker, PromotionRecord,
    PromotionResult, PromotionStats,
};
pub use health::{DependencyState, HealthMode, HealthStatus};
pub use history::{HistoryEventType, MemoryHistoryEntry};
pub use identity::PrincipalContext;
pub use memory::*;
