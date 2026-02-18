//! Configuration management with hot-reload support

use crate::error::{AppError, Result};
use arc_swap::ArcSwap;
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::SystemTime,
};

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
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuthConfig {
    #[serde(default)]
    pub enabled: bool,
    /// 管理员 API Key（用于管理其他 Key）
    #[serde(default)]
    pub admin_key: Option<String>,
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
        if self.server.port == 0 {
            return Err(AppError::Config("Invalid port".into()));
        }
        if !self.llm.providers.contains_key(&self.llm.default_provider) {
            return Err(AppError::Config(format!(
                "Default provider '{}' missing",
                self.llm.default_provider
            )));
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
