# MemoryOS-Rust v1.0.0 Release Checklist

**最后更新**: 2026-02-25
**当前版本**: v1.0.0-rc
**目标**: 所有 P0/P1 修复完成后发布 v1.0.0

---

## 轮训审查流程

每次审查必须按以下顺序执行：

1. **Bug / 功能缺陷** — 代码是否有 panic、unwrap、空实现、死代码
2. **文档同步** — 文档是否与代码一致（版本号、功能描述、架构图）
3. **进度追踪** — 本文件是否反映最新状态
4. **代码-文档偏差** — 文档声称的功能是否真正实现
5. **技术栈一致性** — 是否引入了不必要的依赖（如 SQLx、PostgreSQL 等）
6. **基座复用** — 是否有重复代码可以抽取到共享 crate

---

## P0 — 必须修复（阻塞发布）

| # | 问题 | 文件 | 状态 |
|---|------|------|------|
| 1 | state.rs 5 处 `.expect()` 可 panic | `crates/memoryos-gateway/src/state.rs` | ✅ DONE (PR #57) |
| 2 | main.rs 1 处 `.expect()` 可 panic | `crates/memoryos-gateway/src/main.rs` | ✅ DONE (PR #57) |
| 3 | config.toml 含占位符 API Key 未校验 | `config.toml` | ✅ DONE (PR #57) |

## P1 — 应该修复（v1.0 质量要求）

| # | 问题 | 文件 | 状态 |
|---|------|------|------|
| 4 | config 文件不一致（结构/格式不同） | `config*.toml` | ✅ DONE (PR #57) |
| 5 | CI 安全审计忽略 4 个 CVE 无文档说明 | `.github/workflows/ci.yml` | ✅ DONE (PR #57) |
| 6 | README badge 仍显示 "Early Development" | `README.md` | ✅ DONE (PR #57) |
| 7 | defense 路由 4 个函数 + middleware 标记 dead_code 未挂载 | `routes/defense.rs`, `middleware/defense.rs` | ⬜ TODO |
| 8 | Dockerfile 无 HEALTHCHECK，中文注释混杂 | `Dockerfile` | ✅ DONE (PR #57) |
| 9 | Worker 初始化缺少重试逻辑 | `crates/memoryos-worker/src/main.rs` | ⬜ TODO |

## P2 — 可选优化

| # | 问题 | 文件 | 状态 |
|---|------|------|------|
| 10 | benchmark workflow 被禁用 | `.github/workflows/benchmarks.yml` | ⬜ TODO |
| 11 | lazy_static 在 workspace 级别但只有 metrics 用 | `Cargo.toml` | ⬜ TODO |
| 12 | wiki-gen clone_to_temp() 9 个 connector 返回 "not supported" | `crates/memoryos-wiki-gen/src/storage/` | ⬜ TODO |
| 13 | state.json / PROCESS.md 进度信息过时 | `docs/state.json`, `PROCESS.md` | ⬜ TODO |

---

## 已完成

### PR #56 (merged)
- ✅ MCP SSE 空壳 → proper error return
- ✅ MCP 无用依赖移除
- ✅ reqwest 版本统一 0.11→0.12
- ✅ 删除死代码 routes/chat.rs + StubUpstreamClient
- ✅ admin main.rs 6 处 expect() → Result
- ✅ 文档版本号 v0.13.0 → v1.0.0-rc
- ✅ Rust 版本统一 1.75+，Cargo.toml 添加 rust-version
- ✅ 删除不存在的 saas 分支引用
- ✅ 存储连接器列表 8→17
- ✅ ChatResponse 添加 usage 字段
- ✅ NATS 移到 full profile
- ✅ vs Mem0 对比表清理 TBD 行

### PR #57 (pending)
- ✅ state.rs AppState::new → Result（消除 5 处 expect）
- ✅ main.rs QdrantMultiModalStorage expect → map_err
- ✅ config.toml 移除占位符 key，auth 默认关闭
- ✅ config.docker.toml 修复 ${ENV} → api_key_env
- ✅ config.production.toml 重写为当前格式
- ✅ examples/config.production.toml 重写
- ✅ CI CVE ignore 添加文档说明
- ✅ README badge Early Development → Release Candidate
- ✅ Dockerfile 添加 HEALTHCHECK + curl + 英文注释
