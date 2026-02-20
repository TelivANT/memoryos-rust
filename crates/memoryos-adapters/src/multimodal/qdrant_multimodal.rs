use async_trait::async_trait;
use memoryos_core::{AppError, MultiModalContent, MultiModalMessage};
use memoryos_ports::MultiModalStorage;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, Distance, Filter, PointStruct, ScrollPointsBuilder,
    SearchPointsBuilder, Value, VectorParamsBuilder,
};
use qdrant_client::Qdrant;
use std::collections::HashMap;
use tracing::info;

const MULTIMODAL_COLLECTION: &str = "multimodal_messages";
const VECTOR_DIM: u64 = 1536;

pub struct QdrantMultiModalStorage {
    qdrant: Qdrant,
}

impl QdrantMultiModalStorage {
    pub async fn new(qdrant_url: &str) -> Result<Self, AppError> {
        let qdrant = Qdrant::from_url(qdrant_url)
            .build()
            .map_err(|e| AppError::Config(format!("Failed to connect to Qdrant: {}", e)))?;

        let storage = Self { qdrant };
        storage.ensure_collection().await?;
        Ok(storage)
    }

    async fn ensure_collection(&self) -> Result<(), AppError> {
        let collections = self
            .qdrant
            .list_collections()
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == MULTIMODAL_COLLECTION);

        if !exists {
            self.qdrant
                .create_collection(
                    CreateCollectionBuilder::new(MULTIMODAL_COLLECTION)
                        .vectors_config(VectorParamsBuilder::new(VECTOR_DIM, Distance::Cosine)),
                )
                .await
                .map_err(|e| {
                    AppError::ExternalService(format!("Failed to create collection: {}", e))
                })?;
        }

        Ok(())
    }

    fn message_to_payload(user_id: &str, message: &MultiModalMessage) -> HashMap<String, Value> {
        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("user_id".to_string(), user_id.into());
        payload.insert("role".to_string(), message.role.clone().into());
        payload.insert(
            "timestamp".to_string(),
            message.timestamp.to_rfc3339().into(),
        );

        let text = message.extract_text();
        payload.insert("text_content".to_string(), text.into());

        let content_json = serde_json::to_string(&message.contents).unwrap_or_default();
        payload.insert("contents".to_string(), content_json.into());

        let content_types: Vec<String> = message
            .contents
            .iter()
            .map(|c| match c {
                MultiModalContent::Text { .. } => "text".to_string(),
                MultiModalContent::Image { .. } => "image".to_string(),
                MultiModalContent::Audio { .. } => "audio".to_string(),
                MultiModalContent::Video { .. } => "video".to_string(),
            })
            .collect();
        let types_json = serde_json::to_string(&content_types).unwrap_or_default();
        payload.insert("content_types".to_string(), types_json.into());

        payload
    }

    fn payload_to_message(payload: &HashMap<String, Value>) -> Option<MultiModalMessage> {
        let role = payload.get("role")?.as_str()?.to_string();
        let timestamp_str = payload.get("timestamp")?.as_str()?;
        let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_str)
            .ok()?
            .with_timezone(&chrono::Utc);
        let contents_json = payload.get("contents")?.as_str()?;
        let contents: Vec<MultiModalContent> = serde_json::from_str(contents_json).ok()?;

        Some(MultiModalMessage {
            role,
            contents,
            timestamp,
        })
    }
}

#[async_trait]
impl MultiModalStorage for QdrantMultiModalStorage {
    async fn store_multimodal_message(
        &self,
        user_id: &str,
        message: MultiModalMessage,
    ) -> Result<(), AppError> {
        use qdrant_client::qdrant::UpsertPointsBuilder;

        info!("Storing multimodal message for user: {}", user_id);

        let embedding = message
            .get_embeddings()
            .into_iter()
            .next()
            .unwrap_or_else(|| vec![0.0; VECTOR_DIM as usize]);

        let embedding: Vec<f32> = if embedding.len() == VECTOR_DIM as usize {
            embedding
        } else {
            let mut padded = vec![0.0f32; VECTOR_DIM as usize];
            for (i, v) in embedding.iter().enumerate().take(VECTOR_DIM as usize) {
                padded[i] = *v;
            }
            padded
        };

        let payload = Self::message_to_payload(user_id, &message);
        let point_id = uuid::Uuid::now_v7().to_string();
        let point = PointStruct::new(point_id, embedding, payload);

        self.qdrant
            .upsert_points(UpsertPointsBuilder::new(MULTIMODAL_COLLECTION, vec![point]))
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        Ok(())
    }

    async fn search_by_text(
        &self,
        user_id: &str,
        _query: &str,
        limit: usize,
    ) -> Result<Vec<MultiModalMessage>, AppError> {
        let filter = Filter::must([Condition::matches("user_id", user_id.to_string())]);

        let results = self
            .qdrant
            .scroll(
                ScrollPointsBuilder::new(MULTIMODAL_COLLECTION)
                    .filter(filter)
                    .limit(limit as u32)
                    .with_payload(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        let messages = results
            .result
            .iter()
            .filter_map(|p| Self::payload_to_message(&p.payload))
            .collect();

        Ok(messages)
    }

    async fn search_by_image(
        &self,
        user_id: &str,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MultiModalMessage>, AppError> {
        let filter = Filter::must([Condition::matches("user_id", user_id.to_string())]);

        let results = self
            .qdrant
            .search_points(
                SearchPointsBuilder::new(MULTIMODAL_COLLECTION, embedding, limit as u64)
                    .filter(filter)
                    .with_payload(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        let messages = results
            .result
            .iter()
            .filter_map(|p| Self::payload_to_message(&p.payload))
            .collect();

        Ok(messages)
    }

    async fn search_by_audio(
        &self,
        user_id: &str,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MultiModalMessage>, AppError> {
        self.search_by_image(user_id, embedding, limit).await
    }

    async fn get_recent_multimodal(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<MultiModalMessage>, AppError> {
        let filter = Filter::must([Condition::matches("user_id", user_id.to_string())]);

        let results = self
            .qdrant
            .scroll(
                ScrollPointsBuilder::new(MULTIMODAL_COLLECTION)
                    .filter(filter)
                    .limit(limit as u32)
                    .with_payload(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        let mut messages: Vec<MultiModalMessage> = results
            .result
            .iter()
            .filter_map(|p| Self::payload_to_message(&p.payload))
            .collect();

        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(messages)
    }
}
