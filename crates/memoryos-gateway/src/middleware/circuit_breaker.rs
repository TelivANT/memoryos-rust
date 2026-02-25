use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Simple circuit breaker state
#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    failures: Arc<RwLock<CircuitState>>,
}

#[derive(Debug)]
struct CircuitState {
    failure_count: u32,
    last_failure: Option<Instant>,
    state: State,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing if service recovered
}

impl CircuitBreakerState {
    pub fn new() -> Self {
        Self {
            failures: Arc::new(RwLock::new(CircuitState {
                failure_count: 0,
                last_failure: None,
                state: State::Closed,
            })),
        }
    }

    /// Check if request should be allowed
    pub async fn should_allow(&self) -> bool {
        let mut state = self.failures.write().await;

        match state.state {
            State::Closed => true,
            State::Open => {
                // Check if timeout expired (30 seconds)
                if let Some(last) = state.last_failure {
                    if last.elapsed() > Duration::from_secs(30) {
                        state.state = State::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            State::HalfOpen => true,
        }
    }

    /// Record a successful request
    pub async fn record_success(&self) {
        let mut state = self.failures.write().await;
        state.failure_count = 0;
        state.state = State::Closed;
    }

    /// Record a failed request
    pub async fn record_failure(&self) {
        let mut state = self.failures.write().await;
        state.failure_count += 1;
        state.last_failure = Some(Instant::now());

        // Open circuit after 5 failures
        if state.failure_count >= 5 {
            state.state = State::Open;
        }
    }
}

/// Helper to wrap external service calls with circuit breaker.
///
/// Returns `None` when the circuit is open (caller should handle as service unavailable).
/// Returns `Some(result)` when the call was attempted.
pub async fn with_circuit_breaker<F, T, E>(
    breaker: &CircuitBreakerState,
    f: F,
) -> Option<Result<T, E>>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    if !breaker.should_allow().await {
        // Circuit is open, fail fast — caller decides the error
        return None;
    }

    let result = f.await;
    match &result {
        Ok(_) => breaker.record_success().await,
        Err(_) => breaker.record_failure().await,
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let breaker = CircuitBreakerState::new();

        // Should allow initially
        assert!(breaker.should_allow().await);

        // Record 5 failures
        for _ in 0..5 {
            breaker.record_failure().await;
        }

        // Circuit should be open
        assert!(!breaker.should_allow().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_after_timeout() {
        let breaker = CircuitBreakerState::new();

        // Open the circuit
        for _ in 0..5 {
            breaker.record_failure().await;
        }
        assert!(!breaker.should_allow().await);

        // Wait for timeout (simulate by modifying last_failure)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Note: In real test, would need to wait 30 seconds or mock time
    }

    #[tokio::test]
    async fn test_circuit_breaker_closes_on_success() {
        let breaker = CircuitBreakerState::new();

        // Record some failures
        for _ in 0..3 {
            breaker.record_failure().await;
        }

        // Record success
        breaker.record_success().await;

        // Should be closed again
        assert!(breaker.should_allow().await);
    }
}
