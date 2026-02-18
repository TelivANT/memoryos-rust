# 架构文档

**版本**: v0.2.0  
**更新时间**: 2026-02-18

---

## 📋 目录

- [系统架构](#系统架构)
- [六边形架构](#六边形架构)
- [核心模块](#核心模块)
- [数据流](#数据流)
- [单机到集群演进](#单机到集群演进)
- [设计决策](#设计决策)

---

## 系统架构

### 整体架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Layer                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │  cURL    │  │  Python  │  │   Web    │  │  Mobile  │       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘       │
└───────┼─────────────┼─────────────┼─────────────┼──────────────┘
        │             │             │             │
        └─────────────┴─────────────┴─────────────┘
                      │
        ┌─────────────▼─────────────┐
        │      HTTP/HTTPS           │
        │   (REST API + SSE)        │
        └─────────────┬─────────────┘
                      │
┌─────────────────────▼─────────────────────────────────────────┐
│                    Gateway Layer                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Axum HTTP Server                          │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐    │  │
│  │  │  Routes  │  │  Middleware │  │  Error Handler │    │  │
│  │  └──────────┘  └──────────┘  └──────────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              3-Tier LLM Router                         │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐            │  │
│  │  │  Tier 1  │  │  Tier 2  │  │  Tier 3  │            │  │
│  │  │ (Simple) │  │ (Medium) │  │ (Complex)│            │  │
│  │  └──────────┘  └──────────┘  └──────────┘            │  │
│  └────────────────────────────────────────────────────────┘  │
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│                    Worker Layer                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │       Redis Stream Consumer Group (chat_log)          │  │
│  │  - consume event                                      │  │
│  │  - idempotency by event_id                            │  │
│  │  - DLQ on failure                                     │  │
│  └────────────────────────────────────────────────────────┘  │
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│                     Core Layer                                │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Business Logic                            │  │
│  │  ┌──────────────────┐  ┌──────────────────────────┐   │  │
│  │  │  Config Manager  │  │   Health Monitor         │   │  │
│  │  │  (Hot Reload)    │  │   (Real-time Check)      │   │  │
│  │  └──────────────────┘  └──────────────────────────┘   │  │
│  │                                                        │  │
│  │  ┌──────────────────────────────────────────────────┐ │  │
│  │  │          Memory Manager                          │ │  │
│  │  │  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │ │  │
│  │  │  │  Short   │  │   Mid    │  │    Long      │  │ │  │
│  │  │  │  Term    │  │   Term   │  │    Term      │  │ │  │
│  │  │  └──────────┘  └──────────┘  └──────────────┘  │ │  │
│  │  └──────────────────────────────────────────────────┘ │  │
│  │                                                        │  │
│  │  ┌──────────────────────────────────────────────────┐ │  │
│  │  │          FAQ System (NEW)                        │ │  │
│  │  │  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │ │  │
│  │  │  │  Heat    │  │  Auto    │  │    Wiki      │  │ │  │
│  │  │  │ Tracker  │  │ Promoter │  │   Exporter   │  │ │  │
│  │  │  └──────────┘  └──────────┘  └──────────────┘  │ │  │
│  │  │  QA → Candidate → FAQ → Wiki Export            │ │  │
│  │  └──────────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────┘  │
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│                    Ports Layer (Interfaces)                   │
│  ┌──────────────────┐              ┌──────────────────────┐  │
│  │   LlmAdapter     │              │   MemoryStorage      │  │
│  │     (trait)      │              │      (trait)         │  │
│  └──────────────────┘              └──────────────────────┘  │
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│                  Adapters Layer (Implementations)             │
│  ┌──────────────────────────────────────────────────────────┐│
│  │              LLM Adapters                                ││
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐   ││
│  │  │ OpenAI  │  │ Gemini  │  │ Claude  │  │ Ollama  │   ││
│  │  ├─────────┤  ├─────────┤  ├─────────┤               ││
│  │  │DeepSeek │  │OpenRouter│ │Azure OA │               ││
│  │  └─────────┘  └─────────┘  └─────────┘               ││
│  └──────────────────────────────────────────────────────────┘│
│                                                               │
│  ┌──────────────────────────────────────────────────────────┐│
│  │            Storage Adapters                              ││
│  │  ┌─────────┐  ┌─────────┐  ┌──────────────────────┐    ││
│  │  │  Redis  │  │ Qdrant  │  │  Noop (Fallback)     │    ││
│  │  └─────────┘  └─────────┘  └──────────────────────┘    ││
│  └──────────────────────────────────────────────────────────┘│
└───────────────────────┬───────────────────────────────────────┘
                        │
┌───────────────────────▼───────────────────────────────────────┐
│                  External Services                            │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐         │
│  │ OpenAI  │  │ Gemini  │  │  Redis  │  │ Qdrant  │         │
│  │Claude/..│  │Azure/.. │  │  Server │  │  Server │         │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘         │
└───────────────────────────────────────────────────────────────┘
```

---

## 六边形架构

### 核心原则

**1. 依赖倒置**:
- Core 不依赖 Adapters
- Adapters 实现 Ports 定义的接口
- 通过依赖注入组装

**2. 端口和适配器**:
- **端口（Ports）**: 定义接口
- **适配器（Adapters）**: 实现接口

**3. 可测试性**:
- Core 可独立测试
- Adapters 可 Mock

### 层次划分

```
┌─────────────────────────────────────────────┐
│              Gateway (HTTP)                 │  ← 入站适配器
│         (memoryos-gateway)                  │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│         Core (Business Logic)               │  ← 核心领域
│          (memoryos-core)                    │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│          Ports (Interfaces)                 │  ← 端口定义
│         (memoryos-ports)                    │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│      Adapters (Implementations)             │  ← 出站适配器
│        (memoryos-adapters)                  │
└─────────────────────────────────────────────┘
```

---

## 核心模块

### 1. Gateway Layer

**职责**:
- HTTP 请求处理
- 路由分发
- 中间件（日志、错误处理）
- SSE 流式响应

**关键组件**:
```rust
// 应用状态
pub struct AppState {
    pub router: LlmRouter,
    pub memory_manager: Arc<dyn MemoryManager>,
    pub health_status: Arc<RwLock<HealthStatus>>,
}

// 路由定义
pub fn create_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/memory/store", post(store_memory))
        .route("/health/status", get(health_status))
}
```

### 2. Router (3-Tier)

**职责**:
- 请求分类（简单/中等/复杂）
- 模型选择
- 负载均衡

**分层策略**:
```rust
pub enum Tier {
    Tier1,  // 简单任务 → gpt-4o-mini / gemini-flash
    Tier2,  // 中等任务 → gpt-4o / claude-sonnet
    Tier3,  // 复杂任务 → o1 / claude-opus
}

impl LlmRouter {
    fn classify_tier(&self, request: &ChatRequest) -> Tier {
        // 基于消息长度、复杂度分类
        if request.messages.len() < 5 {
            Tier::Tier1
        } else if request.messages.len() < 20 {
            Tier::Tier2
        } else {
            Tier::Tier3
        }
    }
}
```

### 3. Memory Manager

**职责**:
- 三层记忆管理
- 记忆存储和检索
- 优雅降级

**架构**:
```
┌─────────────────────────────────────────┐
│         Memory Manager                  │
├─────────────────────────────────────────┤
│  Short-term  │  Mid-term  │  Long-term │
│   (Redis)    │  (Qdrant)  │  (Qdrant)  │
├─────────────────────────────────────────┤
│         Graceful Degradation            │
│  Redis Down → Noop / In-Memory         │
│  Qdrant Down → Noop / In-Memory        │
└─────────────────────────────────────────┘
```

### 4. FAQ System (NEW)

**职责**:
- 自动识别高频问答
- 智能提升为 FAQ
- 定时导出为知识库

**架构**:
```
┌─────────────────────────────────────────────────────┐
│                  FAQ System                         │
├─────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────┐ │
│  │ HeatTracker  │  │AutoPromoter  │  │  Wiki    │ │
│  │              │  │              │  │ Exporter │ │
│  │ • 访问计数   │  │ • 扫描候选   │  │ • 筛选   │ │
│  │ • 热度计算   │  │ • 自动提升   │  │ • 分类   │ │
│  │ • 统计分析   │  │ • 历史记录   │  │ • 导出   │ │
│  └──────────────┘  └──────────────┘  └──────────┘ │
├─────────────────────────────────────────────────────┤
│              Memory Type Flow                       │
│  QA → FaqCandidate → Faq → Wiki Export             │
│  (访问≥10)  (热度≥50)  (30天+)                      │
└─────────────────────────────────────────────────────┘
```

**提升规则**:
```rust
// QA → FaqCandidate
if access_count >= 10 && heat_score >= 50.0 {
    promote_to_candidate();
}

// FaqCandidate → Faq
if access_count >= 20 && heat_score >= 100.0 {
    promote_to_faq();
}

// Faq → Wiki Export
if age_days >= 30 && access_count >= 10 {
    export_to_wiki();
}
```
```

**实现**:
```rust
pub struct DefaultMemoryManager {
    short_term: Arc<dyn MemoryStorage>,
    mid_term: Arc<dyn MemoryStorage>,
    long_term: Arc<dyn MemoryStorage>,
}

// 降级实现
pub struct NoopMemoryManager;

impl MemoryManager for NoopMemoryManager {
    async fn store(&self, _memory: Memory) -> Result<(), AppError> {
        Ok(())  // 不存储，但不报错
    }
}
```

### 4. Health Monitor

**职责**:
- 实时健康检查
- 后端可用性探测
- 降级模式切换

**状态机**:
```
┌──────────────────┐
│ FullyOperational │ ← Redis ✓, Qdrant ✓
└────────┬─────────┘
         │
         │ Redis ✗ or Qdrant ✗
         ▼
┌──────────────────┐
│  DegradedReady   │ ← 部分后端不可用
└────────┬─────────┘
         │
         │ 所有后端 ✗
         ▼
┌──────────────────┐
│     NotReady     │ ← 服务不可用
└──────────────────┘
```

### 5. Worker (Async Memory Pipeline)

**职责**:
- 消费 `chat_log` 事件（Redis Stream）。
- 调用 `MemoryManager` 执行记忆写入。
- 失败事件写入 `chat_log:dlq` 并 ACK 原消息避免阻塞。

**关键语义**:
- 幂等键必须使用 `event_id`。
- 多实例通过 consumer group 横向扩展。
- 同组内一个消息只会分配给一个 consumer。
- gateway 通过 `MEMORYOS_ASYNC_MEMORY_PIPELINE` 控制是否启用异步事件生产。
- 异步发布失败时，gateway 回退同步写入。

### 6. Config Manager

**职责**:
- 配置加载
- 热更新（无需重启）
- 环境变量覆盖

**实现**:
```rust
pub struct ConfigManager {
    config: Arc<ArcSwap<Config>>,
    watcher: RecommendedWatcher,
}

impl ConfigManager {
    pub fn get(&self) -> Arc<Config> {
        self.config.load_full()
    }
    
    fn reload(&self) {
        let new_config = Config::from_file("config.toml")?;
        self.config.store(Arc::new(new_config));
    }
}
```

---

## 数据流

### 1. 聊天请求流程

```
Client
  │
  │ POST /v1/chat/completions
  ▼
Gateway (Axum)
  │
  │ 解析请求
  ▼
Router (3-Tier)
  │
  │ 分类 → Tier
  │ 选择模型
  ▼
LLM Adapter
  │
  │ HTTP 调用
  ▼
External LLM API
  │
  │ 响应
  ▼
Gateway
  │
  │ 格式化
  ▼
Client
```

### 2. 流式响应流程

```
Client
  │
  │ POST /v1/chat/completions (stream=true)
  ▼
Gateway
  │
  │ 调用 router.route_stream()
  ▼
LLM Adapter
  │
  │ HTTP Stream
  ▼
External LLM API
  │
  │ SSE: data: {...}
  │ SSE: data: {...}
  │ SSE: data: [DONE]
  ▼
Gateway
  │
  │ 转发 SSE
  ▼
Client (逐块接收)
```

### 3. 记忆存储流程

```
Client
  │
  │ POST /v1/memory/store
  ▼
Gateway
  │
  │ 解析请求
  ▼
Memory Manager
  │
  ├─→ Short-term (Redis)
  │   └─→ 存储最近对话
  │
  ├─→ Mid-term (Redis)
  │   └─→ 存储会话摘要
  │
  └─→ Long-term (Qdrant)
      └─→ 向量化存储
```

### 4. 降级流程

```
Health Monitor
  │
  │ 定期检查
  ▼
Redis Ping
  │
  ├─→ ✓ 可用
  │
  └─→ ✗ 不可用
      │
      ▼
  切换到 NoopMemoryManager
      │
      ▼
  设置 X-Degraded-Mode: true
```

---

## 单机到集群演进

### 拓扑差异

单机：
- 1 gateway + redis + qdrant（worker 可选，仅异步管道需要）

集群：
- N gateway + M worker + shared redis + shared qdrant + LB/Ingress

### 演进原则

1. 先扩中间件，再扩 worker，最后扩 gateway。
2. gateway/worker 必须连接同一 Redis/Qdrant。
3. worker 扩容仅通过新增 consumer，不改 group 名称。
4. 全链路可观测：`queue depth`、`dlq size`、`health status` 同步监控。

## 设计决策

### 1. 为什么选择六边形架构？

**优点**:
- ✅ 核心业务逻辑与外部依赖解耦
- ✅ 易于测试（可 Mock 外部服务）
- ✅ 易于扩展（添加新 Adapter）
- ✅ 清晰的依赖方向

**权衡**:
- ⚠️ 增加了抽象层
- ⚠️ 需要更多样板代码

### 2. 为什么使用 3-Tier Router？

**优点**:
- ✅ 成本优化（简单任务用便宜模型）
- ✅ 性能优化（简单任务响应快）
- ✅ 灵活性（可动态调整策略）

**权衡**:
- ⚠️ 分类逻辑需要调优
- ⚠️ 可能误判任务复杂度

### 3. 为什么实现优雅降级？

**优点**:
- ✅ 高可用性（部分故障不影响整体）
- ✅ 用户体验（服务持续可用）
- ✅ 运维友好（无需紧急修复）

**实现**:
- NoopMemoryManager（空实现）
- 健康检查标记降级模式
- 响应头通知客户端

### 4. 为什么支持配置热更新？

**优点**:
- ✅ 无需重启（零停机）
- ✅ 快速调整（API Key 轮换）
- ✅ 动态配置（模型切换）

**实现**:
- ArcSwap（原子指针交换）
- 文件监听（notify crate）
- 后台任务（tokio task）

### 5. 为什么使用 Rust？

**优点**:
- ✅ 性能（接近 C/C++）
- ✅ 内存安全（无 GC）
- ✅ 并发安全（类型系统保证）
- ✅ 生态成熟（Tokio, Axum）

**权衡**:
- ⚠️ 学习曲线陡峭
- ⚠️ 编译时间较长

---

## 扩展性

### 添加新的 LLM Provider

**步骤**:
1. 实现 `LlmAdapter` trait
2. 注册到 `LlmRouter`
3. 添加配置项
4. 添加测试

**示例**:
```rust
// 1. 实现 trait
pub struct NewLlmAdapter { /* ... */ }

#[async_trait]
impl LlmAdapter for NewLlmAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        // 实现
    }
}

// 2. 注册
let router = LlmRouter {
    new_llm: Arc::new(NewLlmAdapter::new()),
    // ...
};
```

### 添加新的存储后端

**步骤**:
1. 实现 `MemoryStorage` trait
2. 注册到 `MemoryManager`
3. 添加配置项
4. 添加降级逻辑

### 添加新的 API 端点

**步骤**:
1. 定义路由处理函数
2. 注册到 `create_routes()`
3. 添加测试
4. 更新 API 文档

---

## 性能考虑

### 1. 异步 I/O
- 使用 Tokio 异步运行时
- 非阻塞 HTTP 调用
- 并发处理多个请求

### 2. 连接池
- HTTP 客户端复用连接
- Redis 连接池
- Qdrant 连接池

### 3. 缓存
- 配置缓存（ArcSwap）
- 健康状态缓存
- 可选：响应缓存

### 4. 流式响应
- 减少内存占用
- 降低首字节延迟
- 提升用户体验

---

## 安全考虑

### 1. API Key 管理
- 环境变量存储
- 不记录到日志
- 不返回给客户端

### 2. 错误处理
- 不泄露内部信息
- 统一错误格式
- 记录详细日志

### 3. 输入验证
- 请求参数校验
- 防止注入攻击
- 限制请求大小

---

## 未来规划

### 短期（1-3 个月）
- [ ] 添加认证中间件
- [ ] 实现速率限制
- [ ] 添加 Prometheus 指标
- [ ] OpenAI 真正透传

### 中期（3-6 个月）
- [ ] 支持更多 LLM Provider
- [ ] 实现请求缓存
- [ ] 添加 A/B 测试
- [ ] 实现智能路由

### 长期（6-12 个月）
- [ ] 分布式部署
- [ ] 多租户支持
- [ ] 成本分析仪表板
- [ ] 自动扩缩容

---

**最后更新**: 2026-02-17
