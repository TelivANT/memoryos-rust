//! Rate limiting middleware

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response, Json};
use serde_json::json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Simple in-memory rate limiter
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    pub async fn check(&self, ip: IpAddr) -> bool {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        // Periodic cleanup: evict stale IPs every 256 checks
        if requests.len() > 1000 {
            requests.retain(|_, history| {
                history.retain(|&time| now.duration_since(time) < self.window);
                !history.is_empty()
            });
        }

        let history = requests.entry(ip).or_insert_with(Vec::new);
        history.retain(|&time| now.duration_since(time) < self.window);

        if history.len() >= self.max_requests {
            return false;
        }

        history.push(now);
        true
    }
}

/// Rate limit middleware (configurable via config.toml)
///
/// NOTE: This rate limiter is process-local (in-memory). In multi-instance
/// deployments behind a load balancer, each instance maintains independent
/// counters. For production multi-instance setups, consider using Redis-based
/// distributed rate limiting (e.g., redis-cell or a sliding window counter).
pub async fn rate_limit_middleware(
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Extract IP from request
    let ip = req
        .extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip())
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));

    // Rate limit from config (default: 100 requests per minute)
    // Note: This uses a static limiter for simplicity. For production,
    // consider passing config through AppState or using a service layer.
    static LIMITER: once_cell::sync::Lazy<RateLimiter> = once_cell::sync::Lazy::new(|| {
        // Read from env or use default
        let limit = std::env::var("MEMORYOS_RATE_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);
        RateLimiter::new(limit, Duration::from_secs(60))
    });

    if !LIMITER.check(ip).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": {
                    "type": "RateLimitExceeded",
                    "message": "Too many requests. Please try again later."
                }
            })),
        ));
    }

    Ok(next.run(req).await)
}
