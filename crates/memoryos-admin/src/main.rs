use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use memoryos_core::{
    rbac::RbacManager,
    security::{AuditConfig, AuditLogger},
    tenant::TenantManager,
};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod routes;

#[derive(Clone)]
pub struct AdminState {
    pub rbac_manager: RbacManager,
    pub tenant_manager: TenantManager,
    pub audit_logger: Arc<AuditLogger>,
    pub admin_token: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let rbac_path = data_dir.join("rbac_users.json");
    let tenant_path = data_dir.join("tenants.json");

    let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    if admin_token.is_empty() {
        tracing::warn!("==================================================");
        tracing::warn!("  ADMIN_TOKEN not set! Admin API is UNPROTECTED!");
        tracing::warn!("  Set ADMIN_TOKEN env var for production use.");
        tracing::warn!("==================================================");
    }

    let rbac_manager = RbacManager::new(&rbac_path)
        .await
        .map_err(|e| format!("Failed to init RBAC manager: {}", e))?;
    let tenant_manager = TenantManager::new(&tenant_path)
        .await
        .map_err(|e| format!("Failed to init Tenant manager: {}", e))?;

    let state = AdminState {
        rbac_manager,
        tenant_manager,
        audit_logger,
        admin_token: admin_token.clone(),
    };

    let allowed_origins = std::env::var("ADMIN_CORS_ORIGINS").unwrap_or_default();
    let cors = if allowed_origins.is_empty() {
        let origin = "http://localhost:3000"
            .parse()
            .map_err(|_| "Failed to parse default CORS origin")?;
        CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::exact(origin))
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Vec<axum::http::HeaderValue> = allowed_origins
            .split(',')
            .filter_map(|o| o.trim().parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    let protected_routes = Router::new()
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
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(
            move |headers: HeaderMap, req: Request, next: Next| {
                let token = admin_token.clone();
                async move {
                    if token.is_empty() {
                        return Ok::<Response, Response>(next.run(req).await);
                    }
                    let provided = headers
                        .get("Authorization")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|h: &str| h.strip_prefix("Bearer "))
                        .unwrap_or("");
                    let token_match = provided.as_bytes().ct_eq(token.as_bytes());
                    if !bool::from(token_match) {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            axum::Json(serde_json::json!({
                                "error": {
                                    "code": "unauthorized",
                                    "message": "Valid ADMIN_TOKEN required"
                                }
                            })),
                        )
                            .into_response());
                    }
                    Ok(next.run(req).await)
                }
            },
        ));

    let app = Router::new()
        .route("/health", get(routes::system::health_check))
        .merge(protected_routes)
        .layer(cors);

    let host = std::env::var("ADMIN_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("ADMIN_PORT").unwrap_or_else(|_| "9090".to_string());
    let addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|_| format!("Invalid admin host/port: {}:{}", host, port))?;

    tracing::info!("MemoryOS Admin Service listening on {}", addr);
    tracing::info!("This service should be deployed on internal network / VPN only");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind admin port {}: {}", addr, e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Admin server error: {}", e))?;

    Ok(())
}
