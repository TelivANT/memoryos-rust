use memoryos_core::AppError;
use qdrant_client::{
    qdrant::{Condition, Filter, PointStruct, ScrollPointsBuilder, Value},
    Qdrant,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

const API_KEY_COLLECTION: &str = "api_keys";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyMetadata {
    pub user_id: String,
    pub description: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub permissions: Vec<String>,
    pub is_active: bool,
}

pub struct ApiKeyStore {
    qdrant: Qdrant,
}

impl ApiKeyStore {
    pub async fn new(qdrant_url: &str) -> Result<Self, AppError> {
        let qdrant = Qdrant::from_url(qdrant_url)
            .build()
            .map_err(|e| AppError::Config(format!("Failed to connect to Qdrant: {}", e)))?;

        let store = Self { qdrant };
        store.ensure_collection().await?;
        Ok(store)
    }

    async fn ensure_collection(&self) -> Result<(), AppError> {
        use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder};

        let collections = self
            .qdrant
            .list_collections()
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == API_KEY_COLLECTION);

        if !exists {
            self.qdrant
                .create_collection(
                    CreateCollectionBuilder::new(API_KEY_COLLECTION)
                        .vectors_config(VectorParamsBuilder::new(1, Distance::Cosine)),
                )
                .await
                .map_err(|e| {
                    AppError::ExternalService(format!("Failed to create collection: {}", e))
                })?;
        }

        Ok(())
    }

    /// Hash API key using SHA-256
    fn hash_api_key(api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Validate API key and check expiration
    pub async fn validate_key(&self, api_key: &str) -> Result<bool, AppError> {
        match self.get_metadata(api_key).await? {
            Some(meta) => {
                // Check if active
                if !meta.is_active {
                    return Ok(false);
                }

                // Check expiration
                if let Some(expires_at) = &meta.expires_at {
                    let expiry = chrono::DateTime::parse_from_rfc3339(expires_at)
                        .map_err(|e| AppError::Internal(format!("Invalid expires_at: {}", e)))?;
                    if expiry < chrono::Utc::now() {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub async fn create_key(
        &self,
        api_key: &str,
        metadata: ApiKeyMetadata,
    ) -> Result<(), AppError> {
        use qdrant_client::qdrant::UpsertPointsBuilder;

        let key_hash = Self::hash_api_key(api_key);

        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("key_hash".to_string(), key_hash.into());
        payload.insert("user_id".to_string(), metadata.user_id.clone().into());
        payload.insert(
            "description".to_string(),
            metadata.description.clone().into(),
        );
        payload.insert("created_at".to_string(), metadata.created_at.clone().into());
        payload.insert("is_active".to_string(), metadata.is_active.into());

        if let Some(expires_at) = &metadata.expires_at {
            payload.insert("expires_at".to_string(), expires_at.clone().into());
        }

        let permissions_json = serde_json::to_string(&metadata.permissions)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;
        payload.insert("permissions".to_string(), permissions_json.into());

        // Use UUID v7 for point_id (time-ordered + unique)
        let point_id = Uuid::now_v7().to_string();
        let point = PointStruct::new(point_id, vec![0.0], payload);

        self.qdrant
            .upsert_points(UpsertPointsBuilder::new(API_KEY_COLLECTION, vec![point]))
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        Ok(())
    }

    pub async fn delete_key(&self, api_key: &str) -> Result<(), AppError> {
        let key_hash = Self::hash_api_key(api_key);

        // Query to find point_id
        let filter = Filter::must([Condition::matches("key_hash", key_hash)]);
        let search_result = self
            .qdrant
            .scroll(
                ScrollPointsBuilder::new(API_KEY_COLLECTION)
                    .filter(filter)
                    .limit(1)
                    .build(),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        if let Some(point) = search_result.result.first() {
            if let Some(point_id) = &point.id {
                // Delete by point_id
                self.qdrant
                    .delete_points(
                        qdrant_client::qdrant::DeletePointsBuilder::new(API_KEY_COLLECTION)
                            .points(vec![point_id.clone()]),
                    )
                    .await
                    .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;
            }
        }

        Ok(())
    }

    pub async fn get_metadata(&self, api_key: &str) -> Result<Option<ApiKeyMetadata>, AppError> {
        let key_hash = Self::hash_api_key(api_key);

        // Search by key_hash filter
        let filter = Filter::must([Condition::matches("key_hash", key_hash.clone())]);

        let results = self
            .qdrant
            .scroll(
                ScrollPointsBuilder::new(API_KEY_COLLECTION)
                    .filter(filter)
                    .limit(1)
                    .with_payload(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        if let Some(point) = results.result.first() {
            let payload = &point.payload;

            let user_id = Self::get_string_field(payload, "user_id")?;
            let description = Self::get_string_field(payload, "description")?;
            let created_at = Self::get_string_field(payload, "created_at")?;
            let is_active = Self::get_bool_field(payload, "is_active")?;
            let expires_at = Self::get_string_field(payload, "expires_at").ok();

            let permissions_json = Self::get_string_field(payload, "permissions")?;
            let permissions: Vec<String> = serde_json::from_str(&permissions_json)
                .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?;

            Ok(Some(ApiKeyMetadata {
                user_id,
                description,
                created_at,
                expires_at,
                permissions,
                is_active,
            }))
        } else {
            Ok(None)
        }
    }

    fn get_string_field(payload: &HashMap<String, Value>, key: &str) -> Result<String, AppError> {
        payload
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Internal(format!("Missing field: {}", key)))
    }

    fn get_bool_field(payload: &HashMap<String, Value>, key: &str) -> Result<bool, AppError> {
        payload
            .get(key)
            .and_then(|v| v.as_bool())
            .ok_or_else(|| AppError::Internal(format!("Missing field: {}", key)))
    }
}
