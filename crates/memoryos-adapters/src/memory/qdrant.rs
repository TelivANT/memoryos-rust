//! Qdrant adapter for vector storage (simplified)

use async_trait::async_trait;
use memoryos_core::{AppError, LongTermMemory, MidTermSegment};
use memoryos_ports::VectorStorage;
use qdrant_client::{
    qdrant::{
        point_id::PointIdOptions, value::Kind, vector_output, Condition, CreateCollectionBuilder,
        Distance, Filter, GetPointsBuilder, PointId, PointStruct, SearchPointsBuilder,
        UpsertPointsBuilder, Value, VectorParamsBuilder, VectorsOutput,
    },
    Qdrant,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

pub struct QdrantStorage {
    client: Arc<Qdrant>,
    shortterm_collection: String,
    segment_collection: String,
    longterm_collection: String,
}

impl QdrantStorage {
    pub async fn new(url: &str) -> Result<Self, AppError> {
        let client = Qdrant::from_url(url)
            .skip_compatibility_check()
            .build()
            .map_err(|e| AppError::Config(format!("Failed to connect to Qdrant: {}", e)))?;

        let storage = Self {
            client: Arc::new(client),
            shortterm_collection: "short_term_messages".to_string(),
            segment_collection: "mid_term_segments".to_string(),
            longterm_collection: "long_term_memory".to_string(),
        };

        storage.ensure_collections().await?;
        Ok(storage)
    }

    /// 获取 Qdrant 客户端（用于 History Storage）
    pub fn client(&self) -> &Arc<Qdrant> {
        &self.client
    }

    async fn ensure_collections(&self) -> Result<(), AppError> {
        // 检查现有 collections
        let collections = self.client.list_collections().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to list Qdrant collections: {}", e))
        })?;

        let existing: std::collections::HashSet<_> = collections
            .collections
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        // 创建 short-term messages collection
        if !existing.contains(self.shortterm_collection.as_str()) {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.shortterm_collection)
                        .vectors_config(VectorParamsBuilder::new(1536, Distance::Cosine)),
                )
                .await
                .map_err(|e| {
                    AppError::ExternalService(format!(
                        "Failed to create collection '{}': {}",
                        self.shortterm_collection, e
                    ))
                })?;
            debug!("Created Qdrant collection: {}", self.shortterm_collection);
        }

        // 创建 mid-term segments collection
        if !existing.contains(self.segment_collection.as_str()) {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.segment_collection)
                        .vectors_config(VectorParamsBuilder::new(1536, Distance::Cosine)),
                )
                .await
                .map_err(|e| {
                    AppError::ExternalService(format!(
                        "Failed to create collection '{}': {}",
                        self.segment_collection, e
                    ))
                })?;
            debug!("Created Qdrant collection: {}", self.segment_collection);
        }

        // 创建 long-term memory collection
        if !existing.contains(self.longterm_collection.as_str()) {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.longterm_collection)
                        .vectors_config(VectorParamsBuilder::new(1536, Distance::Cosine)),
                )
                .await
                .map_err(|e| {
                    AppError::ExternalService(format!(
                        "Failed to create collection '{}': {}",
                        self.longterm_collection, e
                    ))
                })?;
            debug!("Created Qdrant collection: {}", self.longterm_collection);
        }

        Ok(())
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        self.client
            .health_check()
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant health check failed: {}", e)))?;
        Ok(())
    }

    pub async fn delete_points_by_id(&self, id: &uuid::Uuid) -> Result<(), AppError> {
        use qdrant_client::qdrant::DeletePointsBuilder;
        let point_id = qdrant_client::qdrant::PointId::from(id.to_string());
        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.segment_collection)
                    .points(vec![point_id])
                    .wait(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant delete point failed: {}", e)))?;
        Ok(())
    }

    pub async fn delete_user_data(&self, user_id: &str) -> Result<(), AppError> {
        use qdrant_client::qdrant::{Condition, DeletePointsBuilder, Filter};

        for collection in [
            &self.shortterm_collection,
            &self.segment_collection,
            &self.longterm_collection,
        ] {
            let filter = Filter::must([Condition::matches("user_id", user_id.to_string())]);
            self.client
                .delete_points(
                    DeletePointsBuilder::new(collection)
                        .points(filter)
                        .wait(true),
                )
                .await
                .map_err(|e| {
                    AppError::ExternalService(format!(
                        "Qdrant delete from '{}' failed: {}",
                        collection, e
                    ))
                })?;
        }
        Ok(())
    }

    fn build_mid_term_segment(
        &self,
        user_id: &str,
        payload: HashMap<String, Value>,
        point_id: Option<PointId>,
        vectors: Option<VectorsOutput>,
        score: Option<f32>,
    ) -> MidTermSegment {
        let summary = payload_string(&payload, "summary").unwrap_or_default();
        let heat = payload_f64(&payload, "heat").unwrap_or(0.0) as f32;
        let created_at = payload_string(&payload, "created_at")
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        MidTermSegment {
            id: point_id
                .and_then(point_id_to_uuid)
                .unwrap_or_else(uuid::Uuid::now_v7),
            user_id: user_id.to_string(),
            summary,
            embedding: vectors.map(convert_vectors).unwrap_or_default(),
            heat,
            created_at,
            tenant_id: payload_string(&payload, "tenant_id"),
            access_count: payload
                .get("access_count")
                .and_then(|v| v.as_integer())
                .unwrap_or(0) as u32,
            heat_score: payload
                .get("heat_score")
                .and_then(|v| v.as_double())
                .unwrap_or(0.0) as f32,
            last_accessed: payload
                .get("last_accessed")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            memory_type: payload
                .get("memory_type")
                .and_then(|v| v.as_str())
                .and_then(|s| match s.as_ref() {
                    "qa" => Some(memoryos_core::MemoryType::QA),
                    "faq_candidate" => Some(memoryos_core::MemoryType::FaqCandidate),
                    "faq" => Some(memoryos_core::MemoryType::Faq),
                    _ => None,
                })
                .unwrap_or(memoryos_core::MemoryType::QA),
            version: payload
                .get("version")
                .and_then(|v| v.as_integer())
                .unwrap_or(1) as u32,
            tags: payload
                .get("tags")
                .and_then(|v| match v.kind.as_ref()? {
                    Kind::ListValue(list) => Some(
                        list.values
                            .iter()
                            .filter_map(|item| match item.kind.as_ref()? {
                                Kind::StringValue(s) => Some(s.clone()),
                                _ => None,
                            })
                            .collect(),
                    ),
                    _ => None,
                })
                .unwrap_or_default(),
            updated_at: payload
                .get("updated_at")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            previous_version_id: payload
                .get("previous_version_id")
                .and_then(|v| v.as_str())
                .and_then(|s| uuid::Uuid::parse_str(s).ok()),
            score,
        }
    }
}

fn long_term_point_id(user_id: &str) -> String {
    // Stable UUID id derived from user_id, so arbitrary user ids stay compatible with Qdrant PointId.
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, user_id.as_bytes()).to_string()
}

#[async_trait]
impl VectorStorage for QdrantStorage {
    // ========== Short-Term Memory ==========

    async fn add_short_term_message(
        &self,
        user_id: &str,
        message: memoryos_core::Message,
    ) -> Result<(), AppError> {
        let message_id = uuid::Uuid::now_v7();
        let embedding = message.embedding.unwrap_or_else(|| vec![0.0; 1536]);

        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("user_id".to_string(), Value::from(user_id.to_string()));
        payload.insert("role".to_string(), Value::from(message.role));
        payload.insert("content".to_string(), Value::from(message.content));
        payload.insert(
            "timestamp".to_string(),
            Value::from(message.timestamp.to_rfc3339()),
        );

        let point = PointStruct::new(message_id.to_string(), embedding, payload);

        self.client
            .upsert_points(
                UpsertPointsBuilder::new(&self.shortterm_collection, vec![point]).wait(true),
            )
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Qdrant upsert short-term failed: {}", e))
            })?;

        debug!("Stored short-term message for user: {}", user_id);
        Ok(())
    }

    async fn get_short_term_messages(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<memoryos_core::Message>, AppError> {
        let filter = Filter::must([Condition::matches("user_id", user_id.to_string())]);

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.shortterm_collection, vec![0.0; 1536], limit as u64)
                    .with_payload(true)
                    .filter(filter),
            )
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Qdrant search short-term failed: {}", e))
            })?;

        let mut messages: Vec<memoryos_core::Message> = search_result
            .result
            .into_iter()
            .filter_map(|point| {
                let payload = point.payload;
                Some(memoryos_core::Message {
                    role: payload_string(&payload, "role")?,
                    content: payload_string(&payload, "content")?,
                    timestamp: payload_string(&payload, "timestamp")
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc))?,
                    embedding: None,
                })
            })
            .collect();

        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        messages.truncate(limit);

        debug!(
            "Retrieved {} short-term messages for user: {}",
            messages.len(),
            user_id
        );
        Ok(messages)
    }

    async fn clear_short_term(&self, user_id: &str) -> Result<(), AppError> {
        use qdrant_client::qdrant::{Condition, DeletePointsBuilder, Filter};

        let filter = Filter::must([Condition::matches("user_id", user_id.to_string())]);

        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.shortterm_collection)
                    .points(filter)
                    .wait(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant delete failed: {}", e)))?;

        debug!("Cleared short-term memory for user {}", user_id);
        Ok(())
    }

    // ========== Mid-Term Memory ==========

    async fn store_segment(&self, segment: MidTermSegment) -> Result<(), AppError> {
        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("user_id".to_string(), Value::from(segment.user_id));
        if let Some(ref tid) = segment.tenant_id {
            payload.insert("tenant_id".to_string(), Value::from(tid.clone()));
        }
        payload.insert("summary".to_string(), Value::from(segment.summary));
        payload.insert("heat".to_string(), Value::from(segment.heat as f64));
        payload.insert(
            "created_at".to_string(),
            Value::from(segment.created_at.to_rfc3339()),
        );
        payload.insert(
            "access_count".to_string(),
            Value::from(segment.access_count as i64),
        );
        payload.insert(
            "heat_score".to_string(),
            Value::from(segment.heat_score as f64),
        );
        if let Some(last_accessed) = segment.last_accessed {
            payload.insert(
                "last_accessed".to_string(),
                Value::from(last_accessed.to_rfc3339()),
            );
        }
        let memory_type_str = match segment.memory_type {
            memoryos_core::MemoryType::QA => "qa",
            memoryos_core::MemoryType::FaqCandidate => "faq_candidate",
            memoryos_core::MemoryType::Faq => "faq",
        };
        payload.insert("memory_type".to_string(), Value::from(memory_type_str));
        payload.insert("version".to_string(), Value::from(segment.version as i64));
        if !segment.tags.is_empty() {
            let tag_values: Vec<Value> = segment
                .tags
                .iter()
                .map(|t| Value::from(t.clone()))
                .collect();
            payload.insert(
                "tags".to_string(),
                Value {
                    kind: Some(Kind::ListValue(qdrant_client::qdrant::ListValue {
                        values: tag_values,
                    })),
                },
            );
        }
        if let Some(updated_at) = segment.updated_at {
            payload.insert(
                "updated_at".to_string(),
                Value::from(updated_at.to_rfc3339()),
            );
        }
        if let Some(prev_id) = segment.previous_version_id {
            payload.insert(
                "previous_version_id".to_string(),
                Value::from(prev_id.to_string()),
            );
        }

        let point = PointStruct::new(segment.id.to_string(), segment.embedding.clone(), payload);

        self.client
            .upsert_points(
                UpsertPointsBuilder::new(&self.segment_collection, vec![point]).wait(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant upsert failed: {}", e)))?;

        debug!("Stored mid-term segment: {}", segment.id);
        Ok(())
    }

    async fn search_segments_for_tenant(
        &self,
        user_id: &str,
        tenant_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MidTermSegment>, AppError> {
        let filter = Filter::must([
            Condition::matches("user_id", user_id.to_string()),
            Condition::matches("tenant_id", tenant_id.to_string()),
        ]);

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.segment_collection, query_embedding, limit as u64)
                    .with_payload(true)
                    .with_vectors(true)
                    .filter(filter),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant search failed: {}", e)))?;

        let segments = search_result
            .result
            .into_iter()
            .map(|point| {
                let payload = point.payload;
                let score = Some(point.score);
                self.build_mid_term_segment(user_id, payload, point.id, point.vectors, score)
            })
            .collect::<Vec<_>>();

        Ok(segments)
    }

    async fn search_segments_by_tags_for_tenant(
        &self,
        user_id: &str,
        tenant_id: &str,
        tags: &[String],
        limit: usize,
    ) -> Result<Vec<MidTermSegment>, AppError> {
        use qdrant_client::qdrant::ScrollPointsBuilder;

        let mut must_conditions = vec![
            Condition::matches("user_id", user_id.to_string()),
            Condition::matches("tenant_id", tenant_id.to_string()),
        ];
        for tag in tags {
            must_conditions.push(Condition::matches("tags", tag.clone()));
        }
        let filter = Filter::must(must_conditions);

        let scroll_result = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.segment_collection)
                    .filter(filter)
                    .limit(limit as u32)
                    .with_payload(true)
                    .with_vectors(true),
            )
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Qdrant scroll by tags failed: {}", e))
            })?;

        let segments = scroll_result
            .result
            .into_iter()
            .map(|point| {
                let payload = point.payload;
                self.build_mid_term_segment(user_id, payload, point.id, point.vectors, None)
            })
            .collect::<Vec<_>>();

        Ok(segments)
    }

    async fn search_segments(
        &self,
        user_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MidTermSegment>, AppError> {
        let filter = Filter::must([Condition::matches("user_id", user_id.to_string())]);

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.segment_collection, query_embedding, limit as u64)
                    .with_payload(true)
                    .with_vectors(true)
                    .filter(filter),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant search failed: {}", e)))?;

        debug!("Found {} mid-term segments", search_result.result.len());

        let segments = search_result
            .result
            .into_iter()
            .map(|point| {
                let payload = point.payload;
                let score = Some(point.score);
                self.build_mid_term_segment(user_id, payload, point.id, point.vectors, score)
            })
            .collect::<Vec<_>>();

        Ok(segments)
    }

    async fn search_segments_by_tags(
        &self,
        user_id: &str,
        tags: &[String],
        limit: usize,
    ) -> Result<Vec<MidTermSegment>, AppError> {
        use qdrant_client::qdrant::ScrollPointsBuilder;

        let mut must_conditions = vec![Condition::matches("user_id", user_id.to_string())];
        for tag in tags {
            must_conditions.push(Condition::matches("tags", tag.clone()));
        }
        let filter = Filter::must(must_conditions);

        let scroll_result = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.segment_collection)
                    .filter(filter)
                    .limit(limit as u32)
                    .with_payload(true)
                    .with_vectors(true),
            )
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Qdrant scroll by tags failed: {}", e))
            })?;

        let segments = scroll_result
            .result
            .into_iter()
            .map(|point| {
                let payload = point.payload;
                self.build_mid_term_segment(user_id, payload, point.id, point.vectors, None)
            })
            .collect::<Vec<_>>();

        debug!(
            "Found {} segments by tags for user: {}",
            segments.len(),
            user_id
        );
        Ok(segments)
    }

    async fn store_long_term(&self, memory: LongTermMemory) -> Result<(), AppError> {
        self.store_long_term_with_fencing(memory, None).await
    }

    async fn store_long_term_with_fencing(
        &self,
        memory: LongTermMemory,
        fencing_token: Option<u64>,
    ) -> Result<(), AppError> {
        if let Some(token) = fencing_token {
            let existing = self
                .client
                .get_points(
                    GetPointsBuilder::new(
                        &self.longterm_collection,
                        vec![long_term_point_id(&memory.user_id).into()],
                    )
                    .with_payload(true)
                    .with_vectors(false),
                )
                .await
                .map_err(|e| {
                    AppError::ExternalService(format!("Qdrant get for fencing failed: {}", e))
                })?;

            if let Some(point) = existing.result.first() {
                let current = payload_u64(&point.payload, "lock_version").unwrap_or(0);
                if token <= current {
                    return Err(AppError::RateLimited(format!(
                        "Stale fencing token {} <= {} for user {}",
                        token, current, memory.user_id
                    )));
                }
            }
        }

        // 生成 embedding：使用 profile traits 和 knowledge 的组合
        let text_for_embedding = format!(
            "User traits: {}. Knowledge: {}",
            memory.profile.traits.join(", "),
            memory
                .knowledge
                .iter()
                .map(|k| k.content.as_str())
                .collect::<Vec<_>>()
                .join(". ")
        );

        // 简化版：使用文本长度生成伪随机 embedding
        // 生产环境应该调用真实的 embedding API
        let embedding = generate_simple_embedding(&text_for_embedding);

        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("user_id".to_string(), Value::from(memory.user_id.clone()));
        if let Some(token) = fencing_token {
            payload.insert("lock_version".to_string(), Value::from(token as i64));
        }
        payload.insert(
            "raw_json".to_string(),
            Value::from(
                serde_json::to_string(&memory).map_err(|e| {
                    AppError::Internal(format!("Failed to serialize memory: {}", e))
                })?,
            ),
        );

        let point = PointStruct::new(long_term_point_id(&memory.user_id), embedding, payload);

        self.client
            .upsert_points(
                UpsertPointsBuilder::new(&self.longterm_collection, vec![point]).wait(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant upsert failed: {}", e)))?;

        debug!("Stored long-term memory for user: {}", memory.user_id);
        Ok(())
    }

    async fn get_long_term(&self, user_id: &str) -> Result<Option<LongTermMemory>, AppError> {
        let points = self
            .client
            .get_points(
                GetPointsBuilder::new(
                    &self.longterm_collection,
                    vec![long_term_point_id(user_id).into()],
                )
                .with_payload(true)
                .with_vectors(false),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant get failed: {}", e)))?;

        debug!(
            "Retrieved {} points for user: {}",
            points.result.len(),
            user_id
        );

        let memory = points.result.first().and_then(|point| {
            let payload = &point.payload;
            let raw_json = payload_string(payload, "raw_json")?;
            serde_json::from_str::<LongTermMemory>(&raw_json).ok()
        });

        Ok(memory)
    }
}

fn payload_string(payload: &HashMap<String, Value>, key: &str) -> Option<String> {
    let value = payload.get(key)?;
    match value.kind.as_ref()? {
        Kind::StringValue(s) => Some(s.clone()),
        Kind::IntegerValue(i) => Some(i.to_string()),
        Kind::DoubleValue(f) => Some(f.to_string()),
        Kind::BoolValue(b) => Some(b.to_string()),
        _ => None,
    }
}

fn payload_f64(payload: &HashMap<String, Value>, key: &str) -> Option<f64> {
    let value = payload.get(key)?;
    match value.kind.as_ref()? {
        Kind::DoubleValue(f) => Some(*f),
        Kind::IntegerValue(i) => Some(*i as f64),
        Kind::StringValue(s) => s.parse().ok(),
        _ => None,
    }
}

fn payload_u64(payload: &HashMap<String, Value>, key: &str) -> Option<u64> {
    let value = payload.get(key)?;
    match value.kind.as_ref()? {
        Kind::IntegerValue(i) if *i >= 0 => Some(*i as u64),
        Kind::DoubleValue(f) if *f >= 0.0 => Some(*f as u64),
        Kind::StringValue(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

fn convert_vectors(vectors: VectorsOutput) -> Vec<f32> {
    match vectors.get_vector() {
        Some(vector_output::Vector::Dense(v)) => v.data,
        Some(vector_output::Vector::MultiDense(v)) => v
            .vectors
            .first()
            .map(|first| first.data.clone())
            .unwrap_or_default(),
        Some(vector_output::Vector::Sparse(_)) => vec![],
        None => vec![],
    }
}

fn point_id_to_uuid(point_id: PointId) -> Option<uuid::Uuid> {
    match point_id.point_id_options {
        Some(PointIdOptions::Uuid(id)) => uuid::Uuid::parse_str(&id).ok(),
        Some(PointIdOptions::Num(_)) | None => None,
    }
}

/// 生成简单的伪随机 embedding（用于测试）
/// 生产环境应该调用真实的 embedding API
fn generate_simple_embedding(text: &str) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let hash = hasher.finish();

    // 生成 1536 维的伪随机向量
    (0..1536)
        .map(|i| {
            let seed = hash.wrapping_add(i as u64);
            ((seed % 1000) as f32 / 1000.0) - 0.5 // 范围 [-0.5, 0.5]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_u64_supports_multiple_numeric_shapes() {
        let mut payload = HashMap::new();
        payload.insert("a".to_string(), Value::from(123_i64));
        payload.insert("b".to_string(), Value::from(45.0_f64));
        payload.insert("c".to_string(), Value::from("67".to_string()));

        assert_eq!(payload_u64(&payload, "a"), Some(123));
        assert_eq!(payload_u64(&payload, "b"), Some(45));
        assert_eq!(payload_u64(&payload, "c"), Some(67));
        assert_eq!(payload_u64(&payload, "missing"), None);
    }

    #[test]
    fn long_term_point_id_is_stable_uuid() {
        let a = long_term_point_id("demo-user");
        let b = long_term_point_id("demo-user");
        assert_eq!(a, b);
        assert!(uuid::Uuid::parse_str(&a).is_ok());
    }
}
