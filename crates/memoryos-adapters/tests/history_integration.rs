//! 历史功能集成测试

use memoryos_adapters::QdrantHistoryStorage;
use memoryos_core::{HistoryEventType, MemoryHistoryEntry};
use memoryos_ports::HistoryStorage;
use qdrant_client::Qdrant;
use std::sync::Arc;

#[tokio::test]
#[ignore] // 需要 Qdrant 运行
async fn test_history_storage() {
    // 连接 Qdrant
    let client = Qdrant::from_url("http://localhost:6334")
        .build()
        .expect("Failed to connect to Qdrant");

    let storage = QdrantHistoryStorage::new(Arc::new(client), "test_memory_history".to_string())
        .await
        .expect("Failed to create history storage");

    // 创建测试条目
    let entry = MemoryHistoryEntry {
        id: uuid::Uuid::now_v7().to_string(),
        memory_id: "test_memory_123".to_string(),
        old_content: None,
        new_content: Some("Test content".to_string()),
        event_type: HistoryEventType::Add,
        created_at: chrono::Utc::now(),
        actor_id: Some("test_user".to_string()),
    };

    // 添加条目
    storage
        .add_entry(entry.clone())
        .await
        .expect("Failed to add entry");

    // 查询历史
    let history = storage
        .get_history("test_memory_123")
        .await
        .expect("Failed to get history");

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].memory_id, "test_memory_123");
    assert_eq!(history[0].new_content, Some("Test content".to_string()));

    println!("✅ History storage test passed!");
}
