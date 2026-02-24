use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    SuperAdmin,
    Admin,
    User,
    ReadOnly,
}

impl Role {
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            Role::SuperAdmin => vec![
                Permission::ReadMemory,
                Permission::WriteMemory,
                Permission::ManageUsers,
                Permission::ManageTenants,
                Permission::ViewAudit,
                Permission::ManageConfig,
            ],
            Role::Admin => vec![
                Permission::ReadMemory,
                Permission::WriteMemory,
                Permission::ManageUsers,
                Permission::ViewAudit,
            ],
            Role::User => vec![Permission::ReadMemory, Permission::WriteMemory],
            Role::ReadOnly => vec![Permission::ReadMemory],
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::SuperAdmin => "super_admin",
            Role::Admin => "admin",
            Role::User => "user",
            Role::ReadOnly => "read_only",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "super_admin" => Some(Role::SuperAdmin),
            "admin" => Some(Role::Admin),
            "user" => Some(Role::User),
            "read_only" => Some(Role::ReadOnly),
            _ => None,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ReadMemory,
    WriteMemory,
    ManageUsers,
    ManageTenants,
    ViewAudit,
    ManageConfig,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::ReadMemory => "read_memory",
            Permission::WriteMemory => "write_memory",
            Permission::ManageUsers => "manage_users",
            Permission::ManageTenants => "manage_tenants",
            Permission::ViewAudit => "view_audit",
            Permission::ManageConfig => "manage_config",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read_memory" => Some(Permission::ReadMemory),
            "write_memory" => Some(Permission::WriteMemory),
            "manage_users" => Some(Permission::ManageUsers),
            "manage_tenants" => Some(Permission::ManageTenants),
            "view_audit" => Some(Permission::ViewAudit),
            "manage_config" => Some(Permission::ManageConfig),
            _ => None,
        }
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub user_id: String,
    pub tenant_id: String,
    pub role: Role,
    pub display_name: String,
    pub email: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct RbacManager {
    pool: SqlitePool,
}

impl RbacManager {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self, String> {
        let path = db_path.as_ref();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create RBAC DB dir: {}", e))?;
        }

        let url = format!("sqlite:{}?mode=rwc", path.display());
        let opts = SqliteConnectOptions::from_str(&url)
            .map_err(|e| format!("Invalid RBAC DB path: {}", e))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await
            .map_err(|e| format!("Failed to open RBAC DB: {}", e))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS rbac_users (
                user_id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'user',
                display_name TEXT NOT NULL DEFAULT '',
                email TEXT NOT NULL DEFAULT '',
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to create rbac_users table: {}", e))?;

        Ok(Self { pool })
    }

    pub async fn add_user(&self, user: UserRecord) {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO rbac_users (user_id, tenant_id, role, display_name, email, is_active, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&user.user_id)
        .bind(&user.tenant_id)
        .bind(user.role.as_str())
        .bind(&user.display_name)
        .bind(&user.email)
        .bind(user.is_active)
        .bind(&user.created_at)
        .bind(&user.updated_at)
        .execute(&self.pool)
        .await;
    }

    pub async fn remove_user(&self, user_id: &str) -> bool {
        let result = sqlx::query("DELETE FROM rbac_users WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await;
        matches!(result, Ok(r) if r.rows_affected() > 0)
    }

    pub async fn get_user(&self, user_id: &str) -> Option<UserRecord> {
        sqlx::query_as::<_, UserRow>("SELECT * FROM rbac_users WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.into())
    }

    pub async fn get_user_role(&self, user_id: &str) -> Option<Role> {
        sqlx::query_scalar::<_, String>("SELECT role FROM rbac_users WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .and_then(|s| Role::from_str(&s))
    }

    pub async fn assign_role(&self, user_id: &str, role: Role) -> bool {
        let now = chrono::Utc::now().to_rfc3339();
        let result =
            sqlx::query("UPDATE rbac_users SET role = ?, updated_at = ? WHERE user_id = ?")
                .bind(role.as_str())
                .bind(&now)
                .bind(user_id)
                .execute(&self.pool)
                .await;
        matches!(result, Ok(r) if r.rows_affected() > 0)
    }

    pub async fn check_permission(&self, user_id: &str, permission: Permission) -> bool {
        if let Some(user) = self.get_user(user_id).await {
            if !user.is_active {
                return false;
            }
            user.role.permissions().contains(&permission)
        } else {
            false
        }
    }

    pub async fn list_users(&self) -> Vec<UserRecord> {
        sqlx::query_as::<_, UserRow>("SELECT * FROM rbac_users")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| r.into())
            .collect()
    }

    pub async fn list_users_by_tenant(&self, tenant_id: &str) -> Vec<UserRecord> {
        sqlx::query_as::<_, UserRow>("SELECT * FROM rbac_users WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| r.into())
            .collect()
    }

    pub async fn update_user(
        &self,
        user_id: &str,
        display_name: Option<String>,
        email: Option<String>,
        is_active: Option<bool>,
    ) -> bool {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE rbac_users SET
                display_name = COALESCE(?, display_name),
                email = COALESCE(?, email),
                is_active = COALESCE(?, is_active),
                updated_at = ?
             WHERE user_id = ?",
        )
        .bind(display_name)
        .bind(email)
        .bind(is_active)
        .bind(&now)
        .bind(user_id)
        .execute(&self.pool)
        .await;
        matches!(result, Ok(r) if r.rows_affected() > 0)
    }

    pub async fn user_count(&self) -> usize {
        sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM rbac_users")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0) as usize
    }

    pub async fn user_count_by_tenant(&self, tenant_id: &str) -> usize {
        sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM rbac_users WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0) as usize
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    user_id: String,
    tenant_id: String,
    role: String,
    display_name: String,
    email: String,
    is_active: bool,
    created_at: String,
    updated_at: String,
}

impl From<UserRow> for UserRecord {
    fn from(r: UserRow) -> Self {
        UserRecord {
            user_id: r.user_id,
            tenant_id: r.tenant_id,
            role: Role::from_str(&r.role).unwrap_or(Role::ReadOnly),
            display_name: r.display_name,
            email: r.email,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user(user_id: &str, tenant_id: &str, role: Role) -> UserRecord {
        let now = chrono::Utc::now().to_rfc3339();
        UserRecord {
            user_id: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            role,
            display_name: user_id.to_string(),
            email: format!("{}@test.com", user_id),
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    async fn temp_manager() -> RbacManager {
        let dir = std::env::temp_dir().join(format!("rbac_db_{}", uuid::Uuid::now_v7()));
        RbacManager::new(dir.join("rbac.db")).await.unwrap()
    }

    #[tokio::test]
    async fn test_role_permissions() {
        assert_eq!(Role::SuperAdmin.permissions().len(), 6);
        assert_eq!(Role::Admin.permissions().len(), 4);
        assert_eq!(Role::User.permissions().len(), 2);
        assert_eq!(Role::ReadOnly.permissions().len(), 1);
    }

    #[tokio::test]
    async fn test_add_and_get_user() {
        let mgr = temp_manager().await;
        let user = make_user("alice", "tenant-1", Role::Admin);
        mgr.add_user(user).await;

        let found = mgr.get_user("alice").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().role, Role::Admin);
    }

    #[tokio::test]
    async fn test_check_permission() {
        let mgr = temp_manager().await;
        mgr.add_user(make_user("admin1", "t1", Role::Admin)).await;
        mgr.add_user(make_user("user1", "t1", Role::User)).await;
        mgr.add_user(make_user("ro1", "t1", Role::ReadOnly)).await;

        assert!(
            mgr.check_permission("admin1", Permission::ManageUsers)
                .await
        );
        assert!(!mgr.check_permission("user1", Permission::ManageUsers).await);
        assert!(mgr.check_permission("user1", Permission::WriteMemory).await);
        assert!(!mgr.check_permission("ro1", Permission::WriteMemory).await);
        assert!(mgr.check_permission("ro1", Permission::ReadMemory).await);
    }

    #[tokio::test]
    async fn test_assign_role() {
        let mgr = temp_manager().await;
        mgr.add_user(make_user("bob", "t1", Role::User)).await;

        assert!(!mgr.check_permission("bob", Permission::ManageUsers).await);
        mgr.assign_role("bob", Role::Admin).await;
        assert!(mgr.check_permission("bob", Permission::ManageUsers).await);
    }

    #[tokio::test]
    async fn test_inactive_user_denied() {
        let mgr = temp_manager().await;
        let mut user = make_user("inactive", "t1", Role::SuperAdmin);
        user.is_active = false;
        mgr.add_user(user).await;

        assert!(
            !mgr.check_permission("inactive", Permission::ReadMemory)
                .await
        );
    }

    #[tokio::test]
    async fn test_list_users_by_tenant() {
        let mgr = temp_manager().await;
        mgr.add_user(make_user("a", "t1", Role::User)).await;
        mgr.add_user(make_user("b", "t1", Role::Admin)).await;
        mgr.add_user(make_user("c", "t2", Role::User)).await;

        let t1_users = mgr.list_users_by_tenant("t1").await;
        assert_eq!(t1_users.len(), 2);
        let t2_users = mgr.list_users_by_tenant("t2").await;
        assert_eq!(t2_users.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_user() {
        let mgr = temp_manager().await;
        mgr.add_user(make_user("del", "t1", Role::User)).await;
        assert!(mgr.remove_user("del").await);
        assert!(mgr.get_user("del").await.is_none());
        assert!(!mgr.remove_user("nonexistent").await);
    }

    #[tokio::test]
    async fn test_role_from_str() {
        assert_eq!(Role::from_str("super_admin"), Some(Role::SuperAdmin));
        assert_eq!(Role::from_str("admin"), Some(Role::Admin));
        assert_eq!(Role::from_str("user"), Some(Role::User));
        assert_eq!(Role::from_str("read_only"), Some(Role::ReadOnly));
        assert_eq!(Role::from_str("invalid"), None);
    }

    #[tokio::test]
    async fn test_update_user() {
        let mgr = temp_manager().await;
        mgr.add_user(make_user("upd", "t1", Role::User)).await;

        mgr.update_user("upd", Some("Updated Name".to_string()), None, None)
            .await;
        let user = mgr.get_user("upd").await.unwrap();
        assert_eq!(user.display_name, "Updated Name");
    }

    #[tokio::test]
    async fn test_persistence_roundtrip() {
        let dir = std::env::temp_dir().join(format!("rbac_db_rt_{}", uuid::Uuid::now_v7()));
        let path = dir.join("rbac.db");

        {
            let mgr = RbacManager::new(&path).await.unwrap();
            mgr.add_user(make_user("persist1", "t1", Role::Admin)).await;
            mgr.add_user(make_user("persist2", "t1", Role::User)).await;
            assert_eq!(mgr.user_count().await, 2);
        }

        {
            let mgr = RbacManager::new(&path).await.unwrap();
            assert_eq!(mgr.user_count().await, 2);
            let u = mgr.get_user("persist1").await.unwrap();
            assert_eq!(u.role, Role::Admin);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_persistence_survives_mutation() {
        let dir = std::env::temp_dir().join(format!("rbac_db_mut_{}", uuid::Uuid::now_v7()));
        let path = dir.join("rbac.db");

        {
            let mgr = RbacManager::new(&path).await.unwrap();
            mgr.add_user(make_user("mut1", "t1", Role::User)).await;
            mgr.assign_role("mut1", Role::SuperAdmin).await;
            mgr.add_user(make_user("mut2", "t1", Role::ReadOnly)).await;
            mgr.remove_user("mut2").await;
        }

        {
            let mgr = RbacManager::new(&path).await.unwrap();
            assert_eq!(mgr.user_count().await, 1);
            let u = mgr.get_user("mut1").await.unwrap();
            assert_eq!(u.role, Role::SuperAdmin);
            assert!(mgr.get_user("mut2").await.is_none());
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_persist_no_data_loss() {
        let dir = std::env::temp_dir().join(format!("rbac_db_conc_{}", uuid::Uuid::now_v7()));
        let path = dir.join("rbac.db");
        let mgr = std::sync::Arc::new(RbacManager::new(&path).await.unwrap());

        let mut handles = Vec::new();
        for i in 0..20 {
            let mgr = mgr.clone();
            handles.push(tokio::spawn(async move {
                let id = format!("user_{}", i);
                mgr.add_user(make_user(&id, "t1", Role::User)).await;
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        assert_eq!(mgr.user_count().await, 20);

        let mgr2 = RbacManager::new(&path).await.unwrap();
        assert_eq!(mgr2.user_count().await, 20);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_add_delete_persist() {
        let dir = std::env::temp_dir().join(format!("rbac_db_addel_{}", uuid::Uuid::now_v7()));
        let path = dir.join("rbac.db");
        let mgr = std::sync::Arc::new(RbacManager::new(&path).await.unwrap());

        for i in 0..10 {
            let id = format!("u_{}", i);
            mgr.add_user(make_user(&id, "t1", Role::User)).await;
        }
        assert_eq!(mgr.user_count().await, 10);

        let mut handles = Vec::new();
        for i in 0..5 {
            let mgr = mgr.clone();
            handles.push(tokio::spawn(async move {
                mgr.remove_user(&format!("u_{}", i)).await;
            }));
        }
        for i in 10..15 {
            let mgr = mgr.clone();
            handles.push(tokio::spawn(async move {
                let id = format!("u_{}", i);
                mgr.add_user(make_user(&id, "t1", Role::User)).await;
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        assert_eq!(mgr.user_count().await, 10);

        let mgr2 = RbacManager::new(&path).await.unwrap();
        assert_eq!(mgr2.user_count().await, 10);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
