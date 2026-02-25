# Config Hot-Reload 限制说明

## 当前状态

配置热加载功能 (`MEMORYOS_CONFIG_HOT_RELOAD`) 目前存在架构限制，**仅部分生效**。

## 问题描述

虽然 `ConfigManager` 可以检测并重新加载配置文件，但由于 `AppState` 使用 `Arc<AppConfig>` 的不可变引用，**已创建的服务实例不会使用新配置**。

### 受影响的组件

以下组件在配置更新后**不会**自动应用新配置：

- ✅ LLM 适配器 (OpenAI, Claude, etc.)
- ✅ Router 配置
- ✅ Security Shield 规则
- ✅ Rate Limiter 限制
- ✅ Circuit Breaker 阈值
- ✅ Memory Manager 设置
- ✅ 所有其他 AppState 中的组件

## 当前解决方案

**重启服务以应用新配置**：

```bash
# Docker Compose
docker-compose restart memoryos-gateway

# Kubernetes
kubectl rollout restart deployment/memoryos-gateway

# 本地开发
# Ctrl+C 然后重新运行
cargo run --bin memoryos-gateway
```

## 未来计划 (v1.1.0)

### 架构重构方案

将 `Arc<AppConfig>` 改为 `Arc<RwLock<AppConfig>>`：

```rust
// 当前
pub struct AppState {
    pub config: Arc<AppConfig>,  // 不可变
    // ...
}

// 未来
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,  // 可变
    // ...
}
```

### 影响范围

需要更新所有访问 `state.config` 的代码：

```rust
// 当前
let timeout = state.config.llm.timeout;

// 未来
let timeout = state.config.read().await.llm.timeout;
```

### 工作量评估

- 影响文件: ~15 个
- 影响代码行: ~100+ 处
- 测试更新: 需要
- 文档更新: 需要

## 临时解决方案

### 1. 使用环境变量覆盖

关键配置可通过环境变量动态调整（无需重启）：

```bash
# Rate Limiter
export MEMORYOS_RATE_LIMIT_REQUESTS=200
export MEMORYOS_RATE_LIMIT_WINDOW=60

# Circuit Breaker
export MEMORYOS_CIRCUIT_BREAKER_THRESHOLD=10
export MEMORYOS_CIRCUIT_BREAKER_TIMEOUT=30
```

### 2. 使用 API 动态调整

部分配置支持通过 API 动态更新：

```bash
# 更新 Rate Limiter (需要实现)
curl -X POST http://localhost:8080/admin/config/rate-limit \
  -H "Content-Type: application/json" \
  -d '{"requests": 200, "window": 60}'
```

### 3. 使用 Kubernetes ConfigMap

Kubernetes 环境可使用 ConfigMap + 滚动更新：

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: memoryos-config
data:
  config.toml: |
    [llm]
    timeout = 30
    # ...
```

```bash
# 更新 ConfigMap
kubectl edit configmap memoryos-config

# 滚动重启
kubectl rollout restart deployment/memoryos-gateway
```

## 相关 Issue

- P1-12: Config Hot-Reload 无效
- 优先级: 中 (v1.1.0)
- 状态: DEFERRED (需架构重构)

## 参考

- [配置文档](./ops/config_reference.md)
- [部署指南](./ops/deployment.md)
- [架构文档](../ARCHITECTURE.md)
