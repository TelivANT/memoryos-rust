use memoryos_core::history::{HistoryEventType, MemoryHistoryEntry};
use memoryos_ports::HistoryStorage;

#[tokio::test]
async fn qdrant_history_add_and_get() {
    // 需要 Qdrant 运行，跳过
    if std::env::var("QDRANT_URL").is_err() {
        return;
    }

    let url = std::env::var("QDRANT_URL").unwrap();
    let client = qdrant_client::Qdrant::from_url(&url).build().unwrap();
    let storage = super::QdrantHistoryStorage::new(
        std::sync::Arc::new(client),
        "test_history".to_string(),
    ).await.unwrap();

    let entry = MemoryHistoryEntry {
        id: uuid::Uuid::now_v7().to_string(),
        memory_id: "test_memory".to_string(),
        old_content: None,
        new_content: Some("test content".to_string()),
        event_type: HistoryEventType::Add,
        created_at: chrono::Utc::now(),
        actor_id: Some("test_user".to_string()),
    };

    storage.add_entry(entry.clone()).await.unwrap();
    let entries = storage.get_history("test_memory").await.unwrap();
    
    assert!(!entries.is_empty());
    assert_eq!(entries[0].memory_id, "test_memory");
}

#[test]
fn history_entry_serialization() {
    let entry = MemoryHistoryEntry {
        id: "test_id".to_string(),
        memory_id: "mem_123".to_string(),
        old_content: Some("old".to_string()),
        new_content: Some("new".to_string()),
        event_type: HistoryEventType::Update,
        created_at: chrono::Utc::now(),
        actor_id: Some("user_1".to_string()),
    };

    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: MemoryHistoryEntry = serde_json::from_str(&json).unwrap();
    
    assert_eq!(entry.id, deserialized.id);
    assert_eq!(entry.memory_id, deserialized.memory_id);
}

#[test]
fn history_event_type_variants() {
    assert_eq!(format!("{:?}", HistoryEventType::Add), "Add");
    assert_eq!(format!("{:?}", HistoryEventType::Update), "Update");
    assert_eq!(format!("{:?}", HistoryEventType::Delete), "Delete");
}
