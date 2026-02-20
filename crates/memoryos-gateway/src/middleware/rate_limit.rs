//! Rate limiting middleware

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
    Json,
};
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

/// Rate limit middleware
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

    // Simple rate limit: 100 requests per minute
    static LIMITER: once_cell::sync::Lazy<RateLimiter> =
        once_cell::sync::Lazy::new(|| RateLimiter::new(100, Duration::from_secs(60)));

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
