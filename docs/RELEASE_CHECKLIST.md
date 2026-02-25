# MemoryOS-Rust v1.0.0 Release Checklist

**最后更新**: 2026-02-25
**当前版本**: v1.0.0-rc
**目标**: 所有 P0/P1 修复完成后发布 v1.0.0

---

## 轮训审查流程（六条准则）

每次审查必须按以下顺序执行：

1. **Bug / 功能缺陷** — 代码是否有 panic、unwrap、空实现、死代码
2. **文档同步** — 文档是否与代码一致（版本号、功能描述、架构图）
3. **进度追踪** — 本文件是否反映最新状态
4. **代码-文档偏差** — 文档声称的功能是否真正实现
5. **技术栈一致性** — 是否引入了不必要的依赖（如 SQLx、PostgreSQL 等）
6. **基座复用** — 是否有重复代码可以抽取到共享 crate

---

## 三次审查报告修复 (Round 6 — PR #96)

### P0 — 功能阻塞 / 安全危险

| # | 问题 | 状态 |
|---|------|------|
| 1 | `unsafe { std::mem::zeroed() }` — circuit_breaker.rs UB | ✅ DONE — 重写 `with_circuit_breaker` 返回 `Option<Result<T,E>>`，消除 unsafe |
| 2 | Streaming 路径跳过 PII 脱敏 | ✅ DONE — streaming handler 加入完整 PII sanitization |
| 3 | Streaming 路径跳过路由决策 | ✅ DONE — streaming handler 加入 compliance 路由（敏感内容→local） |
| 4 | Streaming 不是真正的 streaming | ⚠️ 已知限制 — 当前 chat_stream 收集全部 chunks 后 SSE 发送，见 docs/STREAMING.md |
| 5 | Gemini/Claude/Ollama 不支持 streaming | ⚠️ 设计如此 — trait 默认返回 BadRequest，调用方需检查 |
| 6 | "FAQ Content Placeholder" 硬编码兜底 | ✅ DONE — 移除 placeholder，FAQ 无答案时不返回 DirectHit |
| 7 | FAQ Bloom Filter 永远为空 | ✅ DONE — AppState::new() 启动时从 Qdrant 加载 FAQ 数据填充 Bloom filter |

### P1 — 工程质量 / 可靠性问题

| # | 问题 | 状态 |
|---|------|------|
| 8 | Circuit Breaker 未接入 | ✅ DONE — handlers.rs LLM 调用路径接入 circuit breaker |
| 9 | Retry 机制未接入 | ✅ DONE — handlers.rs LLM 调用路径接入 retry_with_backoff |
| 10 | Config Hot-Reload 实际无效 | ✅ DONE — 添加启动警告日志，文档化限制 (CONFIG_HOT_RELOAD_LIMITATION.md) |
| 11 | Rate Limiter 进程级别非分布式 | ✅ DONE — 添加文档注释说明限制和 Redis 替代方案 |
| 12 | Token 计数 len()/4 对 CJK 不准 | ✅ DONE — 区分 CJK 字符和 ASCII，CJK ~1.5 tok/char |
| 13 | generate_simple_embedding 是随机 hash | ✅ DONE — 所有 fallback 路径添加明确 warn 日志 |
| 14 | Embedding fallback 静默降级 | ✅ DONE — 所有 fallback 日志标注 "similarity search degraded" |
| 15 | #[allow(dead_code)] 残留 | ✅ 审计完成 — 剩余 4 处均合理 (worker_monitor, vue parser, pinecone, nats) |
| 16 | metrics 注册 .expect() | ✅ DONE — 添加注释说明 lazy_static + register 是 Prometheus 标准模式 |
| 17 | Consolidation embedding 失败用零向量 | ✅ DONE — 改为 generate_simple_embedding fallback |
| 18 | RBAC 对未知 token 直接放行 | ✅ DONE — admin 路由即使 auth.enabled=false 也要求认证 |

### P2 — 文档 / UX / 维护问题

| # | 问题 | 状态 |
|---|------|------|
| 19 | 7 个多语言 README 可能过时 | ✅ DONE — 每个翻译版本顶部添加"可能落后于英文版"声明 |
| 20 | ARCHITECTURE.md v0.x.0 版本残留 | ✅ DONE — 版本历程表添加"历史记录"说明，移除 section 标题中的版本号 |
| 21 | DESIGN.md 版本标记过时 | ✅ DONE — 移除 section 标题中的版本号，添加历史版本说明 |
| 22 | 60 个文档职责重叠 | ⚠️ 长期优化 — 建议后续合并或建立导航索引 |
| 23 | async_memory_pipeline 硬编码 false | ✅ DONE — 改为读取 MEMORYOS_ASYNC_MEMORY_PIPELINE 环境变量 |
| 24 | OpenAPI spec 缺失 | ✅ 已存在 — docs/openapi.yaml 已在 PR #55-#61 中添加 |
| 25 | X-User-ID / X-Tenant-ID 无验证 | ✅ DONE — 添加格式验证（字母数字+连字符，最长 128 字符） |

### 长期架构 / 技术栈风险

| # | 问题 | 状态 |
|---|------|------|
| 26 | 无集成测试 | ⚠️ 长期 — 需要 Redis + Qdrant 环境 |
| 27 | 无数据迁移策略 | ⚠️ 长期 — 建议 v1.1 实现 |
| 28 | 无请求超时 | ✅ DONE — 所有 10 个 LLM adapter HTTP client 添加 120s 超时 |

---

## 剩余 #[allow(dead_code)] 清单 (4 处，均合理)

| 文件 | 原因 |
|------|------|
| state.rs: current_worker_monitor (1 处) | 异步管道监控，按需启用 |
| adapters/memory/nats.rs (1 处) | NATS client 字段，JetStream 使用 |
| adapters/memory/pinecone.rs (1 处) | Pinecone 响应反序列化结构 |
| parser/vue.rs: SfcSection.tag (1 处) | 内部数据结构，调试用字段 |

---

## 历史审计轮次

### Round 1-4 (PR #57-#61)
详见 git history。修复了初始 20 个审计问题中的 14 个。

### Round 5 (PR #62) — 二次审查报告
- 删除 router.rs 孤儿文件 (181 行)
- 删除 UpstreamClient/NormalizedRequest/NormalizedResponse 死接口
- Admin 数据文件路径统一为 rbac_users.json/tenants.json
- 移除 context_injector 死字段
- StorageConnector trait 加 clone_to_temp 默认实现
- local provider 从硬编码改为读 config
- TODO 注释清理、README TBD 修复、MCP SSE 标注

### Round 6 (PR #96) — 三次深度审查报告 (28 项)
- 消除 `unsafe { std::mem::zeroed() }` UB — 重写为 Option 返回
- Streaming handler 加入 PII 脱敏 + compliance 路由
- FAQ router 移除 placeholder，无答案时不返回 DirectHit
- FAQ Bloom filter 启动时从 Qdrant 加载数据
- Circuit breaker + retry 接入 LLM 调用路径
- Token 计数改进 CJK 支持
- Embedding fallback 添加明确降级日志
- Consolidation 零向量改为 hash fallback
- RBAC admin 路由保护（auth disabled 时仍需认证）
- 7 个多语言 README 添加翻译滞后声明
- ARCHITECTURE.md / DESIGN.md 版本号清理
- async_memory_pipeline 改为环境变量驱动
- X-User-ID / X-Tenant-ID 格式验证
- 所有 10 个 LLM adapter 添加 120s HTTP 超时
- Config hot-reload 限制文档化

---

## PR 合并记录

| PR | 内容 | 状态 |
|----|------|------|
| #55 | RBAC/Tenant JSON 文件持久化 | ✅ merged |
| #56 | P0-P2 审计修复 (20 项) | ✅ merged |
| #57 | Release Checklist P0: 消除 panic | ✅ merged |
| #58 | K8s 健康探针、死代码清理 | ✅ merged |
| #59 | state.json/PROCESS.md 文档同步 | ✅ merged |
| #60 | CHANGELOG + dead_code 清理 | ✅ merged |
| #61 | 最终 checklist 状态更新 | ✅ merged |
| #62 | 二次审查报告修复 (12 项) | ✅ merged |
| #96 | 三次深度审查修复 (28 项) | ✅ merged |
