use memoryos_adapters::{
    llm::{ClaudeAdapter, GeminiAdapter, OllamaAdapter, OpenAiAdapter},
    memory::{DefaultMemoryManager, QdrantStorage, RedisStorage},
};
use memoryos_core::{
    config::AppConfig,
    llm::{ContextInjector, ModelRouter, RouterConfig, StandardInjector, TieredRouter},
    rbac::RbacManager,
    security::{SecurityConfig, SecurityShield},
    tenant::TenantManager,
    AppError,
};
use memoryos_ports::{EventBus, HistoryStorage, LlmAdapter, MemoryManager, VectorStorage};
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
    pub vector_store: Arc<dyn VectorStorage>,
    pub qdrant_storage: Arc<QdrantStorage>,
    pub redis_storage: Arc<RedisStorage>,
    pub providers: HashMap<String, Arc<dyn LlmAdapter>>,
    pub memory_manager: Arc<RwLock<Arc<dyn MemoryManager>>>,
    pub history_storage: Option<Arc<dyn HistoryStorage>>,
    pub worker_monitor: Arc<RwLock<WorkerMonitorSnapshot>>,
    pub api_key_store: Option<Arc<ApiKeyStore>>,
    pub async_memory_pipeline: bool,
    pub event_bus: Option<Arc<dyn EventBus>>,
    pub faq_matcher: Arc<tokio::sync::RwLock<memoryos_core::OptimizedFaqMatcher>>,
    pub rbac_manager: Option<RbacManager>,
    pub tenant_manager: Option<TenantManager>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, AppError> {
        // 1. Init Storage (Qdrant Only)
        let vector_store = Arc::new(
            QdrantStorage::new(&config.storage.vector.url)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to init Qdrant: {}", e)))?,
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
                other => {
                    tracing::warn!("Skipping unsupported provider type: {}", other);
                    continue;
                }
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
            .ok_or_else(|| {
                AppError::Config(format!(
                    "Default LLM provider '{}' not found in [llm.providers]",
                    config.llm.default_provider
                ))
            })?
            .clone();

        let redis_storage = Arc::new(
            RedisStorage::new(
                &config.storage.redis.url,
                config.storage.redis.ttl_seconds,
                config.storage.redis.max_messages,
            )
            .map_err(|e| AppError::Internal(format!("Failed to init Redis storage: {}", e)))?,
        );

        let memory_manager: Arc<dyn MemoryManager> = Arc::new(
            DefaultMemoryManager::new_with_coordinator(
                vector_store.clone(),
                default_llm,
                redis_storage.clone(),
            )
            .with_embedding_config(&config.embedding),
        );

        // 5. Init History Storage (uses same Qdrant client)
        let history_storage: Option<Arc<dyn HistoryStorage>> =
            match memoryos_adapters::history::QdrantHistoryStorage::new(
                vector_store.client().clone(),
                "memory_history".to_string(),
            )
            .await
            {
                Ok(hs) => {
                    tracing::info!(
                        "History storage initialized (Qdrant collection: memory_history)"
                    );
                    Some(Arc::new(hs))
                }
                Err(e) => {
                    tracing::warn!("History storage init failed, feature disabled: {}", e);
                    None
                }
            };

        // 6. Init Worker Monitor
        let worker_monitor = Arc::new(RwLock::new(WorkerMonitorSnapshot::from_env(false)));

        // 6. Init API Key Store (if using Qdrant)
        let api_key_store = if config.auth.use_redis_store {
            match ApiKeyStore::new(&config.storage.vector.url).await {
                Ok(store) => Some(Arc::new(store)),
                Err(e) => {
                    tracing::warn!("API Key Store init failed, feature disabled: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let data_dir = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".memoryos");
        let _ = std::fs::create_dir_all(&data_dir);

        let rbac_manager = match RbacManager::new(data_dir.join("rbac_users.json")).await {
            Ok(mgr) => {
                tracing::info!("RBAC manager initialized");
                Some(mgr)
            }
            Err(e) => {
                tracing::warn!("RBAC manager init failed, feature disabled: {}", e);
                None
            }
        };
        let tenant_manager = match TenantManager::new(data_dir.join("tenants.json")).await {
            Ok(mgr) => {
                tracing::info!("Tenant manager initialized");
                Some(mgr)
            }
            Err(e) => {
                tracing::warn!("Tenant manager init failed, feature disabled: {}", e);
                None
            }
        };

        let event_bus: Option<Arc<dyn EventBus>> = {
            let stream_key =
                std::env::var("MEMORYOS_WORKER_STREAM").unwrap_or_else(|_| "chat_log".to_string());
            match memoryos_adapters::RedisStreamEventBus::new(
                &config.storage.redis.url,
                &stream_key,
            ) {
                Ok(bus) => Some(Arc::new(bus)),
                Err(e) => {
                    tracing::warn!("EventBus init failed, async events disabled: {}", e);
                    None
                }
            }
        };

        let faq_matcher = Arc::new(RwLock::new(memoryos_core::OptimizedFaqMatcher::new(10_000)));

        Ok(Self {
            config: Arc::new(config),
            router,
            shield,
            context_injector,
            vector_store: vector_store.clone(),
            qdrant_storage: vector_store,
            redis_storage,
            providers,
            memory_manager: Arc::new(RwLock::new(memory_manager)),
            history_storage,
            worker_monitor,
            api_key_store,
            async_memory_pipeline: false,
            event_bus,
            faq_matcher,
            rbac_manager,
            tenant_manager,
        })
    }

    pub fn get_adapter(&self, name: &str) -> Option<Arc<dyn LlmAdapter>> {
        self.providers.get(name).cloned()
    }

    pub async fn current_health(&self) -> HealthStatus {
        let redis_ok = self.redis_storage.health_check().await.is_ok();
        let qdrant_ok = self.qdrant_storage.health_check().await.is_ok();
        let degraded = !redis_ok || !qdrant_ok;
        let mode = if degraded {
            "degraded".to_string()
        } else {
            "ready".to_string()
        };
        HealthStatus {
            redis: redis_ok,
            qdrant: qdrant_ok,
            degraded,
            mode,
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
