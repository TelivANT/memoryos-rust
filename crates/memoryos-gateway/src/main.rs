use axum::{
    routing::{get, post},
    Router,
};
use memoryos_core::{AppError, ConfigManager};
use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod handlers;
mod middleware;
mod routes;
mod state;
mod worker_monitor;

use handlers::{chat_completions, health_check, health_status};
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

    // Admin 路由（需要认证 + admin 权限）
    let admin_routes = Router::new()
        .route("/v1/admin/keys", post(routes::admin::create_api_key))
        .route(
            "/v1/admin/keys/:key",
            axum::routing::delete(routes::admin::delete_api_key),
        ) // 改用 DELETE
        .layer(axum::middleware::from_fn_with_state(
            state_arc.clone(),
            middleware::admin_only, // 使用 admin_only 中间件
        ))
        .with_state(state_arc.clone());

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
        .merge(protected_routes)
        .merge(admin_routes)
        .with_state(state);

    if config.auth.enabled {
        tracing::info!("🔒 API Key authentication enabled");
    } else {
        tracing::warn!("⚠️  API Key authentication DISABLED - service is publicly accessible!");
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
