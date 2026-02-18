# 快速开始

5 分钟快速上手 MemoryOS-Rust

---

## 📋 环境要求

- **Rust**: 1.93+
- **Docker**: 20.10+
- **Docker Compose**: 2.0+

---

## 🚀 快速开始

### 1. 启动依赖服务

```bash
# 启动 Redis 和 Qdrant
docker-compose up -d redis qdrant

# 验证服务
docker ps
```

### 2. 配置

```bash
# 复制配置模板
cp config.example.toml config.toml

# 编辑配置，填入 API Key
vim config.toml
```

**最小配置**:
```toml
[server]
host = "0.0.0.0"
port = 8080

[llm]
provider = "openai"
api_key = "sk-your-key"  # 替换为你的 API Key
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"

[redis]
url = "redis://localhost:6379"

[qdrant]
url = "http://localhost:6334"
```

### 3. 运行

```bash
# 开发模式
cargo run

# 生产模式
cargo run --release
```

### 4. 测试

```bash
# 健康检查
curl http://localhost:8080/health/status

# 聊天测试
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ]
  }'
```

---

## 🔧 配置说明

### LLM 提供商

支持 7 种 LLM 提供商：

```toml
# OpenAI
[llm]
provider = "openai"
api_key = "sk-..."
base_url = "https://api.openai.com/v1"

# Gemini
[llm]
provider = "gemini"
api_key = "..."
base_url = "https://generativelanguage.googleapis.com/v1beta"

# Claude
[llm]
provider = "claude"
api_key = "..."
base_url = "https://api.anthropic.com/v1"

# Ollama (本地)
[llm]
provider = "ollama"
api_key = ""
base_url = "http://localhost:11434/v1"
```

### Embedding 配置（可选）

```bash
# 环境变量方式
export OPENAI_API_KEY="sk-your-key"
export EMBEDDING_MODEL="text-embedding-3-small"

# 不配置则使用 fallback embedding
```

---

## 📊 验证部署

### 1. 健康检查

```bash
# Liveness
curl http://localhost:8080/health/live

# Readiness
curl http://localhost:8080/health/ready

# 详细状态
curl http://localhost:8080/health/status
```

**正常响应**:
```json
{
  "mode": "ready",
  "redis": "up",
  "qdrant": "up"
}
```

### 2. 聊天测试

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "What is MemoryOS?"}
    ],
    "temperature": 0.7
  }'
```

### 3. 记忆测试

```bash
# 添加记忆
curl -X POST http://localhost:8080/v1/memory/add \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test_user",
    "message": {
      "role": "user",
      "content": "My name is Alice"
    }
  }'

# 检索记忆
curl "http://localhost:8080/v1/memory/retrieve?user_id=test_user&query=name"

# 获取用户画像
curl "http://localhost:8080/v1/memory/profile?user_id=test_user"
```

---

## ❓ 常见问题

### 1. Redis 连接失败

**错误**: `Failed to connect to Redis`

**解决**:
```bash
# 检查 Redis 是否运行
docker ps | grep redis

# 重启 Redis
docker-compose restart redis

# 检查配置
cat config.toml | grep redis
```

### 2. Qdrant 连接失败

**错误**: `Failed to connect to Qdrant`

**解决**:
```bash
# 检查 Qdrant 是否运行
docker ps | grep qdrant

# 重启 Qdrant
docker-compose restart qdrant

# 访问 Qdrant UI
open http://localhost:6333/dashboard
```

### 3. LLM API 调用失败

**错误**: `OpenAI API error 401`

**解决**:
```bash
# 检查 API Key
echo $OPENAI_API_KEY

# 或检查配置文件
cat config.toml | grep api_key

# 测试 API Key
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```

### 4. 编译失败

**错误**: `error: could not compile`

**解决**:
```bash
# 清理并重新编译
cargo clean
cargo build

# 更新依赖
cargo update
```

### 5. 端口被占用

**错误**: `Address already in use`

**解决**:
```bash
# 查找占用端口的进程
lsof -i :8080

# 杀死进程
kill -9 <PID>

# 或修改配置文件端口
vim config.toml
# 修改 port = 8081
```

---

## 🎯 下一步

- 📖 [架构设计](./ARCHITECTURE.md) - 了解系统架构
- 📡 [API 文档](./API.md) - 查看完整 API
- 🛠️ [开发指南](./DEVELOPMENT.md) - 参与开发
- 🚀 [部署指南](./DEPLOYMENT.md) - 生产部署

---

## 🆘 获取帮助

- **GitHub Issues**: https://github.com/BAI-LAB/MemoryOS/issues
- **Discord**: https://discord.gg/SqVj7QvZ
- **文档**: https://bai-lab.github.io/MemoryOS/docs
