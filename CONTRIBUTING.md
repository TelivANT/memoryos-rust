# 贡献指南 (Contributing Guide)

感谢你对 MemoryOS-Rust 的关注！我们欢迎所有形式的贡献。

## 📋 目录

- [开始之前](#开始之前)
- [开发流程](#开发流程)
- [代码规范](#代码规范)
- [提交规范](#提交规范)
- [测试要求](#测试要求)
- [文档更新](#文档更新)
- [问题反馈](#问题反馈)

---

## 🚀 开始之前

### 1. 环境准备

确保你的开发环境满足以下要求：

- **Rust**: 1.93+ (`rustup update`)
- **Docker**: 用于运行依赖服务 (Redis, Qdrant)
- **Git**: 版本控制

### 2. Fork 并克隆仓库

```bash
# Fork 仓库到你的 GitHub 账号
# 然后克隆你的 fork
git clone https://github.com/YOUR_USERNAME/memoryos-rust.git
cd memoryos-rust

# 添加上游仓库
git remote add upstream https://github.com/TelivANT/memoryos-rust.git
```

### 3. 启动依赖服务

```bash
docker-compose up -d
```

### 4. 构建项目

```bash
cargo build
cargo test
```

---

## 🔄 开发流程

### Step 1: 同步最新代码

```bash
git checkout main
git pull upstream main
```

### Step 2: 创建功能分支

```bash
# 功能分支命名规范: feature/功能名称
git checkout -b feature/your-feature-name

# 修复分支命名规范: fix/问题描述
git checkout -b fix/issue-description
```

### Step 3: 记录工作日志

在 [WORK_LOG.md](./WORK_LOG.md) 中添加你的任务：

```markdown
## 2026-02-XX - [你的名字]

### 🎯 任务
- [ ] 实现 XXX 功能
- [ ] 修复 XXX 问题

### 📝 进度
- 开始时间: 2026-02-XX 10:00
- 预计完成: 2026-02-XX 18:00
- 状态: 🟡 进行中

### 🐛 遇到的问题
- 问题描述...
```

### Step 4: 开发与测试

```bash
# 开发过程中持续测试
cargo test

# 检查代码格式
cargo fmt --check

# 运行 Clippy 检查
cargo clippy -- -D warnings
```

### Step 5: 提交代码

```bash
git add .
git commit -m "feat: 添加 XXX 功能"
git push origin feature/your-feature-name
```

### Step 6: 创建 Pull Request

1. 访问你的 fork 仓库页面
2. 点击 "New Pull Request"
3. 填写 PR 描述（参考下方模板）
4. 等待 CI 通过和代码审查

---

## 📝 代码规范

### Rust 代码风格

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

### 命名规范

```rust
// 模块名: snake_case
mod memory_storage;

// 结构体: PascalCase
struct MemoryConfig;

// 函数/变量: snake_case
fn process_memory() {}
let user_id = "123";

// 常量: SCREAMING_SNAKE_CASE
const MAX_RETRIES: u32 = 3;
```

### 错误处理

```rust
// 使用 Result 和 thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Invalid configuration: {0}")]
    Config(String),
}

// 函数返回 Result
pub async fn save_memory(data: &str) -> Result<(), MemoryError> {
    // ...
}
```

### 异步代码

```rust
// 使用 async/await
pub async fn fetch_data() -> Result<Data, Error> {
    let response = client.get(url).await?;
    Ok(response.json().await?)
}

// 使用 tokio::spawn 处理并发
tokio::spawn(async move {
    process_task().await;
});
```

---

## 📦 提交规范

我们使用 [Conventional Commits](https://www.conventionalcommits.org/) 规范：

### 提交类型

- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档更新
- `style`: 代码格式调整（不影响功能）
- `refactor`: 代码重构
- `perf`: 性能优化
- `test`: 测试相关
- `chore`: 构建/工具链相关

### 提交示例

```bash
# 新功能
git commit -m "feat: 添加 Graph Memory 可视化功能"

# 修复 bug
git commit -m "fix: 修复 Redis 连接池泄漏问题"

# 文档更新
git commit -m "docs: 更新 API 文档示例"

# 性能优化
git commit -m "perf: 优化 Qdrant 批量查询性能"
```

---

## ✅ 测试要求

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryStorage::new();
        let result = storage.save("test").await;
        assert!(result.is_ok());
    }
}
```

### 集成测试

在 `tests/` 目录下添加集成测试：

```rust
// tests/memory_integration.rs
#[tokio::test]
async fn test_full_memory_flow() {
    // 测试完整的记忆流程
}
```

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_memory_storage

# 运行集成测试
cargo test --test memory_integration
```

---

## 📚 文档更新

### 代码文档

```rust
/// 保存用户记忆到存储系统
///
/// # Arguments
/// * `user_id` - 用户唯一标识
/// * `content` - 记忆内容
///
/// # Returns
/// * `Ok(())` - 保存成功
/// * `Err(MemoryError)` - 保存失败
///
/// # Example
/// ```
/// let result = save_memory("user123", "Hello").await?;
/// ```
pub async fn save_memory(user_id: &str, content: &str) -> Result<(), MemoryError> {
    // ...
}
```

### 更新相关文档

如果你的改动影响以下内容，请同步更新文档：

- **API 变更**: 更新 [docs/API.md](./docs/API.md)
- **配置变更**: 更新 [docs/QUICKSTART.md](./docs/QUICKSTART.md)
- **架构变更**: 更新 [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)
- **新功能**: 更新 [README.md](./README.md) 和 [CHANGELOG.md](./CHANGELOG.md)

---

## 🐛 问题反馈

### 提交 Issue

在提交 Issue 前，请先搜索是否已有类似问题。

**Bug 报告模板**:

```markdown
## 问题描述
简要描述遇到的问题

## 复现步骤
1. 执行 XXX 命令
2. 访问 XXX 接口
3. 观察到 XXX 错误

## 预期行为
应该发生什么

## 实际行为
实际发生了什么

## 环境信息
- OS: macOS 14.0
- Rust: 1.93.0
- MemoryOS-Rust: 0.2.0

## 日志/截图
粘贴相关日志或截图
```

**功能请求模板**:

```markdown
## 功能描述
简要描述你希望添加的功能

## 使用场景
为什么需要这个功能？解决什么问题？

## 建议实现
（可选）你认为应该如何实现
```

---

## 🎯 Pull Request 模板

```markdown
## 变更类型
- [ ] 新功能 (feat)
- [ ] Bug 修复 (fix)
- [ ] 文档更新 (docs)
- [ ] 代码重构 (refactor)
- [ ] 性能优化 (perf)
- [ ] 测试相关 (test)

## 变更描述
简要描述你的改动

## 相关 Issue
Closes #123

## 测试
- [ ] 已添加单元测试
- [ ] 已添加集成测试
- [ ] 本地测试通过
- [ ] CI 测试通过

## 文档
- [ ] 已更新相关文档
- [ ] 已更新 CHANGELOG.md
- [ ] 已更新 WORK_LOG.md

## 检查清单
- [ ] 代码已通过 `cargo fmt` 格式化
- [ ] 代码已通过 `cargo clippy` 检查
- [ ] 所有测试通过 (`cargo test`)
- [ ] 提交信息符合规范
```

---

## 🤝 协作机制

我们使用双轨记录系统：

1. **WORK_LOG.md**: 人类可读的工作日志
2. **docs/state.json**: 机器可读的项目状态

请在开发过程中保持这两个文件的同步更新。

---

## 📞 联系方式

- **GitHub Issues**: [提交问题](https://github.com/TelivANT/memoryos-rust/issues)
- **原项目**: [MemoryOS](https://github.com/BAI-LAB/MemoryOS)
- **邮件**: 查看原项目 README 中的联系方式

---

## 📄 许可

通过贡献代码，你同意你的贡献将在 Apache 2.0 许可下发布。

---

**感谢你的贡献！** 🎉
