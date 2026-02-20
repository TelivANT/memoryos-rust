use criterion::{black_box, criterion_group, criterion_main, Criterion};
use memoryos_core::GraphManager;

fn bench_entity_extraction(c: &mut Criterion) {
    let gm = GraphManager::new();
    let text = "John Smith works at Google in Mountain View. Alice Johnson studies at MIT and lives in Boston. Bob Williams manages the engineering team at Microsoft.";

    c.bench_function("extract_entities", |b| {
        b.iter(|| gm.extract_entities(black_box(text)));
    });
}

fn bench_relation_extraction(c: &mut Criterion) {
    let gm = GraphManager::new();
    let text = "John Smith works at Google. Alice Johnson lives in Boston. Bob Williams manages the engineering team.";

    c.bench_function("extract_relations", |b| {
        b.iter(|| gm.extract_relations(black_box(text)));
    });
}

fn bench_extract_and_merge(c: &mut Criterion) {
    let texts = vec![
        "John Smith works at Google in Mountain View.",
        "Alice Johnson studies at MIT in Boston.",
        "Bob Williams manages the team at Microsoft.",
        "Carol Davis lives in San Francisco and works at Apple.",
        "Eve Brown created the new product at Amazon.",
    ];

    c.bench_function("extract_and_merge_5_texts", |b| {
        b.iter(|| {
            let mut gm = GraphManager::new();
            for text in &texts {
                gm.extract_and_merge(black_box(text));
            }
        });
    });
}

fn bench_graph_query(c: &mut Criterion) {
    let mut gm = GraphManager::new();
    for i in 0..100 {
        gm.extract_and_merge(&format!(
            "Person{} works at Company{} in City{}.",
            i,
            i % 10,
            i % 5
        ));
    }

    c.bench_function("query_by_label", |b| {
        b.iter(|| gm.query_by_label(black_box("Person50")));
    });

    c.bench_function("get_all_entities", |b| {
        b.iter(|| gm.get_all_entities());
    });

    c.bench_function("get_all_triples", |b| {
        b.iter(|| gm.get_all_triples());
    });
}

criterion_group!(
    benches,
    bench_entity_extraction,
    bench_relation_extraction,
    bench_extract_and_merge,
    bench_graph_query
);
criterion_main!(benches);
