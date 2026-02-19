//! 简单性能测试工具
//!
//! 运行: cargo run --release --bin perf_test

use memoryos_adapters::QdrantStorage;
use memoryos_core::Message;
use memoryos_ports::VectorStorage;
use std::sync::Arc;
use std::time::Instant;

fn create_message(i: usize) -> Message {
    Message {
        role: "user".to_string(),
        content: format!("Test message {}", i),
        timestamp: chrono::Utc::now(),
        embedding: None,
    }
}

async fn test_add_performance(storage: &Arc<QdrantStorage>, iterations: usize) -> f64 {
    let user_id = format!("perf_user_{}", uuid::Uuid::now_v7());
    let start = Instant::now();

    for i in 0..iterations {
        storage
            .add_short_term_message(&user_id, create_message(i))
            .await
            .unwrap();
    }

    let elapsed = start.elapsed();
    storage.clear_short_term(&user_id).await.unwrap();

    elapsed.as_secs_f64() / iterations as f64
}

async fn test_get_performance(
    storage: &Arc<QdrantStorage>,
    msg_count: usize,
    iterations: usize,
) -> f64 {
    let user_id = format!("perf_user_{}", uuid::Uuid::now_v7());

    // 预填充
    for i in 0..msg_count {
        storage
            .add_short_term_message(&user_id, create_message(i))
            .await
            .unwrap();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        storage
            .get_short_term_messages(&user_id, msg_count)
            .await
            .unwrap();
    }
    let elapsed = start.elapsed();

    storage.clear_short_term(&user_id).await.unwrap();
    elapsed.as_secs_f64() / iterations as f64
}

async fn test_concurrent_performance(
    storage: &Arc<QdrantStorage>,
    concurrency: usize,
    iterations: usize,
) -> f64 {
    let start = Instant::now();

    for _ in 0..iterations {
        let user_id = format!("perf_user_{}", uuid::Uuid::now_v7());
        let mut handles = vec![];

        for i in 0..concurrency {
            let storage = storage.clone();
            let user_id = user_id.clone();
            let handle = tokio::spawn(async move {
                storage
                    .add_short_term_message(&user_id, create_message(i))
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        storage.clear_short_term(&user_id).await.unwrap();
    }

    let elapsed = start.elapsed();
    elapsed.as_secs_f64() / iterations as f64
}

#[tokio::main]
async fn main() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 MemoryOS-Rust Performance Test");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let storage = Arc::new(
        QdrantStorage::new("http://localhost:6333")
            .await
            .expect("Failed to connect to Qdrant"),
    );

    // Test 1: Add Message
    println!("🧪 Test 1: add_short_term_message");
    let add_time = test_add_performance(&storage, 100).await;
    println!("  ⏱️  Average: {:.2}ms per operation", add_time * 1000.0);
    println!("  📈 Throughput: {:.0} ops/sec\n", 1.0 / add_time);

    // Test 2: Get Messages
    println!("🧪 Test 2: get_short_term_messages");
    for count in [5, 10, 20] {
        let get_time = test_get_performance(&storage, count, 50).await;
        println!(
            "  ⏱️  {} messages: {:.2}ms per operation",
            count,
            get_time * 1000.0
        );
    }
    println!();

    // Test 3: Concurrent Writes
    println!("🧪 Test 3: Concurrent Operations");
    for concurrency in [1, 5, 10, 20] {
        let concurrent_time = test_concurrent_performance(&storage, concurrency, 20).await;
        println!(
            "  ⏱️  {} concurrent: {:.2}ms per batch",
            concurrency,
            concurrent_time * 1000.0
        );
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Performance Test Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}
