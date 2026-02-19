use async_trait::async_trait;
use memoryos_core::{AppError, LongTermMemory, MidTermSegment};
use memoryos_ports::VectorStorage;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing::debug;

pub struct ChromaStorage {
    client: Client,
    base_url: String,
    segment_collection: String,
    longterm_collection: String,
}

#[derive(Serialize)]
struct ChromaAddRequest {
    ids: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    metadatas: Vec<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
struct ChromaQueryRequest {
    query_embeddings: Vec<Vec<f32>>,
    n_results: usize,
    #[serde(rename = "where")]
    filter: Option<HashMap<String, String>>,
    include: Vec<String>,
}

#[derive(Deserialize)]
struct ChromaQueryResponse {
    ids: Vec<Vec<String>>,
    embeddings: Option<Vec<Vec<Vec<f32>>>>,
    metadatas: Option<Vec<Vec<HashMap<String, serde_json::Value>>>>,
}

impl ChromaStorage {
    pub async fn new(base_url: String, segment_collection: String, longterm_collection: String) -> Result<Self, AppError> {
        let client = Client::new();

        // 创建 collections
        for collection in &[&segment_collection, &longterm_collection] {
            let url = format!("{}/api/v1/collections", base_url);
            let _ = client
                .post(&url)
                .json(&json!({
                    "name": collection,
                    "metadata": {"description": "MemoryOS storage"}
                }))
                .send()
                .await;
        }

        Ok(Self {
            client,
            base_url,
            segment_collection,
            longterm_collection,
        })
    }
}

#[async_trait]
impl VectorStorage for ChromaStorage {
    async fn store_segment(&self, segment: MidTermSegment) -> Result<(), AppError> {
        let mut metadata = HashMap::new();
        metadata.insert("user_id".to_string(), json!(segment.user_id));
        metadata.insert("summary".to_string(), json!(segment.summary));
        metadata.insert("heat".to_string(), json!(segment.heat));
        metadata.insert("created_at".to_string(), json!(segment.created_at.to_rfc3339()));
        metadata.insert("access_count".to_string(), json!(segment.access_count));
        metadata.insert("heat_score".to_string(), json!(segment.heat_score));

        let request = ChromaAddRequest {
            ids: vec![segment.id.to_string()],
            embeddings: vec![segment.embedding],
            metadatas: vec![metadata],
        };

        let url = format!("{}/api/v1/collections/{}/add", self.base_url, self.segment_collection);
        self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Chroma add failed: {}", e)))?;

        debug!("Stored mid-term segment in Chroma: {}", segment.id);
        Ok(())
    }

    async fn search_segments(
        &self,
        user_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MidTermSegment>, AppError> {
        let mut filter = HashMap::new();
        filter.insert("user_id".to_string(), user_id.to_string());

        let request = ChromaQueryRequest {
            query_embeddings: vec![query_embedding],
            n_results: limit,
            filter: Some(filter),
            include: vec!["embeddings".to_string(), "metadatas".to_string()],
        };

        let url = format!("{}/api/v1/collections/{}/query", self.base_url, self.segment_collection);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Chroma query failed: {}", e)))?
            .json::<ChromaQueryResponse>()
            .await
            .map_err(|e| AppError::ExternalService(format!("Chroma parse failed: {}", e)))?;

        let mut segments = Vec::new();
        if let (Some(ids), Some(embeddings), Some(metadatas)) = 
            (response.ids.first(), response.embeddings.as_ref().and_then(|e| e.first()), response.metadatas.as_ref().and_then(|m| m.first())) {
            
            for ((id, embedding), metadata) in ids.iter().zip(embeddings.iter()).zip(metadatas.iter()) {
                let segment = MidTermSegment {
                    id: uuid::Uuid::parse_str(id).unwrap_or_else(|_| uuid::Uuid::now_v7()),
                    user_id: user_id.to_string(),
                    summary: metadata.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    embedding: embedding.clone(),
                    heat: metadata.get("heat").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    created_at: metadata.get("created_at")
                        .and_then(|v| v.as_str())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(chrono::Utc::now),
                    access_count: metadata.get("access_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    heat_score: metadata.get("heat_score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    last_accessed: None,
                    memory_type: memoryos_core::MemoryType::QA,
                };
                segments.push(segment);
            }
        }

        debug!("Found {} segments in Chroma", segments.len());
        Ok(segments)
    }

    async fn store_long_term(&self, memory: LongTermMemory) -> Result<(), AppError> {
        let mut metadata = HashMap::new();
        metadata.insert("user_id".to_string(), json!(memory.user_id));
        metadata.insert("profile".to_string(), json!(serde_json::to_string(&memory.profile).unwrap_or_default()));
        metadata.insert("knowledge".to_string(), json!(serde_json::to_string(&memory.knowledge).unwrap_or_default()));
        metadata.insert("updated_at".to_string(), json!(memory.updated_at.to_rfc3339()));

        let request = ChromaAddRequest {
            ids: vec![memory.user_id.clone()],
            embeddings: vec![vec![0.0; 384]],
            metadatas: vec![metadata],
        };

        let url = format!("{}/api/v1/collections/{}/add", self.base_url, self.longterm_collection);
        self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Chroma store long-term failed: {}", e)))?;

        debug!("Stored long-term memory in Chroma for user: {}", memory.user_id);
        Ok(())
    }

    async fn get_long_term(&self, user_id: &str) -> Result<Option<LongTermMemory>, AppError> {
        let url = format!("{}/api/v1/collections/{}/get", self.base_url, self.longterm_collection);
        let body = json!({ "ids": [user_id], "include": ["metadatas"] });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Chroma get failed: {}", e)))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| AppError::ExternalService(format!("Chroma parse failed: {}", e)))?;

        if let Some(metadatas) = response.get("metadatas").and_then(|m| m.as_array()) {
            if let Some(metadata) = metadatas.first() {
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
