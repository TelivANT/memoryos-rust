//! Memory Manager 集成测试
//! 测试完整的记忆管理流程

use memoryos_adapters::{DefaultMemoryManager, RedisStorage, QdrantStorage};
use memoryos_core::Message;
use memoryos_ports::{MemoryManager, ShortTermStorage, VectorStorage};
use std::sync::Arc;

#[tokio::test]
async fn test_memory_manager_full_flow() {
    // 跳过如果没有 Redis 和 Qdrant
    if std::env::var("REDIS_URL").is_err() || std::env::var("QDRANT_URL").is_err() {
        println!("Skipping: requires REDIS_URL and QDRANT_URL");
        return;
    }

    let redis_url = std::env::var("REDIS_URL").unwrap();
    let qdrant_url = std::env::var("QDRANT_URL").unwrap();

    // 初始化存储
    let redis = Arc::new(RedisStorage::new(&redis_url, 3600, 20).unwrap()) as Arc<dyn ShortTermStorage>;
    let qdrant_client = qdrant_client::Qdrant::from_url(&qdrant_url).build().unwrap();
    let qdrant = Arc::new(QdrantStorage::new(Arc::new(qdrant_client)).await.unwrap()) as Arc<dyn VectorStorage>;

    // 创建 Memory Manager
    let manager = DefaultMemoryManager::new(redis, qdrant, None).await.unwrap();

    // 测试添加消息
    let message = Message {
        role: "user".to_string(),
        content: "Hello, I'm testing the memory system".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let result = manager.add_message("test_user", message).await;
    assert!(result.is_ok());

    // 测试检索上下文
    let context = manager.retrieve_context("test_user", "testing").await;
    assert!(context.is_ok());
}

#[tokio::test]
async fn test_memory_manager_without_dependencies() {
    // 测试降级模式
    use memoryos_adapters::NoopMemoryManager;
    
    let manager = NoopMemoryManager;
    
    let message = Message {
        role: "user".to_string(),
        content: "test".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let result = manager.add_message("test_user", message).await;
    assert!(result.is_ok());

    let context = manager.retrieve_context("test_user", "test").await;
    assert!(context.is_ok());
}
