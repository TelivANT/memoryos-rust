# 部署文档

**版本**: v0.13.0
**仓库**: [TelivANT/memoryos-rust](https://github.com/TelivANT/memoryos-rust)

---

## 目录

- [系统要求](#系统要求)
- [单节点部署](#单节点部署)
- [Docker 部署](#docker-部署)
- [配置说明](#配置说明)
- [生产部署](#生产部署)
- [K8s 集群部署](#k8s-集群部署)
- [Admin 服务部署](#admin-服务部署)
- [监控和日志](#监控和日志)
- [故障排查](#故障排查)

---

## 系统要求

### 最低要求（开发/测试）

| 项目 | 要求 |
|------|------|
| 操作系统 | Linux / macOS / Windows |
| CPU | 2 核 |
| 内存 | 2 GB |
| 磁盘 | 1 GB |
| Rust | 1.75+ stable |
| Docker | 20.10+ |
| Docker Compose | 2.0+ |

### 推荐配置（生产）

| 项目 | 要求 |
|------|------|
| 操作系统 | Linux (Ubuntu 22.04+) |
| CPU | 4 核 |
| 内存 | 8 GB |
| 磁盘 | 20 GB SSD |

### 依赖服务

| 服务 | 用途 | 默认端口 |
|------|------|----------|
| **Redis** | 分布式协调（Session、锁、缓存、限流） | 6379 |
| **Qdrant** | 向量存储（STM/MTM/LTM、API Key、IP 封禁） | 6333 (gRPC) / 6334 (HTTP) |
| **LLM API** | OpenAI / Deepseek / Ollama 等 | — |

---

## 单节点部署

### 1. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. 克隆项目

```bash
git clone https://github.com/TelivANT/memoryos-rust.git
cd memoryos-rust
```

### 3. 启动依赖服务

```bash
docker compose up -d redis qdrant
```

### 4. 配置

```bash
cp config.example.toml config.toml
# 编辑 config.toml，填入 LLM API Key
```

完整配置参考下方 [配置说明](#配置说明)。

### 5. 编译运行

```bash
# Release 构建（推荐生产使用）
cargo build --release
./target/release/memoryos-gateway

# 或开发模式
cargo run --bin memoryos-gateway
```

### 6. 验证

```bash
curl http://localhost:8080/health/status
# {"mode":"ready","redis":"up","qdrant":"up"}
```

---

## Docker 部署

### 使用 Docker Compose（推荐）

```bash
# 构建并启动所有服务
docker compose up -d --build

# 查看服务状态
docker compose ps

# 查看日志
docker compose logs -f memoryos-gateway
```

### 仅构建镜像

```bash
docker build -t memoryos-gateway:latest .
```

Dockerfile 使用 `rust:slim-bookworm` 基础镜像，采用 `cargo-chef` 分层构建优化缓存。

### 运行镜像

```bash
docker run -d \
  --name memoryos-gateway \
  -p 8080:8080 \
  -v $(pwd)/config.toml:/app/config.toml \
  -e RUST_LOG=info \
  memoryos-gateway:latest
```

---

## 配置说明

配置文件 `config.toml`，格式与 `config.example.toml` 一致。

### 完整配置参考

```toml
[server]
host = "0.0.0.0"
port = 8080
worker_threads = 4
timeout_seconds = 60

[llm]
default_provider = "openai"
default_model = "gpt-4o-mini"

# LLM Provider 配置（支持 10 种）
# 每个 provider 需要 type、base_url、api_key（或 api_key_env）
[llm.providers.openai]
type = "openai"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-key"
# api_key_env = "OPENAI_API_KEY"  # 从环境变量读取

# 示例：添加更多 provider
# [llm.providers.ollama]
# type = "ollama"
# base_url = "http://localhost:11434/v1"
# api_key = ""

# [llm.providers.deepseek]
# type = "deepseek"
# base_url = "https://api.deepseek.com/v1"
# api_key_env = "DEEPSEEK_API_KEY"

[storage.redis]
url = "redis://localhost:6379"
ttl_seconds = 3600
max_messages = 20

[storage.vector]
url = "http://localhost:6334"

# Embedding 配置（可选）
# 不配置则使用内置 fallback embedding
[embedding]
# api_key = "sk-your-embedding-key"
# base_url = "https://api.openai.com/v1"
# model = "text-embedding-3-small"

# 认证配置
[auth]
enabled = false
# admin_keys = ["admin-secret-key"]
# api_keys = ["user-api-key-1"]
# use_redis_store = false
```

### 支持的 LLM Provider

| Provider | type 值 | 说明 |
|----------|---------|------|
| OpenAI | `openai` | GPT-4o / GPT-4o-mini 等 |
| Claude | `claude` | Anthropic Claude 系列 |
| Gemini | `gemini` | Google Gemini 系列 |
| Ollama | `ollama` | 本地模型 |
| Deepseek | `deepseek` | Deepseek 系列 |
| OpenRouter | `openrouter` | OpenRouter 聚合 |
| Azure OpenAI | `azure-openai` | Azure 托管 OpenAI |
| Cohere | `cohere` | Cohere 系列 |
| Groq | `groq` | Groq 加速推理 |
| Mistral | `mistral` | Mistral 系列 |

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `RUST_LOG` | 日志级别 | `info` |
| `MEMORYOS_ASYNC_MEMORY_PIPELINE` | 启用异步记忆管道 | `false` |
| `MEMORYOS_CONFIG_HOT_RELOAD` | 配置热重载 | `true` |
| `MEMORYOS_CONNECTOR_SECRET` | 连接器加密密钥 | 内置默认值 |

### 配置热重载

默认启用，每 5 秒检查 `config.toml` 变更并自动重载。禁用方式：

```bash
MEMORYOS_CONFIG_HOT_RELOAD=false ./target/release/memoryos-gateway
```

---

## 生产部署

### 安全清单

- [ ] `auth.enabled = true`
- [ ] 设置强 admin_keys 和 api_keys
- [ ] Redis 设置密码：`url = "redis://:password@host:6379"`
- [ ] Qdrant 启用 API Key 认证
- [ ] 使用 HTTPS（反向代理 TLS 终结）
- [ ] 设置 `MEMORYOS_CONNECTOR_SECRET` 环境变量
- [ ] 限制 Admin 服务 (`:9090`) 仅内网访问

### 反向代理（Nginx 示例）

```nginx
upstream memoryos {
    server 127.0.0.1:8080;
}

server {
    listen 443 ssl;
    server_name api.example.com;

    ssl_certificate /etc/ssl/certs/cert.pem;
    ssl_certificate_key /etc/ssl/private/key.pem;

    location / {
        proxy_pass http://memoryos;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # SSE 支持（流式聊天）
        proxy_buffering off;
        proxy_cache off;
    }
}
```

---

## K8s 集群部署

### 部署清单示例

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: memoryos-gateway
spec:
  replicas: 3
  selector:
    matchLabels:
      app: memoryos-gateway
  template:
    metadata:
      labels:
        app: memoryos-gateway
    spec:
      containers:
        - name: gateway
          image: memoryos-gateway:latest
          ports:
            - containerPort: 8080
          env:
            - name: RUST_LOG
              value: "info"
          volumeMounts:
            - name: config
              mountPath: /app/config.toml
              subPath: config.toml
          resources:
            requests:
              cpu: "500m"
              memory: "512Mi"
            limits:
              cpu: "2"
              memory: "2Gi"
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /health/status
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 10
      volumes:
        - name: config
          configMap:
            name: memoryos-config
---
apiVersion: v1
kind: Service
metadata:
  name: memoryos-gateway
spec:
  selector:
    app: memoryos-gateway
  ports:
    - port: 8080
      targetPort: 8080
  type: ClusterIP
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: memoryos-gateway
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: memoryos-gateway
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

---

## Admin 服务部署

Admin 服务（`:9090`）提供内网管理功能，包括用户管理、租户管理、RBAC、审计查看和系统监控。

```bash
cargo run --bin memoryos-admin
```

> Admin 服务应仅在内网/VPN 中暴露，不对公网开放。

---

## 监控和日志

### Prometheus 指标

Gateway 暴露 `/metrics` 端点，Prometheus 格式。

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'memoryos'
    static_configs:
      - targets: ['localhost:8080']
    scrape_interval: 15s
```

### 日志

通过 `RUST_LOG` 环境变量控制日志级别：

```bash
# 仅 info 级别
RUST_LOG=info ./target/release/memoryos-gateway

# 详细调试
RUST_LOG=debug ./target/release/memoryos-gateway

# 按模块过滤
RUST_LOG=memoryos_gateway=debug,memoryos_core=info ./target/release/memoryos-gateway
```

### 审计日志

安全审计日志持久化到 `~/.memoryos/audit.jsonl`，可通过 API 查询：

```bash
curl -X POST http://localhost:8080/v1/security/audit/logs \
  -H "Content-Type: application/json" \
  -d '{"limit": 50}'
```

---

## 故障排查

### Redis 连接失败

```
ERROR: Redis connection failed
```

- 确认 Redis 已启动：`docker compose ps redis`
- 确认 URL 正确：`redis://localhost:6379`
- 如果使用密码：`redis://:password@localhost:6379`

系统会自动降级运行（无短期记忆，核心功能不受影响）。

### Qdrant 连接失败

```
ERROR: Qdrant connection failed
```

- 确认 Qdrant 已启动：`docker compose ps qdrant`
- 确认 URL 正确：`http://localhost:6334`（HTTP 端口）
- Qdrant Web UI：`http://localhost:6333/dashboard`

系统会自动降级运行（无向量搜索，核心功能不受影响）。

### LLM API 错误

```
ERROR: LLM request failed: 401 Unauthorized
```

- 确认 API Key 正确
- 确认 `base_url` 正确
- 确认 `default_provider` 与 `[llm.providers.xxx]` 的 key 匹配

### 端口被占用

```
ERROR: Failed to bind port
```

- 检查端口占用：`lsof -i :8080`
- 修改 `config.toml` 中的 `port`
