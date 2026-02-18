# MemoryOS-Rust 用户手册

**版本**: 0.2.0  
**更新**: 2026-02-18

---

## 📖 目录

1. [快速开始](#快速开始)
2. [部署方式](#部署方式)
3. [API 使用](#api-使用)
4. [认证配置](#认证配置)
5. [运维管理](#运维管理)
6. [故障排查](#故障排查)

---

## 🚀 快速开始

### 本地开发

```bash
# 1. 克隆项目
git clone https://github.com/BAI-LAB/MemoryOS.git
cd MemoryOS/MemoryOS-Rust

# 2. 启动中间件
docker-compose up -d redis qdrant

# 3. 配置环境变量
export OPENAI_API_KEY="your-key"
export GEMINI_API_KEY="your-key"

# 4. 启动服务
cargo run --bin memoryos-gateway
```

访问: http://localhost:8080/health/status

---

## 🎯 部署方式

### 方式 1: Docker Compose（推荐用于开发）

```bash
# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f gateway

# 停止服务
docker-compose down
```

**优点**: 简单快速  
**缺点**: 不支持自动扩缩容

---

### 方式 2: K3s 集群（推荐用于生产）

```bash
# 一键部署
./scripts/deploy-full.sh
```

**包含内容**:
- ✅ K3s 集群
- ✅ Redis (持久化)
- ✅ Qdrant (持久化)
- ✅ Gateway (2 副本)
- ✅ 自动扩缩容
- ✅ 健康检查

**详细文档**: [K3S_DEPLOYMENT.md](./K3S_DEPLOYMENT.md)

---

### 方式 3: 手动部署

```bash
# 1. 编译
cargo build --release --bin memoryos-gateway

# 2. 配置
cp config.example.toml config.toml
vim config.toml

# 3. 启动
./target/release/memoryos-gateway
```

---

## 📡 API 使用

### 1. 健康检查（无需认证）

```bash
curl http://localhost:8080/health/status
```

**响应**:
```json
{
  "status": "ok",
  "timestamp": "2026-02-18T11:22:33Z",
  "version": "0.2.0"
}
```

---

### 2. 聊天 API

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-3-pro-preview",
    "messages": [
      {"role": "user", "content": "Hello"}
    ]
  }'
```

**响应**:
```json
{
  "id": "chatcmpl-xxx",
  "object": "chat.completion",
  "created": 1708234953,
  "model": "gemini-3-pro-preview",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello! How can I help you?"
    },
    "finish_reason": "stop"
  }]
}
```

---

### 3. 添加记忆

```bash
curl -X POST http://localhost:8080/v1/memory/add \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "alice_001",
    "role": "user",
    "content": "My name is Alice and I work as a data scientist"
  }'
```

**响应**:
```json
{
  "status": "ok",
  "message": "Memory added successfully"
}
```

---

### 4. 检索记忆

```bash
curl -X POST http://localhost:8080/v1/memory/retrieve \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "alice_001",
    "query": "What do you know about me?"
  }'
```

**响应**:
```json
{
  "short_term": [
    {
      "role": "user",
      "content": "My name is Alice...",
      "timestamp": "2026-02-18T11:00:00Z"
    }
  ],
  "long_term": {
    "profile": {
      "name": "Alice",
      "occupation": "data scientist"
    },
    "knowledge": [
      "Works as a data scientist"
    ]
  }
}
```

---

## 🔒 认证配置

### 静态 API Key（适合小规模）

编辑 `config.toml`:

```toml
[auth]
enabled = true
use_redis_store = false
admin_key = "admin-secret-key"
api_keys = [
    "user-key-1",
    "user-key-2"
]
```

---

### 动态 API Key（适合大规模）

```toml
[auth]
enabled = true
use_redis_store = true  # 实际使用 Qdrant
admin_key = "admin-secret-key"
```

**创建 API Key**:
```bash
curl -X POST http://localhost:8080/admin/keys \
  -H "Authorization: Bearer admin-secret-key" \
  -H "Content-Type: application/json" \
  -d '{
    "api_key": "user-alice-key-abc123",
    "user_id": "alice@company.com",
    "description": "Alice dev key",
    "permissions": ["chat", "memory"]
  }'
```

**删除 API Key**:
```bash
curl -X DELETE http://localhost:8080/admin/keys \
  -H "Authorization: Bearer admin-secret-key" \
  -H "Content-Type: application/json" \
  -d '{
    "api_key": "user-alice-key-abc123"
  }'
```

**详细文档**: [AUTH.md](./AUTH.md)

---

## 🛠️ 运维管理

### Docker Compose 环境

```bash
# 查看状态
docker-compose ps

# 查看日志
docker-compose logs -f gateway
docker-compose logs -f redis
docker-compose logs -f qdrant

# 重启服务
docker-compose restart gateway

# 更新配置
vim config.toml
docker-compose restart gateway
```

---

### K3s 环境

```bash
# 查看所有资源
kubectl get all -n memoryos

# 查看 Pod 状态
kubectl get pods -n memoryos

# 查看日志
kubectl logs -n memoryos -l app=memoryos-gateway -f

# 扩容
kubectl scale deployment memoryos-gateway -n memoryos --replicas=5

# 更新配置
kubectl edit configmap memoryos-config -n memoryos
kubectl rollout restart deployment memoryos-gateway -n memoryos

# 更新镜像
docker build -t memoryos-gateway:v2 .
docker save memoryos-gateway:v2 | ssh root@server "k3s ctr images import -"
kubectl set image deployment/memoryos-gateway gateway=memoryos-gateway:v2 -n memoryos
```

---

### 监控指标

```bash
# 健康检查
curl http://localhost:8080/health/status

# 详细健康信息
curl http://localhost:8080/health/detailed

# 系统指标
curl http://localhost:8080/metrics
```

---

## 🐛 故障排查

### 问题 1: 服务无法启动

**症状**: Gateway 启动失败

**检查步骤**:
```bash
# 1. 检查配置文件
cat config.toml

# 2. 检查中间件连接
redis-cli -h localhost -p 6379 ping
curl http://localhost:6333/health

# 3. 查看详细日志
RUST_LOG=debug ./target/release/memoryos-gateway
```

**常见原因**:
- Redis/Qdrant 未启动
- 配置文件格式错误
- 端口被占用

---

### 问题 2: API 返回 401 Unauthorized

**症状**: 所有 API 请求返回 401

**检查步骤**:
```bash
# 1. 确认认证已启用
grep "enabled = true" config.toml

# 2. 检查 API Key
curl -H "Authorization: Bearer YOUR_KEY" http://localhost:8080/v1/chat/completions

# 3. 查看日志
docker-compose logs gateway | grep -i auth
```

**解决方案**:
- 确认使用正确的 API Key
- 检查 Header 格式: `Authorization: Bearer <key>`
- 验证 API Key 在配置文件或 Qdrant 中存在

---

### 问题 3: 记忆检索为空

**症状**: `/v1/memory/retrieve` 返回空结果

**检查步骤**:
```bash
# 1. 确认记忆已添加
curl -X POST http://localhost:8080/v1/memory/add \
  -H "Authorization: Bearer YOUR_KEY" \
  -d '{"user_id":"test","role":"user","content":"test"}'

# 2. 检查 Qdrant 数据
curl http://localhost:6333/collections

# 3. 检查 user_id 是否匹配
# 添加和检索必须使用相同的 user_id
```

---

### 问题 4: K3s Pod 无法启动

**症状**: Pod 状态为 CrashLoopBackOff

**检查步骤**:
```bash
# 1. 查看 Pod 详情
kubectl describe pod <pod-name> -n memoryos

# 2. 查看日志
kubectl logs <pod-name> -n memoryos

# 3. 检查镜像
kubectl get pods -n memoryos -o jsonpath='{.items[*].spec.containers[*].image}'

# 4. 检查资源
kubectl top pods -n memoryos
```

**常见原因**:
- 镜像不存在或拉取失败
- 资源不足（CPU/内存）
- ConfigMap 配置错误
- 依赖服务未就绪

---

### 问题 5: 性能问题

**症状**: 响应缓慢或超时

**检查步骤**:
```bash
# 1. 检查资源使用
docker stats  # Docker Compose
kubectl top pods -n memoryos  # K3s

# 2. 检查中间件性能
redis-cli --latency
curl http://localhost:6333/metrics

# 3. 检查日志
grep -i "slow\|timeout\|error" gateway.log
```

**优化建议**:
- 增加 Gateway 副本数
- 调整 worker_threads 配置
- 优化向量检索参数
- 增加 Redis/Qdrant 资源

---

## 📚 相关文档

- [快速开始](./QUICKSTART.md) - 5 分钟上手
- [API 文档](./API.md) - 完整 API 参考
- [K3s 部署](./K3S_DEPLOYMENT.md) - K8s 自动化部署
- [认证系统](./AUTH.md) - API Key 管理
- [架构设计](./ARCHITECTURE.md) - 系统架构

---

## 🆘 获取帮助

- **GitHub Issues**: https://github.com/BAI-LAB/MemoryOS/issues
- **Discord**: https://discord.gg/SqVj7QvZ
- **微信群**: 见 README 二维码
- **邮箱**: baiting@bupt.edu.cn
