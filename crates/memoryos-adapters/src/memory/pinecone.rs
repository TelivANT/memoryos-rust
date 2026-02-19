use async_trait::async_trait;
use memoryos_core::{AppError, LongTermMemory, MidTermSegment};
use memoryos_ports::VectorStorage;
use reqwest::Client;

pub struct PineconeStorage {
    client: Client,
    api_key: String,
    environment: String,
    index: String,
}

impl PineconeStorage {
    pub fn new(api_key: String, environment: String, index: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            environment,
            index,
        }
    }
}

#[async_trait]
impl VectorStorage for PineconeStorage {
    async fn store_segment(&self, _segment: MidTermSegment) -> Result<(), AppError> {
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
