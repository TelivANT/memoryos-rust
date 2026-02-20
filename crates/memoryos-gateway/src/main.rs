use axum::{
    routing::{get, post},
    Router,
};
use memoryos_core::{AppError, ConfigManager};
use std::{path::PathBuf, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod handlers;
mod middleware;
mod routes;
mod state;
mod worker_monitor;

use handlers::{chat_completions, health_check, health_status};
use routes::faq::{create_faq_routes, FaqState};
use routes::graph::{create_graph_routes, GraphState};
use routes::memory_manage::{create_memory_manage_routes, MemoryManageState};
use routes::multimodal::{create_multimodal_routes, MultiModalState};
use routes::security::{create_security_routes, SecurityState};
use state::AppState;
use worker_monitor::spawn_worker_monitor;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // 1. Init Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Load Config
    let mut config_manager = ConfigManager::new()?;
    let config = config_manager.get();
    let async_memory_enabled = std::env::var("MEMORYOS_ASYNC_MEMORY_PIPELINE")
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);

    // 3. Init App State (Connect to DBs)
    let state = AppState::new((*config).clone()).await;

    if async_memory_enabled {
        tracing::info!(
            "Gateway async memory pipeline is enabled; deploy worker as optional consumer for queued memory tasks."
        );
        spawn_worker_monitor(
            config.storage.redis.url.clone(),
            state.worker_monitor.clone(),
        );
    } else {
        tracing::info!(
            "Gateway running in standalone sync-memory mode; worker deployment is optional and not required."
        );
    }

    // 4. Spawn config hot-reload task
    let config_reload_enabled = std::env::var("MEMORYOS_CONFIG_HOT_RELOAD")
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(true); // 默认启用

    if config_reload_enabled {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                match config_manager.reload_if_changed() {
                    Ok(true) => tracing::info!("✅ Config hot-reloaded successfully"),
                    Ok(false) => {} // 无变化
                    Err(e) => tracing::warn!("⚠️  Config reload failed: {}", e),
                }
            }
        });
        tracing::info!("✅ Config hot-reload enabled (check every 5s)");
    }

    // 5. Setup Router
    let state_arc = Arc::new(state.clone());

    // FAQ 管理路由
    let heat_tracker = Arc::new(memoryos_core::HeatTracker::new(
        memoryos_core::HeatConfig::default(),
    ));
    let faq_state = FaqState {
        heat_tracker: heat_tracker.clone(),
        auto_promoter: Arc::new(memoryos_core::AutoPromoter::new(
            memoryos_core::AutoPromotionConfig::default(),
            heat_tracker,
        )),
        vector_store: state.vector_store.clone(),
    };

    // Admin 路由（需要认证 + admin 权限）
    let admin_routes = Router::new()
        .route("/v1/admin/keys", post(routes::admin::create_api_key))
        .route(
            "/v1/admin/keys/:key",
            axum::routing::delete(routes::admin::delete_api_key),
        )
        .layer(axum::middleware::from_fn_with_state(
            state_arc.clone(),
            middleware::admin_only,
        ))
        .with_state(state_arc.clone());

    // FAQ 管理路由（独立 state，嵌套到 admin 路径下）
    let faq_routes = create_faq_routes(faq_state);

    // Graph 路由 (v0.4.0)
    let graph_state = GraphState {
        graph_manager: std::sync::Arc::new(tokio::sync::RwLock::new(
            memoryos_core::GraphManager::new(),
        )),
    };
    let graph_routes = create_graph_routes(graph_state);

    // Memory management 路由 (v0.6.0)
    let memory_manage_state = MemoryManageState {
        vector_store: state.vector_store.clone(),
    };
    let memory_manage_routes = create_memory_manage_routes(memory_manage_state);

    // Multimodal 路由 (v0.5.0)
    let multimodal_storage = std::sync::Arc::new(
        memoryos_adapters::multimodal::QdrantMultiModalStorage::new(&config.storage.vector.url)
            .await
            .expect("Failed to init QdrantMultiModalStorage"),
    );
    let multimodal_state = MultiModalState {
        storage: multimodal_storage,
    };
    let multimodal_routes = create_multimodal_routes(multimodal_state);

    // Security 路由 (v0.8.0)
    let data_dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".memoryos");
    let _ = std::fs::create_dir_all(&data_dir);

    let audit_path = data_dir.join("audit.jsonl");
    let gdpr_path = data_dir.join("gdpr.json");

    let audit_logger = std::sync::Arc::new(memoryos_core::AuditLogger::new(
        memoryos_core::AuditConfig {
            persist_path: Some(audit_path.to_string_lossy().to_string()),
            ..memoryos_core::AuditConfig::default()
        },
    ));

    let gdpr_path_str = gdpr_path.to_string_lossy().to_string();
    let gdpr_manager = std::sync::Arc::new(tokio::sync::RwLock::new(
        memoryos_core::GdprManager::with_persistence(&gdpr_path_str),
    ));
    let security_state = SecurityState {
        audit_logger,
        gdpr_manager,
    };
    let security_routes = create_security_routes(security_state);

    // 需要认证的路由
    let protected_routes = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/memory/add", post(routes::memory::add_message))
        .route(
            "/v1/memory/retrieve",
            post(routes::memory::retrieve_context),
        )
        .route(
            "/v1/memory/:memory_id/history",
            get(routes::history::get_memory_history),
        )
        .layer(axum::middleware::from_fn_with_state(
            state_arc.clone(),
            middleware::auth_middleware,
        ));

    // 公开路由（健康检查不需要认证）
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/health/status", get(health_status))
        .route("/metrics", get(routes::metrics::metrics_handler))
        .merge(protected_routes)
        .merge(admin_routes)
        .with_state(state)
        .nest("/v1/admin/faq", faq_routes)
        .nest("/v1/graph", graph_routes)
        .nest("/v1/memory/manage", memory_manage_routes)
        .nest("/v1/multimodal", multimodal_routes)
        .nest("/v1/security", security_routes)
        .layer(axum::middleware::from_fn_with_state(
            state_arc.clone(),
            middleware::rbac_middleware,
        ))
        .layer(axum::middleware::from_fn(middleware::metrics_middleware));

    if config.auth.enabled {
        tracing::info!("API Key authentication enabled");
    } else {
        tracing::warn!("========================================================");
        tracing::warn!("  WARNING: API Key authentication is DISABLED!");
        tracing::warn!("  The service is publicly accessible without any auth.");
        tracing::warn!("  Set `auth.enabled = true` in config.toml for production.");
        tracing::warn!("========================================================");
    }

    // 6. Start Server
    let addr: std::net::SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|e| AppError::Config(format!("Invalid host/port: {}", e)))?;

    tracing::info!("MemoryOS Gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to bind port: {}", e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::Internal(format!("Server error: {}", e)))?;

    Ok(())
}
