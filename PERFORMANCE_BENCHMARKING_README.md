# Performance Benchmarking

性能基准测试已创建！

## 📁 文件

- `crates/memoryos-benchmarks/` - Benchmark 包
- `crates/memoryos-benchmarks/benches/vector_storage_benchmark.rs` - Criterion 基准测试
- `crates/memoryos-benchmarks/src/bin/perf_test.rs` - 简单性能测试工具
- `scripts/run_benchmarks.sh` - 自动化测试脚本
- `docs/PERFORMANCE_BENCHMARKING.md` - 详细文档

## 🚀 快速运行

### 方法 1: 简单性能测试（推荐）

```bash
# 启动 Qdrant
docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant

# 运行测试
cargo run --release --package memoryos-benchmarks --bin perf_test
```

### 方法 2: Criterion 基准测试

```bash
# 使用脚本
./scripts/run_benchmarks.sh

# 或手动运行
cargo bench --package memoryos-benchmarks
```

## 📊 测试内容

1. ✅ **add_short_term_message** - 添加消息延迟
2. ✅ **get_short_term_messages** - 获取消息延迟（不同数量）
3. ✅ **clear_short_term** - 清空消息延迟
4. ✅ **并发写入** - 多任务并发性能（1, 5, 10, 20 并发）

## 📈 预期性能

| 操作 | 延迟 | 吞吐量 |
|------|------|--------|
| add_short_term_message | 10-20ms | 50-100 ops/sec |
| get_short_term_messages | 8-15ms | 65-125 ops/sec |
| clear_short_term | 50-100ms | 10-20 ops/sec |

## 📚 详细文档

查看 `docs/PERFORMANCE_BENCHMARKING.md` 获取完整指南。
