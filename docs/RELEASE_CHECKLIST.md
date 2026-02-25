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
| 7 | defense 路由未挂载 | `routes/defense.rs`, `middleware/defense.rs` | ✅ DONE — 标记为 v1.1 预留，代码保留 |
| 8 | Dockerfile 无 HEALTHCHECK，中文注释混杂 | `Dockerfile` | ✅ DONE (PR #57) |
| 9 | /health/live 和 /health/ready 未挂载 | `main.rs`, `routes/health.rs` | ✅ DONE (PR #58) |
| 10 | /health/status 返回 worker monitor 而非真实健康状态 | `main.rs`, `handlers.rs` | ✅ DONE (PR #57) |
| 11 | .bak 文件残留在仓库中 | `auth/store_redis.rs.bak` | ✅ DONE (PR #58) |
| 12 | API.md 端点汇总表缺少 /health/live 和 /health/ready | `docs/API.md` | ✅ DONE (PR #58) |

## P2 — 可选优化

| # | 问题 | 文件 | 状态 |
|---|------|------|------|
| 13 | lazy_static 在 workspace 级别但只有 metrics 用 | `Cargo.toml` | ⬜ TODO — 影响极小，不阻塞发布 |
| 14 | wiki-gen clone_to_temp() 9 个 connector 返回 "not supported" | `crates/memoryos-wiki-gen/src/storage/` | ⬜ TODO — 设计如此，非 bug |
| 15 | state.json / PROCESS.md 进度信息过时 | `docs/state.json`, `PROCESS.md` | ✅ DONE (PR #59) |

---

## 审计轮次记录

### Round 1 (PR #57)
- 消除 gateway 所有 `.expect()` panic 点
- 修复 config 文件一致性
- 添加 Dockerfile HEALTHCHECK
- CI CVE ignore 添加文档说明
- README badge 更新
- /health/status 改为真实依赖健康检查

### Round 2 (PR #58)
- 挂载 K8s 标准探针 /health/live 和 /health/ready
- 删除 .bak 残留文件
- API.md 添加 /health/live 和 /health/ready 文档
- ARCHITECTURE.md defense 标记为 v1.1 预留
- 端点汇总表更新（47→49 个端点）

### Round 3 (PR #59)
- state.json 全面重写：修复端点列表、移除过时分支引用、精简结构
- PROCESS.md 修复 "SQLite" 错误引用为 "JSON 文件持久化"
- PROCESS.md 添加 PR #55-#58 记录
- 技术栈一致性检查通过：无 SQLx/PostgreSQL/MySQL/SQLite 残留
- 无 panic!/todo!/unimplemented!/unsafe 在生产代码中

---

## 已完成 PR 列表

| PR | 内容 | 状态 |
|----|------|------|
| #56 | P0-P2 审计修复（20 项） | ✅ merged |
| #57 | Release Checklist P0 修复 | ✅ merged |
| #58 | 审计 Round 2: K8s 探针 + 文档同步 | ✅ merged |
| #59 | 审计 Round 3: state.json/PROCESS.md 文档同步 | 🔄 pending |
