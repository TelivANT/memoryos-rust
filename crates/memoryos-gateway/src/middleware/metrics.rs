//! Prometheus-backed metrics middleware
//!
//! Instruments every HTTP request with `memoryos_http_requests_total` (counter)
//! and `memoryos_http_request_duration_seconds` (histogram) from the
//! `memoryos-metrics` crate. The `/metrics` path itself is excluded to avoid
//! polluting the scraped data.

use axum::{extract::Request, middleware::Next, response::Response};
use memoryos_metrics::{HTTP_REQUESTS_TOTAL, HTTP_REQUEST_DURATION};
use std::time::Instant;
use tracing::info;

/// Normalize request path to avoid high-cardinality labels.
/// Replaces UUIDs and numeric IDs with placeholders.
fn normalize_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = segments
        .iter()
        .map(|seg| {
            if seg.len() >= 32 && seg.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
                ":id".to_string()
            } else if seg.chars().all(|c| c.is_ascii_digit()) && !seg.is_empty() {
                ":id".to_string()
            } else {
                seg.to_string()
            }
        })
        .collect();
    normalized.join("/")
}

/// Axum middleware that records Prometheus HTTP metrics for every request.
pub async fn metrics_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let raw_path = req.uri().path().to_string();

    let response = next.run(req).await;

    if raw_path == "/metrics" {
        return response;
    }

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();
    let path = normalize_path(&raw_path);

    HTTP_REQUESTS_TOTAL
        .with_label_values(&[&method, &path, &status])
        .inc();
    HTTP_REQUEST_DURATION
        .with_label_values(&[&method, &path])
        .observe(duration);

    info!(
        method = %method,
        path = %raw_path,
        status = %status,
        duration_ms = %format!("{:.1}", duration * 1000.0),
        "Request completed"
    );

    response
}
