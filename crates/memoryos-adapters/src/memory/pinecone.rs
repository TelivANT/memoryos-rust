use async_trait::async_trait;
use memoryos_core::{AppError, LongTermMemory, MidTermSegment};
use memoryos_ports::VectorStorage;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing::debug;

pub struct PineconeStorage {
    client: Client,
    api_key: String,
    environment: String,
    shortterm_index: String,
    segment_index: String,
    longterm_index: String,
}

#[derive(Serialize)]
struct PineconeUpsertRequest {
    vectors: Vec<PineconeVector>,
    namespace: Option<String>,
}

#[derive(Serialize)]
struct PineconeVector {
    id: String,
    values: Vec<f32>,
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct PineconeQueryRequest {
    vector: Vec<f32>,
    #[serde(rename = "topK")]
    top_k: usize,
    filter: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "includeMetadata")]
    include_metadata: bool,
    #[serde(rename = "includeValues")]
    include_values: bool,
    namespace: Option<String>,
}

#[derive(Deserialize)]
struct PineconeQueryResponse {
    matches: Vec<PineconeMatch>,
}

#[derive(Deserialize)]
struct PineconeMatch {
    id: String,
    values: Option<Vec<f32>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
struct PineconeFetchRequest {
    ids: Vec<String>,
    namespace: Option<String>,
}

#[derive(Deserialize)]
struct PineconeFetchResponse {
    vectors: HashMap<String, PineconeFetchedVector>,
}

#[derive(Deserialize)]
struct PineconeFetchedVector {
    id: String,
    values: Option<Vec<f32>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

impl PineconeStorage {
    fn parse_message(metadata: HashMap<String, serde_json::Value>) -> Option<memoryos_core::Message> {
        Some(memoryos_core::Message {
            role: metadata.get("role")?.as_str()?.to_string(),
            content: metadata.get("content")?.as_str()?.to_string(),
            timestamp: metadata.get("timestamp")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))?,
            embedding: None,
        })
    }

    pub fn new(api_key: String, environment: String, segment_index: String, longterm_index: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            environment,
            shortterm_index: "memoryos-shortterm".to_string(),
            segment_index,
            longterm_index,
        }
    }

    fn get_index_url(&self, index_name: &str) -> String {
        format!("https://{}-{}.svc.{}.pinecone.io", index_name, self.environment, self.environment)
    }
}

#[async_trait]
impl VectorStorage for PineconeStorage {
    // ========== Short-Term Memory ==========
    
    async fn add_short_term_message(&self, user_id: &str, message: memoryos_core::Message) -> Result<(), AppError> {
        let message_id = uuid::Uuid::now_v7();
        let embedding = message.embedding.unwrap_or_else(|| vec![0.0; 1536]);
        
        let mut metadata = HashMap::new();
        metadata.insert("user_id".to_string(), json!(user_id));
        metadata.insert("role".to_string(), json!(message.role));
        metadata.insert("content".to_string(), json!(message.content));
        metadata.insert("timestamp".to_string(), json!(message.timestamp.to_rfc3339()));
        
        let vector = PineconeVector {
            id: message_id.to_string(),
            values: embedding,
            metadata,
        };
        
        let request = PineconeUpsertRequest {
            vectors: vec![vector],
            namespace: Some(user_id.to_string()),
        };
        
        let url = format!("{}/vectors/upsert", self.get_index_url(&self.shortterm_index));
        self.client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone upsert short-term failed: {}", e)))?;
        
        debug!("Stored short-term message in Pinecone for user: {}", user_id);
        Ok(())
    }
    
    async fn get_short_term_messages(&self, user_id: &str, limit: usize) -> Result<Vec<memoryos_core::Message>, AppError> {
        let mut filter = HashMap::new();
        filter.insert("user_id".to_string(), json!({"$eq": user_id}));
        
        let request = PineconeQueryRequest {
            vector: vec![0.0; 1536],
            top_k: limit,
            filter: Some(filter),
            include_metadata: true,
            include_values: false,
            namespace: Some(user_id.to_string()),
        };
        
        let url = format!("{}/query", self.get_index_url(&self.shortterm_index));
        let response = self.client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone query short-term failed: {}", e)))?
            .json::<PineconeQueryResponse>()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone parse failed: {}", e)))?;
        
        let mut messages: Vec<memoryos_core::Message> = response.matches
            .into_iter()
            .filter_map(|m| Self::parse_message(m.metadata?))
            .collect();
        
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        messages.truncate(limit);
        
        debug!("Retrieved {} short-term messages from Pinecone", messages.len());
        Ok(messages)
    }
    
    async fn clear_short_term(&self, user_id: &str) -> Result<(), AppError> {
        // Pinecone: Delete all vectors in user's namespace
        let url = format!("{}/vectors/delete", self.get_index_url(&self.shortterm_index));
        
        let request = json!({
            "deleteAll": true,
            "namespace": user_id
        });
        
        self.client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone delete request failed: {}", e)))?
            .error_for_status()
            .map_err(|e| AppError::ExternalService(format!("Pinecone delete failed: {}", e)))?;
        
        debug!("Cleared short-term memory for user {}", user_id);
        Ok(())
    }
    
    // ========== Mid-Term Memory ==========
    
    async fn store_segment(&self, segment: MidTermSegment) -> Result<(), AppError> {
        let mut metadata = HashMap::new();
        metadata.insert("user_id".to_string(), json!(segment.user_id));
        metadata.insert("summary".to_string(), json!(segment.summary));
        metadata.insert("heat".to_string(), json!(segment.heat));
        metadata.insert("created_at".to_string(), json!(segment.created_at.to_rfc3339()));
        metadata.insert("access_count".to_string(), json!(segment.access_count));
        metadata.insert("heat_score".to_string(), json!(segment.heat_score));

        let vector = PineconeVector {
            id: segment.id.to_string(),
            values: segment.embedding,
            metadata,
        };

        let request = PineconeUpsertRequest {
            vectors: vec![vector],
            namespace: Some(segment.user_id.clone()),
        };

        let url = format!("{}/vectors/upsert", self.get_index_url(&self.segment_index));
        self.client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone upsert failed: {}", e)))?;

        debug!("Stored mid-term segment in Pinecone: {}", segment.id);
        Ok(())
    }

    async fn search_segments(
        &self,
        user_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MidTermSegment>, AppError> {
        let mut filter = HashMap::new();
        filter.insert("user_id".to_string(), json!({"$eq": user_id}));

        let request = PineconeQueryRequest {
            vector: query_embedding,
            top_k: limit,
            filter: Some(filter),
            include_metadata: true,
            include_values: true,
            namespace: Some(user_id.to_string()),
        };

        let url = format!("{}/query", self.get_index_url(&self.segment_index));
        let response = self.client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone query failed: {}", e)))?
            .json::<PineconeQueryResponse>()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone parse failed: {}", e)))?;

        let segments: Vec<MidTermSegment> = response.matches
            .into_iter()
            .filter_map(|m| {
                let metadata = m.metadata?;
                Some(MidTermSegment {
                    id: uuid::Uuid::parse_str(&m.id).unwrap_or_else(|_| uuid::Uuid::now_v7()),
                    user_id: user_id.to_string(),
                    summary: metadata.get("summary")?.as_str()?.to_string(),
                    embedding: m.values.unwrap_or_default(),
                    heat: metadata.get("heat")?.as_f64()? as f32,
                    created_at: metadata.get("created_at")
                        .and_then(|v| v.as_str())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(chrono::Utc::now),
                    access_count: metadata.get("access_count")?.as_u64()? as u32,
                    heat_score: metadata.get("heat_score")?.as_f64()? as f32,
                    last_accessed: None,
                    memory_type: memoryos_core::MemoryType::QA,
                })
            })
            .collect();

        debug!("Found {} segments in Pinecone", segments.len());
        Ok(segments)
    }

    async fn store_long_term(&self, memory: LongTermMemory) -> Result<(), AppError> {
        let mut metadata = HashMap::new();
        metadata.insert("user_id".to_string(), json!(memory.user_id));
        metadata.insert("profile".to_string(), json!(serde_json::to_string(&memory.profile).unwrap_or_default()));
        metadata.insert("knowledge".to_string(), json!(serde_json::to_string(&memory.knowledge).unwrap_or_default()));
        metadata.insert("updated_at".to_string(), json!(memory.updated_at.to_rfc3339()));

        let vector = PineconeVector {
            id: memory.user_id.clone(),
            values: vec![0.0; 384],
            metadata,
        };

        let request = PineconeUpsertRequest {
            vectors: vec![vector],
            namespace: Some("longterm".to_string()),
        };

        let url = format!("{}/vectors/upsert", self.get_index_url(&self.longterm_index));
        self.client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone store long-term failed: {}", e)))?;

        debug!("Stored long-term memory in Pinecone for user: {}", memory.user_id);
        Ok(())
    }

    async fn get_long_term(&self, user_id: &str) -> Result<Option<LongTermMemory>, AppError> {
        let request = PineconeFetchRequest {
            ids: vec![user_id.to_string()],
            namespace: Some("longterm".to_string()),
        };

        let url = format!("{}/vectors/fetch", self.get_index_url(&self.longterm_index));
        let response = self.client
            .post(&url)
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone fetch failed: {}", e)))?
            .json::<PineconeFetchResponse>()
            .await
            .map_err(|e| AppError::ExternalService(format!("Pinecone parse failed: {}", e)))?;

        if let Some(vector) = response.vectors.get(user_id) {
            if let Some(metadata) = &vector.metadata {
                let profile_str = metadata.get("profile").and_then(|v| v.as_str()).unwrap_or("{}");
                let knowledge_str = metadata.get("knowledge").and_then(|v| v.as_str()).unwrap_or("[]");
                
                let profile = serde_json::from_str(profile_str).ok();
                let knowledge = serde_json::from_str(knowledge_str).ok();
                
                if let (Some(profile), Some(knowledge)) = (profile, knowledge) {
                    let memory = LongTermMemory {
                        user_id: user_id.to_string(),
                        profile,
                        knowledge,
                        graph: None,
                        updated_at: metadata.get("updated_at")
                            .and_then(|v| v.as_str())
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .unwrap_or_else(chrono::Utc::now),
                    };
                    return Ok(Some(memory));
                }
            }
        }

        Ok(None)
    }
}
