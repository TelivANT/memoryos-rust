# Streaming Support

## 概述

MemoryOS Gateway 现在支持 OpenAI 兼容的流式响应（Server-Sent Events）。

## 使用方式

### 请求示例

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'
```

### 响应格式

流式响应使用 SSE (Server-Sent Events) 格式：

```
data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","model":"gpt-4o-mini","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","model":"gpt-4o-mini","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","model":"gpt-4o-mini","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

## 实现细节

### 架构

1. **请求检测**: `chat_completions` handler 检查 `request.stream` 字段
2. **路由分发**: 如果 `stream=true`，调用 `chat_completions_stream`
3. **LLM 调用**: 调用 `LlmAdapter::chat_stream()` 获取流式数据
4. **SSE 转换**: 将 `ChatStreamChunk` 转换为 SSE Event
5. **响应返回**: 返回 `Sse<Stream>` 响应

### LLM 适配器支持

当前实现：
- ✅ OpenAI: 支持 streaming
- ✅ DeepSeek: 支持 streaming
- ⏳ 其他适配器: 默认返回 "不支持 streaming" 错误

### 限制

1. **收集式 streaming**: 当前 `chat_stream()` 先收集全部 chunks 再通过 SSE 发送，非逐字实时流
2. **FAQ 缓存**: Streaming 模式下不检查 FAQ 缓存
3. **事件总线**: Streaming 模式下不发布 chat 事件

### 安全保障

- ✅ PII 脱敏: Streaming 路径与非 streaming 路径一致
- ✅ Compliance 检查: 敏感内容强制路由到 Local LLM
- ✅ 请求拦截: 违规内容直接返回 BadRequest

## 未来改进

1. 为所有 LLM 适配器实现 streaming 支持
2. Streaming 模式下支持完整的路由逻辑
3. 添加 streaming 性能监控
4. 支持 streaming 中断和取消
