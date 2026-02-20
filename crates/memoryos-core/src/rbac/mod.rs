use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

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
    users: Arc<RwLock<HashMap<String, UserRecord>>>,
    persist_path: Option<PathBuf>,
}

impl RbacManager {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            persist_path: None,
        }
    }

    pub fn with_persistence(persist_path: impl AsRef<Path>) -> Self {
        let path = persist_path.as_ref().to_path_buf();
        let users = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(data) => match serde_json::from_str::<Vec<UserRecord>>(&data) {
                    Ok(records) => {
                        let mut map = HashMap::new();
                        for r in records {
                            map.insert(r.user_id.clone(), r);
                        }
                        tracing::info!("Loaded {} RBAC users from {}", map.len(), path.display());
                        map
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse RBAC data: {}", e);
                        HashMap::new()
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read RBAC file: {}", e);
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };
        Self {
            users: Arc::new(RwLock::new(users)),
            persist_path: Some(path),
        }
    }

    async fn persist(&self) {
        if let Some(ref path) = self.persist_path {
            let users = self.users.read().await;
            let records: Vec<&UserRecord> = users.values().collect();
            if let Ok(data) = serde_json::to_string_pretty(&records) {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = std::fs::write(path, data) {
                    tracing::error!("Failed to persist RBAC data: {}", e);
                }
            }
        }
    }

    pub async fn add_user(&self, user: UserRecord) {
        let mut users = self.users.write().await;
        users.insert(user.user_id.clone(), user);
        drop(users);
        self.persist().await;
    }

    pub async fn remove_user(&self, user_id: &str) -> bool {
        let mut users = self.users.write().await;
        let removed = users.remove(user_id).is_some();
        drop(users);
        if removed {
            self.persist().await;
        }
        removed
    }

    pub async fn get_user(&self, user_id: &str) -> Option<UserRecord> {
        let users = self.users.read().await;
        users.get(user_id).cloned()
    }

    pub async fn get_user_role(&self, user_id: &str) -> Option<Role> {
        let users = self.users.read().await;
        users.get(user_id).map(|u| u.role)
    }

    pub async fn assign_role(&self, user_id: &str, role: Role) -> bool {
        let mut users = self.users.write().await;
        if let Some(user) = users.get_mut(user_id) {
            user.role = role;
            user.updated_at = chrono::Utc::now().to_rfc3339();
            drop(users);
            self.persist().await;
            true
        } else {
            false
        }
    }

    pub async fn check_permission(&self, user_id: &str, permission: Permission) -> bool {
        let users = self.users.read().await;
        if let Some(user) = users.get(user_id) {
            if !user.is_active {
                return false;
            }
            user.role.permissions().contains(&permission)
        } else {
            false
        }
    }

    pub async fn list_users(&self) -> Vec<UserRecord> {
        let users = self.users.read().await;
        users.values().cloned().collect()
    }

    pub async fn list_users_by_tenant(&self, tenant_id: &str) -> Vec<UserRecord> {
        let users = self.users.read().await;
        users
            .values()
            .filter(|u| u.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    pub async fn update_user(
        &self,
        user_id: &str,
        display_name: Option<String>,
        email: Option<String>,
        is_active: Option<bool>,
    ) -> bool {
        let mut users = self.users.write().await;
        if let Some(user) = users.get_mut(user_id) {
            if let Some(name) = display_name {
                user.display_name = name;
            }
            if let Some(email) = email {
                user.email = email;
            }
            if let Some(active) = is_active {
                user.is_active = active;
            }
            user.updated_at = chrono::Utc::now().to_rfc3339();
            drop(users);
            self.persist().await;
            true
        } else {
            false
        }
    }

    pub async fn user_count(&self) -> usize {
        let users = self.users.read().await;
        users.len()
    }

    pub async fn user_count_by_tenant(&self, tenant_id: &str) -> usize {
        let users = self.users.read().await;
        users.values().filter(|u| u.tenant_id == tenant_id).count()
    }
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
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

    #[tokio::test]
    async fn test_role_permissions() {
        assert_eq!(Role::SuperAdmin.permissions().len(), 6);
        assert_eq!(Role::Admin.permissions().len(), 4);
        assert_eq!(Role::User.permissions().len(), 2);
        assert_eq!(Role::ReadOnly.permissions().len(), 1);
    }

    #[tokio::test]
    async fn test_add_and_get_user() {
        let mgr = RbacManager::new();
        let user = make_user("alice", "tenant-1", Role::Admin);
        mgr.add_user(user).await;

        let found = mgr.get_user("alice").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().role, Role::Admin);
    }

    #[tokio::test]
    async fn test_check_permission() {
        let mgr = RbacManager::new();
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
        let mgr = RbacManager::new();
        mgr.add_user(make_user("bob", "t1", Role::User)).await;

        assert!(!mgr.check_permission("bob", Permission::ManageUsers).await);
        mgr.assign_role("bob", Role::Admin).await;
        assert!(mgr.check_permission("bob", Permission::ManageUsers).await);
    }

    #[tokio::test]
    async fn test_inactive_user_denied() {
        let mgr = RbacManager::new();
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
        let mgr = RbacManager::new();
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
        let mgr = RbacManager::new();
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
        let mgr = RbacManager::new();
        mgr.add_user(make_user("upd", "t1", Role::User)).await;

        mgr.update_user("upd", Some("Updated Name".to_string()), None, None)
            .await;
        let user = mgr.get_user("upd").await.unwrap();
        assert_eq!(user.display_name, "Updated Name");
    }
}
