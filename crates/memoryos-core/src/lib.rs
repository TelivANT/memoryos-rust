pub mod config;
pub mod error;
pub mod faq;
pub mod health;
pub mod history;
pub mod identity;
pub mod llm;
pub mod memory;
pub mod optimization;
pub mod security;
pub mod wiki;

pub use config::{AppConfig, ConfigManager};
pub use error::AppError;
pub use faq::{
    AutoPromoter, AutoPromotionConfig, ExportResult, ExportTarget, HeatConfig, HeatStats,
    HeatTracker, PromotionRecord, PromotionResult, PromotionStats, WikiExportBackend,
    WikiExportConfig, WikiExporter,
};
pub use health::{DependencyState, HealthMode, HealthStatus};
pub use history::{HistoryEventType, MemoryHistoryEntry};
pub use identity::PrincipalContext;
pub use memory::*;
pub use optimization::{
    BatchEmbedder, BloomFilter, EmbeddingCache, HeatBuffer, IncrementalSummarizer,
    OptimizedFaqMatcher, OptimizedRetriever, SimilarityFilter,
};
pub use security::{
    AuditConfig, AuditEvent, AuditEventType, AuditLogger, AuditOutcome, DataEncryptor,
    EncryptedPayload, EncryptionConfig, GdprManager,
};
