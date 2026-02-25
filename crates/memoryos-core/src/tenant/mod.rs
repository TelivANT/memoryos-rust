use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub description: String,
    pub max_users: u32,
    pub storage_quota_mb: u64,
    pub api_rate_limit: u32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: String,
}

#[derive(Clone)]
pub struct TenantManager {
    tenants: std::sync::Arc<RwLock<HashMap<String, Tenant>>>,
    persist_path: PathBuf,
}

impl TenantManager {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self, String> {
        let path = db_path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create tenant dir: {}", e))?;
        }

        let tenants = if path.exists() {
            let data = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| format!("Failed to read tenant file: {}", e))?;
            let list: Vec<Tenant> = serde_json::from_str(&data).unwrap_or_default();
            list.into_iter().map(|t| (t.id.clone(), t)).collect()
        } else {
            HashMap::new()
        };

        Ok(Self {
            tenants: std::sync::Arc::new(RwLock::new(tenants)),
            persist_path: path,
        })
    }

    async fn persist(&self) {
        let tenants = self.tenants.read().await;
        let list: Vec<&Tenant> = tenants.values().collect();
        if let Ok(data) = serde_json::to_string_pretty(&list) {
            let _ = tokio::fs::write(&self.persist_path, data).await;
        }
    }

    pub async fn create_tenant(&self, tenant: Tenant) -> Result<(), String> {
        {
            let mut tenants = self.tenants.write().await;
            if tenants.contains_key(&tenant.id) {
                return Err(format!("Tenant '{}' already exists", tenant.id));
            }
            tenants.insert(tenant.id.clone(), tenant);
        }
        self.persist().await;
        Ok(())
    }

    pub async fn get_tenant(&self, tenant_id: &str) -> Option<Tenant> {
        let tenants = self.tenants.read().await;
        tenants.get(tenant_id).cloned()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_tenant(
        &self,
        tenant_id: &str,
        name: Option<String>,
        description: Option<String>,
        max_users: Option<u32>,
        storage_quota_mb: Option<u64>,
        api_rate_limit: Option<u32>,
        enabled: Option<bool>,
    ) -> bool {
        let found = {
            let mut tenants = self.tenants.write().await;
            if let Some(t) = tenants.get_mut(tenant_id) {
                if let Some(v) = name {
                    t.name = v;
                }
                if let Some(v) = description {
                    t.description = v;
                }
                if let Some(v) = max_users {
                    t.max_users = v;
                }
                if let Some(v) = storage_quota_mb {
                    t.storage_quota_mb = v;
                }
                if let Some(v) = api_rate_limit {
                    t.api_rate_limit = v;
                }
                if let Some(v) = enabled {
                    t.enabled = v;
                }
                t.updated_at = chrono::Utc::now().to_rfc3339();
                true
            } else {
                false
            }
        };
        if found {
            self.persist().await;
        }
        found
    }

    pub async fn delete_tenant(&self, tenant_id: &str) -> bool {
        let removed = {
            let mut tenants = self.tenants.write().await;
            tenants.remove(tenant_id).is_some()
        };
        if removed {
            self.persist().await;
        }
        removed
    }

    pub async fn list_tenants(&self) -> Vec<Tenant> {
        let tenants = self.tenants.read().await;
        tenants.values().cloned().collect()
    }

    pub async fn is_tenant_enabled(&self, tenant_id: &str) -> bool {
        let tenants = self.tenants.read().await;
        tenants.get(tenant_id).map(|t| t.enabled).unwrap_or(false)
    }

    pub async fn tenant_count(&self) -> usize {
        let tenants = self.tenants.read().await;
        tenants.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tenant(id: &str) -> Tenant {
        let now = chrono::Utc::now().to_rfc3339();
        Tenant {
            id: id.to_string(),
            name: format!("Tenant {}", id),
            description: "Test tenant".to_string(),
            max_users: 100,
            storage_quota_mb: 1024,
            api_rate_limit: 1000,
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    async fn temp_manager() -> TenantManager {
        let dir = std::env::temp_dir().join(format!("tenant_json_{}", uuid::Uuid::now_v7()));
        TenantManager::new(dir.join("tenants.json")).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_tenant() {
        let mgr = temp_manager().await;
        mgr.create_tenant(make_tenant("t1")).await.unwrap();

        let t = mgr.get_tenant("t1").await;
        assert!(t.is_some());
        assert_eq!(t.unwrap().name, "Tenant t1");
    }

    #[tokio::test]
    async fn test_duplicate_tenant() {
        let mgr = temp_manager().await;
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        let result = mgr.create_tenant(make_tenant("t1")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_tenant() {
        let mgr = temp_manager().await;
        mgr.create_tenant(make_tenant("t1")).await.unwrap();

        mgr.update_tenant(
            "t1",
            Some("New Name".to_string()),
            None,
            None,
            None,
            None,
            None,
        )
        .await;
        let t = mgr.get_tenant("t1").await.unwrap();
        assert_eq!(t.name, "New Name");
    }

    #[tokio::test]
    async fn test_delete_tenant() {
        let mgr = temp_manager().await;
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        assert!(mgr.delete_tenant("t1").await);
        assert!(mgr.get_tenant("t1").await.is_none());
        assert!(!mgr.delete_tenant("nonexistent").await);
    }

    #[tokio::test]
    async fn test_list_tenants() {
        let mgr = temp_manager().await;
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        mgr.create_tenant(make_tenant("t2")).await.unwrap();
        assert_eq!(mgr.list_tenants().await.len(), 2);
    }

    #[tokio::test]
    async fn test_tenant_enabled() {
        let mgr = temp_manager().await;
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        assert!(mgr.is_tenant_enabled("t1").await);

        mgr.update_tenant("t1", None, None, None, None, None, Some(false))
            .await;
        assert!(!mgr.is_tenant_enabled("t1").await);
    }

    #[tokio::test]
    async fn test_tenant_count() {
        let mgr = temp_manager().await;
        assert_eq!(mgr.tenant_count().await, 0);
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        mgr.create_tenant(make_tenant("t2")).await.unwrap();
        assert_eq!(mgr.tenant_count().await, 2);
    }

    #[tokio::test]
    async fn test_persistence_roundtrip() {
        let dir = std::env::temp_dir().join(format!("tenant_json_rt_{}", uuid::Uuid::now_v7()));
        let path = dir.join("tenants.json");

        {
            let mgr = TenantManager::new(&path).await.unwrap();
            mgr.create_tenant(make_tenant("pt1")).await.unwrap();
            mgr.create_tenant(make_tenant("pt2")).await.unwrap();
            assert_eq!(mgr.tenant_count().await, 2);
        }

        {
            let mgr = TenantManager::new(&path).await.unwrap();
            assert_eq!(mgr.tenant_count().await, 2);
            let t = mgr.get_tenant("pt1").await.unwrap();
            assert_eq!(t.name, "Tenant pt1");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_persistence_survives_mutation() {
        let dir = std::env::temp_dir().join(format!("tenant_json_mut_{}", uuid::Uuid::now_v7()));
        let path = dir.join("tenants.json");

        {
            let mgr = TenantManager::new(&path).await.unwrap();
            mgr.create_tenant(make_tenant("mt1")).await.unwrap();
            mgr.create_tenant(make_tenant("mt2")).await.unwrap();
            mgr.update_tenant(
                "mt1",
                Some("Renamed".to_string()),
                None,
                None,
                None,
                None,
                None,
            )
            .await;
            mgr.delete_tenant("mt2").await;
        }

        {
            let mgr = TenantManager::new(&path).await.unwrap();
            assert_eq!(mgr.tenant_count().await, 1);
            let t = mgr.get_tenant("mt1").await.unwrap();
            assert_eq!(t.name, "Renamed");
            assert!(mgr.get_tenant("mt2").await.is_none());
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
