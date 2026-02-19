# 向量数据库支持

MemoryOS-Rust 支持三种向量数据库，用户可以根据需求选择：

## 支持的向量数据库

### 1. Qdrant (默认，完整实现) ✅

**状态**: 生产可用

**特性**:
- 完整的 VectorStorage 实现
- 支持中期记忆段存储和检索
- 支持长期记忆存储和检索
- 支持 Fencing Token (防止并发冲突)
- 高性能向量搜索

**配置示例**:
```toml
[vector_storage]
type = "qdrant"
url = "http://localhost:6334"
segment_collection = "mid_term_segments"
longterm_collection = "long_term_memory"
```

**使用示例**:
```rust
use memoryos_adapters::memory::QdrantStorage;

let storage = QdrantStorage::new(
    "http://localhost:6334".to_string(),
    "mid_term_segments".to_string(),
    "long_term_memory".to_string(),
).await?;
```

---

### 2. Chroma (完整实现) ✅

**状态**: 生产可用

**特性**:
- 完整的 VectorStorage 实现
- 支持中期记忆段存储和检索
- 支持长期记忆存储和检索
- 轻量级，易于部署
- 支持 REST API

**配置示例**:
```toml
[vector_storage]
type = "chroma"
base_url = "http://localhost:8000"
segment_collection = "mid_term_segments"
longterm_collection = "long_term_memory"
```

**使用示例**:
```rust
use memoryos_adapters::memory::ChromaStorage;

let storage = ChromaStorage::new(
    "http://localhost:8000".to_string(),
    "mid_term_segments".to_string(),
    "long_term_memory".to_string(),
).await?;
```

**部署 Chroma**:
```bash
# Docker 部署
docker pull chromadb/chroma
docker run -p 8000:8000 chromadb/chroma

# 或使用 pip 安装
pip install chromadb
chroma run --host 0.0.0.0 --port 8000
```

---

### 3. Pinecone (完整实现) ✅

**状态**: 生产可用

**特性**:
- 完整的 VectorStorage 实现
- 支持中期记忆段存储和检索
- 支持长期记忆存储和检索
- 云托管，无需自建
- 高可用性和扩展性

**配置示例**:
```toml
[vector_storage]
type = "pinecone"
api_key = "your-api-key"
environment = "us-west1-gcp"
segment_index = "memoryos-segments"
longterm_index = "memoryos-longterm"
```

**使用示例**:
```rust
use memoryos_adapters::memory::PineconeStorage;

let storage = PineconeStorage::new(
    "your-api-key".to_string(),
    "us-west1-gcp".to_string(),
    "memoryos-segments".to_string(),
    "memoryos-longterm".to_string(),
);
```

**创建 Pinecone 索引**:
```bash
# 使用 Pinecone CLI 或 Web 控制台创建索引
# 维度: 根据你的 embedding 模型 (例如 384, 768, 1536)
# Metric: cosine
```

---

## 对比表

| 特性 | Qdrant | Chroma | Pinecone |
|------|--------|--------|----------|
| **部署方式** | 自托管 | 自托管 | 云托管 |
| **性能** | 高 | 中 | 高 |
| **扩展性** | 高 | 中 | 极高 |
| **成本** | 免费 | 免费 | 付费 |
| **易用性** | 中 | 高 | 高 |
| **生产就绪** | ✅ | ✅ | ✅ |
| **Fencing Token** | ✅ | ❌ | ❌ |

---

## 选择建议

### 选择 Qdrant 如果:
- 需要自托管解决方案
- 需要高性能和低延迟
- 需要 Fencing Token 防止并发冲突
- 有 Kubernetes 集群

### 选择 Chroma 如果:
- 需要轻量级解决方案
- 快速原型开发
- 本地开发和测试
- 预算有限

### 选择 Pinecone 如果:
- 不想管理基础设施
- 需要全球分布式部署
- 需要自动扩展
- 有预算支持云服务

---

## 统一接口

所有三个向量数据库都实现了相同的 `VectorStorage` trait:

```rust
#[async_trait]
pub trait VectorStorage: Send + Sync {
    /// 存储中期记忆段
    async fn store_segment(&self, segment: MidTermSegment) -> Result<(), AppError>;
    
    /// 搜索相似的中期记忆段
    async fn search_segments(
        &self,
        user_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<MidTermSegment>, AppError>;
    
    /// 存储长期记忆
    async fn store_long_term(&self, memory: LongTermMemory) -> Result<(), AppError>;
    
    /// 获取长期记忆
    async fn get_long_term(&self, user_id: &str) -> Result<Option<LongTermMemory>, AppError>;
}
```

这意味着你可以轻松切换向量数据库，无需修改业务逻辑代码。

---

## 迁移指南

### 从 Qdrant 迁移到 Chroma:

1. 部署 Chroma 服务
2. 更新配置文件
3. 运行数据迁移脚本 (TODO)
4. 重启服务

### 从 Chroma 迁移到 Pinecone:

1. 创建 Pinecone 账号和索引
2. 更新配置文件
3. 运行数据迁移脚本 (TODO)
4. 重启服务

---

## 性能基准 (TODO)

待补充各向量数据库的性能测试结果。

---

## 常见问题

### Q: 可以同时使用多个向量数据库吗？
A: 目前不支持，但可以通过配置文件切换。

### Q: 如何备份向量数据？
A: 
- Qdrant: 使用快照功能
- Chroma: 备份数据目录
- Pinecone: 使用导出 API

### Q: 向量维度如何确定？
A: 取决于你使用的 embedding 模型:
- OpenAI text-embedding-ada-002: 1536
- BAAI/bge-m3: 1024
- all-MiniLM-L6-v2: 384

---

## 贡献

欢迎贡献更多向量数据库适配器！

**待支持的向量数据库**:
- Milvus
- Weaviate
- Vespa
- pgvector (PostgreSQL)

---

## 更新日志

### 2026-02-19
- ✅ 完善 Chroma 适配器实现
- ✅ 完善 Pinecone 适配器实现
- ✅ 三个向量数据库并存，用户可选

### 2026-02-18
- ✅ Qdrant 适配器完整实现
- ⚠️ Chroma 和 Pinecone 仅空实现

---

## 相关文档

- [ARCHITECTURE.md](./ARCHITECTURE.md) - 系统架构
- [NATS_ALTERNATIVE.md](./NATS_ALTERNATIVE.md) - 短期存储选项
- [DEFENSE_SYSTEM.md](./DEFENSE_SYSTEM.md) - IP 防御系统
