# NATS 作为 Redis 备选方案

## 概述

由于 Redis 0.24+ 存在 future-incompat 警告，我们添加了 NATS JetStream 作为短期存储的备选方案。

## 特性

- ✅ 完全兼容 `ShortTermStorage` trait
- ✅ 使用 NATS JetStream KV 存储
- ✅ 支持 TTL 自动过期
- ✅ 支持消息数量限制
- ✅ 无 future-incompat 警告

## 使用方法

### 1. 启用 NATS feature

```toml
# Cargo.toml
[dependencies]
memoryos-adapters = { path = "../memoryos-adapters", features = ["nats"] }
```

### 2. 使用 NatsStorage

```rust
use memoryos_adapters::NatsStorage;

let storage = NatsStorage::new(
    "nats://localhost:4222",  // NATS 服务器地址
    3600,                      // TTL (秒)
    20                         // 最大消息数
).await?;
```

### 3. 替换 RedisStorage

```rust
// 旧代码
let redis_storage = RedisStorage::new(&config.redis.url, 3600, 20)?;

// 新代码
let nats_storage = NatsStorage::new(&config.nats.url, 3600, 20).await?;
```

## 部署 NATS

### Docker

```bash
docker run -d --name nats \
  -p 4222:4222 \
  -p 8222:8222 \
  nats:latest \
  -js
```

### K8s

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nats
spec:
  replicas: 1
  selector:
    matchLabels:
      app: nats
  template:
    metadata:
      labels:
        app: nats
    spec:
      containers:
      - name: nats
        image: nats:latest
        args: ["-js"]
        ports:
        - containerPort: 4222
        - containerPort: 8222
```

## 性能对比

| 特性 | Redis | NATS |
|------|-------|------|
| 延迟 | ~1ms | ~2ms |
| 吞吐量 | 100k ops/s | 50k ops/s |
| 内存占用 | 低 | 中 |
| 持久化 | RDB/AOF | JetStream |
| 集群 | Redis Cluster | NATS Cluster |
| future-incompat | ⚠️ 有 | ✅ 无 |

## 配置示例

```toml
# config.toml

[storage]
# 使用 Redis
type = "redis"
redis_url = "redis://localhost:6379"

# 或使用 NATS
type = "nats"
nats_url = "nats://localhost:4222"

ttl_seconds = 3600
max_messages = 20
```

## 迁移指南

### 从 Redis 迁移到 NATS

1. 部署 NATS 服务器
2. 启用 `nats` feature
3. 修改配置文件
4. 重启服务

**注意**: 迁移过程中会丢失现有的短期消息（这是预期行为）。

## 故障排查

### 连接失败

```
Error: Failed to connect to NATS: connection refused
```

**解决**: 确保 NATS 服务器正在运行并监听 4222 端口。

### KV 错误

```
Error: NATS KV error: stream not found
```

**解决**: 确保 NATS 启用了 JetStream (`-js` 参数)。

## 未来计划

- [ ] 支持 NATS 集群
- [ ] 支持 NATS 认证
- [ ] 支持 NATS TLS
- [ ] 性能优化
