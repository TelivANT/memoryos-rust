//! Memory Storage Port

use async_trait::async_trait;
use memoryos_core::{AppError, LongTermMemory, MemoryContext, Message, MidTermSegment};

/// Short-term memory storage (Redis)
#[async_trait]
pub trait ShortTermStorage: Send + Sync {
    /// 添加消息到 short-term memory
    async fn add_message(&self, user_id: &str, message: Message) -> Result<(), AppError>;

    /// 获取最近 N 条消息
    async fn get_recent(&self, user_id: &str, limit: usize) -> Result<Vec<Message>, AppError>;

    /// 清空用户的 short-term memory
    async fn clear(&self, user_id: &str) -> Result<(), AppError>;
}

/// Vector storage (Qdrant/Chroma/Pinecone)
/// Handles ALL memory tiers: Short-term, Mid-term, Long-term
#[async_trait]
pub trait VectorStorage: Send + Sync {
    // ========== Short-Term Memory ==========

    /// Add a message to short-term memory
    async fn add_short_term_message(&self, user_id: &str, message: Message)
        -> Result<(), AppError>;

    /// Get recent N messages from short-term memory (sorted by timestamp desc)
    async fn get_short_term_messages(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<Message>, AppError>;

    /// Clear short-term memory for a user
    async fn clear_short_term(&self, user_id: &str) -> Result<(), AppError>;

    // ========== Mid-Term Memory ==========

    /// 存储 mid-term segment
    async fn store_segment(&self, segment: MidTermSegment) -> Result<(), AppError>;

    /// 搜索相似的 segments
    async fn search_segments(
        &self,
        user_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MidTermSegment>, AppError>;

    // ========== Long-Term Memory ==========

    /// 存储 long-term memory
    async fn store_long_term(&self, memory: LongTermMemory) -> Result<(), AppError>;

    /// 存储 long-term memory with optional fencing token.
    async fn store_long_term_with_fencing(
        &self,
        memory: LongTermMemory,
        _fencing_token: Option<u64>,
    ) -> Result<(), AppError> {
        self.store_long_term(memory).await
    }

    /// 获取 long-term memory
    async fn get_long_term(&self, user_id: &str) -> Result<Option<LongTermMemory>, AppError>;
}

/// Concurrency and idempotency primitives for distributed writes.
#[async_trait]
pub trait ConcurrencyControl: Send + Sync {
    /// Acquire a lock and return fencing token on success.
    async fn acquire_fencing_lock(
        &self,
        lock_key: &str,
        owner_id: &str,
        ttl_ms: u64,
    ) -> Result<Option<u64>, AppError>;

    /// Extend lock TTL if owner still holds it.
    async fn renew_fencing_lock(
        &self,
        lock_key: &str,
        owner_id: &str,
        fencing_token: u64,
        ttl_ms: u64,
    ) -> Result<bool, AppError>;

    /// Release lock if owner still holds it.
    async fn release_fencing_lock(
        &self,
        lock_key: &str,
        owner_id: &str,
        fencing_token: u64,
    ) -> Result<bool, AppError>;

    /// Apply fencing token to a write target version key.
    /// Returns true when token is accepted (strictly newer).
    async fn enforce_fencing_version(
        &self,
        version_key: &str,
        fencing_token: u64,
    ) -> Result<bool, AppError>;

    /// Check if event was already processed.
    async fn is_event_processed(&self, event_id: &str) -> Result<bool, AppError>;

    /// Mark event as processed with TTL.
    async fn mark_event_processed(
        &self,
        event_id: &str,
        ttl_seconds: usize,
    ) -> Result<(), AppError>;
}

/// Memory manager (orchestrates all memory layers)
#[async_trait]
pub trait MemoryManager: Send + Sync {
    /// 添加新的对话消息
    async fn add_message(&self, user_id: &str, message: Message) -> Result<(), AppError>;

    /// Add message with optional event_id for idempotency.
    async fn add_message_with_event(
        &self,
        user_id: &str,
        message: Message,
        _event_id: Option<&str>,
    ) -> Result<(), AppError> {
        self.add_message(user_id, message).await
    }

    /// 检索相关的 memory context
    async fn retrieve_context(&self, user_id: &str, query: &str)
        -> Result<MemoryContext, AppError>;
}
