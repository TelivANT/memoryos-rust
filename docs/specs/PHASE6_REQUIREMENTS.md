# Phase 6: 功能完善与商业化准备

**版本**: v1.0  
**创建时间**: 2026-02-17  
**预计周期**: 2-3 周  
**优先级**: P0（核心功能完善）

---

## 📋 目录

- [概述](#概述)
- [目标](#目标)
- [需求清单](#需求清单)
- [技术方案](#技术方案)
- [验收标准](#验收标准)
- [时间规划](#时间规划)

---

## 概述

Phase 6 聚焦于**功能完整度**和**商业化准备**，将 Phase 1-5 的基础设施转化为可商用的产品。

### 核心问题
1. **功能不完整**：LLM 总结、Profile 提取都是 mock
2. **性能未验证**：缺少压测数据
3. **商业化缺失**：无认证、计费、多租户

### 解决方案
- 实现真正的 LLM 调用（总结、提取）
- 添加本地 Embedding 支持（ONNX）
- 实现 Worker 异步任务队列
- 添加 API Key 认证和配额限制
- 完善性能测试和优化

---

## 目标

### 功能目标
- ✅ 真实的 LLM 总结和 Profile 提取
- ✅ 本地 Embedding 模型支持
- ✅ Worker 异步任务队列
- ✅ API Key 认证和鉴权
- ✅ 配额限制和计费准备

### 性能目标
- ✅ 压测验证 10,000+ QPS
- ✅ P99 延迟 < 200ms
- ✅ Embedding 缓存命中率 > 80%

### 商业目标
- ✅ 支持多租户隔离
- ✅ API Key 管理界面
- ✅ 使用量统计和计费数据

---

## 需求清单

### 6.1 功能完善 (P0)

#### 6.1.1 真实 LLM 总结
**当前问题**:
```rust
// 现在：直接拼接
let summary = messages.iter()
    .map(|m| format!("{}: {}", m.role, m.content))
    .join("\n");
```

**目标**:
```rust
// 改为：调用 LLM 生成摘要
let summary = self.llm.summarize(messages).await?;
```

**需求**:
- [ ] 实现 `LlmAdapter::summarize()` 方法
- [ ] 支持自定义总结 prompt
- [ ] 支持流式总结（可选）
- [ ] 添加总结质量评估

**验收标准**:
- 总结长度 < 原文 50%
- 保留关键信息（人名、时间、事件）
- 支持中英文

---

#### 6.1.2 真实 Profile 提取
**当前问题**:
```rust
// 现在：简单规则匹配
if text.contains("i like") {
    preferences.push(extract_after("i like"));
}
```

**目标**:
```rust
// 改为：LLM 结构化提取
let profile = self.llm.extract_profile(messages).await?;
```

**需求**:
- [ ] 实现 `LlmAdapter::extract_profile()` 方法
- [ ] 使用 JSON Schema 约束输出格式
- [ ] 支持增量更新（合并新旧 Profile）
- [ ] 添加置信度评分

**输出格式**:
```json
{
  "traits": ["friendly", "technical"],
  "preferences": ["Rust", "open source"],
  "background": "Software engineer with 5 years experience",
  "knowledge": ["distributed systems", "AI/ML"],
  "confidence": 0.85
}
```

**验收标准**:
- 提取准确率 > 90%（基于评估数据集）
- 支持多轮对话增量更新
- 处理时间 < 2s

---

#### 6.1.3 本地 Embedding 模型
**当前问题**:
- 依赖 OpenAI API（成本高、延迟高）
- 无法离线使用

**目标**:
- 支持本地 ONNX 模型
- 支持多种 Embedding 模型
- 自动 fallback 到 OpenAI

**需求**:
- [ ] 集成 `ort` (ONNX Runtime) crate
- [ ] 支持 BGE-M3 模型（中英文）
- [ ] 支持 all-MiniLM-L6-v2（英文）
- [ ] 实现模型自动下载和缓存
- [ ] 添加 Embedding 质量对比测试

**配置**:
```toml
[embedding]
provider = "local"  # local | openai
model = "BAAI/bge-m3"
model_path = "./models/bge-m3.onnx"
fallback_to_openai = true
```

**验收标准**:
- 本地 Embedding 延迟 < 50ms
- 向量相似度与 OpenAI 相关性 > 0.85
- 支持批量处理（batch size = 32）

---

#### 6.1.4 Worker 异步任务队列
**当前问题**:
- Worker 服务是空壳
- 所有任务在 Gateway 同步执行

**目标**:
- 实现异步任务队列
- 支持任务重试和死信队列
- 支持任务优先级

**需求**:
- [ ] 选择消息队列（Redis Stream 或 NATS）
- [ ] 实现 Task Producer（Gateway）
- [ ] 实现 Task Consumer（Worker）
- [ ] 支持任务类型：
  - `consolidate_memory` - 合并 STM 到 MTM
  - `extract_profile` - 提取用户画像
  - `export_knowledge` - 导出知识库
- [ ] 添加任务监控和告警

**任务格式**:
```json
{
  "task_id": "uuid",
  "task_type": "consolidate_memory",
  "user_id": "user_123",
  "payload": { "message_count": 20 },
  "priority": 1,
  "retry_count": 0,
  "created_at": "2026-02-17T19:00:00Z"
}
```

**验收标准**:
- 任务处理延迟 < 5s
- 支持至少 1000 tasks/s
- 失败任务自动重试（最多 3 次）

---

### 6.2 性能优化 (P0)

#### 6.2.1 压测验证
**需求**:
- [ ] 编写 `wrk` 压测脚本
- [ ] 编写 `k6` 压测脚本
- [ ] 测试场景：
  - 纯聊天（无记忆）
  - 聊天 + 记忆检索
  - 聊天 + 记忆写入
  - 流式响应
- [ ] 生成压测报告

**目标指标**:
```
场景1: 纯聊天
- QPS: 10,000+
- P99 延迟: < 100ms
- 错误率: < 0.1%

场景2: 聊天 + 记忆
- QPS: 5,000+
- P99 延迟: < 200ms
- 错误率: < 0.5%
```

**验收标准**:
- 完整的压测报告（Markdown + 图表）
- 识别性能瓶颈并优化
- 达到目标 QPS

---

#### 6.2.2 真正的 LRU 缓存
**当前问题**:
```rust
// 现在：达到容量直接清空
if cache.len() >= max_size {
    cache.clear();
}
```

**目标**:
```rust
// 改为：真正的 LRU
use lru::LruCache;
let cache = LruCache::new(max_size);
```

**需求**:
- [ ] 使用 `lru` crate
- [ ] 支持 TTL（过期时间）
- [ ] 添加缓存统计（命中率、驱逐次数）
- [ ] 支持缓存预热

**验收标准**:
- 缓存命中率 > 80%
- LRU 驱逐策略正确
- 内存占用可控

---

#### 6.2.3 连接池优化
**需求**:
- [ ] 配置 Redis 连接池大小
- [ ] 配置 Qdrant 连接池大小
- [ ] 配置 HTTP Client 连接池
- [ ] 添加连接池监控

**配置**:
```toml
[redis]
pool_size = 100
pool_timeout_ms = 5000

[qdrant]
pool_size = 50
timeout_ms = 3000

[http_client]
pool_max_idle_per_host = 50
timeout_ms = 10000
```

**验收标准**:
- 连接池利用率 > 70%
- 无连接泄漏
- 超时配置合理

---

### 6.3 商业化准备 (P1)

#### 6.3.1 API Key 认证
**需求**:
- [ ] 实现 API Key 生成和管理
- [ ] 实现 API Key 验证中间件
- [ ] 支持 Key 权限控制（read/write）
- [ ] 支持 Key 过期时间
- [ ] 添加 Key 使用日志

**数据模型**:
```rust
struct ApiKey {
    key: String,           // sk_live_xxx
    user_id: String,
    tenant_id: String,
    permissions: Vec<Permission>,
    expires_at: Option<DateTime>,
    created_at: DateTime,
}

enum Permission {
    ChatRead,
    ChatWrite,
    MemoryRead,
    MemoryWrite,
    Admin,
}
```

**API**:
```
POST   /v1/keys          - 创建 API Key
GET    /v1/keys          - 列出 API Keys
DELETE /v1/keys/:key_id  - 删除 API Key
```

**验收标准**:
- 支持 Bearer Token 认证
- 无效 Key 返回 401
- 权限不足返回 403

---

#### 6.3.2 配额限制
**需求**:
- [ ] 实现 Token 计数
- [ ] 实现配额检查中间件
- [ ] 支持多种配额类型：
  - 每日请求数
  - 每日 Token 数
  - 并发请求数
- [ ] 配额超限返回 429

**数据模型**:
```rust
struct Quota {
    tenant_id: String,
    daily_requests: u64,
    daily_tokens: u64,
    concurrent_requests: u32,
}

struct Usage {
    tenant_id: String,
    date: Date,
    requests_used: u64,
    tokens_used: u64,
}
```

**验收标准**:
- 配额检查延迟 < 10ms
- 支持配额重置（每日 00:00）
- 提供配额查询 API

---

#### 6.3.3 多租户隔离
**需求**:
- [ ] 添加 `tenant_id` 字段
- [ ] 实现租户数据隔离
- [ ] 实现租户配置隔离
- [ ] 添加租户管理 API

**数据模型**:
```rust
struct Tenant {
    tenant_id: String,
    name: String,
    plan: Plan,  // free | pro | enterprise
    config: TenantConfig,
    created_at: DateTime,
}

struct TenantConfig {
    max_users: u32,
    max_memory_per_user: u64,
    embedding_model: String,
    llm_model: String,
}
```

**验收标准**:
- 租户间数据完全隔离
- 支持租户级配置
- 支持租户级统计

---

#### 6.3.4 使用量统计
**需求**:
- [ ] 实现请求日志记录
- [ ] 实现 Token 计数
- [ ] 实现使用量聚合
- [ ] 提供统计查询 API

**统计维度**:
- 按租户统计
- 按用户统计
- 按 API 端点统计
- 按时间统计（小时/天/月）

**API**:
```
GET /v1/usage/summary?tenant_id=xxx&start_date=xxx&end_date=xxx
```

**响应**:
```json
{
  "tenant_id": "tenant_123",
  "period": "2026-02-17",
  "total_requests": 10000,
  "total_tokens": 500000,
  "total_cost": 5.00,
  "breakdown": {
    "chat": 8000,
    "memory": 2000
  }
}
```

**验收标准**:
- 统计数据准确
- 查询延迟 < 500ms
- 支持导出 CSV

---

## 技术方案

### 架构调整

```
┌─────────────────────────────────────────────────────────┐
│                    Client Layer                         │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│              Gateway (新增认证层)                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Auth Middleware (API Key + 配额检查)            │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Usage Tracking (请求日志 + Token 计数)          │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────┬────────────────────────────────────┘
                     │
        ┌────────────┼────────────┐
        ↓            ↓            ↓
   ┌────────┐  ┌─────────┐  ┌──────────┐
   │ Redis  │  │ Qdrant  │  │ Postgres │ (新增)
   │  STM   │  │ MTM/LTM │  │ Metadata │
   └────────┘  └─────────┘  └──────────┘
        │            │            │
        └────────────┼────────────┘
                     ↓
        ┌────────────────────────┐
        │   Message Queue        │
        │   (Redis Stream/NATS)  │ (新增)
        └────────────┬───────────┘
                     ↓
┌────────────────────▼────────────────────────────────────┐
│              Worker (实现异步任务)                       │
│  ┌──────────┬──────────┬──────────┬──────────┐        │
│  │Consolidate│ Summarize│  Extract │  Export  │        │
│  │   Task   │   Task   │   Task   │   Task   │        │
│  └──────────┴──────────┴──────────┴──────────┘        │
└─────────────────────────────────────────────────────────┘
```

### 新增 Crate

```
crates/
├── memoryos-auth/          # 认证鉴权
│   ├── api_key.rs
│   ├── middleware.rs
│   └── quota.rs
├── memoryos-embedding/     # 本地 Embedding
│   ├── onnx.rs
│   ├── models.rs
│   └── cache.rs
├── memoryos-tasks/         # 异步任务
│   ├── queue.rs
│   ├── worker.rs
│   └── types.rs
└── memoryos-analytics/     # 统计分析
    ├── tracker.rs
    ├── aggregator.rs
    └── reporter.rs
```

---

## 验收标准

### 功能验收
- [ ] 所有单元测试通过
- [ ] 所有集成测试通过
- [ ] 手动测试通过

### 性能验收
- [ ] 压测达到目标 QPS
- [ ] P99 延迟达标
- [ ] 缓存命中率达标

### 商业验收
- [ ] API Key 认证正常工作
- [ ] 配额限制正常工作
- [ ] 使用量统计准确

---

## 时间规划

### Week 1: 功能完善
- Day 1-2: 真实 LLM 总结和 Profile 提取
- Day 3-4: 本地 Embedding 模型集成
- Day 5-7: Worker 异步任务队列

### Week 2: 性能优化
- Day 1-2: 压测脚本和报告
- Day 3-4: LRU 缓存和连接池优化
- Day 5-7: 性能调优和瓶颈解决

### Week 3: 商业化
- Day 1-2: API Key 认证
- Day 3-4: 配额限制和多租户
- Day 5-7: 使用量统计和文档

---

## 风险和依赖

### 技术风险
- ONNX 模型集成可能遇到兼容性问题
- 消息队列选型需要权衡（Redis Stream vs NATS）
- 压测可能暴露未知性能瓶颈

### 依赖
- ONNX Runtime 安装
- Postgres 数据库（新增）
- 消息队列服务（新增）

---

## 下一步

1. **Review 需求文档**：确认优先级和范围
2. **技术选型**：确定消息队列和 Embedding 模型
3. **开始实施**：按照时间规划逐步推进

**准备好了吗？我们可以开始实施！** 🚀
