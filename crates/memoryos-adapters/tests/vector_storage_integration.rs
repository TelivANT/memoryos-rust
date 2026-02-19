//! 向量存储短期记忆集成测试

use memoryos_adapters::{ChromaStorage, PineconeStorage, QdrantStorage};
use memoryos_core::Message;
use memoryos_ports::VectorStorage;
use std::sync::Arc;

async fn test_short_term_memory<T: VectorStorage>(storage: Arc<T>, test_name: &str) {
    let user_id = format!("test_user_{}", uuid::Uuid::now_v7());
    println!("\n🧪 Testing {} - Short-term Memory", test_name);

    // 1. 添加短期记忆
    println!("  ➤ Adding messages...");
    let msg1 = Message {
        role: "user".to_string(),
        content: "Hello, I'm a software engineer".to_string(),
        timestamp: chrono::Utc::now(),
        embedding: None,
    };
    let msg2 = Message {
        role: "assistant".to_string(),
        content: "Nice to meet you! What programming languages do you use?".to_string(),
        timestamp: chrono::Utc::now(),
        embedding: None,
    };
    let msg3 = Message {
        role: "user".to_string(),
        content: "I mainly use Rust and Python".to_string(),
        timestamp: chrono::Utc::now(),
        embedding: None,
    };

    storage
        .add_short_term_message(&user_id, msg1.clone())
        .await
        .expect("Failed to add message 1");
    storage
        .add_short_term_message(&user_id, msg2.clone())
        .await
        .expect("Failed to add message 2");
    storage
        .add_short_term_message(&user_id, msg3.clone())
        .await
        .expect("Failed to add message 3");
    println!("  ✅ Added 3 messages");

    // 2. 获取短期记忆
    println!("  ➤ Retrieving messages...");
    let messages = storage
        .get_short_term_messages(&user_id, 10)
        .await
        .expect("Failed to get messages");
    assert_eq!(messages.len(), 3, "Should have 3 messages");
    assert_eq!(messages[0].content, msg1.content);
    assert_eq!(messages[1].content, msg2.content);
    assert_eq!(messages[2].content, msg3.content);
    println!("  ✅ Retrieved 3 messages correctly");

    // 3. 测试限制数量
    println!("  ➤ Testing limit...");
    let limited = storage
        .get_short_term_messages(&user_id, 2)
        .await
        .expect("Failed to get limited messages");
    assert_eq!(limited.len(), 2, "Should respect limit");
    println!("  ✅ Limit works correctly");

    // 4. 测试用户隔离
    println!("  ➤ Testing user isolation...");
    let other_user = format!("other_user_{}", uuid::Uuid::now_v7());
    let other_messages = storage
        .get_short_term_messages(&other_user, 10)
        .await
        .expect("Failed to get other user messages");
    assert_eq!(
        other_messages.len(),
        0,
        "Other user should have no messages"
    );
    println!("  ✅ User isolation works");

    // 5. 清空短期记忆
    println!("  ➤ Clearing messages...");
    storage
        .clear_short_term(&user_id)
        .await
        .expect("Failed to clear messages");
    let after_clear = storage
        .get_short_term_messages(&user_id, 10)
        .await
        .expect("Failed to get messages after clear");
    assert_eq!(after_clear.len(), 0, "Should have no messages after clear");
    println!("  ✅ Clear works correctly");

    println!("✅ {} - All tests passed!\n", test_name);
}

#[tokio::test]
#[ignore]
async fn test_qdrant_short_term_memory() {
    let storage = QdrantStorage::new("http://localhost:6333")
        .await
        .expect("Failed to create Qdrant storage");
    test_short_term_memory(Arc::new(storage), "Qdrant").await;
}

#[tokio::test]
#[ignore]
async fn test_chroma_short_term_memory() {
    let storage = ChromaStorage::new(
        "http://localhost:8000".to_string(),
        "test_mid_term".to_string(),
        "test_long_term".to_string(),
    )
    .await
    .expect("Failed to create Chroma storage");
    test_short_term_memory(Arc::new(storage), "Chroma").await;
}

#[tokio::test]
#[ignore]
async fn test_pinecone_short_term_memory() {
    let api_key = std::env::var("PINECONE_API_KEY").expect("PINECONE_API_KEY not set");
    let environment =
        std::env::var("PINECONE_ENVIRONMENT").unwrap_or_else(|_| "us-east-1-aws".to_string());
    let storage = PineconeStorage::new(
        api_key,
        environment,
        "test-mid-term".to_string(),
        "test-long-term".to_string(),
    );
    test_short_term_memory(Arc::new(storage), "Pinecone").await;
}

#[tokio::test]
#[ignore]
async fn test_concurrent_operations() {
    println!("\n🧪 Testing Concurrent Operations");
    let storage = Arc::new(
        QdrantStorage::new("http://localhost:6333")
            .await
            .expect("Failed to create storage"),
    );
    let user_id = format!("concurrent_user_{}", uuid::Uuid::now_v7());

    let mut handles = vec![];
    for i in 0..10 {
        let storage = storage.clone();
        let user_id = user_id.clone();
        let handle = tokio::spawn(async move {
            let msg = Message {
                role: "user".to_string(),
                content: format!("Message {}", i),
                timestamp: chrono::Utc::now(),
                embedding: None,
            };
            storage
                .add_short_term_message(&user_id, msg)
                .await
                .expect("Failed to add message");
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task failed");
    }
    println!("  ✅ Added 10 messages concurrently");

    let messages = storage
        .get_short_term_messages(&user_id, 20)
        .await
        .expect("Failed to get messages");
    assert_eq!(messages.len(), 10, "Should have 10 messages");
    println!("  ✅ All messages stored correctly");

    storage
        .clear_short_term(&user_id)
        .await
        .expect("Failed to clear");
    println!("✅ Concurrent operations test passed!\n");
}
