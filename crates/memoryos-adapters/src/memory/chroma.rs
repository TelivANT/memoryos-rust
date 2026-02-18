use async_trait::async_trait;
use memoryos_core::{AppError, LongTermMemory, MidTermSegment};
use memoryos_ports::VectorStorage;
use reqwest::Client;
use serde_json::json;

pub struct ChromaStorage {
    client: Client,
    base_url: String,
    collection: String,
}

impl ChromaStorage {
    pub async fn new(base_url: String, collection: String) -> Result<Self, AppError> {
        let client = Client::new();

        // 创建 collection
        let url = format!("{}/api/v1/collections", base_url);
        let _ = client
            .post(&url)
            .json(&json!({
                "name": collection,
                "metadata": {"description": "MemoryOS storage"}
            }))
            .send()
            .await;

        Ok(Self {
            client,
            base_url,
            collection,
        })
    }
}

#[async_trait]
impl VectorStorage for ChromaStorage {
    async fn store_segment(&self, _segment: MidTermSegment) -> Result<(), AppError> {
        // 简化实现
        Ok(())
    }

    async fn search_segments(
        &self,
        _user_id: &str,
        _query_embedding: Vec<f32>,
        _limit: usize,
    ) -> Result<Vec<MidTermSegment>, AppError> {
        Ok(vec![])
    }

    async fn store_long_term(&self, _memory: LongTermMemory) -> Result<(), AppError> {
        Ok(())
    }

    async fn get_long_term(&self, _user_id: &str) -> Result<Option<LongTermMemory>, AppError> {
        Ok(None)
    }
}
