use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use memoryos_core::{BloomFilter, EmbeddingCache, SimilarityFilter};
use tokio::runtime::Runtime;

fn bench_bloom_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_filter");

    for size in [100, 1000, 10000] {
        let mut filter = BloomFilter::new(size, 0.01);
        for i in 0..size {
            filter.insert(&format!("item_{}", i));
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("contains", size), &size, |b, &size| {
            b.iter(|| filter.contains(black_box(&format!("item_{}", size / 2))));
        });

        group.bench_with_input(BenchmarkId::new("insert", size), &size, |b, _| {
            let mut f = BloomFilter::new(size, 0.01);
            let mut i = 0;
            b.iter(|| {
                f.insert(black_box(&format!("new_item_{}", i)));
                i += 1;
            });
        });
    }
    group.finish();
}

fn bench_embedding_cache(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("embedding_cache");

    let cache = EmbeddingCache::new(1000);
    rt.block_on(async {
        for i in 0..500 {
            cache.put(format!("key_{}", i), vec![i as f32; 128]).await;
        }
    });

    group.bench_function("cache_hit", |b| {
        b.to_async(&rt)
            .iter(|| async { cache.get(black_box("key_250")).await });
    });

    group.bench_function("cache_miss", |b| {
        b.to_async(&rt)
            .iter(|| async { cache.get(black_box("nonexistent_key")).await });
    });

    group.finish();
}

fn bench_similarity_filter(c: &mut Criterion) {
    let vec_a: Vec<f32> = (0..1536).map(|i| (i as f32).sin()).collect();
    let vec_b: Vec<f32> = (0..1536).map(|i| (i as f32).cos()).collect();

    c.bench_function("cosine_similarity_1536d", |b| {
        b.iter(|| SimilarityFilter::cosine_similarity(black_box(&vec_a), black_box(&vec_b)));
    });

    let candidates: Vec<(Vec<f32>, usize)> = (0..100)
        .map(|j| {
            (
                (0..1536).map(|i| ((i + j) as f32).sin()).collect(),
                j as usize,
            )
        })
        .collect();

    c.bench_function("filter_100_candidates", |b| {
        b.iter(|| {
            SimilarityFilter::filter_similar(black_box(&vec_a), black_box(candidates.clone()), 0.8)
        });
    });
}

criterion_group!(
    benches,
    bench_bloom_filter,
    bench_embedding_cache,
    bench_similarity_filter
);
criterion_main!(benches);
