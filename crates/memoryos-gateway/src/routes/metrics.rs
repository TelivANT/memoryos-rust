//! Metrics routes – serves Prometheus text exposition format

use axum::{http::StatusCode, response::IntoResponse};

/// GET /metrics – Prometheus scrape endpoint
pub async fn metrics_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
        memoryos_metrics::gather_metrics(),
    )
}
