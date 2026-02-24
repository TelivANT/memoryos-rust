# 快速开始

5 分钟快速上手 MemoryOS-Rust 单节点本地部署。

**版本**: v0.13.0
**仓库**: [TelivANT/memoryos-rust](https://github.com/TelivANT/memoryos-rust)

---

## 环境要求

- **Rust**: 1.75+（推荐 stable 最新版）
- **Docker**: 20.10+
- **Docker Compose**: 2.0+
- **LLM API Key**: OpenAI / Deepseek / Ollama 等任意一个

---

## 第一步：启动依赖服务

```bash
# 启动 Redis（协调层）和 Qdrant（向量存储）
docker compose up -d redis qdrant

# 验证服务就绪
docker compose ps
# redis   ... Up (healthy)
# qdrant  ... Up (healthy)
```

默认端口：Redis `:6379`、Qdrant `:6333`（gRPC）/ `:6334`（HTTP）。

---

## 第二步：配置

```bash
# 复制配置模板
cp config.example.toml config.toml
```

编辑 `config.toml`，填入你的 LLM API Key：

```toml
[server]
host = "0.0.0.0"
port = 8080
worker_threads = 4
timeout_seconds = 60

[llm]
default_provider = "openai"
default_model = "gpt-4o-mini"

[llm.providers.openai]
type = "openai"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-key"

[storage.redis]
url = "redis://localhost:6379"
ttl_seconds = 3600
max_messages = 20

[storage.vector]
url = "http://localhost:6334"

[auth]
enabled = false
```

> **使用 Ollama？** 将 provider 改为 ollama：
> ```toml
> [llm]
> default_provider = "ollama"
> default_model = "llama3"
>
> [llm.providers.ollama]
> type = "ollama"
> base_url = "http://localhost:11434/v1"
> api_key = ""
> ```

---

## 第三步：启动 Gateway

```bash
# 编译并运行
cargo run --bin memoryos-gateway
```

看到以下日志表示启动成功：
```
INFO memoryos_gateway: MemoryOS Gateway listening on 0.0.0.0:8080
```

---

## 第四步：验证

### 健康检查

```bash
curl http://localhost:8080/health
# {"status":"ok"}

curl http://localhost:8080/health/status
# {"mode":"ready","redis":"up","qdrant":"up"}
```

### 添加记忆

```bash
curl -X POST http://localhost:8080/v1/memory/add \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test_user",
    "role": "user",
    "content": "My name is Alice and I work as a data scientist at Google."
  }'
# {"status":"ok"}
```

### 检索记忆

```bash
curl -X POST http://localhost:8080/v1/memory/retrieve \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test_user",
    "query": "What do you know about me?"
  }'
```

### 带记忆的聊天

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "user", "content": "What is my name and where do I work?"}
    ]
  }'
```

MemoryOS 会自动将之前存储的记忆注入到 LLM 上下文中，模型会知道你是 Alice，在 Google 做数据科学。

---

## 可选：启用认证

生产环境必须启用认证：

```toml
[auth]
enabled = true
admin_keys = ["admin-secret-key-change-me"]
api_keys = ["user-api-key-1"]
```

之后所有 `/v1/*` 请求需要携带 API Key：

```bash
curl -X POST http://localhost:8080/v1/memory/add \
  -H "Authorization: Bearer user-api-key-1" \
  -H "Content-Type: application/json" \
  -d '{"user_id": "test_user", "role": "user", "content": "Hello"}'
```

---

## 可选：Docker 一键部署

```bash
# 构建并启动所有服务
docker compose up -d --build

# 查看日志
docker compose logs -f memoryos-gateway
```

---

## 下一步

- [API 文档](API.md) — 完整的 47 个端点说明
- [部署文档](DEPLOYMENT.md) — 生产部署、K8s、监控
- [用户手册](USER_MANUAL.md) — 功能使用指南
- [架构设计](ARCHITECTURE.md) — 系统架构详解
