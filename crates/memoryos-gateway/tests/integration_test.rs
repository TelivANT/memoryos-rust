#![cfg(feature = "integration-tests")]

use memoryos_adapters::memory::{QdrantStorage, RedisStorage};
use memoryos_core::{GdprManager, Message, OptimizedFaqMatcher};
use memoryos_ports::{EventBus, ShortTermStorage, VectorStorage};

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

fn qdrant_url() -> String {
    std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".to_string())
}

#[tokio::test]
async fn test_redis_health_check() {
    let storage = RedisStorage::new(&redis_url(), 3600, 100).expect("Redis init");
    assert!(storage.health_check().await.is_ok());
}

#[tokio::test]
async fn test_qdrant_health_check() {
    let storage = QdrantStorage::new(&qdrant_url())
        .await
        .expect("Qdrant init");
    assert!(storage.health_check().await.is_ok());
}

#[tokio::test]
async fn test_redis_stm_add_retrieve_clear() {
    let storage = RedisStorage::new(&redis_url(), 3600, 100).expect("Redis init");
    let user_id = &format!("integration_test_user_{}", uuid::Uuid::now_v7());

    let msg = Message {
        role: "user".to_string(),
        content: "Hello integration test".to_string(),
        timestamp: chrono::Utc::now(),
        embedding: None,
    };
    storage.add_message(user_id, msg).await.unwrap();

    let recent = storage.get_recent(user_id, 10).await.unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].content, "Hello integration test");

    storage.clear(user_id).await.unwrap();
    let after_clear = storage.get_recent(user_id, 10).await.unwrap();
    assert!(after_clear.is_empty());
}

#[tokio::test]
async fn test_qdrant_stm_add_clear() {
    let storage = QdrantStorage::new(&qdrant_url())
        .await
        .expect("Qdrant init");
    let user_id = &format!("integration_test_user_{}", uuid::Uuid::now_v7());

    let msg = Message {
        role: "user".to_string(),
        content: "Qdrant integration test".to_string(),
        timestamp: chrono::Utc::now(),
        embedding: Some(vec![0.1; 1536]),
    };
    storage.add_short_term_message(user_id, msg).await.unwrap();
    storage.clear_short_term(user_id).await.unwrap();
}

#[tokio::test]
async fn test_qdrant_segment_store_search() {
    let storage = QdrantStorage::new(&qdrant_url())
        .await
        .expect("Qdrant init");
    let user_id = &format!("integration_test_user_{}", uuid::Uuid::now_v7());

    let segment = memoryos_core::MidTermSegment {
        id: uuid::Uuid::now_v7(),
        user_id: user_id.to_string(),
        summary: "Integration test segment".to_string(),
        embedding: vec![0.5; 1536],
        heat: 1.0,
        created_at: chrono::Utc::now(),
        tenant_id: None,
        access_count: 0,
        heat_score: 0.0,
        last_accessed: None,
        memory_type: memoryos_core::MemoryType::QA,
        version: 1,
        tags: vec!["test".to_string()],
        updated_at: None,
        previous_version_id: None,
        score: None,
    };

    storage.store_segment(segment).await.unwrap();

    let results = storage
        .search_segments(user_id, vec![0.5; 1536], 10)
        .await
        .unwrap();
    assert!(!results.is_empty());
    assert!(results[0].score.is_some());

    storage.delete_user_data(user_id).await.unwrap();
}

#[tokio::test]
async fn test_gdpr_deletion_flow() {
    let mut gdpr = GdprManager::new();

    gdpr.record_consent("test_user", "data_processing", true);
    assert!(gdpr.has_consent("test_user", "data_processing"));

    let req = gdpr.request_deletion("test_user").unwrap();
    assert_eq!(
        req.status,
        memoryos_core::security::gdpr::DeletionStatus::Pending
    );

    gdpr.complete_deletion("test_user").unwrap();
    assert!(!gdpr.has_consent("test_user", "data_processing"));

    let reqs = gdpr.get_deletion_requests("test_user");
    assert_eq!(
        reqs[0].status,
        memoryos_core::security::gdpr::DeletionStatus::Completed
    );
}

#[tokio::test]
async fn test_optimized_faq_matcher_bloom_filter() {
    let mut matcher = OptimizedFaqMatcher::new(100);
    matcher.add_faq(
        "What is the WiFi password?",
        "The password is 12345678".to_string(),
    );

    let hit = matcher.match_faq("What is the WiFi password?").await;
    assert!(hit.is_some());
    assert_eq!(hit.unwrap(), "The password is 12345678");

    let miss = matcher.match_faq("Unrelated question").await;
    assert!(miss.is_none());
}

#[tokio::test]
async fn test_qdrant_delete_user_data_all_collections() {
    let storage = QdrantStorage::new(&qdrant_url())
        .await
        .expect("Qdrant init");
    let user_id = &format!("integration_gdpr_user_{}", uuid::Uuid::now_v7());

    let msg = Message {
        role: "user".to_string(),
        content: "GDPR test message".to_string(),
        timestamp: chrono::Utc::now(),
        embedding: Some(vec![0.2; 1536]),
    };
    storage.add_short_term_message(user_id, msg).await.unwrap();

    let segment = memoryos_core::MidTermSegment {
        id: uuid::Uuid::now_v7(),
        user_id: user_id.to_string(),
        summary: "GDPR test segment".to_string(),
        embedding: vec![0.3; 1536],
        heat: 1.0,
        created_at: chrono::Utc::now(),
        tenant_id: None,
        access_count: 0,
        heat_score: 0.0,
        last_accessed: None,
        memory_type: memoryos_core::MemoryType::QA,
        version: 1,
        tags: vec![],
        updated_at: None,
        previous_version_id: None,
        score: None,
    };
    storage.store_segment(segment).await.unwrap();

    storage.delete_user_data(user_id).await.unwrap();

    let results = storage
        .search_segments(user_id, vec![0.3; 1536], 10)
        .await
        .unwrap();
    let user_results: Vec<_> = results.iter().filter(|s| s.user_id == *user_id).collect();
    assert!(user_results.is_empty());
}

#[tokio::test]
async fn test_eventbus_publish() {
    let redis_url = redis_url();
    let bus = memoryos_adapters::RedisStreamEventBus::new(&redis_url, "test_chat_log_integration")
        .expect("EventBus init");

    let event_id = uuid::Uuid::now_v7().to_string();
    let payload = serde_json::json!({
        "user_id": "test_user",
        "query": "hello",
    });

    let result = bus.publish_chat_log(&event_id, payload).await;
    assert!(result.is_ok());
}
