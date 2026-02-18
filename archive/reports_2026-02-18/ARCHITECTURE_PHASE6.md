# Phase 6 架构设计文档

**版本**: v2.0  
**创建时间**: 2026-02-17  
**更新时间**: 2026-02-17  
**状态**: 📝 设计中

---

## 📋 目录

- [系统架构](#系统架构)
- [新增组件](#新增组件)
- [数据流](#数据流)
- [技术栈](#技术栈)
- [接口设计](#接口设计)
- [数据模型](#数据模型)
- [安全设计](#安全设计)

---

## 系统架构

### 整体架构图 (Phase 6)

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Layer                            │
│  ┌──────────┬──────────┬──────────┬──────────┬──────────┐     │
│  │  cURL    │  Python  │   Web    │  Mobile  │  Cursor  │     │
│  └────┬─────┴────┬─────┴────┬─────┴────┬─────┴────┬─────┘     │
└───────┼──────────┼──────────┼──────────┼──────────┼───────────┘
        │          │          │          │          │
        └──────────┴──────────┴──────────┴──────────┘
                      │
        ┌─────────────▼─────────────┐
        │      HTTP/HTTPS           │
        │   (REST API + SSE)        │
        │   Bearer Token Auth       │ ← 新增认证
        └─────────────┬─────────────┘
                      │
┌─────────────────────▼─────────────────────────────────────────┐
│                    Gateway Layer (Enhanced)                   │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Axum HTTP Server                          │  │
│  │  ┌──────────┬──────────┬──────────┬──────────────┐    │  │
│  │  │  Routes  │Middleware│  Error   │  Metrics     │    │  │
│  │  └──────────┴──────────┴──────────┴──────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Auth Middleware (新增)                    │  │
│  │  ┌──────────┬──────────┬──────────┬──────────────┐    │  │
│  │  │ API Key  │  Quota   │  Tenant  │  Usage Log   │    │  │
│  │  │  Verify  │  Check   │  Isolate │  Track       │    │  │
│  │  └──────────┴──────────┴──────────┴──────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              3-Tier LLM Router (Enhanced)              │  │
│  │  ┌──────────┬──────────┬──────────┬──────────────┐    │  │
│  │  │  Tier 1  │  Tier 2  │  Tier 3  │  Summarize   │    │  │
│  │  │ (Simple) │ (Medium) │ (Complex)│  Extract     │    │  │
│  │  └──────────┴──────────┴──────────┴──────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│                     Core Layer (Enhanced)                     │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Business Logic                            │  │
│  │  ┌──────────────────┬──────────────────────────────┐  │  │
│  │  │  Config Manager  │   Health Monitor             │  │  │
│  │  │  (Hot Reload)    │   (Real-time Check)          │  │  │
│  │  └──────────────────┴──────────────────────────────┘  │  │
│  │                                                        │  │
│  │  ┌──────────────────────────────────────────────────┐ │  │
│  │  │          Memory Manager (Enhanced)               │ │  │
│  │  │  ┌──────────┬──────────┬──────────────────────┐ │ │  │
│  │  │  │  Short   │   Mid    │    Long              │ │ │  │
│  │  │  │  Term    │   Term   │    Term              │ │ │  │
│  │  │  │  (Redis) │ (Qdrant) │  (Qdrant)            │ │ │  │
│  │  │  └──────────┴──────────┴──────────────────────┘ │ │  │
│  │  │  ┌──────────────────────────────────────────┐   │ │  │
│  │  │  │  Embedding Provider (新增)               │   │ │  │
│  │  │  │  ┌────────┬────────┬────────────────┐   │   │ │  │
│  │  │  │  │ ONNX   │ OpenAI │  LRU Cache     │   │   │ │  │
│  │  │  │  │ BGE-M3 │Fallback│  (1000 items)  │   │   │ │  │
│  │  │  │  └────────┴────────┴────────────────┘   │   │ │  │
│  │  │  └──────────────────────────────────────────┘   │ │  │
│  │  └──────────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────┘  │
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│                    Ports Layer (Interfaces)                   │
│  ┌──────────────────┬──────────────────┬──────────────────┐  │
│  │   LlmAdapter     │   MemoryStorage  │ EmbeddingProvider│  │
│  │     (trait)      │      (trait)     │     (trait)      │  │
│  │  + summarize()   │                  │  + embed()       │  │
│  │  + extract()     │                  │  + embed_batch() │  │
│  └──────────────────┴──────────────────┴──────────────────┘  │
└───────────────────────┬───────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ↓               ↓               ↓
   ┌────────┐      ┌─────────┐    ┌──────────┐
   │ Redis  │      │ Qdrant  │    │ Postgres │ ← 新增
   │  STM   │      │ MTM/LTM │    │ Metadata │
   │ Stream │ ←新增│ Vector  │    │ Auth/Stat│
   └────┬───┘      └─────────┘    └──────────┘
        │
        ↓
┌───────────────────────────────────────────────────────────────┐
│              Message Queue (Redis Stream) (新增)              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Task Queue: consolidate | extract | export           │  │
│  └────────────────────────────────────────────────────────┘  │
└───────────────────────┬───────────────────────────────────────┘
                        │
                        ↓
┌───────────────────────────────────────────────────────────────┐
│              Worker Service (Enhanced)                        │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Task Consumer                             │  │
│  │  ┌──────────┬──────────┬──────────┬──────────────┐    │  │
│  │  │Consolidate│Summarize │  Extract │  Export      │    │  │
│  │  │   Task   │   Task   │   Task   │   Task       │    │  │
│  │  └──────────┴──────────┴──────────┴──────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
```

---

## 新增组件

### 1. memoryos-embedding (新增 Crate)

**职责**: 提供统一的 Embedding 接口，支持本地和远程模型

**结构**:
```
crates/memoryos-embedding/
├── src/
│   ├── lib.rs              # 导出接口
│   ├── provider.rs         # EmbeddingProvider trait
│   ├── onnx.rs             # ONNX 实现
│   ├── openai.rs           # OpenAI 实现
│   ├── cache.rs            # LRU 缓存
│   ├── models.rs           # 模型管理
│   └── tokenizer.rs        # 分词器
└── models/                 # 模型文件
    └── bge-m3/
        ├── model.onnx
        └── tokenizer.json
```

**核心接口**:
```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
    fn dimension(&self) -> usize;
    fn name(&self) -> &str;
}
```

---

### 2. memoryos-tasks (新增 Crate)

**职责**: 异步任务队列，处理耗时操作

**结构**:
```
crates/memoryos-tasks/
├── src/
│   ├── lib.rs              # 导出接口
│   ├── queue.rs            # Redis Stream 队列
│   ├── producer.rs         # 任务生产者
│   ├── consumer.rs         # 任务消费者
│   ├── types.rs            # 任务类型定义
│   └── handlers/           # 任务处理器
│       ├── consolidate.rs
│       ├── extract.rs
│       └── export.rs
```

**任务类型**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    ConsolidateMemory { user_id: String, message_count: usize },
    ExtractProfile { user_id: String, messages: Vec<Message> },
    ExportKnowledge { user_id: String, format: ExportFormat },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub task_type: TaskType,
    pub priority: u8,
    pub retry_count: u8,
    pub created_at: DateTime<Utc>,
}
```

---

### 3. memoryos-auth (新增 Crate)

**职责**: 认证鉴权和配额管理

**结构**:
```
crates/memoryos-auth/
├── src/
│   ├── lib.rs              # 导出接口
│   ├── api_key.rs          # API Key 管理
│   ├── middleware.rs       # 认证中间件
│   ├── quota.rs            # 配额限制
│   ├── tenant.rs           # 多租户
│   └── models.rs           # 数据模型
```

**数据模型**:
```rust
#[derive(Debug, Clone)]
pub struct ApiKey {
    pub key: String,           // sk_live_xxx
    pub user_id: String,
    pub tenant_id: String,
    pub permissions: Vec<Permission>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum Permission {
    ChatRead,
    ChatWrite,
    MemoryRead,
    MemoryWrite,
    Admin,
}

#[derive(Debug, Clone)]
pub struct Quota {
    pub tenant_id: String,
    pub daily_requests: u64,
    pub daily_tokens: u64,
    pub concurrent_requests: u32,
}
```

---

### 4. memoryos-analytics (新增 Crate)

**职责**: 使用量统计和分析

**结构**:
```
crates/memoryos-analytics/
├── src/
│   ├── lib.rs              # 导出接口
│   ├── tracker.rs          # 请求追踪
│   ├── aggregator.rs       # 数据聚合
│   ├── reporter.rs         # 报表生成
│   └── models.rs           # 数据模型
```

**数据模型**:
```rust
#[derive(Debug, Clone)]
pub struct RequestLog {
    pub request_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub latency_ms: u64,
    pub tokens_used: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UsageSummary {
    pub tenant_id: String,
    pub period: String,        // "2026-02-17"
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub breakdown: HashMap<String, u64>,
}
```

---

## 数据流

### 1. 聊天请求流程 (带认证)

```
Client
  │
  ├─ POST /v1/chat/completions
  │  Headers: Authorization: Bearer sk_live_xxx
  │  Body: { model, messages, stream }
  │
  ↓
Gateway
  │
  ├─ Auth Middleware
  │  ├─ 验证 API Key
  │  ├─ 检查权限
  │  ├─ 检查配额
  │  └─ 记录请求日志
  │
  ├─ LLM Router
  │  ├─ 分类请求复杂度
  │  └─ 选择 LLM Tier
  │
  ├─ Memory Manager
  │  ├─ 检索上下文 (STM + MTM + LTM)
  │  ├─ 注入到 Prompt
  │  └─ 调用 LLM
  │
  ├─ LLM Adapter
  │  ├─ 调用上游 LLM
  │  └─ 返回响应
  │
  ├─ Memory Manager
  │  ├─ 保存对话到 STM
  │  ├─ 检查是否需要 Consolidation
  │  └─ 如果需要，发送任务到队列
  │
  ↓
Response
  │
  └─ 返回给 Client
```

---

### 2. 异步任务流程

```
Gateway
  │
  ├─ 检测到 STM 满 (20 条消息)
  │
  ├─ 创建 ConsolidateMemory 任务
  │  Task {
  │    task_id: "uuid",
  │    task_type: ConsolidateMemory,
  │    user_id: "user_123",
  │    priority: 1
  │  }
  │
  ├─ 发送到 Redis Stream
  │  XADD memoryos:tasks * task_data
  │
  ↓
Redis Stream Queue
  │
  ↓
Worker
  │
  ├─ 消费任务 (XREAD)
  │
  ├─ 处理任务
  │  ├─ 获取 STM 消息
  │  ├─ 调用 LLM 总结
  │  ├─ 生成 Embedding
  │  ├─ 存储到 Qdrant MTM
  │  └─ 清理 STM
  │
  ├─ 标记任务完成 (XACK)
  │
  └─ 如果失败，重试 (最多 3 次)
```

---

### 3. Embedding 生成流程

```
Memory Manager
  │
  ├─ 需要生成 Embedding
  │  text = "用户对话内容"
  │
  ↓
Embedding Provider
  │
  ├─ 检查缓存
  │  cache.get(text)
  │  ├─ 命中 → 返回缓存
  │  └─ 未命中 → 继续
  │
  ├─ 尝试主 Provider (ONNX)
  │  ├─ 分词 (Tokenizer)
  │  ├─ 推理 (ONNX Runtime)
  │  └─ 返回 Vec<f32>
  │
  ├─ 如果失败，尝试 Fallback (OpenAI)
  │  ├─ 调用 OpenAI API
  │  └─ 返回 Vec<f32>
  │
  ├─ 缓存结果
  │  cache.set(text, embedding)
  │
  ↓
返回 Embedding
```

---

## 技术栈

### 新增依赖

```toml
[workspace.dependencies]
# ONNX Runtime
ort = "2.0"
tokenizers = "0.15"
ndarray = "0.15"

# LRU 缓存
lru = "0.12"

# 消息队列
redis = { version = "0.32", features = ["streams", "tokio-comp"] }

# 数据库
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls", "uuid", "time"] }

# 认证
jsonwebtoken = "9.2"
argon2 = "0.5"
sha2 = "0.10"

# 序列化
bincode = "1.3"
```

---

## 接口设计

### 1. LLM 增强接口

```rust
// crates/memoryos-ports/src/llm.rs
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    // 现有方法
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<Vec<ChatStreamChunk>, AppError>;
    
    // 新增方法
    async fn summarize(
        &self,
        messages: &[Message],
        max_length: Option<usize>,
    ) -> Result<String, AppError>;
    
    async fn extract_profile(
        &self,
        messages: &[Message],
    ) -> Result<ExtractedProfile, AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedProfile {
    pub traits: Vec<String>,
    pub preferences: Vec<String>,
    pub background: Option<String>,
    pub knowledge: Vec<String>,
    pub confidence: f32,
}
```

---

### 2. Embedding 接口

```rust
// crates/memoryos-embedding/src/provider.rs
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
    fn dimension(&self) -> usize;
    fn name(&self) -> &str;
}
```

---

### 3. 任务队列接口

```rust
// crates/memoryos-tasks/src/queue.rs
#[async_trait]
pub trait TaskQueue: Send + Sync {
    async fn enqueue(&self, task: Task) -> Result<String, TaskError>;
    async fn dequeue(&self, timeout_ms: u64) -> Result<Option<Task>, TaskError>;
    async fn ack(&self, task_id: &str) -> Result<(), TaskError>;
    async fn nack(&self, task_id: &str) -> Result<(), TaskError>;
}
```

---

### 4. 认证接口

```rust
// crates/memoryos-auth/src/api_key.rs
#[async_trait]
pub trait ApiKeyManager: Send + Sync {
    async fn create_key(&self, user_id: &str, tenant_id: &str) -> Result<ApiKey, AuthError>;
    async fn verify_key(&self, key: &str) -> Result<ApiKey, AuthError>;
    async fn revoke_key(&self, key: &str) -> Result<(), AuthError>;
    async fn list_keys(&self, user_id: &str) -> Result<Vec<ApiKey>, AuthError>;
}
```

---

## 数据模型

### Postgres Schema

```sql
-- API Keys 表
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(64) UNIQUE NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    permissions TEXT[] NOT NULL,
    expires_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_key ON api_keys(key);
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_tenant_id ON api_keys(tenant_id);

-- Quotas 表
CREATE TABLE quotas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) UNIQUE NOT NULL,
    daily_requests BIGINT NOT NULL DEFAULT 10000,
    daily_tokens BIGINT NOT NULL DEFAULT 1000000,
    concurrent_requests INT NOT NULL DEFAULT 100,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Usage 表
CREATE TABLE usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) NOT NULL,
    date DATE NOT NULL,
    requests_used BIGINT NOT NULL DEFAULT 0,
    tokens_used BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, date)
);

CREATE INDEX idx_usage_tenant_date ON usage(tenant_id, date);

-- Request Logs 表 (时序数据)
CREATE TABLE request_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id VARCHAR(64) NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    endpoint VARCHAR(255) NOT NULL,
    method VARCHAR(10) NOT NULL,
    status_code INT NOT NULL,
    latency_ms BIGINT NOT NULL,
    tokens_used BIGINT NOT NULL DEFAULT 0,
    timestamp TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_request_logs_tenant_timestamp ON request_logs(tenant_id, timestamp DESC);
CREATE INDEX idx_request_logs_timestamp ON request_logs(timestamp DESC);
```

---

## 安全设计

### 1. API Key 生成

```rust
// 格式: sk_{env}_{random}
// 示例: sk_live_1a2b3c4d5e6f7g8h9i0j

pub fn generate_api_key(env: &str) -> String {
    let random = generate_random_string(32);
    format!("sk_{}_{}", env, random)
}

fn generate_random_string(len: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
```

### 2. 认证流程

```rust
// 1. 提取 Bearer Token
let auth_header = request.headers().get("Authorization");
let token = extract_bearer_token(auth_header)?;

// 2. 验证 API Key
let api_key = api_key_manager.verify_key(&token).await?;

// 3. 检查权限
if !api_key.has_permission(Permission::ChatWrite) {
    return Err(AuthError::PermissionDenied);
}

// 4. 检查配额
let quota = quota_manager.check_quota(&api_key.tenant_id).await?;
if quota.is_exceeded() {
    return Err(AuthError::QuotaExceeded);
}

// 5. 记录请求
analytics.track_request(&api_key.tenant_id, &request).await?;
```

### 3. 数据隔离

```rust
// 所有查询都带 tenant_id
async fn get_user_memory(user_id: &str, tenant_id: &str) -> Result<Memory, AppError> {
    sqlx::query_as!(
        Memory,
        "SELECT * FROM memories WHERE user_id = $1 AND tenant_id = $2",
        user_id,
        tenant_id
    )
    .fetch_one(&pool)
    .await
}
```

---

## 性能优化

### 1. Embedding 缓存

```rust
use lru::LruCache;

pub struct EmbeddingCache {
    cache: Arc<RwLock<LruCache<String, Vec<f32>>>>,
}

impl EmbeddingCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
        }
    }
    
    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        self.cache.write().await.get(text).cloned()
    }
    
    pub async fn put(&self, text: String, embedding: Vec<f32>) {
        self.cache.write().await.put(text, embedding);
    }
}
```

### 2. 连接池配置

```toml
[redis]
pool_size = 100
pool_timeout_ms = 5000
connection_timeout_ms = 3000

[qdrant]
pool_size = 50
timeout_ms = 3000

[postgres]
max_connections = 50
min_connections = 10
acquire_timeout_ms = 5000

[http_client]
pool_max_idle_per_host = 50
timeout_ms = 10000
```

---

## 监控和可观测性

### 1. 关键指标

```rust
// Prometheus 指标
pub struct Metrics {
    // 请求指标
    pub requests_total: Counter,
    pub requests_duration: Histogram,
    pub requests_in_flight: Gauge,
    
    // 认证指标
    pub auth_success: Counter,
    pub auth_failure: Counter,
    pub quota_exceeded: Counter,
    
    // Embedding 指标
    pub embedding_cache_hits: Counter,
    pub embedding_cache_misses: Counter,
    pub embedding_duration: Histogram,
    
    // 任务指标
    pub tasks_enqueued: Counter,
    pub tasks_processed: Counter,
    pub tasks_failed: Counter,
    pub task_duration: Histogram,
}
```

### 2. 日志格式

```json
{
  "timestamp": "2026-02-17T19:00:00Z",
  "level": "INFO",
  "target": "memoryos_gateway",
  "message": "Request processed",
  "request_id": "req_123",
  "tenant_id": "tenant_456",
  "user_id": "user_789",
  "endpoint": "/v1/chat/completions",
  "method": "POST",
  "status": 200,
  "latency_ms": 150,
  "tokens_used": 500
}
```

---

**Phase 6 架构设计文档 - 完成！** 🏗️
