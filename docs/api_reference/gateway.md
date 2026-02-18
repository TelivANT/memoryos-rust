# API Reference: Gateway Chat Interface

> **Module**: `src/api/routes.rs`
> **Status**: Stable
> **Version**: v1

This document describes the primary interface for user interaction with MemoryOS. It follows the standard OpenAI API format but includes MemoryOS-specific extensions.

---

## 1. Create Chat Completion (POST)

Generates a model response for the given chat conversation, enhanced with long-term memory retrieval.

### 1.1 Prerequisites
*   **Authorization**: A valid API Key must be provided in the `Authorization` header.
*   **Initialization**: The Gateway service must be running and connected to Redis/Qdrant.

### 1.2 Request Structure

**Endpoint**: `POST /v1/chat/completions`

**Headers**:
| Name | Value | Required | Description |
| :--- | :--- | :--- | :--- |
| `Authorization` | `Bearer sk-user-123` | ✅ Yes | User Identity Key. |
| `Content-Type` | `application/json` | ✅ Yes | Payload format. |
| `x-memoryos-compliance` | `false` | ❌ No | Override compliance check (if allowed). |

**Body (JSON)**:
```json
{
  "model": "gpt-4o",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "What is the company wifi password?"}
  ],
  "stream": true,
  "temperature": 0.7
}
```

### 1.3 Processing Logic (The "Deployment Flow")

1.  **Auth**: Validate `sk-user-123` against Redis.
2.  **Sanitization**: Scrub PII from input `messages`.
3.  **Router Check**:
    *   Calculate `hot_score` via Qdrant Global Search.
    *   If `hot_score > 0.92` AND `type=faq` -> **Tier 0 (Direct Hit)**.
    *   If `hot_score > 0.85` -> **Tier 1 (Local Llama)**.
    *   Else -> **Tier 2 (Cloud OpenAI)**.
4.  **Context Injection**:
    *   Retrieve top-k memories from Qdrant.
    *   Inject into `system` prompt: `Context: [Wifi pass is 123456]`.
5.  **Forwarding**: Send modified request to Upstream.

### 1.4 Response Format

**Success (Streamed)**:
```text
data: {"id":"chatcmpl-123", "object":"chat.completion.chunk", "created":1694268190, "model":"gpt-4o", "choices":[{"index":0, "delta":{"content":"The "}, "finish_reason":null}]}

data: {"id":"chatcmpl-123", "object":"chat.completion.chunk", "created":1694268190, "model":"gpt-4o", "choices":[{"index":0, "delta":{"content":"Wifi "}, "finish_reason":null}]}
...
data: [DONE]
```

**Success (Non-Streamed)**:
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4o",
  "usage": {
    "prompt_tokens": 50,
    "completion_tokens": 12,
    "total_tokens": 62
  },
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "The Wifi password is 'MemoryOS-2026'."
    },
    "finish_reason": "stop",
    "index": 0
  }]
}
```

**Response Headers**:
*   `X-MemoryOS-Status`: `OK` or `Degraded`.
*   `X-MemoryOS-Router-Decision`: `Local` or `Cloud` (Debug info).

**Error Responses**:
*   `401 Unauthorized`: Invalid API Key.
*   `429 Too Many Requests`: Rate limit exceeded or Token Budget exhausted.
*   `503 Service Unavailable`: All upstream backends (Local & Cloud) are down.

---

## 2. API Capabilities List

| API | Method | Description |
| :--- | :--- | :--- |
| `CreateCompletion` | `POST` | (Legacy) Text completion interface. |
| `ChatCompletion` | `POST` | (Main) Chat interface with memory injection. |
| `RetrieveContext` | `POST` | **Debug Only**. Returns the raw memory context for a query without calling LLM. |
| `Feedback` | `POST` | Submit Like/Dislike for a specific message ID. |

## 3. Session Management (Stateful Chat)

### 3.1 Create/Resume Session
*   **Method**: `POST /v1/chat/completions` (Standard Endpoint)
*   **Header**: `X-Session-ID: sess_abc123` (Optional)
*   **Logic**:
    *   If provided: Gateway loads last 10 messages from Redis `session:{id}`.
    *   If missing: Stateless mode (only relies on Vector Memory).

### 3.2 List Sessions
*   **Endpoint**: `GET /v1/sessions`
*   **Response**: List of active sessions sorted by `last_active`.

---

## 4. Memory Management (Right to be Forgotten)

### 4.1 Delete Recent Memories
*   **Endpoint**: `DELETE /v1/memory/recent`
*   **Query Param**: `count=N` (Default 1).
*   **Effect**: Removes last N user/assistant pairs from STM (Redis) and cancels pending Worker tasks.
*   **Use Case**: User typed password by mistake.

### 4.2 Delete Specific Fact
*   **Endpoint**: `DELETE /v1/memory/facts/{id}`
*   **Effect**: Hard delete from Qdrant/SQL.

