# 开发文档

**版本**: v0.2.0  
**更新时间**: 2026-02-18

---

## 📋 目录

- [开发环境设置](#开发环境设置)
- [项目结构](#项目结构)
- [架构设计](#架构设计)
- [开发规范](#开发规范)
- [单机到集群开发流程](#单机到集群开发流程)
- [测试指南](#测试指南)
- [贡献指南](#贡献指南)

---

## 开发环境设置

### 1. 安装依赖

**Rust**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

**开发工具**:
```bash
# 代码格式化
rustup component add rustfmt

# 代码检查
rustup component add clippy

# IDE 支持
cargo install rust-analyzer
```

**可选服务**:
```bash
# Middleware Demo (Redis + Qdrant)
docker-compose -f docker-compose.middleware-demo.yml up -d
```

**Compose 文件说明**:
- `docker-compose.standalone.yml`: 单机全栈
- `docker-compose.cluster.yml`: 双 gateway 集群演示
- `docker-compose.middleware-demo.yml`: 中间件 demo / 故障注入场景

### 2. 远程开发服务器

**服务器信息**:
- **地址**: `root@104.194.91.83`
- **端口**: `26974`
- **认证**: SSH Key（已配置免密登录）
- **Docker**: ✅ 已安装
- **用途**: 测试部署、性能测试

**连接服务器**:
```bash
ssh root@104.194.91.83 -p 26974
```

**部署到服务器**:
```bash
# 一键部署
./deploy-remote.sh

# 手动同步
rsync -avz -e "ssh -p 26974" . root@104.194.91.83:/opt/memoryos/
```

**访问服务**:
- Gateway: http://104.194.91.83:8080
- Health: http://104.194.91.83:8080/health
- Metrics: http://104.194.91.83:8080/metrics

**注意**: 此服务器仅供开发测试使用，使用 SSH Key 认证，安全可靠。

### 2. 克隆项目
```bash
git clone https://github.com/BAI-LAB/MemoryOS.git
cd MemoryOS/MemoryOS-Rust
```

### 3. 配置环境
```bash
cp config.example.toml config.toml
# 编辑 config.toml 填入你的 API keys
```

### 4. 编译运行
```bash
# 检查编译
cargo check --workspace

# 运行测试
cargo test --workspace

# 启动服务
cargo run --package memoryos-gateway

# 可选：仅在启用异步记忆管道时启动 Worker（单节点可不启）
# export MEMORYOS_ASYNC_MEMORY_PIPELINE=true
# cargo run --package memoryos-worker

# 启动中间件（本地调试）
docker-compose -f docker-compose.middleware-demo.yml up -d
```

---

## 项目结构

```
MemoryOS-Rust/
├── crates/
│   ├── memoryos-core/          # 核心业务逻辑
│   │   ├── src/
│   │   │   ├── config.rs       # 配置管理
│   │   │   ├── error.rs        # 错误定义
│   │   │   ├── health.rs       # 健康检查
│   │   │   └── memory/         # 记忆系统
│   │   └── Cargo.toml
│   │
│   ├── memoryos-ports/         # 端口定义（接口）
│   │   ├── src/
│   │   │   ├── llm.rs          # LLM 适配器接口
│   │   │   └── memory.rs       # 记忆存储接口
│   │   └── Cargo.toml
│   │
│   ├── memoryos-adapters/      # 适配器实现
│   │   ├── src/
│   │   │   ├── llm/            # LLM 适配器
│   │   │   │   ├── openai.rs
│   │   │   │   ├── gemini.rs
│   │   │   │   ├── claude.rs
│   │   │   │   ├── ollama.rs
│   │   │   │   ├── deepseek.rs
│   │   │   │   ├── openrouter.rs
│   │   │   │   └── azure_openai.rs
│   │   │   └── memory/         # 存储适配器
│   │   │       ├── redis.rs
│   │   │       ├── qdrant.rs
│   │   │       └── manager.rs
│   │   └── Cargo.toml
│   │
│   ├── memoryos-gateway/       # HTTP 网关
│       ├── src/
│       │   ├── main.rs         # 入口
│       │   ├── router.rs       # 3-Tier 路由
│       │   └── routes/         # API 路由
│       │       ├── chat.rs
│       │       ├── memory.rs
│       │       └── health.rs
│       └── Cargo.toml
│
│   └── memoryos-worker/        # 异步记忆 Worker
│       ├── src/
│       │   └── main.rs         # Redis Stream consumer
│       └── Cargo.toml
│
├── config.toml                 # 配置文件
├── Cargo.toml                  # Workspace 配置
└── docs/                       # 文档
```

---

## 架构设计

### 六边形架构（Hexagonal Architecture）

```
┌─────────────────────────────────────────────┐
│           Gateway (HTTP Layer)              │
│  ┌─────────────────────────────────────┐   │
│  │  Routes: /chat, /memory, /health    │   │
│  └─────────────────────────────────────┘   │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│              Core (Business Logic)          │
│  ┌──────────────┐      ┌─────────────────┐ │
│  │  LLM Router  │      │ Memory Manager  │ │
│  │  (3-Tier)    │      │  (Short/Mid/    │ │
│  │              │      │   Long-term)    │ │
│  └──────────────┘      └─────────────────┘ │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│            Ports (Interfaces)               │
│  ┌──────────────┐      ┌─────────────────┐ │
│  │ LlmAdapter   │      │ MemoryStorage   │ │
│  │   trait      │      │     trait       │ │
│  └──────────────┘      └─────────────────┘ │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│          Adapters (Implementations)         │
│  ┌──────────────┐      ┌─────────────────┐ │
│  │ OpenAI       │      │ Redis           │ │
│  │ Gemini       │      │ Qdrant          │ │
│  │ Claude       │      │ Noop (Fallback) │ │
│  │ Ollama       │      │                 │ │
│  │ DeepSeek     │      │                 │ │
│  │ OpenRouter   │      │                 │ │
│  │ Azure OpenAI │      │                 │ │
│  └──────────────┘      └─────────────────┘ │
└─────────────────────────────────────────────┘
```

### 核心概念

**1. 端口（Ports）**:
- 定义业务接口
- 与具体实现解耦
- 位于 `memoryos-ports`

**2. 适配器（Adapters）**:
- 实现端口接口
- 对接外部服务
- 位于 `memoryos-adapters`

**3. 核心（Core）**:
- 业务逻辑
- 不依赖外部实现
- 位于 `memoryos-core`

**4. 网关（Gateway）**:
- HTTP 接口
- 路由和中间件
- 位于 `memoryos-gateway`

**5. Worker**:
- 异步消费 `chat_log`
- 幂等去重 + DLQ 兜底
- 位于 `memoryos-worker`

---

## 开发规范

### 代码风格

**格式化**:
```bash
cargo fmt --all
```

**Lint 检查**:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### 命名规范

**文件名**: `snake_case.rs`
```rust
// ✅ 正确
src/llm_adapter.rs
src/memory_manager.rs

// ❌ 错误
src/LLMAdapter.rs
src/memoryManager.rs
```

**类型名**: `PascalCase`
```rust
// ✅ 正确
struct ChatRequest { }
enum HealthMode { }
trait LlmAdapter { }

// ❌ 错误
struct chat_request { }
enum health_mode { }
```

**函数名**: `snake_case`
```rust
// ✅ 正确
fn get_response() { }
async fn chat_stream() { }

// ❌ 错误
fn GetResponse() { }
fn chatStream() { }
```

**常量**: `SCREAMING_SNAKE_CASE`
```rust
// ✅ 正确
const MAX_RETRIES: u32 = 3;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// ❌ 错误
const maxRetries: u32 = 3;
const default_timeout: Duration = Duration::from_secs(30);
```

### 错误处理

**禁止 unwrap/expect**:
```rust
// ❌ 错误 - 会 panic
let value = some_option.unwrap();
let result = some_result.expect("failed");

// ✅ 正确 - 返回 Result
let value = some_option.ok_or(AppError::NotFound("value".into()))?;
let result = some_result.map_err(|e| AppError::InternalError(e.to_string()))?;
```

**统一错误类型**:
```rust
// 所有公共函数返回 Result<T, AppError>
pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
    // ...
}
```

**错误转换**:
```rust
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::ExternalServiceError(err.to_string())
    }
}
```

### 异步编程

**使用 async/await**:
```rust
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError>;
}
```

**避免阻塞**:
```rust
// ❌ 错误 - 阻塞异步运行时
std::thread::sleep(Duration::from_secs(1));

// ✅ 正确 - 异步等待
tokio::time::sleep(Duration::from_secs(1)).await;
```

### 日志记录

**使用 tracing**:
```rust
use tracing::{info, warn, error, debug};

// 信息日志
info!("Request processed successfully");

// 警告日志
warn!("Redis connection failed, using fallback");

// 错误日志
error!("Failed to call LLM: {}", err);

// 调试日志
debug!("Request body: {:?}", request);
```

**结构化日志**:
```rust
info!(
    method = %request.method,
    path = %request.path,
    status = response.status,
    duration_ms = duration.as_millis(),
    "Request completed"
);
```

---

## 单机到集群开发流程

### 本地单机开发

1. 启动中间件：`docker-compose -f docker-compose.middleware-demo.yml up -d`
2. 启动 gateway：`cargo run -p memoryos-gateway`
3. 默认单节点无需 worker（同步路径可用）。
4. 若要验证异步路径：`export MEMORYOS_ASYNC_MEMORY_PIPELINE=true` 并启动 `cargo run -p memoryos-worker`
5. 调用 `/v1/memory/add` 或 `redis-cli XADD chat_log ...` 验证异步消费。

### 集群演练开发

1. 启动 `docker-compose.cluster.yml` 做双 gateway 演练。
2. 启动多个 worker，保持同一 `MEMORYOS_WORKER_GROUP`。
3. 每个 worker 设置唯一 `MEMORYOS_WORKER_CONSUMER`。
4. 检查 `chat_log` 积压与 `chat_log:dlq` 增长趋势。

### 变更约束

1. 事件字段变更必须保持至少一版向后兼容。
2. `event_id` 语义变更必须同步更新 gateway/worker/docs/tests。
3. 重试策略变更必须明确 ACK 与 DLQ 分界条件。

### 冒烟脚本

```bash
# 需要本机可用 redis-cli
export MEMORYOS_ASYNC_MEMORY_PIPELINE=true
./scripts/smoke_async_pipeline.sh
```

一键 Demo（自动拉起中间件 + gateway + worker）：

```bash
./scripts/demo_async_pipeline.sh
```

DLQ 回放：

```bash
DRY_RUN=1 COUNT=20 ./scripts/replay_dlq.sh
DRY_RUN=0 COUNT=20 ./scripts/replay_dlq.sh
```

## 测试指南

### 单元测试

**测试模块**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_load() {
        let config = Config::from_file("config.example.toml").unwrap();
        assert_eq!(config.server.port, 8080);
    }

    #[tokio::test]
    async fn test_health_check() {
        let status = HealthStatus::new();
        assert_eq!(status.mode, HealthMode::FullyOperational);
    }
}
```

**运行测试**:
```bash
# 所有测试
cargo test --workspace

# 单个包
cargo test --package memoryos-core

# 单个测试
cargo test test_config_load

# 显示输出
cargo test -- --nocapture
```

### 集成测试

**测试文件** (`tests/integration_test.rs`):
```rust
use memoryos_gateway::*;

#[tokio::test]
async fn test_chat_api() {
    let app = create_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"model":"gpt-4o-mini","messages":[]}"#))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}
```

### Mock 测试

**Mock LLM Adapter**:
```rust
struct MockLlmAdapter;

#[async_trait]
impl LlmAdapter for MockLlmAdapter {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, AppError> {
        Ok(ChatResponse {
            id: "test".into(),
            model: "mock".into(),
            choices: vec![],
        })
    }
    
    fn name(&self) -> &str {
        "mock"
    }
}
```

### 测试覆盖率

```bash
# 安装 tarpaulin
cargo install cargo-tarpaulin

# 生成覆盖率报告
cargo tarpaulin --workspace --out Html
```

---

## 贡献指南

### 1. Fork 项目
```bash
# 在 GitHub 上 Fork
# 克隆你的 Fork
git clone https://github.com/YOUR_USERNAME/MemoryOS.git
cd MemoryOS/MemoryOS-Rust
```

### 2. 创建分支
```bash
git checkout -b feature/your-feature-name
```

### 3. 开发

**遵循规范**:
- ✅ 代码格式化: `cargo fmt`
- ✅ Lint 检查: `cargo clippy`
- ✅ 测试通过: `cargo test`
- ✅ 文档更新

**提交信息**:
```bash
# 格式: <type>: <description>
git commit -m "feat: add streaming support"
git commit -m "fix: resolve Gemini API key leak"
git commit -m "docs: update API documentation"
```

**类型**:
- `feat` - 新功能
- `fix` - 修复 bug
- `docs` - 文档更新
- `style` - 代码格式
- `refactor` - 重构
- `test` - 测试
- `chore` - 构建/工具

### 4. 推送
```bash
git push origin feature/your-feature-name
```

### 5. 创建 Pull Request

**PR 标题**:
```
feat: Add streaming support for chat API
```

**PR 描述**:
```markdown
## 变更说明
- 添加 `chat_stream` 方法到 LlmAdapter trait
- 实现 OpenAI 流式响应
- 更新 Gateway 支持 SSE

## 测试
- [x] 单元测试通过
- [x] 集成测试通过
- [x] 手动测试流式 API

## 相关 Issue
Closes #123
```

### 6. Code Review

**响应反馈**:
- 及时回复评论
- 修改代码
- 更新测试

**合并要求**:
- ✅ 所有测试通过
- ✅ Code review 通过
- ✅ 无冲突
- ✅ 文档更新

---

## 常见任务

### 添加新的 LLM Adapter

**1. 定义 Adapter**:
```rust
// crates/memoryos-adapters/src/llm/new_llm.rs
pub struct NewLlmAdapter {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

#[async_trait]
impl LlmAdapter for NewLlmAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        // 实现调用逻辑
    }
    
    fn name(&self) -> &str {
        "new_llm"
    }
}
```

**2. 注册到 Router**:
```rust
// crates/memoryos-gateway/src/router.rs
pub struct LlmRouter {
    new_llm: Arc<dyn LlmAdapter>,
    // ...
}
```

**3. 添加配置**:
```toml
# config.toml
[llm]
new_llm_api_key = "..."
new_llm_base_url = "..."
```

**4. 添加测试**:
```rust
#[tokio::test]
async fn test_new_llm_adapter() {
    // ...
}
```

### 添加新的 API 端点

**1. 定义路由**:
```rust
// crates/memoryos-gateway/src/routes/new_route.rs
pub async fn new_endpoint(
    State(state): State<Arc<AppState>>,
    Json(request): Json<NewRequest>,
) -> Result<Json<NewResponse>, AppError> {
    // 实现逻辑
    Ok(Json(response))
}
```

**2. 注册路由**:
```rust
// crates/memoryos-gateway/src/routes/mod.rs
pub fn create_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/new", post(new_endpoint))
        // ...
}
```

**3. 添加测试**:
```rust
#[tokio::test]
async fn test_new_endpoint() {
    // ...
}
```

---

## 调试技巧

### 启用详细日志
```bash
RUST_LOG=debug cargo run
```

### 使用 rust-gdb
```bash
rust-gdb target/debug/memoryos-gateway
```

### 性能分析
```bash
# 安装 flamegraph
cargo install flamegraph

# 生成火焰图
cargo flamegraph
```

---

## 资源链接

- **Rust 官方文档**: https://doc.rust-lang.org/
- **Tokio 文档**: https://tokio.rs/
- **Axum 文档**: https://docs.rs/axum/
- **项目 GitHub**: https://github.com/BAI-LAB/MemoryOS

---

**最后更新**: 2026-02-17
