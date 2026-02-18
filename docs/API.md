# API 文档

**版本**: v0.2.0  
**基础 URL**: `http://localhost:8080`  
**更新时间**: 2026-02-18

---

## 📋 目录

- [健康检查 API](#健康检查-api)
- [聊天 API](#聊天-api)
- [记忆 API](#记忆-api)
- [错误处理](#错误处理)
- [降级模式](#降级模式)

---

## 健康检查 API

### GET /health/live
存活检查，服务是否运行。

**响应**:
```json
{
  "status": "ok"
}
```

**状态码**: 200 OK

---

### GET /health/ready
就绪检查，服务是否可以处理请求。

**⚠️ v0.2.0 变更**: 现在返回**实时状态**，非启动时快照。

**响应**:

**正常模式**:
```json
{
  "mode": "ready",
  "redis": "up",
  "qdrant": "up"
}
```

**降级模式**:
```json
{
  "mode": "degraded_ready",
  "redis": "up",
  "qdrant": "down"
}
```

**响应头**:
```
X-MemoryOS-Status: degraded
```

**不可用模式**:
```json
{
  "mode": "not_ready",
  "redis": "down",
  "qdrant": "down"
}
```

**状态码**: 
- 200 OK - 服务就绪（包括降级模式）
- 503 Service Unavailable - 服务不可用

---

### GET /health/status
详细健康状态（实时探测）。

**响应**:
```json
{
  "mode": "ready",
  "redis": "up",
  "qdrant": "up"
}
```

**状态码**: 200 OK

---

## 聊天 API

### POST /v1/chat/completions
OpenAI 兼容的聊天补全接口。

**请求头**:
```
Content-Type: application/json
```

**请求体**:
```json
{
  "model": "gpt-4o-mini",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant."
    },
    {
      "role": "user",
      "content": "Hello, how are you?"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 1000,
  "stream": false
}
```

**参数说明**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| model | string | 是 | 模型名称 |
| messages | array | 是 | 消息列表 |
| temperature | float | 否 | 温度 (0-2) |
| max_tokens | int | 否 | 最大 token 数 |
| stream | bool | 否 | 是否流式返回 |

**非流式响应** (stream=false):
```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1708156800,
  "model": "gpt-4o-mini",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! I'm doing well, thank you for asking."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 15,
    "total_tokens": 35
  }
}
```

**流式响应** (stream=true):
```
Content-Type: text/event-stream

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1708156800,"model":"gpt-4o-mini","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1708156800,"model":"gpt-4o-mini","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1708156800,"model":"gpt-4o-mini","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":"stop"}]}

data: [DONE]
```

**响应头** (降级模式):
```
X-Degraded-Mode: true
```

**状态码**:
- 200 OK - 成功
- 400 Bad Request - 请求参数错误
- 500 Internal Server Error - 服务器错误
- 503 Service Unavailable - 服务不可用

---

## 记忆 API

### POST /v1/memory/store
存储记忆。

**请求体**:
```json
{
  "user_id": "user123",
  "session_id": "session456",
  "content": "User prefers dark mode",
  "metadata": {
    "category": "preference",
    "importance": "high"
  }
}
```

**响应**:
```json
{
  "memory_id": "mem_abc123",
  "stored_at": "2026-02-17T14:30:00Z"
}
```

**状态码**: 200 OK

---

### POST /v1/memory/retrieve
检索记忆。

**请求体**:
```json
{
  "user_id": "user123",
  "query": "What are user's preferences?",
  "limit": 10
}
```

**响应**:
```json
{
  "memories": [
    {
      "memory_id": "mem_abc123",
      "content": "User prefers dark mode",
      "score": 0.95,
      "metadata": {
        "category": "preference",
        "importance": "high"
      },
      "created_at": "2026-02-17T14:30:00Z"
    }
  ]
}
```

**状态码**: 200 OK

---

### GET /v1/memory/history/{user_id}
获取用户记忆历史。

**路径参数**:
- `user_id` - 用户 ID

**查询参数**:
- `limit` - 返回数量限制 (默认 50)
- `offset` - 偏移量 (默认 0)

**响应**:
```json
{
  "user_id": "user123",
  "total": 100,
  "memories": [
    {
      "memory_id": "mem_abc123",
      "content": "User prefers dark mode",
      "created_at": "2026-02-17T14:30:00Z"
    }
  ]
}
```

**状态码**: 200 OK

---

## 错误处理

### 错误响应格式
```json
{
  "error": {
    "type": "BadRequest",
    "message": "Invalid model name",
    "details": "Model 'invalid-model' is not supported"
  }
}
```

### 错误类型

| 类型 | HTTP 状态码 | 说明 |
|------|-------------|------|
| BadRequest | 400 | 请求参数错误 |
| Unauthorized | 401 | 未授权 |
| NotFound | 404 | 资源不存在 |
| InternalError | 500 | 内部错误 |
| ServiceUnavailable | 503 | 服务不可用 |
| ExternalServiceError | 502 | 外部服务错误 |

### 常见错误

**模型不支持**:
```json
{
  "error": {
    "type": "BadRequest",
    "message": "Model not supported",
    "details": "Available providers/models depend on llm.provider: openai/gemini/claude/ollama/deepseek/openrouter/azure-openai"
  }
}
```

**后端不可用**:
```json
{
  "error": {
    "type": "ServiceUnavailable",
    "message": "Memory service unavailable",
    "details": "Redis connection failed"
  }
}
```

**降级模式**:
- 响应头包含 `X-Degraded-Mode: true`
- 部分功能可能不可用
- 服务仍可处理基本请求

---

## 🔧 使用示例

### cURL

**非流式聊天**:
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "user", "content": "Hello"}
    ],
    "stream": false
  }'
```

**流式聊天**:
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "user", "content": "Hello"}
    ],
    "stream": true
  }'
```

**健康检查**:
```bash
curl http://localhost:8080/health/status
```

### Python

```python
import requests

# 非流式聊天
response = requests.post(
    "http://localhost:8080/v1/chat/completions",
    json={
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "stream": False
    }
)
print(response.json())

# 流式聊天
response = requests.post(
    "http://localhost:8080/v1/chat/completions",
    json={
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "stream": True
    },
    stream=True
)

for line in response.iter_lines():
    if line:
        print(line.decode('utf-8'))
```

### JavaScript

```javascript
// 非流式聊天
const response = await fetch('http://localhost:8080/v1/chat/completions', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    model: 'gpt-4o-mini',
    messages: [
      { role: 'user', content: 'Hello' }
    ],
    stream: false
  })
});

const data = await response.json();
console.log(data);

// 流式聊天
const response = await fetch('http://localhost:8080/v1/chat/completions', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    model: 'gpt-4o-mini',
    messages: [
      { role: 'user', content: 'Hello' }
    ],
    stream: true
  })
});

const reader = response.body.getReader();
const decoder = new TextDecoder();

while (true) {
  const { done, value } = await reader.read();
  if (done) break;
  
  const chunk = decoder.decode(value);
  console.log(chunk);
}
```

---

## 降级模式

### 什么是降级模式？

当部分后端服务（Redis 或 Qdrant）不可用时，系统自动进入降级模式，保证核心功能可用。

### 降级策略

| Redis | Qdrant | 模式 | 可用功能 |
|-------|--------|------|---------|
| ✅ | ✅ | **Full** | STM + MTM + LTM + LLM |
| ✅ | ❌ | **Degraded** | STM + LLM（无向量检索） |
| ❌ | ✅ | **Degraded** | MTM + LTM + LLM（无短期记忆） |
| ❌ | ❌ | **Noop** | 仅 LLM（无记忆功能） |

### 如何检测降级模式？

**方法 1: 检查响应头**
```bash
curl -I http://localhost:8080/v1/chat/completions

# 降级模式返回
X-MemoryOS-Status: degraded
```

**方法 2: 健康检查 API**
```bash
curl http://localhost:8080/health/ready

# 降级模式返回
{
  "mode": "degraded_ready",
  "redis": "up",
  "qdrant": "down"
}
```

### 降级模式下的行为

**Full Mode**:
- ✅ 所有功能正常
- ✅ 完整的记忆管理
- ✅ 向量检索
- ✅ 用户画像

**Degraded Mode**:
- ✅ LLM 调用正常
- ⚠️ 部分记忆功能不可用
- ⚠️ 响应头包含 `X-MemoryOS-Status: degraded`

**Noop Mode**:
- ✅ LLM 调用正常
- ❌ 记忆功能完全不可用
- ⚠️ 返回 503 Service Unavailable

---

## 高级参数透传

### OpenAI 兼容参数

**v0.2.0 新增**: 支持所有 OpenAI API 参数透传。

**标准参数**:
```json
{
  "model": "gpt-4o-mini",
  "messages": [...],
  "temperature": 0.7,
  "max_tokens": 1000,
  "stream": false
}
```

**高级参数**（自动透传）:
```json
{
  "model": "gpt-4o-mini",
  "messages": [...],
  "temperature": 0.7,
  "top_p": 0.9,
  "frequency_penalty": 0.5,
  "presence_penalty": 0.3,
  "stop": ["\n", "END"],
  "n": 1,
  "seed": 42,
  "response_format": {"type": "json_object"}
}
```

**Function Calling**:
```json
{
  "model": "gpt-4o-mini",
  "messages": [...],
  "tools": [{
    "type": "function",
    "function": {
      "name": "get_weather",
      "description": "Get weather info",
      "parameters": {
        "type": "object",
        "properties": {
          "location": {"type": "string"}
        }
      }
    }
  }],
  "tool_choice": "auto"
}
```

**说明**: 所有未在文档中列出的参数都会自动透传到上游 LLM API。

---

## 📝 注意事项

1. **API Key**: 当前版本不需要 API Key，生产环境需要添加认证
2. **速率限制**: 当前版本无速率限制，生产环境需要添加
3. **降级模式**: 检查响应头 `X-MemoryOS-Status` 判断是否降级
4. **流式响应**: 使用 SSE 格式，需要支持流式读取
5. **错误重试**: 建议实现指数退避重试策略
6. **参数透传**: 支持所有 OpenAI API 参数，参考 [OpenAI 文档](https://platform.openai.com/docs/api-reference/chat/create)

---

**版本**: v0.2.0  
**最后更新**: 2026-02-18
