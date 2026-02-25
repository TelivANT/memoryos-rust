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

## 二次审查报告修复 (Round 5)

### P0 — 残留代码 / 孤儿文件

| # | 问题 | 状态 |
|---|------|------|
| 1 | router.rs 孤儿文件 (181 行不编译) | ✅ DONE — 已删除 |
| 2 | defense 模块未挂载 | ✅ DONE — 标记 v1.1 预留 (PR #58) |
| 3 | UpstreamClient/NormalizedRequest/NormalizedResponse 死接口 | ✅ DONE — 已删除 |
| 4 | Admin 用 rbac.db/tenants.db，Gateway 用 .json | ✅ DONE — Admin 统一为 rbac_users.json/tenants.json |

### P1 — 重复定义 / 工程质量

| # | 问题 | 状态 |
|---|------|------|
| 5 | ChatRequest/ChatMessage 重复定义 3 处 | ✅ 分析完成 — core 版是 ContextInjector 专用简化类型，wiki-gen 版是独立 crate 设计。ports 版是权威定义。类型不互通是因为职责不同，非 bug |
| 6 | context_injector 死字段 | ✅ DONE — 从 AppState 中移除字段和初始化代码 |
| 7 | 剩余 #[allow(dead_code)] | ✅ 审计完成 — 剩余 9 处均为合理预留 (defense v1.1, NATS full feature, Pinecone optional, vue parser internal, worker monitor) |
| 8 | 9 个 connector clone_to_temp 空实现 | ✅ DONE — trait 加默认实现，删除 9 个重复 Err |

### P1 — 文档残留

| # | 问题 | 状态 |
|---|------|------|
| 9 | v0.13.0 残留 7 处 | ✅ 审计完成 — 均为版本历史表中的历史记录，非当前版本声明 |
| 10 | 3 处 TODO 注释 | ✅ DONE — local provider 改为读 config，测试 TODO 改为 NOTE |

### P2 — 低优先级

| # | 问题 | 状态 |
|---|------|------|
| 11 | README TBD | ✅ DONE — 改为 "pending real-world validation" |
| 12 | MCP SSE 选项 | ✅ DONE — CLI help 标注 "not yet implemented" |

---

## 剩余 #[allow(dead_code)] 清单 (9 处，均合理)

| 文件 | 原因 |
|------|------|
| routes/defense.rs (4 处) | v1.1 预留，IP 防御路由 |
| middleware/defense.rs (1 处) | v1.1 预留，IP 防御中间件 |
| state.rs: current_worker_monitor (1 处) | 异步管道监控，按需启用 |
| adapters/memory/nats.rs (1 处) | NATS 在 `full` feature profile 下 |
| adapters/memory/pinecone.rs (1 处) | Pinecone 可选向量存储 |
| parser/vue.rs: SfcSection.tag (1 处) | 内部数据结构，调试用字段 |

---

## 历史审计轮次

### Round 1-4 (PR #57-#61)
详见 git history。修复了初始 20 个审计问题中的 14 个。

### Round 5 (PR #62) — 二次审查报告
- 删除 router.rs 孤儿文件 (181 行)
- 删除 UpstreamClient/NormalizedRequest/NormalizedResponse 死接口 + 未使用的 serde import
- Admin 数据文件路径统一为 rbac_users.json/tenants.json
- 移除 context_injector 死字段 (AppState)
- StorageConnector trait 加 clone_to_temp 默认实现，删除 9 个重复空实现
- local provider 从硬编码 "local" 改为读 config.router.local_backends
- 3 处 TODO 改为 NOTE
- README TBD 修复
- MCP SSE CLI help 标注 "not yet implemented"

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
| #62 | 二次审查报告修复 (12 项) | 🔄 pending |
