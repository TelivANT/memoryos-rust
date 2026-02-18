# MemoryOS-Rust 完整架构图

**版本**: 0.2.0  
**日期**: 2026-02-17

---

## 🏗️ 系统整体架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Client Layer                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │  cURL    │  │  Python  │  │   Web    │  │  Mobile  │  │   SDK    │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘ │
└───────┼─────────────┼─────────────┼─────────────┼─────────────┼────────┘
        │             │             │             │             │
        └─────────────┴─────────────┴─────────────┴─────────────┘
                                    │
                      ┌─────────────▼─────────────┐
                      │      HTTP/HTTPS           │
                      │   (REST API + SSE)        │
                      └─────────────┬─────────────┘
                                    │
┌───────────────────────────────────▼─────────────────────────────────────┐
│                          Gateway Layer (Axum)                           │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │                      HTTP Server (Axum)                           │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────┐  ┌──────────────┐ │ │
│  │  │  Routes  │  │Middleware│  │Error Handler │  │  CORS/Auth   │ │ │
│  │  └──────────┘  └──────────┘  └──────────────┘  └──────────────┘ │ │
│  └───────────────────────────────────────────────────────────────────┘ │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │                    Application State                              │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │ │
│  │  │Config Manager│  │Health Monitor│  │  Memory Manager      │   │ │
│  │  │(Hot Reload)  │  │(Real-time)   │  │  (Graceful Degrade)  │   │ │
│  │  └──────────────┘  └──────────────┘  └──────────────────────┘   │ │
│  └───────────────────────────────────────────────────────────────────┘ │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │                    3-Tier LLM Router                              │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │ │
│  │  │   Tier 1     │  │   Tier 2     │  │      Tier 3          │   │ │
│  │  │  (Simple)    │  │  (Medium)    │  │    (Complex)         │   │ │
│  │  │ Ollama/Local │  │ GPT-4o-mini  │  │  GPT-4o/Claude-3     │   │ │
│  │  └──────────────┘  └──────────────┘  └──────────────────────┘   │ │
│  └───────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
┌───────────────────▼───┐  ┌────────▼────────┐  ┌──▼──────────────────┐
│   Ports Layer         │  │  Core Layer     │  │  Adapters Layer     │
│  (Interfaces)         │  │  (Domain)       │  │  (Implementations)  │
└───────────────────────┘  └─────────────────┘  └─────────────────────┘
```

---

## 🎯 六边形架构详细图

```
                    ┌─────────────────────────────────────┐
                    │         Gateway (Axum)              │
                    │  ┌──────────────────────────────┐   │
                    │  │  HTTP Routes & Middleware    │   │
                    │  └──────────────────────────────┘   │
                    └──────────────┬──────────────────────┘
                                   │
                    ┌──────────────▼──────────────────────┐
                    │         Ports (Interfaces)          │
                    │  ┌──────────────────────────────┐   │
                    │  │  LlmAdapter                  │   │
                    │  │  MemoryManager               │   │
                    │  │  ShortTermStorage            │   │
                    │  │  VectorStorage               │   │
                    │  │  ConcurrencyControl          │   │
                    │  └──────────────────────────────┘   │
                    └──────────────┬──────────────────────┘
                                   │
        ┌──────────────────────────┼──────────────────────────┐
        │                          │                          │
┌───────▼────────┐      ┌──────────▼──────────┐      ┌───────▼────────┐
│  Core Domain   │      │     Adapters        │      │   External     │
│                │      │                     │      │   Services     │
│ ┌────────────┐ │      │ ┌─────────────────┐│      │ ┌────────────┐ │
│ │ Message    │ │      │ │ OpenAI Adapter  ││      │ │ OpenAI API │ │
│ │ Profile    │ │      │ │ Gemini Adapter  ││      │ │ Gemini API │ │
│ │ Knowledge  │ │      │ │ Claude Adapter  ││      │ │ Claude API │ │
│ │ Segment    │ │      │ │ Ollama Adapter  ││      │ │ Ollama     │ │
│ └────────────┘ │      │ └─────────────────┘│      │ └────────────┘ │
│                │      │                     │      │                │
│ ┌────────────┐ │      │ ┌─────────────────┐│      │ ┌────────────┐ │
│ │ AppError   │ │      │ │ Redis Adapter   ││      │ │ Redis      │ │
│ │ Config     │ │      │ │ Qdrant Adapter  ││      │ │ Qdrant     │ │
│ └────────────┘ │      │ └─────────────────┘│      │ └────────────┘ │
│                │      │                     │      │                │
│ ┌────────────┐ │      │ ┌─────────────────┐│      │                │
│ │ Health     │ │      │ │ Memory Manager  ││      │                │
│ │ Status     │ │      │ │ - Default       ││      │                │
│ └────────────┘ │      │ │ - Degraded      ││      │                │
│                │      │ │ - Noop          ││      │                │
└────────────────┘      │ └─────────────────┘│      └────────────────┘
                        └─────────────────────┘
```

---

## 📊 数据流图

### 1. 聊天请求流程

```
Client
  │
  │ POST /v1/chat/completions
  ▼
Gateway (Axum)
  │
  │ 1. 路由匹配
  │ 2. 中间件处理
  ▼
3-Tier Router
  │
  │ 3. 复杂度分析
  │ 4. 选择 LLM Tier
  ▼
Memory Manager
  │
  ├─► 5. 检索短期记忆 (Redis)
  │     │
  │     ▼
  │   ShortTermStorage
  │     │
  │     ▼
  │   Redis
  │
  ├─► 6. 检索中期记忆 (Qdrant)
  │     │
  │     ▼
  │   VectorStorage
  │     │
  │     ▼
  │   Qdrant (相似度搜索)
  │
  └─► 7. 检索长期记忆 (Qdrant)
        │
        ▼
      VectorStorage
        │
        ▼
      Qdrant (用户画像 + 知识)
        │
        ▼
8. 构建上下文
  │
  ▼
LLM Adapter
  │
  │ 9. 调用 LLM API
  ▼
External LLM (OpenAI/Gemini/Claude...)
  │
  │ 10. 返回响应
  ▼
Memory Manager
  │
  │ 11. 存储新消息到 STM
  │ 12. 检查是否需要合并
  ▼
Gateway
  │
  │ 13. 返回响应
  ▼
Client
```

### 2. 记忆合并流程 (STM → MTM)

```
Memory Manager
  │
  │ 1. 检测 STM 已满
  ▼
Consolidation Process
  │
  ├─► 2. 获取分布式锁
  │     │
  │     ▼
  │   ConcurrencyControl (Fencing Lock)
  │     │
  │     ▼
  │   Redis (SET NX + Lease Renewal)
  │
  ├─► 3. 读取 STM 消息
  │     │
  │     ▼
  │   ShortTermStorage
  │     │
  │     ▼
  │   Redis (LRANGE)
  │
  ├─► 4. 生成 Embedding
  │     │
  │     ▼
  │   Embedding API (OpenAI)
  │
  ├─► 5. 存储到 MTM
  │     │
  │     ▼
  │   VectorStorage
  │     │
  │     ▼
  │   Qdrant (upsert)
  │
  ├─► 6. 清空 STM
  │     │
  │     ▼
  │   ShortTermStorage
  │     │
  │     ▼
  │   Redis (DEL)
  │
  └─► 7. 释放锁
        │
        ▼
      ConcurrencyControl
        │
        ▼
      Redis (DEL)
```

### 3. 用户画像提取流程

```
Memory Manager
  │
  │ 1. 检测 MTM 热度超阈值
  ▼
Profile Extraction
  │
  ├─► 2. 读取 MTM Segment
  │     │
  │     ▼
  │   VectorStorage
  │     │
  │     ▼
  │   Qdrant
  │
  ├─► 3. 应用提取规则
  │     │
  │     ▼
  │   ExtractionPolicy
  │     │
  │     ├─► "i like" → Preference
  │     ├─► "i work as" → Background
  │     └─► "my name is" → Background
  │
  ├─► 4. 更新用户画像
  │     │
  │     ▼
  │   VectorStorage
  │     │
  │     ▼
  │   Qdrant (upsert profile)
  │
  └─► 5. 提取知识
        │
        ▼
      VectorStorage
        │
        ▼
      Qdrant (upsert knowledge)
```

---

## 🔄 配置热更新流程

```
Background Task (tokio::spawn)
  │
  │ 每 5 秒
  ▼
Config Manager
  │
  │ 1. 检查文件修改时间
  ▼
File System
  │
  │ 2. 读取 config.toml
  ▼
Config Manager
  │
  │ 3. 解析配置
  │ 4. 验证配置
  ▼
ArcSwap
  │
  │ 5. 原子更新配置
  ▼
Application State
  │
  │ 6. 所有请求使用新配置
  ▼
Gateway
```

---

## 💚 实时健康检查流程

```
Client
  │
  │ GET /health/ready
  ▼
Gateway
  │
  ▼
Application State
  │
  │ current_health()
  ▼
Health Monitor
  │
  ├─► 1. 探测 Redis
  │     │
  │     │ PING
  │     ▼
  │   Redis
  │     │
  │     ├─► 成功 → "up"
  │     └─► 失败 → "down"
  │
  ├─► 2. 探测 Qdrant
  │     │
  │     │ GET /health
  │     ▼
  │   Qdrant
  │     │
  │     ├─► 成功 → "up"
  │     └─► 失败 → "down"
  │
  └─► 3. 计算模式
        │
        ├─► Redis ✅ + Qdrant ✅ → "ready"
        ├─► Redis ✅ + Qdrant ❌ → "degraded_ready"
        ├─► Redis ❌ + Qdrant ✅ → "degraded_ready"
        └─► Redis ❌ + Qdrant ❌ → "not_ready"
        │
        ▼
      Response
        │
        ├─► "ready" → 200 OK
        ├─► "degraded_ready" → 200 OK + Header
        └─► "not_ready" → 503 Service Unavailable
```

---

## 🛡️ 优雅降级架构

```
                    ┌─────────────────────────┐
                    │   Memory Manager        │
                    │   Selection Logic       │
                    └───────────┬─────────────┘
                                │
                ┌───────────────┼───────────────┐
                │               │               │
        ┌───────▼────────┐  ┌──▼──────────┐  ┌─▼────────────┐
        │ Full Mode      │  │ Degraded    │  │ Noop Mode    │
        │ (Both Ready)   │  │ Mode        │  │ (Both Down)  │
        └───────┬────────┘  │ (One Ready) │  └──┬───────────┘
                │           └──┬──────────┘     │
                │              │                │
    ┌───────────▼──────────────▼────────────────▼───────────┐
    │         DefaultMemoryManager                           │
    │  ┌──────────────┐  ┌──────────────┐                   │
    │  │ Redis ✅     │  │ Qdrant ✅    │                   │
    │  └──────────────┘  └──────────────┘                   │
    │  - 完整功能                                            │
    │  - STM + MTM + LTM                                     │
    └────────────────────────────────────────────────────────┘
                │
    ┌───────────▼──────────────────────────────────────────┐
    │         DegradedMemoryManager                        │
    │  ┌──────────────┐  ┌──────────────┐                 │
    │  │ Redis ✅/❌  │  │ Qdrant ✅/❌ │                 │
    │  └──────────────┘  └──────────────┘                 │
    │  - 部分功能                                          │
    │  - 仅可用的存储                                      │
    └──────────────────────────────────────────────────────┘
                │
    ┌───────────▼──────────────────────────────────────────┐
    │         NoopMemoryManager                            │
    │  ┌──────────────┐  ┌──────────────┐                 │
    │  │ Redis ❌     │  │ Qdrant ❌    │                 │
    │  └──────────────┘  └──────────────┘                 │
    │  - 无记忆功能                                        │
    │  - LLM 仍可用                                        │
    └──────────────────────────────────────────────────────┘
```

---

**更新时间**: 2026-02-17 22:00  
**版本**: 0.2.0
