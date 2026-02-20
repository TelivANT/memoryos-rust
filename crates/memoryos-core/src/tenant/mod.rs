use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
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
    tenants: Arc<RwLock<HashMap<String, Tenant>>>,
}

impl TenantManager {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_tenant(&self, tenant: Tenant) -> Result<(), String> {
        let mut tenants = self.tenants.write().await;
        if tenants.contains_key(&tenant.id) {
            return Err(format!("Tenant '{}' already exists", tenant.id));
        }
        tenants.insert(tenant.id.clone(), tenant);
        Ok(())
    }

    pub async fn get_tenant(&self, tenant_id: &str) -> Option<Tenant> {
        let tenants = self.tenants.read().await;
        tenants.get(tenant_id).cloned()
    }

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
        let mut tenants = self.tenants.write().await;
        if let Some(tenant) = tenants.get_mut(tenant_id) {
            if let Some(n) = name {
                tenant.name = n;
            }
            if let Some(d) = description {
                tenant.description = d;
            }
            if let Some(m) = max_users {
                tenant.max_users = m;
            }
            if let Some(s) = storage_quota_mb {
                tenant.storage_quota_mb = s;
            }
            if let Some(r) = api_rate_limit {
                tenant.api_rate_limit = r;
            }
            if let Some(e) = enabled {
                tenant.enabled = e;
            }
            tenant.updated_at = chrono::Utc::now().to_rfc3339();
            true
        } else {
            false
        }
    }

    pub async fn delete_tenant(&self, tenant_id: &str) -> bool {
        let mut tenants = self.tenants.write().await;
        tenants.remove(tenant_id).is_some()
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

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
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

    #[tokio::test]
    async fn test_create_and_get_tenant() {
        let mgr = TenantManager::new();
        mgr.create_tenant(make_tenant("t1")).await.unwrap();

        let t = mgr.get_tenant("t1").await;
        assert!(t.is_some());
        assert_eq!(t.unwrap().name, "Tenant t1");
    }

    #[tokio::test]
    async fn test_duplicate_tenant() {
        let mgr = TenantManager::new();
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        let result = mgr.create_tenant(make_tenant("t1")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_tenant() {
        let mgr = TenantManager::new();
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
        let mgr = TenantManager::new();
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        assert!(mgr.delete_tenant("t1").await);
        assert!(mgr.get_tenant("t1").await.is_none());
        assert!(!mgr.delete_tenant("nonexistent").await);
    }

    #[tokio::test]
    async fn test_list_tenants() {
        let mgr = TenantManager::new();
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        mgr.create_tenant(make_tenant("t2")).await.unwrap();
        assert_eq!(mgr.list_tenants().await.len(), 2);
    }

    #[tokio::test]
    async fn test_tenant_enabled() {
        let mgr = TenantManager::new();
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        assert!(mgr.is_tenant_enabled("t1").await);

        mgr.update_tenant("t1", None, None, None, None, None, Some(false))
            .await;
        assert!(!mgr.is_tenant_enabled("t1").await);
    }

    #[tokio::test]
    async fn test_tenant_count() {
        let mgr = TenantManager::new();
        assert_eq!(mgr.tenant_count().await, 0);
        mgr.create_tenant(make_tenant("t1")).await.unwrap();
        mgr.create_tenant(make_tenant("t2")).await.unwrap();
        assert_eq!(mgr.tenant_count().await, 2);
    }
}
