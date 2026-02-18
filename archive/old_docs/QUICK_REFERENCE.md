# MemoryOS-Rust 快速参考

**版本**: 0.2.0  
**状态**: ✅ Production Ready  
**更新**: 2026-02-17

---

## 🚀 快速开始

```bash
# 1. 启动依赖
docker-compose -f docker-compose.middleware-demo.yml up -d

# 2. 配置
cp config.example.toml config.toml
vim config.toml  # 填入 API Key

# 3. 运行
cargo run --package memoryos-gateway

# 4. 测试
curl http://localhost:8080/health/status
```

---

## 📡 API 端点

### 健康检查
```bash
GET /health/live        # Liveness probe
GET /health/ready       # Readiness probe (实时检测)
GET /health/status      # 详细状态
```

### 聊天 API
```bash
POST /v1/chat/completions
{
  "model": "gpt-4o",
  "messages": [{"role": "user", "content": "Hello"}],
  "stream": false
}
```

### 流式响应
```bash
POST /v1/chat/completions
{
  "model": "gpt-4o",
  "messages": [{"role": "user", "content": "Hello"}],
  "stream": true
}
```

---

## 🔧 配置

### 环境变量
```bash
# 配置文件路径
export MEMORYOS_CONFIG=config.toml

# 配置热更新（默认启用）
export MEMORYOS_CONFIG_HOT_RELOAD=true

# 异步记忆管道
export MEMORYOS_ASYNC_MEMORY_PIPELINE=false

# 日志级别
export RUST_LOG=info
```

### 配置文件
```toml
[server]
host = "0.0.0.0"
port = 8080

[llm]
provider = "openai"  # openai/gemini/claude/ollama/deepseek/openrouter/azure-openai
api_key = "sk-your-key"
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"

[storage.redis]
url = "redis://localhost:6379"
ttl_seconds = 3600
max_messages = 20

[storage.vector]
url = "http://localhost:6334"
```

---

## 🏥 健康检查

### 正常模式
```json
GET /health/status
{
  "mode": "ready",
  "redis": "up",
  "qdrant": "up"
}
```

### 降级模式
```json
GET /health/status
{
  "mode": "degraded_ready",
  "redis": "down",
  "qdrant": "up"
}
Header: X-MemoryOS-Status: degraded
```

### 不可用模式
```json
GET /health/ready → 503 Service Unavailable
{
  "mode": "not_ready",
  "redis": "bypassed",
  "qdrant": "bypassed"
}
```

---

## 🔌 支持的 LLM

| Provider | Model 示例 | 配置 |
|----------|-----------|------|
| OpenAI | gpt-4o, gpt-4o-mini | provider = "openai" |
| Gemini | gemini-pro, gemini-flash | provider = "gemini" |
| Claude | claude-3-opus, claude-3-sonnet | provider = "claude" |
| Ollama | llama3.2:3b, qwen2.5:7b | provider = "ollama" |
| DeepSeek | deepseek-chat | provider = "deepseek" |
| OpenRouter | 任意模型 | provider = "openrouter" |
| Azure OpenAI | gpt-4 | provider = "azure-openai" |

---

## 🐳 Docker 部署

### 单机模式
```bash
docker-compose -f docker-compose.standalone.yml up -d
```

### 集群模式
```bash
docker-compose -f docker-compose.cluster.yml up -d
```

### 仅中间件
```bash
docker-compose -f docker-compose.middleware-demo.yml up -d
```

---

## 🧪 测试

```bash
# 运行所有测试
cargo test --workspace

# 运行特定包测试
cargo test --package memoryos-core

# 运行特定测试
cargo test test_config_validation
```

---

## 📊 监控

### 日志
```bash
# 查看日志
docker logs -f memoryos-gateway

# 设置日志级别
export RUST_LOG=debug
```

### 指标
```bash
# Prometheus 指标（如果启用）
GET /metrics
```

---

## 🔄 配置热更新

```bash
# 1. 启动服务
cargo run --package memoryos-gateway

# 2. 修改配置
vim config.toml

# 3. 等待 5 秒，自动生效
# 日志输出: ✅ Config hot-reloaded successfully

# 注意：端口等需要重启的配置除外
```

---

## 🛡️ 优雅降级

### 场景 1: Redis 故障
```
Redis ❌ + Qdrant ✅
→ 降级模式：仅向量检索可用
→ LLM 正常工作
→ 返回 200 OK + X-MemoryOS-Status: degraded
```

### 场景 2: Qdrant 故障
```
Redis ✅ + Qdrant ❌
→ 降级模式：仅短期记忆可用
→ LLM 正常工作
→ 返回 200 OK + X-MemoryOS-Status: degraded
```

### 场景 3: 全部故障
```
Redis ❌ + Qdrant ❌
→ Noop 模式：记忆功能不可用
→ LLM 仍然正常工作
→ 返回 503 Service Unavailable
```

---

## 🐛 故障排查

### 编译失败
```bash
# 清理并重新编译
cargo clean
cargo build --release
```

### 测试失败
```bash
# 查看详细错误
cargo test --workspace -- --nocapture
```

### 连接失败
```bash
# 检查 Redis
redis-cli ping

# 检查 Qdrant
curl http://localhost:6334/health
```

### 配置错误
```bash
# 验证配置
cargo run --package memoryos-gateway -- --validate-config
```

---

## 📚 文档

- [README.md](./README.md) - 项目概览
- [ALL_COMPLETE.md](./ALL_COMPLETE.md) - 完成报告
- [CHANGELOG.md](./CHANGELOG.md) - 变更日志
- [docs/API.md](./docs/API.md) - API 文档
- [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) - 架构设计
- [docs/DEPLOYMENT.md](./docs/DEPLOYMENT.md) - 部署指南

---

## 🆘 获取帮助

- **问题反馈**: https://github.com/BAI-LAB/MemoryOS/issues
- **文档**: https://bai-lab.github.io/MemoryOS/docs
- **Discord**: https://discord.gg/SqVj7QvZ

---

**快速参考版本**: 0.2.0  
**最后更新**: 2026-02-17
