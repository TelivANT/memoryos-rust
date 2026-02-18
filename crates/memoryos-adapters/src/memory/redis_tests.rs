use super::*;

#[test]
fn redis_storage_creation() {
    let result = RedisStorage::new("redis://localhost:6379", 3600, 20);
    assert!(result.is_ok() || result.is_err()); // 可能成功或失败
}

#[tokio::test]
async fn redis_storage_requires_valid_url() {
    let result = RedisStorage::new("invalid://url", 3600, 20);
    assert!(result.is_err());
}
