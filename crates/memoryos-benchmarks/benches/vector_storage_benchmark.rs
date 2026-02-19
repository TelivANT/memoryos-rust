//! 性能基准测试
//!
//! 测试向量存储短期记忆操作的性能:
//! - add_short_term_message 延迟
//! - get_short_term_messages 延迟
//! - clear_short_term 延迟
//! - 并发负载测试
//!
//! 运行: cargo bench --bench vector_storage_benchmark

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use memoryos_adapters::QdrantStorage;
use memoryos_core::Message;
use memoryos_ports::VectorStorage;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_test_message(i: usize) -> Message {
    Message {
        role: "user".to_string(),
        content: format!("Test message number {}", i),
        timestamp: chrono::Utc::now(),
        embedding: None,
    }
}

async fn setup_storage() -> Arc<QdrantStorage> {
    Arc::new(
        QdrantStorage::new("http://localhost:6333")
            .await
            .expect("Failed to create storage"),
    )
}

fn bench_add_message(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = rt.block_on(setup_storage());
    let user_id = format!("bench_user_{}", uuid::Uuid::now_v7());

    c.bench_function("add_short_term_message", |b| {
        let mut i = 0;
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            let user_id = user_id.clone();
            let msg = create_test_message(i);
            i += 1;
            async move {
                storage
                    .add_short_term_message(&user_id, black_box(msg))
                    .await
                    .unwrap();
            }
        });
    });

    // 清理
    rt.block_on(storage.clear_short_term(&user_id)).unwrap();
}

fn bench_get_messages(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = rt.block_on(setup_storage());
    let user_id = format!("bench_user_{}", uuid::Uuid::now_v7());

    // 预填充数据
    rt.block_on(async {
        for i in 0..10 {
            storage
                .add_short_term_message(&user_id, create_test_message(i))
                .await
                .unwrap();
        }
    });

    let mut group = c.benchmark_group("get_short_term_messages");
    for limit in [5, 10, 20].iter() {
        group.throughput(Throughput::Elements(*limit as u64));
        group.bench_with_input(BenchmarkId::from_parameter(limit), limit, |b, &limit| {
            b.to_async(&rt).iter(|| {
                let storage = storage.clone();
                let user_id = user_id.clone();
                async move {
                    storage
                        .get_short_term_messages(&user_id, black_box(limit))
                        .await
                        .unwrap();
                }
            });
        });
    }
    group.finish();

    // 清理
    rt.block_on(storage.clear_short_term(&user_id)).unwrap();
}

fn bench_clear_messages(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = rt.block_on(setup_storage());

    c.bench_function("clear_short_term", |b| {
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            let user_id = format!("bench_user_{}", uuid::Uuid::now_v7());
            async move {
                // 添加一些消息
                for i in 0..10 {
                    storage
                        .add_short_term_message(&user_id, create_test_message(i))
                        .await
                        .unwrap();
                }
                // 清空
                storage.clear_short_term(black_box(&user_id)).await.unwrap();
            }
        });
    });
}

fn bench_concurrent_writes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = rt.block_on(setup_storage());

    let mut group = c.benchmark_group("concurrent_writes");
    for concurrency in [1, 5, 10, 20].iter() {
        group.throughput(Throughput::Elements(*concurrency as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| {
                    let storage = storage.clone();
                    async move {
                        let user_id = format!("bench_user_{}", uuid::Uuid::now_v7());
                        let mut handles = vec![];

                        for i in 0..concurrency {
                            let storage = storage.clone();
                            let user_id = user_id.clone();
                            let handle = tokio::spawn(async move {
                                storage
                                    .add_short_term_message(&user_id, create_test_message(i))
                                    .await
                                    .unwrap();
                            });
                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }

                        // 清理
                        storage.clear_short_term(&user_id).await.unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_add_message,
    bench_get_messages,
    bench_clear_messages,
    bench_concurrent_writes
);
criterion_main!(benches);
