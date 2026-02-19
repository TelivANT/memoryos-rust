# Monitoring & Observability Guide

**版本**: v0.3.0  
**更新**: 2026-02-19

---

## 📋 概述

MemoryOS-Rust 提供完整的监控和可观测性解决方案，基于 Prometheus + Grafana 技术栈。

## 🚀 快速开始

### 启动监控服务

```bash
# 启动所有服务（包括监控）
docker-compose up -d

# 仅启动监控服务
docker-compose up -d prometheus grafana
```

### 访问监控界面

- **Grafana**: http://localhost:3000
  - 用户名: `admin`
  - 密码: `admin`
- **Prometheus**: http://localhost:9090
- **Metrics Endpoint**: http://localhost:8080/metrics

---

## 📊 可用指标

### HTTP 请求指标

```promql
# 请求总数
memoryos_http_requests_total

# 请求延迟
memoryos_http_request_duration_seconds
```

### 记忆操作指标

```promql
# 操作总数
memoryos_memory_operations_total

# 操作延迟
memoryos_memory_operation_duration_seconds
```

### 向量数据库指标

```promql
# 操作总数
memoryos_vector_db_operations_total

# 操作延迟
memoryos_vector_db_latency_seconds
```

### 系统指标

```promql
# 活跃用户数
memoryos_active_users

# 短期记忆数量
memoryos_short_term_messages_total

# 中期记忆数量
memoryos_mid_term_segments_total

# 长期记忆数量
memoryos_long_term_memories_total
```

---

## 📈 Grafana 仪表板

### 预配置仪表板

1. **HTTP Requests Rate** - 请求速率
2. **HTTP Request Duration (p95)** - 请求延迟（95分位）
3. **Memory Operations Rate** - 记忆操作速率
4. **Vector DB Latency (p95)** - 向量数据库延迟
5. **Active Users** - 活跃用户数
6. **Memory Storage** - 记忆存储统计

### 导入自定义仪表板

```bash
# 仪表板配置文件
monitoring/grafana-dashboard.json
```

在 Grafana 中:
1. 点击 "+" → "Import"
2. 上传 `grafana-dashboard.json`
3. 选择 Prometheus 数据源

---

## 🚨 告警规则

### 配置的告警

1. **HighRequestLatency** - 高请求延迟
   - 触发条件: p95 延迟 > 1s
   - 持续时间: 5分钟
   - 严重程度: warning

2. **HighErrorRate** - 高错误率
   - 触发条件: 5xx 错误率 > 5%
   - 持续时间: 5分钟
   - 严重程度: critical

3. **VectorDBDown** - 向量数据库宕机
   - 触发条件: Qdrant 不响应
   - 持续时间: 1分钟
   - 严重程度: critical

4. **RedisDown** - Redis 宕机
   - 触发条件: Redis 不响应
   - 持续时间: 1分钟
   - 严重程度: warning

5. **HighMemoryOperationFailureRate** - 高记忆操作失败率
   - 触发条件: 失败率 > 10%
   - 持续时间: 5分钟
   - 严重程度: warning

6. **VectorDBHighLatency** - 向量数据库高延迟
   - 触发条件: p95 延迟 > 500ms
   - 持续时间: 5分钟
   - 严重程度: warning

### 配置告警通知

编辑 `monitoring/alert-rules.yml` 添加通知渠道：

```yaml
# 添加到 Prometheus 配置
alerting:
  alertmanagers:
    - static_configs:
        - targets: ['alertmanager:9093']
```

---

## 📊 常用查询

### 请求速率

```promql
# 每秒请求数
rate(memoryos_http_requests_total[5m])

# 按状态码分组
sum(rate(memoryos_http_requests_total[5m])) by (status)
```

### 延迟分析

```promql
# p50 延迟
histogram_quantile(0.50, rate(memoryos_http_request_duration_seconds_bucket[5m]))

# p95 延迟
histogram_quantile(0.95, rate(memoryos_http_request_duration_seconds_bucket[5m]))

# p99 延迟
histogram_quantile(0.99, rate(memoryos_http_request_duration_seconds_bucket[5m]))
```

### 错误率

```promql
# 错误率
rate(memoryos_http_requests_total{status=~"5.."}[5m]) / rate(memoryos_http_requests_total[5m])
```

### 向量数据库性能

```promql
# 按数据库类型分组的延迟
histogram_quantile(0.95, rate(memoryos_vector_db_latency_seconds_bucket[5m])) by (db_type)

# 操作成功率
rate(memoryos_vector_db_operations_total{status="success"}[5m]) / rate(memoryos_vector_db_operations_total[5m])
```

---

## 🔧 集成到代码

### 在 Gateway 中暴露 metrics

```rust
use memoryos_metrics;

// 添加 /metrics 端点
async fn metrics_handler() -> String {
    memoryos_metrics::gather_metrics()
}

// 在路由中添加
.route("/metrics", get(metrics_handler))
```

### 记录 HTTP 请求

```rust
use memoryos_metrics::{HTTP_REQUESTS_TOTAL, HTTP_REQUEST_DURATION};

// 记录请求
HTTP_REQUESTS_TOTAL
    .with_label_values(&[method, path, status])
    .inc();

// 记录延迟
let timer = HTTP_REQUEST_DURATION
    .with_label_values(&[method, path])
    .start_timer();
// ... 处理请求 ...
timer.observe_duration();
```

### 记录记忆操作

```rust
use memoryos_metrics::{MEMORY_OPERATIONS_TOTAL, MEMORY_OPERATION_DURATION};

let timer = MEMORY_OPERATION_DURATION
    .with_label_values(&["add_message"])
    .start_timer();

match storage.add_short_term_message(user_id, msg).await {
    Ok(_) => {
        MEMORY_OPERATIONS_TOTAL
            .with_label_values(&["add_message", "success"])
            .inc();
    }
    Err(_) => {
        MEMORY_OPERATIONS_TOTAL
            .with_label_values(&["add_message", "error"])
            .inc();
    }
}

timer.observe_duration();
```

---

## 📚 日志聚合

### 结构化日志

MemoryOS 使用结构化 JSON 日志：

```rust
tracing::info!(
    user_id = %user_id,
    operation = "add_message",
    duration_ms = duration.as_millis(),
    "Memory operation completed"
);
```

### ELK Stack 集成（可选）

```yaml
# docker-compose.yml 添加
elasticsearch:
  image: elasticsearch:8.11.0
  environment:
    - discovery.type=single-node

logstash:
  image: logstash:8.11.0
  volumes:
    - ./monitoring/logstash.conf:/usr/share/logstash/pipeline/logstash.conf

kibana:
  image: kibana:8.11.0
  ports:
    - "5601:5601"
```

---

## 🎯 最佳实践

### 1. 设置合理的告警阈值

根据实际负载调整告警阈值，避免告警疲劳。

### 2. 定期审查指标

每周审查关键指标，识别性能趋势。

### 3. 保留历史数据

配置 Prometheus 保留足够的历史数据（建议 30 天）：

```yaml
# prometheus.yml
global:
  storage.tsdb.retention.time: 30d
```

### 4. 使用标签过滤

合理使用标签进行指标过滤和聚合。

### 5. 监控监控系统

确保 Prometheus 和 Grafana 本身也被监控。

---

## 🐛 故障排查

### Prometheus 无法抓取指标

**检查**:
```bash
# 检查 metrics 端点
curl http://localhost:8080/metrics

# 检查 Prometheus targets
curl http://localhost:9090/api/v1/targets
```

### Grafana 无法连接 Prometheus

**检查**:
1. Prometheus 是否运行
2. 数据源配置是否正确
3. 网络连接是否正常

### 告警未触发

**检查**:
1. 告警规则是否正确加载
2. 告警条件是否满足
3. Alertmanager 是否配置

---

## 📚 相关文档

- [Production Deployment](./PRODUCTION_DEPLOYMENT.md) - 生产部署指南
- [Performance Benchmarking](./PERFORMANCE_BENCHMARKING.md) - 性能测试
- [Integration Testing](./INTEGRATION_TESTING.md) - 集成测试

---

**监控愉快！** 📊
