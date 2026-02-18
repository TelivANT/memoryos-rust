//! Core error types for MemoryOS
//!
//! All errors in the system flow through `AppError`, which can be
//! converted to HTTP responses via `IntoResponse`.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

/// Application-wide error type
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Configuration errors (startup failures)
    #[error("Configuration error: {0}")]
    Config(String),

    /// Invalid user input
    #[error("Invalid request: {0}")]
    BadRequest(String),

    /// Authentication/Authorization failures
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Rate limiting
    #[error("Too many requests: {0}")]
    RateLimited(String),

    /// External service failures (Redis, Qdrant, LLM)
    #[error("External service error: {0}")]
    ExternalService(String),

    /// Internal logic errors
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Convert error to HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Config(_) => 500,
            Self::BadRequest(_) => 400,
            Self::Unauthorized(_) => 401,
            Self::NotFound(_) => 404,
            Self::RateLimited(_) => 429,
            Self::ExternalService(_) => 503,
            Self::Internal(_) => 500,
        }
    }

    /// Get error code for API response
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Config(_) => "config_error",
            Self::BadRequest(_) => "bad_request",
            Self::Unauthorized(_) => "unauthorized",
            Self::NotFound(_) => "not_found",
            Self::RateLimited(_) => "rate_limited",
            Self::ExternalService(_) => "service_unavailable",
            Self::Internal(_) => "internal_error",
        }
    }

    /// Convert to JSON error response
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "status": "error",
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
            }
        })
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self.to_json_value())).into_response()
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, AppError>;
