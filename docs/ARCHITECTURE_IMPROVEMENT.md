# 架构改进建议：短期记忆持久化

## 📋 问题描述

**当前架构问题**:

目前短期记忆（Short-Term Memory）存储在 Redis/NATS 中，存在以下问题：

1. **数据丢失风险**: Redis/NATS 重启或故障会导致短期记忆丢失
2. **架构不一致**: 中期和长期记忆存储在向量数据库，短期记忆却在 Redis/NATS
3. **功能受限**: 无法对短期记忆进行语义搜索
4. **用途混淆**: Redis/NATS 应该用于分布式协调，而不是主存储

## 🎯 建议的架构

### 正确的分层

```
┌─────────────────────────────────────────────────────────────┐
│                    Memory Storage (持久化)                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  短期记忆 (STM) → 向量数据库 (Qdrant/Chroma/Pinecone)        │
│  中期记忆 (MTM) → 向量数据库 (Qdrant/Chroma/Pinecone)        │
│  长期记忆 (LTM) → 向量数据库 (Qdrant/Chroma/Pinecone)        │
│                                                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│              Coordination Layer (分布式协调)                 │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Redis/NATS 用途:                                            │
│  • Session 管理 (临时会话状态)                               │
│  • 分布式锁 (防止并发冲突)                                   │
│  • 消息队列 (Worker 通信)                                    │
│  • 热点数据缓存 (加速访问)                                   │
│  • IP 防御临时封禁 (TTL 自动过期)                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 为什么短期记忆应该在向量数据库？

1. **持久化**: 向量数据库提供持久化存储，不会因重启丢失数据
2. **语义搜索**: 可以对短期记忆进行语义搜索，找到相关对话
3. **一致性**: 所有记忆层使用统一的存储架构
4. **扩展性**: 向量数据库支持水平扩展和高可用
5. **备份恢复**: 向量数据库提供完善的备份和恢复机制

### Redis/NATS 的正确用途

**Redis 应该用于**:
- ✅ Session 管理 (临时状态，TTL 自动过期)
- ✅ 分布式锁 (Redlock 算法)
- ✅ 热点数据缓存 (LRU 缓存)
- ✅ IP 防御临时封禁 (TTL 自动过期)
- ✅ Rate Limiting (滑动窗口)
- ❌ ~~短期记忆存储~~ (应该用向量数据库)

**NATS 应该用于**:
- ✅ 消息队列 (Worker 通信)
- ✅ Pub/Sub (事件通知)
- ✅ 分布式协调 (JetStream)
- ✅ 服务发现
- ❌ ~~短期记忆存储~~ (应该用向量数据库)

## 🔧 迁移方案

### 方案 A: 渐进式迁移 (推荐)

**阶段 1: 双写模式** (1-2 周)
```rust
// 同时写入 Redis 和向量数据库
async fn add_message(&self, user_id: &str, message: Message) {
    // 写入 Redis (旧方式，兼容)
    self.redis.add_message(user_id, &message).await?;
    
    // 写入向量数据库 (新方式)
    self.vector_store.add_short_term_message(user_id, message).await?;
}

// 优先从向量数据库读取，失败则从 Redis 读取
async fn get_recent(&self, user_id: &str, limit: usize) {
    match self.vector_store.get_short_term_messages(user_id, limit).await {
        Ok(messages) => Ok(messages),
        Err(_) => self.redis.get_recent(user_id, limit).await, // Fallback
    }
}
```

**阶段 2: 数据迁移** (1 天)
```bash
# 运行迁移脚本，将 Redis 中的短期记忆迁移到向量数据库
cargo run --bin migrate_shortterm_to_vector
```

**阶段 3: 切换读取** (1 周)
```rust
// 只从向量数据库读取
async fn get_recent(&self, user_id: &str, limit: usize) {
    self.vector_store.get_short_term_messages(user_id, limit).await
}
```

**阶段 4: 停止双写** (1 周)
```rust
// 只写入向量数据库
async fn add_message(&self, user_id: &str, message: Message) {
    self.vector_store.add_short_term_message(user_id, message).await
}
```

### 方案 B: 一次性切换 (快速但有风险)

**步骤**:
1. 停止服务
2. 迁移 Redis 数据到向量数据库
3. 更新代码，移除 Redis 短期记忆逻辑
4. 重启服务

**风险**:
- 停机时间较长
- 迁移失败需要回滚
- 无法渐进式验证

## 📊 对比分析

| 特性 | Redis/NATS (当前) | 向量数据库 (建议) |
|------|-------------------|-------------------|
| **持久化** | ❌ 内存存储，重启丢失 | ✅ 持久化存储 |
| **语义搜索** | ❌ 不支持 | ✅ 支持向量搜索 |
| **备份恢复** | ⚠️ 需要 RDB/AOF | ✅ 原生支持 |
| **扩展性** | ⚠️ 有限 | ✅ 水平扩展 |
| **一致性** | ❌ 与其他记忆层不一致 | ✅ 统一架构 |
| **延迟** | ✅ ~1ms | ⚠️ ~5-10ms |
| **成本** | ✅ 低 | ⚠️ 中等 |

## 🚀 实现细节

### 1. 扩展 VectorStorage Trait

```rust
#[async_trait]
pub trait VectorStorage: Send + Sync {
    // ========== Short-Term Memory (NEW) ==========
    
    /// Add a message to short-term memory (with embedding)
    async fn add_short_term_message(&self, user_id: &str, message: Message) -> Result<(), AppError>;
    
    /// Get recent N messages from short-term memory
    async fn get_short_term_messages(&self, user_id: &str, limit: usize) -> Result<Vec<Message>, AppError>;
    
    /// Clear short-term memory for a user
    async fn clear_short_term(&self, user_id: &str) -> Result<(), AppError>;
    
    // ========== Mid-Term Memory ==========
    async fn store_segment(&self, segment: MidTermSegment) -> Result<(), AppError>;
    async fn search_segments(&self, user_id: &str, query_embedding: Vec<f32>, limit: usize) -> Result<Vec<MidTermSegment>, AppError>;
    
    // ========== Long-Term Memory ==========
    async fn store_long_term(&self, memory: LongTermMemory) -> Result<(), AppError>;
    async fn get_long_term(&self, user_id: &str) -> Result<Option<LongTermMemory>, AppError>;
}
```

### 2. Qdrant 实现

```rust
impl VectorStorage for QdrantStorage {
    async fn add_short_term_message(&self, user_id: &str, message: Message) -> Result<(), AppError> {
        let message_id = uuid::Uuid::now_v7();
        let embedding = message.embedding.unwrap_or_else(|| vec![0.0; 1536]);
        
        let mut payload = HashMap::new();
        payload.insert("user_id", user_id);
        payload.insert("role", &message.role);
        payload.insert("content", &message.content);
        payload.insert("timestamp", &message.timestamp.to_rfc3339());
        
        let point = PointStruct::new(message_id, embedding, payload);
        
        self.client
            .upsert_points("short_term_messages", vec![point])
            .await?;
        
        Ok(())
    }
    
    async fn get_short_term_messages(&self, user_id: &str, limit: usize) -> Result<Vec<Message>, AppError> {
        let filter = Filter::must([Condition::matches("user_id", user_id)]);
        
        let results = self.client
            .search_points("short_term_messages", vec![0.0; 1536], limit)
            .filter(filter)
            .await?;
        
        let messages = results.into_iter()
            .map(|point| Message {
                role: point.payload["role"].clone(),
                content: point.payload["content"].clone(),
                timestamp: parse_timestamp(&point.payload["timestamp"]),
                embedding: None,
            })
            .collect();
        
        Ok(messages)
    }
    
    async fn clear_short_term(&self, user_id: &str) -> Result<(), AppError> {
        let filter = Filter::must([Condition::matches("user_id", user_id)]);
        
        self.client
            .delete_points("short_term_messages")
            .filter(filter)
            .await?;
        
        Ok(())
    }
}
```

### 3. 创建 CoordinationLayer Trait

```rust
/// Coordination layer for distributed systems (Redis/NATS)
#[async_trait]
pub trait CoordinationLayer: Send + Sync {
    // Caching
    async fn cache_set(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), AppError>;
    async fn cache_get(&self, key: &str) -> Result<Option<String>, AppError>;
    async fn cache_del(&self, key: &str) -> Result<(), AppError>;
    
    // Pub/Sub
    async fn publish(&self, channel: &str, message: &str) -> Result<(), AppError>;
    async fn subscribe(&self, channel: &str) -> Result<Receiver<String>, AppError>;
    
    // Distributed Lock
    async fn acquire_lock(&self, key: &str, ttl_secs: u64) -> Result<String, AppError>;
    async fn release_lock(&self, key: &str, token: &str) -> Result<(), AppError>;
    
    // Session Management
    async fn session_set(&self, session_id: &str, data: &str, ttl_secs: u64) -> Result<(), AppError>;
    async fn session_get(&self, session_id: &str) -> Result<Option<String>, AppError>;
    async fn session_del(&self, session_id: &str) -> Result<(), AppError>;
}
```

## 📝 配置示例

### 新配置 (推荐)

```toml
[storage]
# 短期记忆存储方式
short_term_backend = "vector"  # "vector" 或 "redis" (兼容模式)

[storage.vector]
type = "qdrant"
url = "http://localhost:6334"
collections = {
    short_term = "short_term_messages",
    mid_term = "mid_term_segments",
    long_term = "long_term_memory"
}

[coordination]
# Redis/NATS 用于分布式协调
backend = "redis"  # "redis" 或 "nats"

[coordination.redis]
url = "redis://localhost:6379"
# 用途: 缓存、锁、Session
cache_ttl = 3600
lock_ttl = 30
session_ttl = 86400

[coordination.nats]
url = "nats://localhost:4222"
# 用途: 消息队列、Pub/Sub
```

### 旧配置 (兼容模式)

```toml
[storage]
short_term_backend = "redis"  # 保持旧行为

[storage.redis]
url = "redis://localhost:6379"
ttl_seconds = 3600
max_messages = 20
```

## ⚠️ 注意事项

1. **Embedding 生成**: 短期记忆需要生成 embedding，会增加延迟
   - 解决方案: 异步生成 embedding，先存储文本
   
2. **查询性能**: 向量搜索比 Redis LIST 慢
   - 解决方案: 使用 Redis 缓存最近 N 条消息
   
3. **存储成本**: 向量数据库存储成本高于 Redis
   - 解决方案: 定期清理过期短期记忆

4. **迁移风险**: 数据迁移可能失败
   - 解决方案: 使用双写模式，渐进式迁移

## 🎯 推荐行动

### 短期 (1-2 周)
1. ✅ 创建此文档，记录问题和方案
2. ✅ 在 VectorStorage trait 中添加短期记忆方法
3. ✅ 实现 Qdrant/Chroma/Pinecone 短期记忆存储
4. ✅ 添加配置选项 `short_term_backend`

### 中期 (1 个月)
1. 实现双写模式
2. 编写数据迁移脚本
3. 在测试环境验证
4. 逐步切换生产环境

### 长期 (3 个月)
1. 完全切换到向量数据库
2. 移除 Redis 短期记忆代码
3. 重构 Redis 为 CoordinationLayer
4. 更新所有文档

## 📚 相关文档

- [ARCHITECTURE.md](./ARCHITECTURE.md) - 系统架构
- [VECTOR_DATABASES.md](./VECTOR_DATABASES.md) - 向量数据库选择
- [NATS_ALTERNATIVE.md](./NATS_ALTERNATIVE.md) - NATS 使用指南
- [REDIS_CONFIGURATION.md](./ops/redis_configuration.md) - Redis 配置

## 🤝 贡献

如果你有更好的方案或建议，欢迎提交 Issue 或 PR！

---

**状态**: 📋 提案阶段  
**优先级**: 🔴 高  
**预计工作量**: 2-4 周  
**风险等级**: ⚠️ 中等  

**创建时间**: 2026-02-19  
**最后更新**: 2026-02-19  
**作者**: Kiro AI
