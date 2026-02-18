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
