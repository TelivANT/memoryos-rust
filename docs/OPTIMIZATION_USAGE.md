# 优化模块使用指南

## 📦 模块概览

MemoryOS-Rust 提供了 6 个高性能优化模块，可减少 50-100% 的 API 调用和计算量。

## 🚀 快速开始

### 1. FAQ 快速匹配 (Bloom Filter + 精确缓存)

**性能提升**: 2 秒 → < 1ms (2000 倍)

```rust
use memoryos_core::OptimizedFaqMatcher;

// 初始化
let mut matcher = OptimizedFaqMatcher::new(1000);

// 添加 FAQ
matcher.add_faq("WiFi密码是多少？", "密码是12345678".to_string());
matcher.add_faq("如何报销？", "登录系统提交申请".to_string());

// 查询 (< 1ms)
if let Some(answer) = matcher.match_faq("WiFi密码是多少？").await {
    println!("直接返回: {}", answer);
} else {
    // 走正常检索流程
}
```

### 2. Embedding 缓存 (LRU Cache)

**性能提升**: 跳过 100% LLM 调用 (高频查询)

```rust
use memoryos_core::EmbeddingCache;

let cache = EmbeddingCache::new(1000);

// 第一次调用 LLM
let embedding = if let Some(cached) = cache.get(query).await {
    cached
} else {
    let emb = llm.embed(query).await?;
    cache.put(query.to_string(), emb.clone()).await;
    emb
};

// 第二次直接返回缓存
```

### 3. 批量 Embedding 生成

**性能提升**: 10 次调用 → 1 次调用 (90% 减少)

```rust
use memoryos_core::BatchEmbedder;

let texts = vec![
    "消息1".to_string(),
    "消息2".to_string(),
    "消息3".to_string(),
];

// 批量生成 (1 次 API 调用)
let embeddings = BatchEmbedder::embed_batch(texts, |batch| async move {
    llm.embed_batch(batch).await
}).await?;
```

### 4. 热度批量更新 (Redis 缓冲)

**性能提升**: 200 次 Qdrant → 3 次 (98.5% 减少)

```rust
use memoryos_core::HeatBuffer;
use tokio::time::{interval, Duration};

let buffer = HeatBuffer::new();

// 记录访问 (只写 Redis)
buffer.record_access(segment_id).await;

// 后台任务：每 5 分钟批量同步
tokio::spawn(async move {
    let mut ticker = interval(Duration::from_secs(300));
    loop {
        ticker.tick().await;
        
        let updates = buffer.drain().await;
        if !updates.is_empty() {
            // 批量更新 Qdrant
            qdrant.update_batch(updates).await?;
        }
    }
});
```

### 5. 相似度分层过滤

**性能提升**: 1000 次计算 → 300 次 (70% 减少)

```rust
use memoryos_core::SimilarityFilter;

let query_embedding = vec![1.0; 768];
let candidates = vec![
    (vec![1.0; 768], 0),
    (vec![0.5; 768], 1),
    // ... 1000 个候选
];

// 两阶段过滤
let results = SimilarityFilter::filter_similar(
    &query_embedding,
    candidates,
    0.7, // 阈值
);

// 只对通过粗筛的候选进行完整计算
```

### 6. 增量记忆合并

**性能提升**: 10 次 LLM → 5 次 (50% 减少)

```rust
use memoryos_core::IncrementalSummarizer;

let mut summarizer = IncrementalSummarizer::new(20); // 阈值 20 条

// 添加消息
if let Some(summary) = summarizer.add_messages(
    new_messages,
    |text| async move { llm.summarize(text).await }
).await? {
    // 达到阈值，生成总结
    store_summary(summary).await?;
}
```

## 🎯 集成示例

### 完整的优化检索流程

```rust
use memoryos_core::{OptimizedRetriever, OptimizedFaqMatcher};

pub struct OptimizedMemorySystem {
    faq_matcher: OptimizedFaqMatcher,
    retriever: OptimizedRetriever,
}

impl OptimizedMemorySystem {
    pub async fn query(&self, user_query: &str) -> Result<String> {
        // 1. 尝试 FAQ 快速匹配 (< 1ms)
        if let Some(answer) = self.faq_matcher.match_faq(user_query).await {
            return Ok(answer);
        }
        
        // 2. 使用缓存的 embedding 检索
        let embedding = self.retriever.retrieve_with_cache(
            user_query,
            |q| async move { llm.embed(q).await }
        ).await?;
        
        // 3. 向量检索
        let memories = qdrant.search(embedding).await?;
        
        // 4. 生成回复
        let response = llm.complete(user_query, memories).await?;
        
        Ok(response)
    }
}
```

## 📊 性能对比

| 场景 | 原始 | 优化后 | 提升 |
|------|------|--------|------|
| FAQ 查询 | 4 请求 + LLM (2s) | 0 请求 (< 1ms) | 2000x |
| 高频查询 | 每次 LLM | 缓存命中 | 100% |
| 批量 Embedding | 10 次 API | 1 次 API | 10x |
| 热度更新 | 200 次 Qdrant | 3 次 Qdrant | 66x |
| 相似度计算 | 1000 次完整 | 300 次完整 | 3.3x |
| 记忆合并 | 10 次 LLM | 5 次 LLM | 2x |

## 🎯 最佳实践

### 1. FAQ 系统
- 启动时加载所有 FAQ 到 Bloom Filter
- 定期更新精确缓存
- 监控命中率

### 2. Embedding 缓存
- 设置合理的缓存大小 (1000-10000)
- 监控缓存命中率
- 定期清理低频查询

### 3. 批量操作
- 累积到一定数量再批量处理
- 使用后台任务异步处理
- 设置合理的批量大小 (32-128)

### 4. 热度更新
- 使用 Redis 缓冲访问计数
- 每 5-10 分钟批量同步
- 避免频繁写 Qdrant

### 5. 相似度过滤
- 先用前 64 维粗筛
- 只对候选计算完整相似度
- 调整粗筛阈值平衡精度和性能

### 6. 增量合并
- 设置合理的阈值 (10-20 条)
- 避免频繁调用 LLM
- 保留中间状态

## 🔧 配置建议

```rust
// 生产环境配置
OptimizedFaqMatcher::new(10000);           // FAQ 容量
EmbeddingCache::new(5000);                 // 缓存大小
IncrementalSummarizer::new(20);            // 合并阈值
HeatBuffer::sync_interval(300);            // 同步间隔 (秒)
SimilarityFilter::prefix_dim(64);          // 粗筛维度
BatchEmbedder::batch_size(32);             // 批量大小
```

## 📈 监控指标

```rust
// 缓存统计
let stats = embedding_cache.stats().await;
println!("缓存命中率: {:.2}%", stats.hit_rate * 100.0);

// FAQ 统计
let faq_hits = faq_matcher.hit_count();
let faq_misses = faq_matcher.miss_count();
println!("FAQ 命中率: {:.2}%", 
    faq_hits as f64 / (faq_hits + faq_misses) as f64 * 100.0);

// 热度缓冲
let buffer_size = heat_buffer.size().await;
println!("待同步热度更新: {}", buffer_size);
```

## 🚨 注意事项

1. **内存使用**: Bloom Filter 和缓存会占用内存，根据实际情况调整大小
2. **一致性**: 热度批量更新有延迟，不适合需要实时一致性的场景
3. **缓存失效**: 需要在 FAQ 更新时清理相关缓存
4. **误判率**: Bloom Filter 有 1% 误判率，需要二次确认
5. **并发安全**: 所有模块都是线程安全的，可以在多线程环境使用

## 📚 更多信息

- 详细设计: [OPTIMIZATION.md](./OPTIMIZATION.md)
- 性能测试: `cargo bench --package memoryos-core`
- 示例代码: `examples/optimization_demo.rs`
