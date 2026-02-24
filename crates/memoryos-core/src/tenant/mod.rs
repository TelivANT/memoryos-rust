use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

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
    pool: SqlitePool,
}

impl TenantManager {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self, String> {
        let path = db_path.as_ref();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create DB dir: {}", e))?;
        }

        let url = format!("sqlite:{}?mode=rwc", path.display());
        let opts = SqliteConnectOptions::from_str(&url)
            .map_err(|e| format!("Invalid DB path: {}", e))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await
            .map_err(|e| format!("Failed to open tenant DB: {}", e))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tenants (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                max_users INTEGER NOT NULL DEFAULT 100,
                storage_quota_mb INTEGER NOT NULL DEFAULT 1024,
                api_rate_limit INTEGER NOT NULL DEFAULT 1000,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to create tenants table: {}", e))?;

        Ok(Self { pool })
    }

    pub async fn create_tenant(&self, tenant: Tenant) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO tenants (id, name, description, max_users, storage_quota_mb, api_rate_limit, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&tenant.id)
        .bind(&tenant.name)
        .bind(&tenant.description)
        .bind(tenant.max_users)
        .bind(tenant.storage_quota_mb as i64)
        .bind(tenant.api_rate_limit)
        .bind(tenant.enabled)
        .bind(&tenant.created_at)
        .bind(&tenant.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Tenant '{}' already exists or DB error: {}", tenant.id, e))?;
        Ok(())
    }

    pub async fn get_tenant(&self, tenant_id: &str) -> Option<Tenant> {
        sqlx::query_as::<_, TenantRow>("SELECT * FROM tenants WHERE id = ?")
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.into())
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
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE tenants SET
                name = COALESCE(?, name),
                description = COALESCE(?, description),
                max_users = COALESCE(?, max_users),
                storage_quota_mb = COALESCE(?, storage_quota_mb),
                api_rate_limit = COALESCE(?, api_rate_limit),
                enabled = COALESCE(?, enabled),
                updated_at = ?
             WHERE id = ?",
        )
        .bind(name)
        .bind(description)
        .bind(max_users.map(|v| v as i32))
        .bind(storage_quota_mb.map(|v| v as i64))
        .bind(api_rate_limit.map(|v| v as i32))
        .bind(enabled)
        .bind(&now)
        .bind(tenant_id)
        .execute(&self.pool)
        .await;

        matches!(result, Ok(r) if r.rows_affected() > 0)
    }

    pub async fn delete_tenant(&self, tenant_id: &str) -> bool {
        let result = sqlx::query("DELETE FROM tenants WHERE id = ?")
            .bind(tenant_id)
            .execute(&self.pool)
            .await;
        matches!(result, Ok(r) if r.rows_affected() > 0)
    }

    pub async fn list_tenants(&self) -> Vec<Tenant> {
        sqlx::query_as::<_, TenantRow>("SELECT * FROM tenants")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| r.into())
            .collect()
    }

    pub async fn is_tenant_enabled(&self, tenant_id: &str) -> bool {
        sqlx::query_scalar::<_, bool>("SELECT enabled FROM tenants WHERE id = ?")
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .unwrap_or(false)
    }

    pub async fn tenant_count(&self) -> usize {
        sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM tenants")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0) as usize
    }
}

#[derive(sqlx::FromRow)]
struct TenantRow {
    id: String,
    name: String,
    description: String,
    max_users: i32,
    storage_quota_mb: i64,
    api_rate_limit: i32,
    enabled: bool,
    created_at: String,
    updated_at: String,
}

impl From<TenantRow> for Tenant {
    fn from(r: TenantRow) -> Self {
        Tenant {
            id: r.id,
            name: r.name,
            description: r.description,
            max_users: r.max_users as u32,
            storage_quota_mb: r.storage_quota_mb as u64,
            api_rate_limit: r.api_rate_limit as u32,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
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
        let dir = std::env::temp_dir().join(format!("tenant_db_{}", uuid::Uuid::now_v7()));
        TenantManager::new(dir.join("tenants.db")).await.unwrap()
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
        let dir = std::env::temp_dir().join(format!("tenant_db_rt_{}", uuid::Uuid::now_v7()));
        let path = dir.join("tenants.db");

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
        let dir = std::env::temp_dir().join(format!("tenant_db_mut_{}", uuid::Uuid::now_v7()));
        let path = dir.join("tenants.db");

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
