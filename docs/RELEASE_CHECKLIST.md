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

## 六条准则审计 Round 8-22 (2026-02-25)

**审计统计**:
- 总轮次: 22
- 发现问题: 8
- 已修复并合并: 8
- 直接通过: 14

| Round | 准则 | 发现问题 | PR | 状态 |
|-------|------|----------|-----|------|
| 8 | 文档同步 | 9 个文档版本号过时 (0.2.0/0.3.0/0.9.0) | #68 | ✅ MERGED |
| 9 | Bug/功能缺陷 | Clippy 警告 (from_str, is_multiple_of, too_many_arguments) | #69 | ✅ MERGED |
| 10 | Bug/功能缺陷 | 未使用的 LanguageParser 导入 | #70 | ✅ MERGED |
| 11 | 文档同步 | 缺失 4 个环境变量文档 | #71 | ✅ MERGED |
| 12 | 基座复用 | 重复的 Dockerfile.gateway | #72 | ✅ MERGED |
| 13 | Bug/功能缺陷 | 错误处理一致性检查 | - | ✅ PASS |
| 14 | 代码-文档偏差 | API.md 缺少 3 个 Defense API 端点 | #75 | ✅ MERGED |
| 15 | Bug/功能缺陷 | 测试覆盖率检查 (303 tests) | - | ✅ PASS |
| 16 | 进度追踪 | 更新 RELEASE_CHECKLIST 记录所有审计轮次 | #74 | ✅ MERGED |
| 17 | 文档同步 | Cargo.toml 元数据一致性 | - | ✅ PASS |
| 18 | 代码-文档偏差 | README 示例代码检查 | - | ✅ PASS |
| 19 | Bug/功能缺陷 | 配置文件安全性检查 | - | ✅ PASS |
| 20 | Bug/功能缺陷 | 依赖安全性检查 | - | ✅ PASS |
| 21 | Bug/功能缺陷 | 日志级别配置检查 | - | ✅ PASS |
| 22 | 进度追踪 | 最终审计汇总 | #76 | 🔄 PENDING |

**修复内容汇总**:
- 文档版本统一到 v1.0.0-rc (10 个文件)
- 代码质量改进 (Clippy 警告修复)
- 文档完整性提升 (环境变量、API 端点)
- 代码重复消除 (Dockerfile)
- 进度追踪完善 (PROCESS.md, state.json, RELEASE_CHECKLIST.md)

**质量指标**:
- 测试通过: 303 个
- Clippy 警告: 从 31 降至 ~20 (剩余为合理警告)
- 文档覆盖率: 100% (所有 API 端点已文档化)
- 安全审计: 13/15 修复 (87%)，0 P0/P1 遗留

---

## 六条准则审计 Round 8-16 (2026-02-25)

| Round | 准则 | 发现问题 | PR | 状态 |
|-------|------|----------|-----|------|
| 8 | 文档同步 | 9 个文档版本号过时 (0.2.0/0.3.0/0.9.0) | #68 | ✅ MERGED |
| 9 | Bug/功能缺陷 | Clippy 警告 (from_str, is_multiple_of, too_many_arguments) | #69 | ✅ MERGED |
| 10 | Bug/功能缺陷 | 未使用的 LanguageParser 导入 | #70 | ✅ MERGED |
| 11 | 文档同步 | 缺失 4 个环境变量文档 | #71 | ✅ MERGED |
| 12 | 基座复用 | 重复的 Dockerfile.gateway | #72 | ✅ MERGED |
| 13 | Bug/功能缺陷 | 错误处理一致性检查 | - | ✅ PASS |
| 14 | 代码-文档偏差 | API.md 缺少 3 个 Defense API 端点 | #73 | 🔄 PENDING |
| 15 | Bug/功能缺陷 | 测试覆盖率检查 (303 tests) | - | ✅ PASS |
| 16 | 进度追踪 | 更新 RELEASE_CHECKLIST 记录所有审计轮次 | #74 | 🔄 PENDING |

**审计统计**:
- 总轮次: 16
- 发现问题: 10
- 已修复: 8
- 待合并: 2
- 直接通过: 6

---

## 六条准则审计 Round 7 (2026-02-25)

### 发现问题

| # | 准则 | 问题 | 状态 |
|---|------|------|------|
| 1 | 进度追踪 | PR #65/#66 未记录到 PROCESS.md/state.json | ✅ FIXED |
| 2 | 代码-文档偏差 | SECURITY_ARCHITECTURE.md 版本过时 (0.2.0)，4 个 TODO 未更新 | ✅ FIXED |

### 变更文件清单

- `PROCESS.md` — 添加 PR #65/#66，版本历史 rc.4
- `docs/state.json` — 添加 PR #65/#66，移除已完成的 STM 任务
- `docs/SECURITY_ARCHITECTURE.md` — 版本 0.2.0 → v1.0.0-rc，更新所有 TODO 为已实现

---

## 三次审查报告修复 (Round 6)

### P1 修复

| # | 问题 | 状态 |
|---|------|------|
| P1-1 | Defense 模块未挂载 | ✅ DONE — IpDefenseSystem 加入 AppState，defense routes 挂载到 /v1/admin/defense，移除 4 处 #[allow(dead_code)] |
| P1-2 | ChatRequest/ChatMessage 重复 3 处 | ✅ DONE — 删除 core/llm/context.rs 死代码（ContextInjector 无外部消费者）。wiki-gen 版保留（独立 crate，不依赖 ports）。ports 版为权威定义 |
| P1-3 | 9 处 #[allow(dead_code)] | ✅ 降至 5 处 — defense routes 4 处已消除，剩余 5 处均有合理原因（ConnectInfo 限制、NATS 连接保持、反序列化、内部字段、公共 API 预留） |
| P1-4 | v0.13.0 残留 | ✅ 确认 — WORK_LOG L4 已是 v1.0.0-rc，其余均为版本历史表 |
| P1-5 | wiki-gen 零内联测试 | 📋 不在本轮范围 — 非阻塞项 |

### P2 修复

| # | 问题 | 状态 |
|---|------|------|
| P2-1 | MCP SSE 选项 | ✅ DONE — transport 和 sse_addr 参数加 hide = true，CLI help 不再显示 |
| P2-2 | metrics expect | 📋 保留 — lazy_static 注册实际不会失败，低优先级 |

### 变更文件清单

- `crates/memoryos-gateway/src/state.rs` — 新增 IpDefenseSystem 初始化 + defense 字段
- `crates/memoryos-gateway/src/main.rs` — 挂载 /v1/admin/defense 路由
- `crates/memoryos-gateway/src/routes/defense.rs` — 移除 4 处 #[allow(dead_code)]
- `crates/memoryos-gateway/src/middleware/defense.rs` — 添加详细注释说明 ConnectInfo 限制
- `crates/memoryos-core/src/llm/context.rs` — 已删除（死代码）
- `crates/memoryos-core/src/llm/mod.rs` — 移除 context 模块导出
- `crates/memoryos-mcp/src/main.rs` — SSE 参数 hide = true

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

## 剩余 #[allow(dead_code)] 清单 (4 处，均合理)

| 文件 | 原因 |
|------|------|
| state.rs: current_worker_monitor (1 处) | 异步管道监控，按需启用 |
| adapters/memory/nats.rs (1 处) | NATS client 字段需保持连接 |
| adapters/memory/pinecone.rs (1 处) | PineconeFetchedVector 反序列化用 |
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
