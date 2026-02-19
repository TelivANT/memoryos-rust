# 性能基准测试指南

**版本**: v0.3.0  
**更新**: 2026-02-19

---

## 📋 概述

本文档描述如何运行 MemoryOS-Rust 的性能基准测试，量化向量存储短期记忆操作的性能表现。

## 🎯 测试目标

测量以下操作的性能指标：

1. **add_short_term_message** - 添加消息延迟
2. **get_short_term_messages** - 获取消息延迟
3. **clear_short_term** - 清空消息延迟
4. **并发写入** - 多任务并发性能

## 🚀 快速开始

### 方法 1: 简单性能测试（推荐）

```bash
# 启动 Qdrant
docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant

# 运行性能测试
cargo run --release --package memoryos-benchmarks --bin perf_test
```

**输出示例**:
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 MemoryOS-Rust Performance Test
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🧪 Test 1: add_short_term_message
  ⏱️  Average: 15.23ms per operation
  📈 Throughput: 66 ops/sec

🧪 Test 2: get_short_term_messages
  ⏱️  5 messages: 8.45ms per operation
  ⏱️  10 messages: 9.12ms per operation
  ⏱️  20 messages: 10.34ms per operation

🧪 Test 3: Concurrent Operations
  ⏱️  1 concurrent: 16.78ms per batch
  ⏱️  5 concurrent: 45.23ms per batch
  ⏱️  10 concurrent: 78.56ms per batch
  ⏱️  20 concurrent: 142.34ms per batch

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Performance Test Complete!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### 方法 2: Criterion 基准测试（详细）

```bash
# 使用自动化脚本
./scripts/run_benchmarks.sh

# 或手动运行
cargo bench --package memoryos-benchmarks
```

**特点**:
- 统计分析（平均值、标准差、置信区间）
- HTML 报告生成
- 历史对比
- 更精确的测量

**查看报告**:
```bash
open target/criterion/report/index.html
```

---

## 📊 性能基准

### 预期性能指标（Qdrant, 本地 Docker）

| 操作 | 延迟 | 吞吐量 |
|------|------|--------|
| add_short_term_message | 10-20ms | 50-100 ops/sec |
| get_short_term_messages (10条) | 8-15ms | 65-125 ops/sec |
| clear_short_term | 50-100ms | 10-20 ops/sec |
| 并发写入 (10并发) | 70-100ms/batch | - |

### 影响因素

**硬件**:
- CPU 性能
- 内存大小
- 磁盘 I/O（SSD vs HDD）

**网络**:
- 本地 Docker: 低延迟
- 远程服务: 增加网络延迟

**负载**:
- 并发用户数
- 消息大小
- 向量维度

---

## 🔧 优化建议

### 1. 批量操作

```rust
// ❌ 逐条添加
for msg in messages {
    storage.add_short_term_message(&user_id, msg).await?;
}

// ✅ 批量添加（如果支持）
storage.add_short_term_messages_batch(&user_id, messages).await?;
```

### 2. 连接池

```rust
// 使用连接池减少连接开销
let storage = QdrantStorage::new_with_pool(
    "http://localhost:6333",
    pool_size: 10,
).await?;
```

### 3. 缓存策略

```rust
// 缓存最近访问的消息
let cache = Arc::new(RwLock::new(LruCache::new(100)));
```

### 4. 异步并发

```rust
// 利用 tokio 并发处理
let handles: Vec<_> = users.iter()
    .map(|user_id| {
        let storage = storage.clone();
        tokio::spawn(async move {
            storage.get_short_term_messages(user_id, 10).await
        })
    })
    .collect();
```

---

## 📈 性能对比

### 向量数据库 vs Redis（理论对比）

| 特性 | 向量数据库 | Redis |
|------|-----------|-------|
| 写入延迟 | 10-20ms | 1-5ms |
| 读取延迟 | 8-15ms | 1-3ms |
| 语义搜索 | ✅ 原生支持 | ❌ 需要额外实现 |
| 持久化 | ✅ 内置 | ⚠️ 需配置 |
| 扩展性 | ✅ 水平扩展 | ⚠️ 集群复杂 |
| 数据丢失风险 | ✅ 低 | ⚠️ 中（内存） |

**结论**: 向量数据库延迟略高（10-15ms），但提供语义搜索、持久化、更好的扩展性。对于记忆系统，这是值得的权衡。

---

## 🔬 自定义测试

### 创建自定义测试

```rust
use memoryos_adapters::QdrantStorage;
use memoryos_ports::VectorStorage;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let storage = QdrantStorage::new("http://localhost:6333")
        .await
        .unwrap();
    
    let user_id = "test_user";
    let start = Instant::now();
    
    // 你的测试代码
    storage.add_short_term_message(&user_id, msg).await.unwrap();
    
    let elapsed = start.elapsed();
    println!("Time: {:?}", elapsed);
}
```

### 测试不同场景

1. **不同消息大小**
   - 短消息（< 100 字符）
   - 中等消息（100-500 字符）
   - 长消息（> 500 字符）

2. **不同用户数量**
   - 单用户
   - 10 用户
   - 100 用户
   - 1000 用户

3. **不同并发级别**
   - 1, 5, 10, 20, 50, 100 并发

---

## 🐛 故障排查

### 性能下降

**症状**: 延迟突然增加

**可能原因**:
1. Qdrant 资源不足（CPU/内存）
2. 磁盘 I/O 瓶颈
3. 网络延迟
4. 数据量过大

**解决方案**:
```bash
# 检查 Qdrant 资源使用
docker stats

# 增加 Docker 资源限制
docker run -d \
  --memory=4g \
  --cpus=2 \
  -p 6333:6333 \
  qdrant/qdrant

# 清理旧数据
curl -X DELETE http://localhost:6333/collections/short_term_messages
```

### 测试失败

**错误**: `Failed to connect to Qdrant`

**解决**:
```bash
# 检查 Qdrant 是否运行
curl http://localhost:6333/health

# 重启 Qdrant
docker restart <container_id>
```

---

## 📚 相关文档

- [Integration Testing](./INTEGRATION_TESTING.md) - 集成测试指南
- [Architecture Improvement](./ARCHITECTURE_IMPROVEMENT.md) - 架构改进说明
- [Vector Databases Guide](./VECTOR_DATABASES.md) - 向量数据库配置

---

## 🎯 下一步

性能测试完成后：

1. ✅ **Production Deployment Guide** - 编写部署文档
2. ✅ **Monitoring Setup** - 配置监控系统
3. ✅ **Optimization** - 根据测试结果优化

---

## 🤝 贡献

发现性能问题或有优化建议？欢迎提交 Issue 或 PR！
