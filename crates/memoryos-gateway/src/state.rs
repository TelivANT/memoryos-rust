use memoryos_adapters::{
    llm::{ClaudeAdapter, GeminiAdapter, OllamaAdapter, OpenAiAdapter},
    memory::{DefaultMemoryManager, QdrantStorage, RedisStorage},
};
use memoryos_core::{
    config::AppConfig,
    llm::{ContextInjector, ModelRouter, RouterConfig, StandardInjector, TieredRouter},
    security::{SecurityConfig, SecurityShield},
};
use memoryos_ports::{HistoryStorage, LlmAdapter, MemoryManager, VectorStorage};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::auth::ApiKeyStore;
use crate::worker_monitor::WorkerMonitorSnapshot;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub router: Arc<dyn ModelRouter>,
    pub shield: Arc<SecurityShield>,
    #[allow(dead_code)]
    pub context_injector: Arc<dyn ContextInjector>,
    #[allow(dead_code)]
    pub vector_store: Arc<dyn VectorStorage>,
    pub providers: HashMap<String, Arc<dyn LlmAdapter>>,
    pub memory_manager: Arc<RwLock<Arc<dyn MemoryManager>>>,
    pub history_storage: Option<Arc<dyn HistoryStorage>>,
    pub worker_monitor: Arc<RwLock<WorkerMonitorSnapshot>>,
    pub api_key_store: Option<Arc<ApiKeyStore>>,
    pub async_memory_pipeline: bool,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Self {
        // 1. Init Storage (Qdrant Only)
        let vector_store = Arc::new(
            QdrantStorage::new(&config.storage.vector.url)
                .await
                .expect("Failed to init Qdrant"),
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

        // 4. Init Memory Manager with Coordinator for idempotency
        let default_llm = providers
            .get(&config.llm.default_provider)
            .expect("Default LLM provider not found")
            .clone();

        let redis_storage = Arc::new(
            RedisStorage::new(
                &config.storage.redis.url,
                config.storage.redis.ttl_seconds,
                config.storage.redis.max_messages,
            )
            .expect("Failed to init Redis storage"),
        );

        let memory_manager: Arc<dyn MemoryManager> =
            Arc::new(DefaultMemoryManager::new_with_coordinator(
                vector_store.clone(),
                default_llm,
                redis_storage,
            ));

        // 5. Init Worker Monitor
        let worker_monitor = Arc::new(RwLock::new(WorkerMonitorSnapshot::from_env(false)));

        // 6. Init API Key Store (if using Qdrant)
        let api_key_store = if config.auth.use_redis_store {
            Some(Arc::new(
                ApiKeyStore::new(&config.storage.vector.url)
                    .await
                    .expect("Failed to init API Key Store"),
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
            async_memory_pipeline: false, // Default to sync mode
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
    #[allow(dead_code)]
    pub redis: bool,
    #[allow(dead_code)]
    pub qdrant: bool,
    pub degraded: bool,
    pub mode: String,
    #[allow(dead_code)]
    pub upstream: bool,
    #[allow(dead_code)]
    pub auth_cache: bool,
}
