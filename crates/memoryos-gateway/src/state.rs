use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use memoryos_core::{
    config::AppConfig,
    llm::{ModelRouter, TieredRouter, RouterConfig, ContextInjector, StandardInjector},
    security::{SecurityShield, SecurityConfig},
};
use memoryos_ports::{LlmAdapter, VectorStorage, MemoryManager, HistoryStorage, ShortTermStorage};
use memoryos_adapters::{
    llm::{OpenAiAdapter, GeminiAdapter, ClaudeAdapter, OllamaAdapter},
    memory::{QdrantStorage, DefaultMemoryManager, RedisStorage},
};

use crate::worker_monitor::WorkerMonitorSnapshot;
use crate::auth::ApiKeyStore;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub router: Arc<dyn ModelRouter>,
    pub shield: Arc<SecurityShield>,
    pub context_injector: Arc<dyn ContextInjector>,
    pub vector_store: Arc<dyn VectorStorage>,
    pub providers: HashMap<String, Arc<dyn LlmAdapter>>,
    pub memory_manager: Arc<RwLock<Arc<dyn MemoryManager>>>,
    pub history_storage: Option<Arc<dyn HistoryStorage>>,
    pub worker_monitor: Arc<RwLock<WorkerMonitorSnapshot>>,
    pub api_key_store: Option<Arc<ApiKeyStore>>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Self {
        // 1. Init Storage (Qdrant Only)
        let vector_store = Arc::new(
            QdrantStorage::new(&config.storage.vector.url).await.expect("Failed to init Qdrant")
        );

        // 2. Init Providers Registry
        let mut providers: HashMap<String, Arc<dyn LlmAdapter>> = HashMap::new();
        
        for (name, cfg) in &config.llm.providers {
            let api_key = cfg.resolve_api_key();
            let adapter: Arc<dyn LlmAdapter> = match cfg.provider_type.as_str() {
                "openai" => Arc::new(OpenAiAdapter::new(api_key, cfg.base_url.clone())),
                "gemini" => Arc::new(GeminiAdapter::new(api_key, cfg.base_url.clone())),
                "claude" => Arc::new(ClaudeAdapter::new(api_key, cfg.base_url.clone())),
                "ollama" => Arc::new(OllamaAdapter::new(cfg.base_url.clone())),
                _ => panic!("Unsupported provider type: {}", cfg.provider_type),
            };
            providers.insert(name.clone(), adapter);
        }

        // 3. Init Core Logic
        let router_config = RouterConfig {
            enable: config.router.enable,
            direct_hit_threshold: config.router.hot_threshold + 0.07,
            hot_threshold: config.router.hot_threshold,
            max_local_tokens: config.router.max_local_tokens,
            local_backends: config.router.local_backends.clone(),
            cloud_model: config.llm.default_provider.clone(), 
        };
        let router = Arc::new(TieredRouter::new(router_config));

        let shield_config = SecurityConfig {
            enable_pii_sanitization: true,
            enable_injection_check: true,
            strict_mode: false,
            sensitive_keywords: config.router.sensitive_keywords.clone(),
        };
        let shield = Arc::new(SecurityShield::new(shield_config));

        let context_injector = Arc::new(StandardInjector::new(2000));

        // 4. Init Memory Manager
        let redis_storage: Arc<dyn ShortTermStorage> = Arc::new(
            RedisStorage::new(&config.storage.redis.url, 3600, 20)
                .expect("Failed to init Redis")
        );
        
        let default_llm = providers.get(&config.llm.default_provider)
            .expect("Default LLM provider not found")
            .clone();
        
        let memory_manager: Arc<dyn MemoryManager> = Arc::new(
            DefaultMemoryManager::new(
                redis_storage,
                vector_store.clone(),
                default_llm,
            )
        );

        // 5. Init Worker Monitor
        let worker_monitor = Arc::new(RwLock::new(WorkerMonitorSnapshot::from_env(false)));

        // 6. Init API Key Store (if using Qdrant)
        let api_key_store = if config.auth.use_redis_store {
            Some(Arc::new(
                ApiKeyStore::new(&config.storage.vector.url)
                    .await
                    .expect("Failed to init API Key Store")
            ))
        } else {
            None
        };

        Self {
            config: Arc::new(config),
            router,
            shield,
            context_injector,
            vector_store,
            providers,
            memory_manager: Arc::new(RwLock::new(memory_manager)),
            history_storage: None, // Optional feature
            worker_monitor,
            api_key_store,
        }
    }

    pub fn get_adapter(&self, name: &str) -> Option<Arc<dyn LlmAdapter>> {
        self.providers.get(name).cloned()
    }

    pub async fn current_health(&self) -> HealthStatus {
        // Simple health check
        HealthStatus {
            redis: true,  // TODO: actual check
            qdrant: true, // TODO: actual check
            degraded: false,
            mode: "ready".to_string(),
            upstream: true,
            auth_cache: true,
        }
    }

    pub async fn degraded_mode(&self) -> bool {
        let health = self.current_health().await;
        health.degraded
    }

    pub async fn current_worker_monitor(&self) -> WorkerMonitorSnapshot {
        self.worker_monitor.read().await.clone()
    }
}

#[derive(Clone)]
pub struct HealthStatus {
    pub redis: bool,
    pub qdrant: bool,
    pub degraded: bool,
    pub mode: String,
    pub upstream: bool,
    pub auth_cache: bool,
}
