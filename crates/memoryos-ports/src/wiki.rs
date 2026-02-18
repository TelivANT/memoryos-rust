use async_trait::async_trait;
use memoryos_core::AppError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WikiDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[async_trait]
pub trait WikiAdapter: Send + Sync {
    /// Publish (Create/Update) a document
    async fn publish(&self, doc: WikiDocument) -> Result<String, AppError>;

    /// Recall (Delete) a document
    async fn recall(&self, doc_id: &str) -> Result<(), AppError>;

    /// Adapter Name (e.g., "s3", "wiki_js")
    fn name(&self) -> &str;
}
