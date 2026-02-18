# 部署文档

**版本**: v0.1.0  
**更新时间**: 2026-02-17

---

## 📋 目录

- [系统要求](#系统要求)
- [快速开始](#快速开始)
- [配置说明](#配置说明)
- [生产部署](#生产部署)
- [Docker 部署](#docker-部署)
- [单机升级到集群](#单机升级到集群)
- [监控和日志](#监控和日志)
- [故障排查](#故障排查)

---

## 系统要求

### 最低要求
- **操作系统**: Linux / macOS / Windows
- **CPU**: 2 核
- **内存**: 2 GB
- **磁盘**: 1 GB

### 推荐配置
- **操作系统**: Linux (Ubuntu 22.04+)
- **CPU**: 4 核
- **内存**: 4 GB
- **磁盘**: 10 GB SSD

### 依赖服务
- **Redis** (可选): 用于短期记忆存储
- **Qdrant** (可选): 用于向量存储
- **OpenAI API**: 或其他兼容的 LLM 服务

---

## 快速开始

### 1. 安装 Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. 克隆项目
```bash
git clone https://github.com/BAI-LAB/MemoryOS.git
cd MemoryOS/MemoryOS-Rust
```

### 3. 配置环境
```bash
# 复制配置文件
cp config.example.toml config.toml

# 编辑配置
vim config.toml
```

### 4. 编译运行
```bash
# 开发模式
cargo run --package memoryos-gateway

# 可选 Worker（仅异步记忆场景需要，单节点可不启）
# export MEMORYOS_ASYNC_MEMORY_PIPELINE=true
# cargo run --package memoryos-worker

# 生产模式
cargo build --release
./target/release/memoryos-gateway
```

### 5. 验证
```bash
curl http://localhost:8080/health/status
```

---

## 配置说明

### 配置文件位置
- 默认: `./config.toml`
- 环境变量: `MEMORYOS_CONFIG=/path/to/config.toml`

### 基础配置

```toml
[server]
host = "0.0.0.0"
port = 8080
worker_threads = 4
timeout_seconds = 60

[llm]
provider = "openai"
api_key = "sk-..."
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
# provider 可选:
# openai / gemini / claude / ollama / deepseek / openrouter / azure-openai

[redis]
url = "redis://localhost:6379"
ttl_seconds = 3600
max_messages = 20

[qdrant]
url = "http://localhost:6334"
```

### 环境变量覆盖

```bash
# 服务器配置
export MEMORYOS__SERVER__HOST=0.0.0.0
export MEMORYOS__SERVER__PORT=8080

# LLM 配置
export MEMORYOS__LLM__PROVIDER=openai
export MEMORYOS__LLM__API_KEY=sk-...
export MEMORYOS__LLM__BASE_URL=https://api.openai.com/v1
export MEMORYOS__LLM__MODEL=gpt-4o-mini

# Redis 配置
export MEMORYOS__REDIS__URL=redis://localhost:6379

# Qdrant 配置
export MEMORYOS__QDRANT__URL=http://localhost:6334

# Worker Redis Stream 配置（可选）
export MEMORYOS_WORKER_STREAM=chat_log
export MEMORYOS_WORKER_GROUP=memoryos-workers
export MEMORYOS_WORKER_CONSUMER=worker-1
export MEMORYOS_WORKER_BATCH_SIZE=32
export MEMORYOS_WORKER_BLOCK_MS=5000

# 异步记忆开关（gateway）
# false: 默认同步写 memory
# true: 优先写入 chat_log，由 worker 异步消费
export MEMORYOS_ASYNC_MEMORY_PIPELINE=true

# 可选：worker 监控轮询周期（秒）
# 仅在异步记忆开关=true时生效；默认 30 秒
export MEMORYOS_WORKER_MONITOR_INTERVAL_SECS=30
```

说明：
- 单节点默认不需要部署 worker。
- 仅在 `MEMORYOS_ASYNC_MEMORY_PIPELINE=true` 时，建议部署 worker 消费队列。
- 当开启异步记忆但未检测到活跃 worker consumer 时，gateway 会输出告警日志。

### Worker 事件格式（Redis Stream）

`memoryos-worker` 默认消费 `chat_log`，支持两种字段格式：

1. 扁平字段：`user_id` / `role` / `content` / `event_id` / `timestamp`
2. JSON 字段：`payload`（包含上述字段）

当 `MEMORYOS_ASYNC_MEMORY_PIPELINE=true` 时，gateway 在 `POST /v1/memory/add` 会发布事件到 `chat_log`。如果发布失败，gateway 自动回退到同步写入，避免数据丢失。

示例：

```bash
redis-cli XADD chat_log * \
  user_id test_user \
  role user \
  content "I like Rust and hiking" \
  event_id evt-worker-demo-1 \
  timestamp "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
```

---

## 生产部署

### 1. 编译优化版本
```bash
cargo build --release --package memoryos-gateway
```

### 2. 创建系统服务

**systemd 服务** (`/etc/systemd/system/memoryos.service`):
```ini
[Unit]
Description=MemoryOS Gateway
After=network.target redis.service

[Service]
Type=simple
User=memoryos
Group=memoryos
WorkingDirectory=/opt/memoryos
Environment="CONFIG_PATH=/opt/memoryos/config.toml"
ExecStart=/opt/memoryos/memoryos-gateway
Restart=always
RestartSec=10

# 安全配置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/memoryos/logs

[Install]
WantedBy=multi-user.target
```

### 3. 启动服务
```bash
# 创建用户
sudo useradd -r -s /bin/false memoryos

# 创建目录
sudo mkdir -p /opt/memoryos/logs
sudo chown -R memoryos:memoryos /opt/memoryos

# 复制文件
sudo cp target/release/memoryos-gateway /opt/memoryos/
sudo cp config.toml /opt/memoryos/

# 启动服务
sudo systemctl daemon-reload
sudo systemctl enable memoryos
sudo systemctl start memoryos

# 查看状态
sudo systemctl status memoryos
```

### 4. Nginx 反向代理

```nginx
upstream memoryos {
    server 127.0.0.1:8080;
}

server {
    listen 80;
    server_name api.example.com;

    # HTTPS 重定向
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;

    # 安全头
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    location / {
        proxy_pass http://memoryos;
        proxy_http_version 1.1;
        
        # 流式支持
        proxy_buffering off;
        proxy_cache off;
        
        # 头部转发
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # 超时设置
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # 健康检查
    location /health {
        proxy_pass http://memoryos;
        access_log off;
    }
}
```

---

## Docker 部署

### 1. Dockerfile

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

RUN cargo build --release --package memoryos-gateway

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/memoryos-gateway .
COPY config.example.toml config.toml

EXPOSE 8080

CMD ["./memoryos-gateway"]
```

### 2. 构建镜像
```bash
docker build -t memoryos:latest .
```

### 3. 运行容器
```bash
docker run -d \
  --name memoryos \
  -p 8080:8080 \
  -v $(pwd)/config.toml:/app/config.toml \
  -e OPENAI_API_KEY=sk-... \
  memoryos:latest
```

### 4. Docker Compose

```yaml
version: '3.8'

services:
  memoryos:
    build: .
    ports:
      - "8080:8080"
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - REDIS_URL=redis://redis:6379
      - QDRANT_URL=http://qdrant:6333
    volumes:
      - ./config.toml:/app/config.toml
      - ./logs:/app/logs
    depends_on:
      - redis
      - qdrant
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    restart: unless-stopped

  qdrant:
    image: qdrant/qdrant:latest
    ports:
      - "6333:6333"
    volumes:
      - qdrant_data:/qdrant/storage
    restart: unless-stopped

volumes:
  redis_data:
  qdrant_data:
```

**启动**:
```bash
docker-compose up -d
```

---

## 单机升级到集群

本节描述从单机（1 gateway + redis + qdrant，worker 可选）升级到集群（N gateway + M worker + shared redis/qdrant）的最小风险流程。

### 升级前检查

1. 备份配置与关键数据（Redis RDB、Qdrant snapshot）。
2. 确认所有节点使用同一份 `config` 模板（仅实例标识不同）。
3. 确认 worker 使用 consumer group 模式（同一 `MEMORYOS_WORKER_GROUP`，不同 `MEMORYOS_WORKER_CONSUMER`）。
4. 确认网关与 worker 都使用同一 Redis/Qdrant 地址。

### 推荐升级顺序

1. 先扩中间件（Redis/Qdrant）到目标容量。
2. 启动第二个 worker 实例，观察 `chat_log` 消费速率与 DLQ。
3. 启动第二个 gateway 实例，放入负载均衡（10% 流量灰度）。
4. 逐步扩容到目标实例数。

### 关键配置差异

单机到集群主要改动：

```bash
# Gateway/Worker 共用
export MEMORYOS__REDIS__URL=redis://redis-cluster:6379
export MEMORYOS__QDRANT__URL=http://qdrant-cluster:6334

# Worker（每个实例唯一 consumer）
export MEMORYOS_WORKER_STREAM=chat_log
export MEMORYOS_WORKER_GROUP=memoryos-workers
export MEMORYOS_WORKER_CONSUMER=worker-node-a   # 每实例不同
```

### 回滚策略

1. 从负载均衡摘除新 gateway 节点。
2. 停止新增 worker 节点（保留原单机 worker）。
3. 恢复升级前 `config` 与服务编排。
4. 必要时从 DLQ 回放失败事件（修复后回放，不直接丢弃）。
   - `DRY_RUN=1 COUNT=20 ./scripts/replay_dlq.sh`
   - `DRY_RUN=0 COUNT=20 ./scripts/replay_dlq.sh`

### 验收检查

1. `GET /health/ready` 在所有 gateway 返回 200。
2. `chat_log` 积压不持续增长。
3. `chat_log:dlq` 无异常增长。
4. 同一 `event_id` 不发生重复写入（检查幂等日志）。
5. 使用 `./scripts/smoke_async_pipeline.sh` 做端到端入队检查（需 `redis-cli`）。
6. 或执行 `./scripts/demo_async_pipeline.sh` 完成一键链路演示。

---

## 监控和日志

### 日志配置

**JSON 格式日志**:
```json
{
  "timestamp": "2026-02-17T14:30:00Z",
  "level": "INFO",
  "target": "memoryos_gateway",
  "message": "Request processed",
  "fields": {
    "method": "POST",
    "path": "/v1/chat/completions",
    "status": 200,
    "duration_ms": 150
  }
}
```

**日志级别**:
- `error` - 仅错误
- `warn` - 警告和错误
- `info` - 信息、警告和错误（推荐）
- `debug` - 调试信息
- `trace` - 详细跟踪

### 健康检查

**Kubernetes Liveness Probe**:
```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 10
```

**Kubernetes Readiness Probe**:
```yaml
readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
```

### Prometheus 监控

**指标端点** (待实现):
```
GET /metrics
```

**关键指标**:
- `http_requests_total` - 请求总数
- `http_request_duration_seconds` - 请求延迟
- `llm_requests_total` - LLM 请求数
- `memory_operations_total` - 记忆操作数
- `degraded_mode` - 降级模式状态

---

## 故障排查

### 服务无法启动

**检查端口占用**:
```bash
lsof -i :8080
```

**检查配置文件**:
```bash
cat config.toml
```

**查看日志**:
```bash
journalctl -u memoryos -f
```

### Redis 连接失败

**测试连接**:
```bash
redis-cli -h localhost -p 6379 ping
```

**服务会自动降级**:
- 检查响应头 `X-Degraded-Mode: true`
- 使用内存存储替代

### Qdrant 连接失败

**测试连接**:
```bash
curl http://localhost:6333/health
```

**服务会自动降级**:
- 向量搜索不可用
- 其他功能正常

### LLM 调用失败

**检查 API Key**:
```bash
echo $OPENAI_API_KEY
```

**测试 API**:
```bash
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```

**查看错误日志**:
```bash
grep "LLM error" logs/memoryos.log
```

### 性能问题

**检查资源使用**:
```bash
top -p $(pgrep memoryos-gateway)
```

**检查连接数**:
```bash
netstat -an | grep :8080 | wc -l
```

**优化建议**:
1. 增加 worker 线程数
2. 启用连接池
3. 添加缓存层
4. 使用负载均衡

---

## 安全建议

1. **API Key 管理**:
   - 使用环境变量或密钥管理服务
   - 定期轮换密钥
   - 不要提交到版本控制

2. **网络安全**:
   - 使用 HTTPS
   - 配置防火墙
   - 限制访问 IP

3. **访问控制**:
   - 添加认证中间件
   - 实现速率限制
   - 记录审计日志

4. **数据安全**:
   - 加密敏感数据
   - 定期备份
   - 实现数据清理策略

---

## 备份和恢复

### Redis 备份
```bash
# 手动备份
redis-cli SAVE

# 自动备份 (crontab)
0 2 * * * redis-cli SAVE && cp /var/lib/redis/dump.rdb /backup/
```

### Qdrant 备份
```bash
# 创建快照
curl -X POST http://localhost:6333/collections/memoryos/snapshots

# 下载快照
curl http://localhost:6333/collections/memoryos/snapshots/{snapshot_name} \
  -o backup.snapshot
```

---

## 升级指南

### 1. 备份数据
```bash
# 备份配置
cp config.toml config.toml.backup

# 备份数据库
redis-cli SAVE
```

### 2. 停止服务
```bash
sudo systemctl stop memoryos
```

### 3. 更新代码
```bash
git pull
cargo build --release
```

### 4. 更新配置
```bash
# 检查配置变更
diff config.toml.backup config.example.toml
```

### 5. 启动服务
```bash
sudo systemctl start memoryos
```

### 6. 验证
```bash
curl http://localhost:8080/health/status
```

### 7. 单机到集群专项

执行专项步骤请参考：

- `docs/ops/upgrade_standalone_to_cluster.md`

---

**最后更新**: 2026-02-17
