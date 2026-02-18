//! History storage port

use async_trait::async_trait;
use memoryos_core::{AppError, MemoryHistoryEntry};

/// History storage trait
#[async_trait]
pub trait HistoryStorage: Send + Sync {
    /// Add a history entry
    async fn add_entry(&self, entry: MemoryHistoryEntry) -> Result<(), AppError>;

    /// Get history for a memory
    async fn get_history(&self, memory_id: &str) -> Result<Vec<MemoryHistoryEntry>, AppError>;

    /// Get a specific history entry
    async fn get_entry(&self, id: &str) -> Result<Option<MemoryHistoryEntry>, AppError>;
}
