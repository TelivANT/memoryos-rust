# Circuit Breaker

## 概述

Circuit Breaker（熔断器）用于防止外部服务故障拖垮整个系统。

## 工作原理

### 三种状态

1. **Closed（关闭）**: 正常运行，所有请求通过
2. **Open（打开）**: 检测到故障，快速失败，拒绝请求
3. **Half-Open（半开）**: 超时后尝试恢复，允许部分请求测试服务

### 触发条件

- **打开熔断器**: 连续 5 次失败
- **半开状态**: 打开后 30 秒
- **关闭熔断器**: 半开状态下请求成功

## 使用方式

### 在 handlers.rs 中的实际用法

```rust
use crate::middleware::circuit_breaker::{with_circuit_breaker, CircuitBreakerState};

// Circuit breaker 已集成到 LLM 调用路径
let result = with_circuit_breaker(
    &state.circuit_breaker,
    retry_with_backoff(&retry_config, "llm_chat", || async {
        adapter.chat(request.clone()).await
    }),
)
.await;

match result {
    Some(Ok(response)) => { /* 正常响应 */ }
    Some(Err(e)) => { /* LLM 调用失败（已重试） */ }
    None => {
        // 熔断器打开，快速失败
        return Err(AppError::ServiceUnavailable("Circuit breaker open".into()));
    }
}
```

### 手动控制

```rust
// 检查是否允许请求
if !state.circuit_breaker.should_allow().await {
    return Err(AppError::ServiceUnavailable);
}

// 记录成功
state.circuit_breaker.record_success().await;

// 记录失败
state.circuit_breaker.record_failure().await;
```

## 配置

当前配置（硬编码）:
- 失败阈值: 5 次
- 超时时间: 30 秒

未来可通过 config.toml 配置。

## 注意事项

1. 当前实现是全局的，所有外部服务共享一个熔断器
2. 生产环境建议为每个外部服务（Redis、Qdrant、LLM）单独配置熔断器
3. 可考虑使用专业库如 `failsafe` 或 `tower-circuit-breaker`
