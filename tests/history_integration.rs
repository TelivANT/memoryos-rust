//! History Storage 集成测试

use memoryos_adapters::QdrantHistoryStorage;
use memoryos_core::history::{HistoryEventType, MemoryHistoryEntry};
use memoryos_ports::HistoryStorage;
use std::sync::Arc;

#[tokio::test]
async fn test_history_storage_full_flow() {
    if std::env::var("QDRANT_URL").is_err() {
        println!("Skipping: requires QDRANT_URL");
        return;
    }

    let qdrant_url = std::env::var("QDRANT_URL").unwrap();
    let client = qdrant_client::Qdrant::from_url(&qdrant_url).build().unwrap();
    let storage = QdrantHistoryStorage::new(
        Arc::new(client),
        "test_history".to_string(),
    ).await.unwrap();

    // 添加历史记录
    let entry = MemoryHistoryEntry {
        id: uuid::Uuid::now_v7().to_string(),
        memory_id: "test_mem_001".to_string(),
        old_content: None,
        new_content: Some("Initial content".to_string()),
        event_type: HistoryEventType::Add,
        created_at: chrono::Utc::now(),
        actor_id: Some("test_user".to_string()),
    };

    storage.add_entry(entry.clone()).await.unwrap();

    // 查询历史
    let history = storage.get_history("test_mem_001").await.unwrap();
    assert!(!history.is_empty());
    assert_eq!(history[0].memory_id, "test_mem_001");

    // 添加更新记录
    let update_entry = MemoryHistoryEntry {
        id: uuid::Uuid::now_v7().to_string(),
        memory_id: "test_mem_001".to_string(),
        old_content: Some("Initial content".to_string()),
        new_content: Some("Updated content".to_string()),
        event_type: HistoryEventType::Update,
        created_at: chrono::Utc::now(),
        actor_id: Some("test_user".to_string()),
    };

    storage.add_entry(update_entry).await.unwrap();

    // 验证历史记录数量
    let history = storage.get_history("test_mem_001").await.unwrap();
    assert!(history.len() >= 2);
}

#[tokio::test]
async fn test_history_storage_multiple_memories() {
    if std::env::var("QDRANT_URL").is_err() {
        println!("Skipping: requires QDRANT_URL");
        return;
    }

    let qdrant_url = std::env::var("QDRANT_URL").unwrap();
    let client = qdrant_client::Qdrant::from_url(&qdrant_url).build().unwrap();
    let storage = QdrantHistoryStorage::new(
        Arc::new(client),
        "test_history_multi".to_string(),
    ).await.unwrap();

    // 添加多个记忆的历史
    for i in 1..=3 {
        let entry = MemoryHistoryEntry {
            id: uuid::Uuid::now_v7().to_string(),
            memory_id: format!("mem_{}", i),
            old_content: None,
            new_content: Some(format!("Content {}", i)),
            event_type: HistoryEventType::Add,
            created_at: chrono::Utc::now(),
            actor_id: Some("test_user".to_string()),
        };
        storage.add_entry(entry).await.unwrap();
    }

    // 验证每个记忆的历史独立
    let history1 = storage.get_history("mem_1").await.unwrap();
    let history2 = storage.get_history("mem_2").await.unwrap();
    
    assert_eq!(history1.len(), 1);
    assert_eq!(history2.len(), 1);
    assert_ne!(history1[0].memory_id, history2[0].memory_id);
}
