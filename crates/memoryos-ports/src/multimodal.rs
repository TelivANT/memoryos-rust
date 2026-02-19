//! Multi-modal storage port

use async_trait::async_trait;
use memoryos_core::{AppError, MultiModalMessage};

/// Multi-modal storage interface
#[async_trait]
pub trait MultiModalStorage: Send + Sync {
    /// Store multi-modal message
    async fn store_multimodal_message(
        &self,
        user_id: &str,
        message: MultiModalMessage,
    ) -> Result<(), AppError>;

    /// Search multi-modal messages by text
    async fn search_by_text(
        &self,
        user_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MultiModalMessage>, AppError>;

    /// Search multi-modal messages by image embedding
    async fn search_by_image(
        &self,
        user_id: &str,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MultiModalMessage>, AppError>;

    /// Search multi-modal messages by audio embedding
    async fn search_by_audio(
        &self,
        user_id: &str,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MultiModalMessage>, AppError>;

    /// Get recent multi-modal messages
    async fn get_recent_multimodal(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<MultiModalMessage>, AppError>;
}
