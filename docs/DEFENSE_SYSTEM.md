# IP 防御系统使用指南

## 概述

MemoryOS-Rust 的 IP 防御系统提供了基于 Redis + Qdrant 的分布式攻击防御能力，支持临时封禁、永久封禁、白名单和滑动窗口限流。

## 架构设计

### 存储架构

```
临时封禁 → Redis (TTL 自动过期)
  - ban:temp:{ip}      (SETEX)
  - ban:count:{ip}     (INCR)
  - rate:{ip}:{type}   (ZSET 滑动窗口)
  - whitelist          (SET)

永久封禁 → Qdrant (持久化存储)
  - collection: ip_blacklist
  - payload: {ip, reason, banned_at}
```

### 请求处理流程

1. **检查白名单** (Redis SET)
   - 在白名单 → 直接放行

2. **检查永久封禁** (Qdrant)
   - 已永久封禁 → 返回 429

3. **检查临时封禁** (Redis GET)
   - 已临时封禁 → 返回 429

4. **滑动窗口限流** (Redis ZSET)
   - 删除过期记录
   - 检查计数
   - 超限 → 触发封禁

5. **记录本次请求**
   - ZADD 添加时间戳
   - EXPIRE 设置过期

## 配置

### config.toml

```toml
[defense]
enabled = true
redis_url = "redis://localhost:6379"
qdrant_url = "http://localhost:6334"

[defense.thresholds]
# 格式: { limit = 请求数, window_secs = 时间窗口(秒), ban_duration_secs = 封禁时长(秒) }

# 认证失败: 5次/分钟 → 封禁15分钟
auth_failure = { limit = 5, window_secs = 60, ban_duration_secs = 900 }

# 普通限流: 100次/分钟 → 封禁5分钟
rate_limit = { limit = 100, window_secs = 60, ban_duration_secs = 300 }

# 提示词注入: 3次/小时 → 封禁1小时
prompt_injection = { limit = 3, window_secs = 3600, ban_duration_secs = 3600 }

# 爬虫行为: 200次/分钟 → 封禁30分钟
scraping = { limit = 200, window_secs = 60, ban_duration_secs = 1800 }

# DDoS 攻击: 500次/分钟 → 永久封禁
ddos = { limit = 500, window_secs = 60, ban_duration_secs = 0 }
```

## 攻击类型

| 类型 | 阈值 | 时间窗口 | 封禁时长 | 说明 |
|------|------|----------|----------|------|
| AuthFailure | 5 | 1分钟 | 15分钟 | 认证失败次数过多 |
| RateLimit | 100 | 1分钟 | 5分钟 | 普通请求限流 |
| PromptInjection | 3 | 1小时 | 1小时 | 提示词注入攻击 |
| Scraping | 200 | 1分钟 | 30分钟 | 爬虫行为 |
| DDoS | 500 | 1分钟 | 永久 | DDoS 攻击 |

## 渐进式惩罚

系统采用渐进式惩罚机制：

- **第 1-4 次违规**: 临时封禁 → Redis (自动过期)
- **第 5 次违规**: 永久封禁 → Qdrant (持久化)
- **DDoS 攻击**: 立即永久封禁 → Qdrant

## 管理 API

### 1. 获取统计信息

```bash
GET /admin/defense/stats

Response:
{
  "total_bans": 100,
  "temp_bans": 80,
  "permanent_bans": 20
}
```

### 2. 添加白名单

```bash
POST /admin/defense/whitelist
Content-Type: application/json

{
  "ip": "192.168.1.100"
}

Response:
{
  "success": true
}
```

### 3. 解封 IP

```bash
DELETE /admin/defense/unban/192.168.1.100

Response:
{
  "success": true
}
```

## 中间件集成

### 在 Gateway 中启用

```rust
use memoryos_gateway::middleware::ip_defense_middleware;

let app = Router::new()
    .route("/api/chat", post(chat_handler))
    .layer(axum::middleware::from_fn_with_state(
        defense_system.clone(),
        ip_defense_middleware,
    ));
```

## 监控和告警

### Redis 监控

```bash
# 查看临时封禁
redis-cli KEYS "ban:temp:*"

# 查看封禁计数
redis-cli KEYS "ban:count:*"

# 查看白名单
redis-cli SMEMBERS whitelist

# 查看限流记录
redis-cli KEYS "rate:*"
```

### Qdrant 监控

```bash
# 查看永久封禁列表
curl http://localhost:6333/collections/ip_blacklist/points/scroll

# 统计永久封禁数量
curl http://localhost:6333/collections/ip_blacklist/points/count
```

## 性能优势

| 操作 | 旧架构 (内存) | 新架构 (Redis+Qdrant) | 改进 |
|------|---------------|------------------------|------|
| 白名单检查 | 内存 HashMap | Redis SET | 跨实例共享 |
| 临时封禁 | 内存 HashMap | Redis SETEX | 持久化 + TTL |
| 永久封禁 | 内存 HashMap | Qdrant | 持久化 + 查询 |
| 限流检查 | 内存 Vec | Redis ZSET | 分布式 |
| 自动过期 | 手动清理 | Redis TTL | 自动 |

## 故障处理

### Redis 不可用

- 系统会返回错误，拒绝请求
- 建议配置 Redis 哨兵或集群

### Qdrant 不可用

- 永久封禁功能失效
- 临时封禁仍然工作
- 建议配置 Qdrant 集群

## 最佳实践

1. **合理设置阈值**
   - 根据业务特点调整限流阈值
   - 避免误封正常用户

2. **监控告警**
   - 监控封禁数量
   - 设置异常告警

3. **白名单管理**
   - 将可信 IP 加入白名单
   - 定期审查白名单

4. **日志记录**
   - 记录所有封禁事件
   - 便于事后分析

5. **定期清理**
   - 定期清理过期的 Redis 数据
   - 审查 Qdrant 中的永久封禁

## 示例场景

### 场景 1: 正常用户

```
请求 1-99: 正常通过
请求 100: 触发限流，临时封禁 5 分钟
5 分钟后: 自动解封
```

### 场景 2: 恶意用户

```
第 1 次违规: 临时封禁 5 分钟
第 2 次违规: 临时封禁 5 分钟
第 3 次违规: 临时封禁 5 分钟
第 4 次违规: 临时封禁 5 分钟
第 5 次违规: 永久封禁 (写入 Qdrant)
```

### 场景 3: DDoS 攻击

```
请求 1-500: 正常通过
请求 501: 触发 DDoS 检测
立即: 永久封禁 (写入 Qdrant)
后续: 所有请求直接返回 429
```

## 故障排查

### 问题: 正常用户被误封

**解决方案**:
1. 检查限流阈值是否过低
2. 将用户 IP 加入白名单
3. 手动解封用户

### 问题: 封禁不生效

**解决方案**:
1. 检查 Redis 连接
2. 检查 Qdrant 连接
3. 查看日志确认封禁记录

### 问题: 性能下降

**解决方案**:
1. 检查 Redis 性能
2. 优化 Qdrant 查询
3. 考虑使用 Redis 集群

## 未来增强

- [ ] 行为模式识别
- [ ] 机器学习异常检测
- [ ] 分布式协调
- [ ] Honeypot 系统
- [ ] 实时告警
- [ ] 可视化监控面板
