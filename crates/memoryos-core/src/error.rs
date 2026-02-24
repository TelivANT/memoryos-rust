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

    /// Forbidden (IP banned, etc.)
    #[error("Forbidden: {0}")]
    Forbidden(String),

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
            Self::Forbidden(_) => 403,
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
            Self::Forbidden(_) => "forbidden",
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
        let status =
            StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self.to_json_value())).into_response()
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_status_code() {
        let err = AppError::Config("bad config".into());
        assert_eq!(err.status_code(), 500);
        assert_eq!(err.error_code(), "config_error");
    }

    #[test]
    fn bad_request_status_code() {
        let err = AppError::BadRequest("missing field".into());
        assert_eq!(err.status_code(), 400);
        assert_eq!(err.error_code(), "bad_request");
    }

    #[test]
    fn unauthorized_status_code() {
        let err = AppError::Unauthorized("invalid token".into());
        assert_eq!(err.status_code(), 401);
        assert_eq!(err.error_code(), "unauthorized");
    }

    #[test]
    fn forbidden_status_code() {
        let err = AppError::Forbidden("ip banned".into());
        assert_eq!(err.status_code(), 403);
        assert_eq!(err.error_code(), "forbidden");
    }

    #[test]
    fn not_found_status_code() {
        let err = AppError::NotFound("user not found".into());
        assert_eq!(err.status_code(), 404);
        assert_eq!(err.error_code(), "not_found");
    }

    #[test]
    fn rate_limited_status_code() {
        let err = AppError::RateLimited("too fast".into());
        assert_eq!(err.status_code(), 429);
        assert_eq!(err.error_code(), "rate_limited");
    }

    #[test]
    fn external_service_status_code() {
        let err = AppError::ExternalService("redis down".into());
        assert_eq!(err.status_code(), 503);
        assert_eq!(err.error_code(), "service_unavailable");
    }

    #[test]
    fn internal_error_status_code() {
        let err = AppError::Internal("unexpected".into());
        assert_eq!(err.status_code(), 500);
        assert_eq!(err.error_code(), "internal_error");
    }

    #[test]
    fn error_display_includes_message() {
        let err = AppError::BadRequest("missing user_id".into());
        let display = format!("{}", err);
        assert!(display.contains("missing user_id"));
    }

    #[test]
    fn to_json_value_structure() {
        let err = AppError::NotFound("memory not found".into());
        let json = err.to_json_value();
        assert_eq!(json["status"], "error");
        assert_eq!(json["error"]["code"], "not_found");
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("memory not found"));
    }

    #[test]
    fn all_variants_have_unique_error_codes() {
        let variants = vec![
            AppError::Config("".into()),
            AppError::BadRequest("".into()),
            AppError::Unauthorized("".into()),
            AppError::Forbidden("".into()),
            AppError::NotFound("".into()),
            AppError::RateLimited("".into()),
            AppError::ExternalService("".into()),
            AppError::Internal("".into()),
        ];
        let codes: Vec<&str> = variants.iter().map(|e| e.error_code()).collect();
        let unique: std::collections::HashSet<&str> = codes.iter().copied().collect();
        // Config and Internal both map to 500 but have different error codes
        assert_eq!(
            codes.len(),
            unique.len(),
            "All error codes should be unique"
        );
    }
}
