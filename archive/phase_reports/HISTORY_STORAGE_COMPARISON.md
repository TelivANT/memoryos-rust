# 历史数据存储方案对比

**日期**: 2026-02-18 03:04  
**问题**: 历史版本数据库能否使用项目自带的向量数据库（Qdrant）？

---

## 📊 方案对比

### 方案 1: Redis List（当前实现）

**优点**:
- ✅ 简单直接 - List 天然支持时间序列
- ✅ 性能高 - O(1) 写入，O(N) 读取
- ✅ 自动过期 - TTL 30 天自动清理
- ✅ 轻量级 - 不需要额外索引

**缺点**:
- ❌ 功能单一 - 只能按 memory_id 查询
- ❌ 无法搜索 - 不支持内容搜索
- ❌ 额外依赖 - 需要 Redis（但项目已有）

**适用场景**:
- 简单的历史记录查询
- 按 memory_id 获取完整历史
- 不需要复杂查询

---

### 方案 2: Qdrant Collection（推荐）

**优点**:
- ✅ **复用现有基础设施** - 项目已有 Qdrant
- ✅ **向量搜索** - 可以语义搜索历史记录
- ✅ **丰富查询** - 支持过滤、排序、分页
- ✅ **统一管理** - 所有数据在一个系统
- ✅ **可扩展** - 支持更复杂的历史分析

**缺点**:
- ⚠️ 稍复杂 - 需要设计 payload 结构
- ⚠️ 性能开销 - 向量计算（但可以用空向量）

**适用场景**:
- 需要搜索历史内容
- 需要复杂过滤（按时间、类型、用户）
- 需要历史分析和统计

---

## 🎯 推荐方案：Qdrant

### 为什么选择 Qdrant？

1. **复用基础设施** - 项目已有 Qdrant，无需额外依赖
2. **功能更强大** - 支持语义搜索、过滤、分页
3. **统一架构** - 所有记忆数据在同一系统
4. **可扩展性** - 未来可以做历史分析、趋势预测

### Collection 设计

```rust
// Collection: memory_history
{
    "id": "hist_uuid_v7",  // UUID v7 - 时间排序
    "vector": [0.0; 384],  // 空向量或内容 embedding
    "payload": {
        "memory_id": "msg_uuid",
        "old_content": "...",
        "new_content": "...",
        "event_type": "Add",
        "created_at": 1708185600,  // Unix timestamp
        "actor_id": "user123"
    }
}
```

### 查询示例

```rust
// 1. 按 memory_id 查询
filter: memory_id == "msg_abc"
order_by: created_at DESC

// 2. 按时间范围查询
filter: created_at >= 1708099200 AND created_at <= 1708185600

// 3. 按事件类型查询
filter: event_type == "Update"

// 4. 语义搜索（如果有 embedding）
query_vector: [0.1, 0.2, ...]
filter: memory_id == "msg_abc"
```

---

## 🔄 迁移方案

### 阶段 1: 创建 Qdrant 实现（1 小时）

```rust
// crates/memoryos-adapters/src/history/qdrant.rs
pub struct QdrantHistoryStorage {
    client: Arc<Qdrant>,
    collection_name: String,
}

impl HistoryStorage for QdrantHistoryStorage {
    async fn add_entry(&self, entry: MemoryHistoryEntry) -> Result<(), AppError> {
        // 插入到 Qdrant collection
    }
    
    async fn get_history(&self, memory_id: &str) -> Result<Vec<MemoryHistoryEntry>, AppError> {
        // 按 memory_id 过滤 + 时间排序
    }
}
```

### 阶段 2: 配置切换（10 分钟）

```toml
[storage.history]
backend = "qdrant"  # 或 "redis"
collection = "memory_history"
```

### 阶段 3: 增强功能（可选）

1. **语义搜索** - 为历史内容生成 embedding
2. **高级过滤** - 按时间范围、事件类型查询
3. **统计分析** - 历史变更趋势、热点记忆

---

## 💡 实现建议

### 最小化实现（推荐）

```rust
// 1. 使用空向量（不做语义搜索）
vector: vec![0.0; 384]

// 2. 只实现基础查询
- get_history(memory_id) - 按 memory_id 查询
- add_entry() - 添加历史记录

// 3. 利用 Qdrant 的过滤功能
filter: {
    "must": [
        {"key": "memory_id", "match": {"value": "msg_abc"}}
    ]
}
order_by: "created_at"
```

### 性能优化

1. **批量插入** - 累积多条记录后批量写入
2. **索引优化** - 为 memory_id, created_at 创建索引
3. **分页查询** - 限制返回数量，避免大结果集

---

## 📊 性能对比

| 指标 | Redis List | Qdrant |
|------|-----------|--------|
| **写入延迟** | ~1ms | ~5ms |
| **查询延迟** | ~2ms | ~10ms |
| **存储开销** | 低 | 中（有向量） |
| **查询能力** | 简单 | 强大 |
| **扩展性** | 低 | 高 |

---

## 🎯 结论

### 短期（当前）
✅ **保持 Redis 实现** - 简单、快速、够用

### 中期（1-2 周）
🔄 **迁移到 Qdrant** - 复用基础设施，功能更强

### 长期（1-2 月）
🚀 **增强功能** - 语义搜索、历史分析、趋势预测

---

## 🔧 立即行动

### 选项 A: 保持现状（推荐）
- ✅ Redis 实现已完成
- ✅ 功能满足需求
- ✅ 性能足够好
- ⏸️ 等待实际需求再优化

### 选项 B: 立即迁移到 Qdrant（1 小时）
- 🔄 创建 QdrantHistoryStorage
- 🔄 实现 HistoryStorage trait
- 🔄 配置切换
- ✅ 复用基础设施

**你的选择？**

---

## 📝 代码示例

### Qdrant 实现预览

```rust
#[async_trait]
impl HistoryStorage for QdrantHistoryStorage {
    async fn add_entry(&self, entry: MemoryHistoryEntry) -> Result<(), AppError> {
        let point = PointStruct {
            id: Some(entry.id.clone().into()),
            vectors: Some(vec![0.0; 384].into()), // 空向量
            payload: serde_json::json!({
                "memory_id": entry.memory_id,
                "old_content": entry.old_content,
                "new_content": entry.new_content,
                "event_type": format!("{:?}", entry.event_type),
                "created_at": entry.created_at.timestamp(),
                "actor_id": entry.actor_id,
            }).try_into()?,
        };
        
        self.client.upsert_points(&self.collection_name, vec![point]).await?;
        Ok(())
    }
    
    async fn get_history(&self, memory_id: &str) -> Result<Vec<MemoryHistoryEntry>, AppError> {
        let filter = Filter {
            must: vec![Condition {
                field: "memory_id".to_string(),
                r#match: Some(Match::Keyword(memory_id.to_string())),
                ..Default::default()
            }],
            ..Default::default()
        };
        
        let results = self.client.scroll(&ScrollPoints {
            collection_name: self.collection_name.clone(),
            filter: Some(filter),
            limit: Some(100),
            order_by: Some(OrderBy {
                key: "created_at".to_string(),
                direction: Some(Direction::Desc as i32),
            }),
            ..Default::default()
        }).await?;
        
        // 转换为 MemoryHistoryEntry
        Ok(results.into_iter().map(|p| /* ... */).collect())
    }
}
```

---

**总结**: Qdrant 是更好的长期方案，但 Redis 当前够用。建议先保持现状，等有实际需求再迁移。
