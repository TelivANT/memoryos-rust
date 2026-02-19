use memoryos_core::AppError;
use qdrant_client::{
    qdrant::{PointStruct, Value},
    Qdrant,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    pub async fn validate_key(&self, api_key: &str) -> Result<bool, AppError> {
        match self.get_metadata(api_key).await? {
            Some(meta) => Ok(meta.is_active),
            None => Ok(false),
        }
    }

    pub async fn create_key(
        &self,
        api_key: &str,
        metadata: ApiKeyMetadata,
    ) -> Result<(), AppError> {
        use qdrant_client::qdrant::{UpsertPointsBuilder, Value};

        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("api_key".to_string(), api_key.to_string().into());
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

        let point_id = Self::hash_api_key(api_key);
        let point = PointStruct::new(point_id, vec![0.0], payload);

        self.qdrant
            .upsert_points(UpsertPointsBuilder::new(API_KEY_COLLECTION, vec![point]))
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        Ok(())
    }

    pub async fn delete_key(&self, api_key: &str) -> Result<(), AppError> {
        use qdrant_client::qdrant::{DeletePointsBuilder, PointId, PointsIdsList};

        let point_id = Self::hash_api_key(api_key);
        let point_id = PointId::from(point_id);

        self.qdrant
            .delete_points(
                DeletePointsBuilder::new(API_KEY_COLLECTION).points(PointsIdsList {
                    ids: vec![point_id],
                }),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        Ok(())
    }

    pub async fn get_metadata(&self, api_key: &str) -> Result<Option<ApiKeyMetadata>, AppError> {
        use qdrant_client::qdrant::{GetPointsBuilder, PointId};

        let point_id = Self::hash_api_key(api_key);
        let point_id = PointId::from(point_id);

        let points = self
            .qdrant
            .get_points(
                GetPointsBuilder::new(API_KEY_COLLECTION, vec![point_id]).with_payload(true),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant error: {}", e)))?;

        if let Some(point) = points.result.first() {
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

    fn hash_api_key(api_key: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        api_key.hash(&mut hasher);
        hasher.finish()
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
