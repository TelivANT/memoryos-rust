use axum::{
    routing::{delete, get, post, put},
    Router,
};
use memoryos_core::{
    rbac::RbacManager,
    security::{AuditConfig, AuditLogger},
    tenant::TenantManager,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod routes;

#[derive(Clone)]
pub struct AdminState {
    pub rbac_manager: RbacManager,
    pub tenant_manager: TenantManager,
    pub audit_logger: Arc<AuditLogger>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let data_dir = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".memoryos");
    let _ = std::fs::create_dir_all(&data_dir);

    let audit_path = data_dir.join("audit.jsonl");
    let audit_logger = Arc::new(AuditLogger::new(AuditConfig {
        persist_path: Some(audit_path.to_string_lossy().to_string()),
        ..AuditConfig::default()
    }));

    let state = AdminState {
        rbac_manager: RbacManager::new(),
        tenant_manager: TenantManager::new(),
        audit_logger,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::system::health_check))
        .route("/api/v1/tenants", get(routes::tenants::list_tenants))
        .route("/api/v1/tenants", post(routes::tenants::create_tenant))
        .route("/api/v1/tenants/:id", get(routes::tenants::get_tenant))
        .route("/api/v1/tenants/:id", put(routes::tenants::update_tenant))
        .route(
            "/api/v1/tenants/:id",
            delete(routes::tenants::delete_tenant),
        )
        .route("/api/v1/users", get(routes::users::list_users))
        .route("/api/v1/users", post(routes::users::create_user))
        .route("/api/v1/users/:id", get(routes::users::get_user))
        .route("/api/v1/users/:id", put(routes::users::update_user))
        .route("/api/v1/users/:id", delete(routes::users::delete_user))
        .route("/api/v1/users/:id/roles", put(routes::users::assign_role))
        .route("/api/v1/audit/logs", get(routes::audit::list_audit_logs))
        .route("/api/v1/system/stats", get(routes::system::system_stats))
        .with_state(state)
        .layer(cors);

    let host = std::env::var("ADMIN_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("ADMIN_PORT").unwrap_or_else(|_| "9090".to_string());
    let addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid admin host/port");

    tracing::info!("MemoryOS Admin Service listening on {}", addr);
    tracing::info!("This service should be deployed on internal network / VPN only");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind admin port");

    axum::serve(listener, app)
        .await
        .expect("Admin server error");
}
