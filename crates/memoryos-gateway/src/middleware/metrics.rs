//! Metrics middleware (simplified)

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tracing::info;

/// Global metrics
pub struct Metrics {
    pub requests_total: AtomicU64,
    pub requests_success: AtomicU64,
    pub requests_error: AtomicU64,
}

impl Metrics {
    pub const fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            requests_success: AtomicU64::new(0),
            requests_error: AtomicU64::new(0),
        }
    }
}

/// Global metrics instance
pub static METRICS: Metrics = Metrics::new();

/// Metrics middleware
pub async fn metrics_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // Increment total requests
    METRICS.requests_total.fetch_add(1, Ordering::Relaxed);

    // Process request
    let response = next.run(req).await;

    // Record metrics
    let duration = start.elapsed();
    let status = response.status();

    if status.is_success() {
        METRICS.requests_success.fetch_add(1, Ordering::Relaxed);
    } else if status.is_server_error() || status.is_client_error() {
        METRICS.requests_error.fetch_add(1, Ordering::Relaxed);
    }

    info!(
        method = %method,
        path = %path,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        "Request completed"
    );

    response
}

/// Get metrics as text (Prometheus format)
pub fn get_metrics_text() -> String {
    let total = METRICS.requests_total.load(Ordering::Relaxed);
    let success = METRICS.requests_success.load(Ordering::Relaxed);
    let error = METRICS.requests_error.load(Ordering::Relaxed);

    format!(
        "# HELP http_requests_total Total number of HTTP requests\n\
         # TYPE http_requests_total counter\n\
         http_requests_total {}\n\
         \n\
         # HELP http_requests_success Number of successful HTTP requests\n\
         # TYPE http_requests_success counter\n\
         http_requests_success {}\n\
         \n\
         # HELP http_requests_error Number of failed HTTP requests\n\
         # TYPE http_requests_error counter\n\
         http_requests_error {}\n",
        total, success, error
    )
}
