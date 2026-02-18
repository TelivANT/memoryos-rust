//! Qdrant-based history storage

use async_trait::async_trait;
use memoryos_core::{AppError, HistoryEventType, MemoryHistoryEntry};
use memoryos_ports::HistoryStorage;
use qdrant_client::{
    qdrant::{
        Condition, CreateCollectionBuilder, Distance, Filter, GetPointsBuilder, PointStruct,
        ScrollPointsBuilder, UpsertPointsBuilder, Value, VectorParamsBuilder,
    },
    Qdrant,
};
use std::sync::Arc;

// 辅助函数：从 Qdrant Value 中提取字符串
fn get_string_value(
    payload: &std::collections::HashMap<String, Value>,
    key: &str,
) -> Option<String> {
    payload.get(key).and_then(|v| {
        if let Some(qdrant_client::qdrant::value::Kind::StringValue(s)) = v.kind.as_ref() {
            Some(s.clone())
        } else {
            None
        }
    })
}

// 辅助函数：从 Qdrant Value 中提取整数
fn get_i64_value(payload: &std::collections::HashMap<String, Value>, key: &str) -> Option<i64> {
    payload.get(key).and_then(|v| {
        if let Some(qdrant_client::qdrant::value::Kind::IntegerValue(i)) = v.kind.as_ref() {
            Some(*i)
        } else {
            None
        }
    })
}

pub struct QdrantHistoryStorage {
    client: Arc<Qdrant>,
    collection_name: String,
}

impl QdrantHistoryStorage {
    pub async fn new(client: Arc<Qdrant>, collection_name: String) -> Result<Self, AppError> {
        let storage = Self {
            client,
            collection_name,
        };

        // 确保 collection 存在
        storage.ensure_collection().await?;

        Ok(storage)
    }

    async fn ensure_collection(&self) -> Result<(), AppError> {
        let collections =
            self.client.list_collections().await.map_err(|e| {
                AppError::ExternalService(format!("Failed to list collections: {}", e))
            })?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection_name);

        if !exists {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection_name)
                        .vectors_config(VectorParamsBuilder::new(1, Distance::Cosine)),
                )
                .await
                .map_err(|e| {
                    AppError::ExternalService(format!("Failed to create collection: {}", e))
                })?;
        }

        Ok(())
    }
}

#[async_trait]
impl HistoryStorage for QdrantHistoryStorage {
    async fn add_entry(&self, entry: MemoryHistoryEntry) -> Result<(), AppError> {
        use qdrant_client::qdrant::Value;
        use std::collections::HashMap;

        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("memory_id".to_string(), entry.memory_id.into());
        payload.insert(
            "event_type".to_string(),
            format!("{:?}", entry.event_type).into(),
        );
        payload.insert(
            "created_at".to_string(),
            entry.created_at.timestamp().into(),
        );

        if let Some(old) = entry.old_content {
            payload.insert("old_content".to_string(), old.into());
        }
        if let Some(new) = entry.new_content {
            payload.insert("new_content".to_string(), new.into());
        }
        if let Some(actor) = entry.actor_id {
            payload.insert("actor_id".to_string(), actor.into());
        }

        let point = PointStruct::new(entry.id.clone(), vec![0.0], payload);

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection_name, vec![point]).wait(true))
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to upsert point: {}", e)))?;

        Ok(())
    }

    async fn get_history(&self, memory_id: &str) -> Result<Vec<MemoryHistoryEntry>, AppError> {
        let scroll_result = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.collection_name)
                    .filter(Filter::must([Condition::matches(
                        "memory_id",
                        memory_id.to_string(),
                    )]))
                    .limit(100)
                    .with_payload(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to scroll points: {}", e)))?;

        let mut entries: Vec<MemoryHistoryEntry> = scroll_result
            .result
            .into_iter()
            .filter_map(|point| {
                let payload = point.payload;

                let id = match point.id?.point_id_options? {
                    qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid) => uuid,
                    qdrant_client::qdrant::point_id::PointIdOptions::Num(num) => num.to_string(),
                };
                let memory_id = get_string_value(&payload, "memory_id")?;
                let old_content = get_string_value(&payload, "old_content");
                let new_content = get_string_value(&payload, "new_content");
                let event_type_str = get_string_value(&payload, "event_type")?;
                let event_type = match event_type_str.as_str() {
                    "Add" => HistoryEventType::Add,
                    "Update" => HistoryEventType::Update,
                    "Delete" => HistoryEventType::Delete,
                    _ => return None,
                };
                let created_at_ts = get_i64_value(&payload, "created_at")?;
                let created_at = chrono::DateTime::from_timestamp(created_at_ts, 0)?;
                let actor_id = get_string_value(&payload, "actor_id");

                Some(MemoryHistoryEntry {
                    id,
                    memory_id,
                    old_content,
                    new_content,
                    event_type,
                    created_at,
                    actor_id,
                })
            })
            .collect();

        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(entries)
    }

    async fn get_entry(&self, id: &str) -> Result<Option<MemoryHistoryEntry>, AppError> {
        let points = self
            .client
            .get_points(
                GetPointsBuilder::new(&self.collection_name, vec![id.into()])
                    .with_payload(true)
                    .with_vectors(false),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to get point: {}", e)))?;

        if let Some(point) = points.result.first() {
            let payload = &point.payload;

            let memory_id = get_string_value(payload, "memory_id")
                .ok_or_else(|| AppError::Internal("Missing memory_id".to_string()))?;

            let old_content = get_string_value(payload, "old_content");
            let new_content = get_string_value(payload, "new_content");

            let event_type_str = get_string_value(payload, "event_type")
                .ok_or_else(|| AppError::Internal("Missing event_type".to_string()))?;

            let event_type = match event_type_str.as_str() {
                "Add" => HistoryEventType::Add,
                "Update" => HistoryEventType::Update,
                "Delete" => HistoryEventType::Delete,
                _ => return Err(AppError::Internal("Invalid event_type".to_string())),
            };

            let created_at_ts = get_i64_value(payload, "created_at")
                .ok_or_else(|| AppError::Internal("Missing created_at".to_string()))?;

            let created_at = chrono::DateTime::from_timestamp(created_at_ts, 0)
                .ok_or_else(|| AppError::Internal("Invalid timestamp".to_string()))?;

            let actor_id = get_string_value(payload, "actor_id");

            Ok(Some(MemoryHistoryEntry {
                id: id.to_string(),
                memory_id,
                old_content,
                new_content,
                event_type,
                created_at,
                actor_id,
            }))
        } else {
            Ok(None)
        }
    }
}
