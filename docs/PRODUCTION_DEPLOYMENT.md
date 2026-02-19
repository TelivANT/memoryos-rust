# MemoryOS-Rust 生产部署指南

**版本**: v0.3.0  
**更新**: 2026-02-19

---

## 📋 概述

本指南描述如何将 MemoryOS-Rust 部署到生产环境，包括架构配置、迁移步骤、监控设置和故障排查。

## 🏗️ 架构概览

### 统一向量存储架构 (v0.3.0)

```
┌─────────────────────────────────────────────────────────┐
│                    Client Applications                   │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│                  MemoryOS Gateway                        │
│  • HTTP API (Axum)                                       │
│  • Request Routing                                       │
│  • Authentication                                        │
└─────────────────────┬───────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   Qdrant     │ │   Chroma     │ │  Pinecone    │
│ (Vector DB)  │ │ (Vector DB)  │ │ (Vector DB)  │
├──────────────┤ ├──────────────┤ ├──────────────┤
│ Short-Term   │ │ Short-Term   │ │ Short-Term   │
│ Mid-Term     │ │ Mid-Term     │ │ Mid-Term     │
│ Long-Term    │ │ Long-Term    │ │ Long-Term    │
└──────────────┘ └──────────────┘ └──────────────┘
        │             │             │
        └─────────────┼─────────────┘
                      │
        ┌─────────────┴─────────────┐
        ▼                           ▼
┌──────────────┐           ┌──────────────┐
│    Redis     │           │     NATS     │
│ (Optional)   │           │  (Optional)  │
├──────────────┤           ├──────────────┤
│ • Lock       │           │ • Message    │
│ • Cache      │           │   Queue      │
│ • Session    │           │ • Pub/Sub    │
└──────────────┘           └──────────────┘
```

### 关键变化 (v0.2.x → v0.3.0)

**之前 (v0.2.x)**:
- 短期记忆: Redis/NATS (易失性)
- 中期记忆: 向量数据库
- 长期记忆: 向量数据库

**现在 (v0.3.0)**:
- **所有记忆层**: 向量数据库（持久化）
- Redis/NATS: 仅用于协调（锁、缓存、消息队列）

**优势**:
- ✅ 数据持久化（无丢失风险）
- ✅ 语义搜索（所有记忆层）
- ✅ 统一架构（简化运维）
- ✅ 更好的扩展性

---

## 🚀 快速部署

### 方法 1: Docker Compose（推荐）

```bash
# 克隆仓库
git clone https://github.com/your-org/memoryos-rust.git
cd memoryos-rust

# 配置环境变量
cp .env.example .env
# 编辑 .env 文件

# 启动所有服务
docker-compose up -d

# 检查状态
docker-compose ps

# 查看日志
docker-compose logs -f gateway
```

### 方法 2: Kubernetes

```bash
# 应用配置
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secrets.yaml

# 部署向量数据库
kubectl apply -f k8s/qdrant.yaml

# 部署 MemoryOS
kubectl apply -f k8s/gateway.yaml
kubectl apply -f k8s/worker.yaml

# 检查状态
kubectl get pods -n memoryos
```

---

## ⚙️ 配置指南

### 1. 向量数据库选择

#### Qdrant（推荐用于自托管）

**优势**:
- 开源免费
- 高性能
- 易于部署
- 完整的 RBAC

**配置**:
```toml
[vector_storage]
type = "qdrant"
url = "http://qdrant:6333"
```

**Docker 部署**:
```bash
docker run -d \
  --name qdrant \
  -p 6333:6333 \
  -p 6334:6334 \
  -v qdrant_storage:/qdrant/storage \
  qdrant/qdrant
```

#### Chroma（推荐用于轻量级部署）

**优势**:
- 轻量级
- 易于使用
- 适合小规模

**配置**:
```toml
[vector_storage]
type = "chroma"
url = "http://chroma:8000"
```

**Docker 部署**:
```bash
docker run -d \
  --name chroma \
  -p 8000:8000 \
  -v chroma_data:/chroma/data \
  chromadb/chroma
```

#### Pinecone（推荐用于云端）

**优势**:
- 完全托管
- 自动扩展
- 高可用

**配置**:
```toml
[vector_storage]
type = "pinecone"
api_key = "your-api-key"
environment = "us-east-1-aws"
```

### 2. Redis 配置（可选）

用于锁、缓存、会话管理。

```toml
[redis]
url = "redis://redis:6379"
pool_size = 10
```

**Docker 部署**:
```bash
docker run -d \
  --name redis \
  -p 6379:6379 \
  -v redis_data:/data \
  redis:7-alpine \
  redis-server --appendonly yes
```

### 3. NATS 配置（可选）

用于消息队列、事件总线。

```toml
[nats]
url = "nats://nats:4222"
```

**Docker 部署**:
```bash
docker run -d \
  --name nats \
  -p 4222:4222 \
  -p 8222:8222 \
  nats:latest
```

### 4. Gateway 配置

```toml
[server]
host = "0.0.0.0"
port = 8080

[vector_storage]
type = "qdrant"
url = "http://qdrant:6333"

[llm]
provider = "openai"
api_key = "sk-..."
model = "gpt-4"

[memory]
short_term_limit = 10
mid_term_heat_threshold = 5.0
long_term_knowledge_capacity = 100
```

---

## 🔄 迁移指南

### 从 v0.2.x 迁移到 v0.3.0

#### 步骤 1: 备份数据

```bash
# 备份 Redis 数据（如果使用）
redis-cli --rdb /backup/dump.rdb

# 备份向量数据库
# Qdrant
curl -X POST http://localhost:6333/collections/snapshot

# 备份配置
cp config.toml config.toml.backup
```

#### 步骤 2: 更新配置

```toml
# 旧配置 (v0.2.x)
[short_term_storage]
type = "redis"
url = "redis://localhost:6379"

# 新配置 (v0.3.0) - 移除此部分
# 短期记忆现在使用 vector_storage
```

#### 步骤 3: 迁移短期记忆数据（可选）

如果需要保留 Redis 中的短期记忆：

```bash
# 运行迁移脚本
cargo run --bin migrate_short_term \
  --redis-url redis://localhost:6379 \
  --qdrant-url http://localhost:6333
```

#### 步骤 4: 部署新版本

```bash
# 停止旧版本
docker-compose down

# 拉取新版本
git pull origin main
git checkout v0.3.0

# 启动新版本
docker-compose up -d

# 验证
curl http://localhost:8080/health
```

#### 步骤 5: 验证迁移

```bash
# 测试短期记忆
curl -X POST http://localhost:8080/api/v1/memory/add \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test_user",
    "message": {
      "role": "user",
      "content": "Hello"
    }
  }'

# 获取短期记忆
curl http://localhost:8080/api/v1/memory/short-term/test_user
```

---

## 📊 监控设置

### 1. 健康检查

```bash
# Gateway 健康检查
curl http://localhost:8080/health

# Qdrant 健康检查
curl http://localhost:6333/health

# Redis 健康检查
redis-cli ping
```

### 2. Prometheus 指标

Gateway 暴露 Prometheus 指标：

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'memoryos-gateway'
    static_configs:
      - targets: ['gateway:8080']
    metrics_path: '/metrics'
```

**关键指标**:
- `memoryos_requests_total` - 总请求数
- `memoryos_request_duration_seconds` - 请求延迟
- `memoryos_memory_operations_total` - 记忆操作数
- `memoryos_vector_db_latency_seconds` - 向量数据库延迟

### 3. 日志收集

使用结构化日志（JSON 格式）：

```toml
[logging]
level = "info"
format = "json"
```

**日志聚合**（推荐 ELK Stack）:
```yaml
# filebeat.yml
filebeat.inputs:
  - type: container
    paths:
      - '/var/lib/docker/containers/*/*.log'
    processors:
      - add_docker_metadata: ~

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
```

### 4. 告警规则

```yaml
# alertmanager.yml
groups:
  - name: memoryos
    rules:
      - alert: HighLatency
        expr: memoryos_request_duration_seconds > 1
        for: 5m
        annotations:
          summary: "High request latency"
      
      - alert: VectorDBDown
        expr: up{job="qdrant"} == 0
        for: 1m
        annotations:
          summary: "Vector database is down"
```

---

## 🔒 安全最佳实践

### 1. API 认证

```toml
[auth]
enabled = true
api_key_header = "X-API-Key"
```

### 2. TLS/SSL

```toml
[server]
tls_enabled = true
tls_cert = "/certs/server.crt"
tls_key = "/certs/server.key"
```

### 3. 网络隔离

```yaml
# docker-compose.yml
networks:
  frontend:
    driver: bridge
  backend:
    driver: bridge
    internal: true  # 不暴露到外部

services:
  gateway:
    networks:
      - frontend
      - backend
  
  qdrant:
    networks:
      - backend  # 仅内部访问
```

### 4. 密钥管理

使用环境变量或密钥管理服务：

```bash
# 使用 Kubernetes Secrets
kubectl create secret generic memoryos-secrets \
  --from-literal=openai-api-key=sk-... \
  --from-literal=pinecone-api-key=...
```

---

## 🚨 故障排查

### 问题 1: 向量数据库连接失败

**症状**: `Failed to connect to Qdrant`

**检查**:
```bash
# 检查服务是否运行
docker ps | grep qdrant

# 检查网络连接
curl http://qdrant:6333/health

# 检查日志
docker logs qdrant
```

**解决**:
```bash
# 重启服务
docker restart qdrant

# 检查配置
cat config.toml | grep vector_storage
```

### 问题 2: 高延迟

**症状**: 请求响应时间 > 1s

**检查**:
```bash
# 运行性能测试
cargo run --release --package memoryos-benchmarks --bin perf_test

# 检查资源使用
docker stats
```

**解决**:
- 增加向量数据库资源
- 启用缓存
- 优化查询参数

### 问题 3: 内存泄漏

**症状**: 内存使用持续增长

**检查**:
```bash
# 监控内存使用
docker stats gateway

# 检查连接池
curl http://localhost:8080/metrics | grep pool
```

**解决**:
- 检查连接池配置
- 重启服务
- 升级到最新版本

---

## 📈 性能优化

### 1. 连接池配置

```toml
[vector_storage]
pool_size = 20
max_idle = 10
connection_timeout = 30
```

### 2. 缓存策略

```toml
[cache]
enabled = true
ttl = 300  # 5 minutes
max_size = 1000
```

### 3. 批量操作

```rust
// 批量添加消息
storage.add_short_term_messages_batch(&user_id, messages).await?;
```

### 4. 异步处理

```toml
[worker]
enabled = true
concurrency = 10
queue_size = 1000
```

---

## 📚 相关文档

- [Integration Testing](./INTEGRATION_TESTING.md) - 集成测试指南
- [Performance Benchmarking](./PERFORMANCE_BENCHMARKING.md) - 性能测试指南
- [Architecture Improvement](./ARCHITECTURE_IMPROVEMENT.md) - 架构改进说明
- [Vector Databases Guide](./VECTOR_DATABASES.md) - 向量数据库配置

---

## 🤝 支持

遇到问题？

- 📖 查看文档: [docs/](./docs/)
- 🐛 提交 Issue: [GitHub Issues](https://github.com/your-org/memoryos-rust/issues)
- 💬 加入讨论: [Discord](https://discord.gg/...)

---

## 📝 更新日志

### v0.3.0 (2026-02-19)

**重大变更**:
- ✅ 统一向量存储架构
- ✅ 短期记忆迁移到向量数据库
- ✅ 移除 ShortTermStorage trait
- ✅ 简化 MemoryManager 构造

**新增功能**:
- ✅ 集成测试基础设施
- ✅ 性能基准测试工具
- ✅ 生产部署指南

**性能提升**:
- 数据持久化（无丢失风险）
- 语义搜索（所有记忆层）
- 统一架构（简化运维）

---

**部署愉快！** 🚀
