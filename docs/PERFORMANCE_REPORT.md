# 性能基准测试报告 (v0.9.0)

**项目**: MemoryOS-Rust  
**版本**: v0.9.0  
**日期**: 2026-02-20  
**运行命令**:

```bash
cargo bench -p memoryos-benchmarks --bench optimization_benchmark -- --noplot
cargo bench -p memoryos-benchmarks --bench graph_benchmark -- --noplot
cargo bench -p memoryos-benchmarks --bench security_benchmark -- --noplot
```

> 说明：以下数据来自本仓库在当前开发机上的 Criterion 输出（bench profile, optimized）。不同机器/负载下结果会有波动。

---

## 1) Optimization Benchmarks

| Benchmark | 典型耗时 (median) | 备注 |
|---|---:|---|
| bloom_filter/contains/100 | ~202 ns | 约 4.95M ops/s |
| bloom_filter/insert/100 | ~225 ns | 约 4.45M ops/s |
| embedding_cache/cache_hit | ~2.02 µs | HashMap + Vec 存取 |
| embedding_cache/cache_miss | ~74 ns | 仅走未命中路径 |
| cosine_similarity_1536d | ~4.28 µs | 1536 维向量余弦相似度 |
| filter_100_candidates | ~246 µs | SimilarityFilter 过滤 100 candidates |

---

## 2) Graph Benchmarks

| Benchmark | 典型耗时 (median) | 备注 |
|---|---:|---|
| extract_entities | ~10.27 µs | 从文本提取实体 |
| extract_relations | ~5.59 µs | 从文本提取关系 |
| extract_and_merge_5_texts | ~57.30 µs | 5 段文本 merge |
| query_by_label | ~4.00 µs | label 索引查询 |
| get_all_entities | ~158 ns | 遍历实体集合 |
| get_all_triples | ~7.82 µs | 遍历所有 triples |

---

## 3) Security Benchmarks

| Benchmark | 典型耗时 (median) | 备注 |
|---|---:|---|
| injection_check_safe | ~153 ns | 安全输入检查 |
| injection_check_malicious | ~46 ns | 命中模式时更快（短路） |
| pii_sanitize | ~1.92 µs | email/phone/SSN/credit_card/API_key 脱敏 |
| encrypt_small_13b | ~2.19 µs | AES-256-GCM 加密 (13 bytes) |
| encrypt_medium_4kb | ~180 µs | AES-256-GCM 加密 (4KB) |
| encrypt_large_64kb | ~2.84 ms | AES-256-GCM 加密 (64KB) |
| decrypt_medium_4kb | ~26.38 µs | AES-256-GCM 解密 (4KB) |
| audit_log_event | ~127 ns | 结构化审计事件写入（内存+可选持久化） |
| audit_get_recent_50 | ~6.35 µs | 最近 50 条 |
| audit_get_by_user_50 | ~6.91 µs | 按 user 过滤 50 条 |

---

## 结论

- **优化模块**的核心数据结构操作基本都在 ns~µs 级别，热点路径足够轻。
- **Graph/安全模块**的纯 CPU 逻辑在 µs 级；AES-256-GCM 在 4KB payload 下约 180µs，加密开销可控。
- **向量存储相关 benchmark**依赖外部 Qdrant 服务；在 CI 或无服务环境下建议跳过或以 degrade 模式运行（本 repo 已对 vector benchmark 做了可用性判断）。
