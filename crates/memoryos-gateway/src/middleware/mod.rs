pub mod auth;
pub mod circuit_breaker;
pub mod defense;
pub mod metrics;
pub mod rate_limit;
pub mod rbac;

pub use auth::{admin_only, auth_middleware};
pub use circuit_breaker::{circuit_breaker_middleware, CircuitBreakerState};
pub use defense::ip_defense_middleware;
pub use metrics::metrics_middleware;
pub use rate_limit::rate_limit_middleware;
pub use rbac::rbac_middleware;

#[cfg(test)]
mod tests;
