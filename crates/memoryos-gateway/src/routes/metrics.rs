//! Metrics routes

use axum::{http::StatusCode, response::IntoResponse};

/// GET /metrics
pub async fn metrics_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        "# No metrics available\n",
    )
}
