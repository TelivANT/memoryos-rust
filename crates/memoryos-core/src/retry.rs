use crate::AppError;
use std::time::Duration;
use tokio::time::sleep;
use tracing::warn;

/// 重试策略配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

/// 带指数退避的重试包装器
pub async fn retry_with_backoff<F, Fut, T>(
    config: &RetryConfig,
    operation_name: &str,
    mut operation: F,
) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    let mut attempt = 0;
    let mut backoff_ms = config.initial_backoff_ms;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;

                if attempt > config.max_retries {
                    warn!(
                        operation = operation_name,
                        attempts = attempt,
                        "Max retries exceeded"
                    );
                    return Err(e);
                }

                // 只对可重试的错误进行重试
                if !is_retryable_error(&e) {
                    return Err(e);
                }

                warn!(
                    operation = operation_name,
                    attempt,
                    max_retries = config.max_retries,
                    backoff_ms,
                    error = %e,
                    "Retrying after error"
                );

                sleep(Duration::from_millis(backoff_ms)).await;

                // 指数退避
                backoff_ms = ((backoff_ms as f64 * config.backoff_multiplier) as u64)
                    .min(config.max_backoff_ms);
            }
        }
    }
}

/// 判断错误是否可重试
fn is_retryable_error(error: &AppError) -> bool {
    match error {
        AppError::BadRequest(_) => false, // 客户端错误不重试
        AppError::Unauthorized(_) => false,
        AppError::Forbidden(_) => false,
        AppError::NotFound(_) => false,
        AppError::Config(_) => false,
        AppError::Internal(_) => true, // 服务器错误可重试
        AppError::ExternalService(_) => true,
        AppError::ServiceUnavailable(_) => true,
        AppError::Timeout(_) => true,
        AppError::RateLimited(_) => true,
    }
}
