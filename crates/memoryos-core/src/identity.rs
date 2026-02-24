use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalContext {
    pub subject_id: String,
    pub tenant_id: String,
    pub auth_method: String,
    pub scopes: Vec<String>,
    pub api_key_id: Option<String>,
    pub token_jti: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn principal_context_serialization() {
        let ctx = PrincipalContext {
            subject_id: "user-123".to_string(),
            tenant_id: "tenant-1".to_string(),
            auth_method: "api_key".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            api_key_id: Some("key-1".to_string()),
            token_jti: None,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: PrincipalContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.subject_id, "user-123");
        assert_eq!(deserialized.scopes.len(), 2);
        assert!(deserialized.token_jti.is_none());
    }
}
