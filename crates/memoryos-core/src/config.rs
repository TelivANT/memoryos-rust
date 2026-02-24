//! Configuration management with hot-reload support

use crate::error::{AppError, Result};
use arc_swap::ArcSwap;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub llm: LlmConfig,
    pub storage: StorageConfig,
    #[serde(default)]
    pub router: RouterConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_embedding_base_url")]
    pub base_url: String,
    #[serde(default = "default_embedding_model")]
    pub model: String,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: default_embedding_base_url(),
            model: default_embedding_model(),
        }
    }
}

fn default_embedding_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuthConfig {
    #[serde(default)]
    pub enabled: bool,
    /// 管理员 API Key（用于管理其他 Key）
    #[serde(default)]
    pub admin_key: Option<String>,
    /// 管理员 API Keys 列表
    #[serde(default)]
    pub admin_keys: Vec<String>,
    /// 静态 API Keys（兼容小规模场景）
    #[serde(default)]
    pub api_keys: Vec<String>,
    /// 使用 Qdrant 存储 API Keys（大规模场景，持久化）
    #[serde(default)]
    pub use_redis_store: bool, // 名字保留兼容性，实际用 Qdrant
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub default_provider: String,
    #[serde(default = "default_model")]
    pub default_model: String,
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub base_url: String,
    pub api_key_env: Option<String>,
    pub api_key: Option<String>,
    #[serde(default)]
    pub max_retries: Option<u32>,
}

impl ProviderConfig {
    pub fn resolve_api_key(&self) -> String {
        if let Some(env_var) = &self.api_key_env {
            std::env::var(env_var).unwrap_or_default()
        } else {
            self.api_key.clone().unwrap_or_default()
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub redis: RedisConfig,
    pub vector: QdrantConfig,
    // NO SQL HERE! Qdrant Only.
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    #[serde(default = "default_redis_ttl")]
    pub ttl_seconds: usize,
    #[serde(default = "default_max_messages")]
    pub max_messages: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouterConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_hot_threshold")]
    pub hot_threshold: f32,
    #[serde(default = "default_max_local_tokens")]
    pub max_local_tokens: usize,
    #[serde(default = "default_sensitive_keywords")]
    pub sensitive_keywords: Vec<String>,
    #[serde(default)]
    pub local_backends: Vec<String>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            enable: true,
            hot_threshold: 0.85,
            max_local_tokens: 2000,
            sensitive_keywords: vec!["confidential".to_string()],
            local_backends: vec![],
        }
    }
}

fn default_worker_threads() -> usize {
    num_cpus::get()
}
fn default_timeout() -> u64 {
    60
}
fn default_model() -> String {
    "gpt-4o-mini".to_string()
}
fn default_redis_ttl() -> usize {
    3600
}
fn default_max_messages() -> usize {
    20
}
fn default_true() -> bool {
    true
}
fn default_hot_threshold() -> f32 {
    0.85
}
fn default_max_local_tokens() -> usize {
    2000
}
fn default_sensitive_keywords() -> Vec<String> {
    vec!["confidential".to_string()]
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let config_path =
            std::env::var("MEMORYOS_CONFIG").unwrap_or_else(|_| "config.toml".to_string());

        let builder = config::Config::builder()
            .add_source(config::File::with_name(&config_path).required(false))
            .add_source(config::Environment::with_prefix("MEMORYOS").separator("__"));

        let config = builder
            .build()
            .map_err(|e| AppError::Config(format!("Failed to load config: {}", e)))?;

        config
            .try_deserialize()
            .map_err(|e| AppError::Config(format!("Invalid config: {}", e)))
    }

    pub fn validate(&self) -> Result<()> {
        // Server validation
        self.validate_server()?;

        // LLM validation
        self.validate_llm()?;

        // Storage validation
        self.validate_storage()?;

        // Auth validation
        self.validate_auth()?;

        Ok(())
    }

    fn validate_server(&self) -> Result<()> {
        if self.server.port == 0 {
            return Err(AppError::Config("server.port cannot be 0".into()));
        }

        if self.server.host.is_empty() {
            return Err(AppError::Config("server.host cannot be empty".into()));
        }

        if self.server.worker_threads == 0 {
            return Err(AppError::Config("server.worker_threads must be > 0".into()));
        }

        if self.server.timeout_seconds == 0 {
            return Err(AppError::Config(
                "server.timeout_seconds must be > 0".into(),
            ));
        }

        Ok(())
    }

    fn validate_llm(&self) -> Result<()> {
        // Check default provider exists
        if !self.llm.providers.contains_key(&self.llm.default_provider) {
            return Err(AppError::Config(format!(
                "llm.default_provider '{}' not found in llm.providers. Available providers: {}",
                self.llm.default_provider,
                self.llm
                    .providers
                    .keys()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        // Check default model is not empty
        if self.llm.default_model.is_empty() {
            return Err(AppError::Config("llm.default_model cannot be empty".into()));
        }

        // Validate each provider
        for (name, provider) in &self.llm.providers {
            self.validate_provider(name, provider)?;
        }

        Ok(())
    }

    fn validate_provider(&self, name: &str, provider: &ProviderConfig) -> Result<()> {
        // Check provider type is valid
        let valid_types = [
            "openai",
            "gemini",
            "claude",
            "ollama",
            "deepseek",
            "openrouter",
            "azure-openai",
            "cohere",
            "groq",
            "mistral",
        ];

        if !valid_types.contains(&provider.provider_type.as_str()) {
            return Err(AppError::Config(format!(
                "llm.providers.{}.type '{}' is invalid. Valid types: {}",
                name,
                provider.provider_type,
                valid_types.join(", ")
            )));
        }

        // Check base_url is not empty
        if provider.base_url.is_empty() {
            return Err(AppError::Config(format!(
                "llm.providers.{}.base_url cannot be empty",
                name
            )));
        }

        // Check base_url is valid URL
        if !provider.base_url.starts_with("http://") && !provider.base_url.starts_with("https://") {
            return Err(AppError::Config(format!(
                "llm.providers.{}.base_url must start with http:// or https://",
                name
            )));
        }

        // Check API key is configured (either directly or via env var)
        let api_key = provider.resolve_api_key();
        if api_key.is_empty() && provider.provider_type != "ollama" {
            return Err(AppError::Config(format!(
                "llm.providers.{}.api_key or api_key_env must be configured (or set the environment variable)",
                name
            )));
        }

        Ok(())
    }

    fn validate_storage(&self) -> Result<()> {
        // Redis validation
        if self.storage.redis.url.is_empty() {
            return Err(AppError::Config("storage.redis.url cannot be empty".into()));
        }

        if !self.storage.redis.url.starts_with("redis://")
            && !self.storage.redis.url.starts_with("rediss://")
        {
            return Err(AppError::Config(
                "storage.redis.url must start with redis:// or rediss://".into(),
            ));
        }

        if self.storage.redis.ttl_seconds == 0 {
            return Err(AppError::Config(
                "storage.redis.ttl_seconds must be > 0".into(),
            ));
        }

        if self.storage.redis.max_messages == 0 {
            return Err(AppError::Config(
                "storage.redis.max_messages must be > 0".into(),
            ));
        }

        // Qdrant validation
        if self.storage.vector.url.is_empty() {
            return Err(AppError::Config(
                "storage.vector.url cannot be empty".into(),
            ));
        }

        if !self.storage.vector.url.starts_with("http://")
            && !self.storage.vector.url.starts_with("https://")
        {
            return Err(AppError::Config(
                "storage.vector.url must start with http:// or https://".into(),
            ));
        }

        Ok(())
    }

    fn validate_auth(&self) -> Result<()> {
        if !self.auth.enabled {
            return Ok(());
        }

        // If auth is enabled, check that at least one admin key is configured
        if self.auth.admin_keys.is_empty() && self.auth.admin_key.is_none() {
            return Err(AppError::Config(
                "auth.enabled is true but no admin_keys or admin_key configured".into(),
            ));
        }

        // Validate admin keys are not empty strings
        for (i, key) in self.auth.admin_keys.iter().enumerate() {
            if key.is_empty() {
                return Err(AppError::Config(format!(
                    "auth.admin_keys[{}] cannot be empty string",
                    i
                )));
            }
        }

        // Validate API keys are not empty strings
        for (i, key) in self.auth.api_keys.iter().enumerate() {
            if key.is_empty() {
                return Err(AppError::Config(format!(
                    "auth.api_keys[{}] cannot be empty string",
                    i
                )));
            }
        }

        Ok(())
    }
}

pub struct ConfigManager {
    config: ArcSwap<AppConfig>,
    config_path: PathBuf,
    last_modified: Option<SystemTime>,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let config_path =
            std::env::var("MEMORYOS_CONFIG").unwrap_or_else(|_| "config.toml".to_string());
        let initial = Arc::new(AppConfig::load()?);
        initial.validate()?;

        let path = PathBuf::from(config_path);
        let modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        Ok(Self {
            config: ArcSwap::from(initial),
            config_path: path,
            last_modified: modified,
        })
    }

    pub fn get(&self) -> Arc<AppConfig> {
        self.config.load_full()
    }

    /// 检查配置文件是否变化，如果变化则重新加载
    /// 返回 Ok(true) 表示重新加载成功，Ok(false) 表示无变化
    pub fn reload_if_changed(&mut self) -> Result<bool> {
        let current_modified = std::fs::metadata(&self.config_path)
            .ok()
            .and_then(|m| m.modified().ok());

        if current_modified != self.last_modified {
            let new_config = Arc::new(AppConfig::load()?);
            new_config.validate()?;
            self.config.store(new_config);
            self.last_modified = current_modified;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_config() -> AppConfig {
        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                provider_type: "openai".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: Some("sk-test-key".to_string()),
                api_key_env: None,
                max_retries: None,
            },
        );

        AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                worker_threads: 4,
                timeout_seconds: 60,
            },
            llm: LlmConfig {
                default_provider: "openai".to_string(),
                default_model: "gpt-4o-mini".to_string(),
                providers,
            },
            storage: StorageConfig {
                redis: RedisConfig {
                    url: "redis://localhost:6379".to_string(),
                    ttl_seconds: 3600,
                    max_messages: 20,
                },
                vector: QdrantConfig {
                    url: "http://localhost:6334".to_string(),
                },
            },
            router: RouterConfig::default(),
            auth: AuthConfig::default(),
            embedding: EmbeddingConfig::default(),
        }
    }

    #[test]
    fn test_valid_config() {
        let config = create_valid_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_port() {
        let mut config = create_valid_config();
        config.server.port = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("port"));
    }

    #[test]
    fn test_empty_host() {
        let mut config = create_valid_config();
        config.server.host = "".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("host"));
    }

    #[test]
    fn test_zero_worker_threads() {
        let mut config = create_valid_config();
        config.server.worker_threads = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("worker_threads"));
    }

    #[test]
    fn test_zero_timeout() {
        let mut config = create_valid_config();
        config.server.timeout_seconds = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout"));
    }

    #[test]
    fn test_missing_default_provider() {
        let mut config = create_valid_config();
        config.llm.default_provider = "nonexistent".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_empty_default_model() {
        let mut config = create_valid_config();
        config.llm.default_model = "".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("default_model"));
    }

    #[test]
    fn test_invalid_provider_type() {
        let mut config = create_valid_config();
        config.llm.providers.insert(
            "invalid".to_string(),
            ProviderConfig {
                provider_type: "invalid_type".to_string(),
                base_url: "https://api.example.com".to_string(),
                api_key: Some("key".to_string()),
                api_key_env: None,
                max_retries: None,
            },
        );
        config.llm.default_provider = "invalid".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid"));
    }

    #[test]
    fn test_empty_base_url() {
        let mut config = create_valid_config();
        config.llm.providers.get_mut("openai").unwrap().base_url = "".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("base_url"));
    }

    #[test]
    fn test_invalid_base_url_scheme() {
        let mut config = create_valid_config();
        config.llm.providers.get_mut("openai").unwrap().base_url =
            "ftp://api.openai.com".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("http"));
    }

    #[test]
    fn test_missing_api_key() {
        let mut config = create_valid_config();
        config.llm.providers.get_mut("openai").unwrap().api_key = None;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("api_key"));
    }

    #[test]
    fn test_ollama_without_api_key() {
        let mut config = create_valid_config();
        config.llm.providers.insert(
            "ollama".to_string(),
            ProviderConfig {
                provider_type: "ollama".to_string(),
                base_url: "http://localhost:11434/v1".to_string(),
                api_key: None,
                api_key_env: None,
                max_retries: None,
            },
        );
        config.llm.default_provider = "ollama".to_string();
        let result = config.validate();
        assert!(result.is_ok()); // Ollama doesn't require API key
    }

    #[test]
    fn test_empty_redis_url() {
        let mut config = create_valid_config();
        config.storage.redis.url = "".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("redis.url"));
    }

    #[test]
    fn test_invalid_redis_url_scheme() {
        let mut config = create_valid_config();
        config.storage.redis.url = "http://localhost:6379".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("redis://"));
    }

    #[test]
    fn test_zero_redis_ttl() {
        let mut config = create_valid_config();
        config.storage.redis.ttl_seconds = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ttl_seconds"));
    }

    #[test]
    fn test_zero_max_messages() {
        let mut config = create_valid_config();
        config.storage.redis.max_messages = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_messages"));
    }

    #[test]
    fn test_empty_vector_url() {
        let mut config = create_valid_config();
        config.storage.vector.url = "".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("vector.url"));
    }

    #[test]
    fn test_invalid_vector_url_scheme() {
        let mut config = create_valid_config();
        config.storage.vector.url = "ftp://localhost:6334".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("http"));
    }

    #[test]
    fn test_auth_enabled_without_admin_keys() {
        let mut config = create_valid_config();
        config.auth.enabled = true;
        config.auth.admin_keys = vec![];
        config.auth.admin_key = None;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("admin"));
    }

    #[test]
    fn test_auth_enabled_with_admin_key() {
        let mut config = create_valid_config();
        config.auth.enabled = true;
        config.auth.admin_key = Some("admin-key".to_string());
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_enabled_with_admin_keys() {
        let mut config = create_valid_config();
        config.auth.enabled = true;
        config.auth.admin_keys = vec!["admin-key-1".to_string(), "admin-key-2".to_string()];
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_admin_key_in_list() {
        let mut config = create_valid_config();
        config.auth.enabled = true;
        config.auth.admin_keys = vec!["admin-key".to_string(), "".to_string()];
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("admin_keys"));
    }

    #[test]
    fn test_empty_api_key_in_list() {
        let mut config = create_valid_config();
        config.auth.enabled = true;
        config.auth.admin_keys = vec!["admin-key".to_string()];
        config.auth.api_keys = vec!["user-key".to_string(), "".to_string()];
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("api_keys"));
    }

    #[test]
    fn test_auth_disabled() {
        let mut config = create_valid_config();
        config.auth.enabled = false;
        config.auth.admin_keys = vec![];
        let result = config.validate();
        assert!(result.is_ok()); // Should pass when auth is disabled
    }
}
