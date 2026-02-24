//! Unit tests for gateway middleware components

#[cfg(test)]
mod rate_limit_tests {
    use crate::middleware::rate_limit::RateLimiter;
    use std::net::IpAddr;
    use std::time::Duration;

    #[tokio::test]
    async fn allows_requests_under_limit() {
        let limiter = RateLimiter::new(5, Duration::from_secs(60));
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        for _ in 0..5 {
            assert!(limiter.check(ip).await);
        }
    }

    #[tokio::test]
    async fn blocks_requests_over_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        // 4th request should be blocked
        assert!(!limiter.check(ip).await);
    }

    #[tokio::test]
    async fn different_ips_have_separate_limits() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip1: IpAddr = "10.0.0.1".parse().unwrap();
        let ip2: IpAddr = "10.0.0.2".parse().unwrap();

        assert!(limiter.check(ip1).await);
        assert!(limiter.check(ip1).await);
        assert!(!limiter.check(ip1).await); // ip1 blocked

        // ip2 should still be allowed
        assert!(limiter.check(ip2).await);
        assert!(limiter.check(ip2).await);
    }

    #[tokio::test]
    async fn window_expiry_allows_new_requests() {
        let limiter = RateLimiter::new(2, Duration::from_millis(50));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(!limiter.check(ip).await);

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should be allowed again
        assert!(limiter.check(ip).await);
    }

    #[tokio::test]
    async fn zero_limit_blocks_all() {
        let limiter = RateLimiter::new(0, Duration::from_secs(60));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(!limiter.check(ip).await);
    }

    #[tokio::test]
    async fn single_request_limit() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(limiter.check(ip).await);
        assert!(!limiter.check(ip).await);
    }
}

#[cfg(test)]
mod metrics_tests {
    use crate::middleware::metrics::normalize_path;

    #[test]
    fn normalizes_uuid_segments() {
        let path = "/v1/memory/550e8400-e29b-41d4-a716-446655440000/context";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/v1/memory/:id/context");
    }

    #[test]
    fn normalizes_numeric_ids() {
        let path = "/v1/users/12345/messages";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/v1/users/:id/messages");
    }

    #[test]
    fn preserves_normal_paths() {
        let path = "/v1/memory/context";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/v1/memory/context");
    }

    #[test]
    fn normalizes_hex_ids() {
        let path = "/api/objects/abcdef0123456789abcdef0123456789";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/objects/:id");
    }

    #[test]
    fn handles_root_path() {
        let path = "/";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/");
    }

    #[test]
    fn handles_empty_path() {
        let path = "";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "");
    }

    #[test]
    fn preserves_short_hex_segments() {
        // Short hex strings (< 32 chars) should NOT be replaced
        let path = "/v1/health/abc";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/v1/health/abc");
    }
}

#[cfg(test)]
mod auth_tests {
    use crate::middleware::auth::{constant_time_contains, extract_bearer_token};
    use axum::http::HeaderMap;

    #[test]
    fn extracts_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer my-secret-token".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), "my-secret-token");
    }

    #[test]
    fn returns_empty_for_missing_auth_header() {
        let headers = HeaderMap::new();
        assert_eq!(extract_bearer_token(&headers), "");
    }

    #[test]
    fn returns_empty_for_non_bearer_auth() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Basic dXNlcjpwYXNz".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), "");
    }

    #[test]
    fn returns_empty_for_bearer_without_token() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer ".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), "");
    }

    #[test]
    fn constant_time_contains_finds_match() {
        let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        assert!(constant_time_contains(&keys, "key2"));
    }

    #[test]
    fn constant_time_contains_no_match() {
        let keys = vec!["key1".to_string(), "key2".to_string()];
        assert!(!constant_time_contains(&keys, "key3"));
    }

    #[test]
    fn constant_time_contains_empty_list() {
        let keys: Vec<String> = vec![];
        assert!(!constant_time_contains(&keys, "anything"));
    }

    #[test]
    fn constant_time_contains_different_lengths() {
        let keys = vec!["short".to_string(), "medium-key".to_string()];
        assert!(!constant_time_contains(&keys, "sh"));
    }
}
