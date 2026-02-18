# LLM Adapter Implementation Specification (P0)

> **Status**: Approved
> **Target**: Phase 2 Implementation
> **Objective**: Normalize all Upstream interactions into standard OpenAI schemas.

## 1. Universal Request Normalization

The Gateway receives standard OpenAI `ChatCompletionRequest`. Adapters must transform this into vendor-specific payloads.

### 1.1 OpenAI -> Google Gemini (Native REST)

**Endpoint**: `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`

**Field Mapping Table**:

| OpenAI Field | Gemini Field | Transformation Logic / Boundary Condition |
| :--- | :--- | :--- |
| `model` | URL Param | Remove `models/` prefix if present. Use as `{model}` in URL. |
| `messages[i].role` | `contents[j].role` | `user` -> `user`; `assistant` -> `model`; **`system` -> Extract to `system_instruction`**. |
| `messages[i].content` | `contents[j].parts[0].text` | Direct copy. |
| `temperature` | `generationConfig.temperature` | Pass through. Clamp to [0.0, 2.0] if out of bounds. |
| `max_tokens` | `generationConfig.maxOutputTokens`| Pass through. |
| `stream` | (URL Param) | If true, change endpoint to `:streamGenerateContent?alt=sse`. |
| `stop` | `generationConfig.stopSequences` | Convert String to `[String]`. |
| `frequency_penalty` | (N/A) | **DROP**. **Action**: Add `X-MemoryOS-Warning: Param 'frequency_penalty' ignored by Gemini`. |
| `logit_bias` | (N/A) | **DROP**. **Action**: Add Warning Header. |

**Boundary Handling (System Prompt)**:
*   **Logic**: Scan all messages. If `role == "system"`, extract content, combine multiple system messages with `
`, and set as top-level `system_instruction`. Remove them from `contents` list.
*   **Failure**: If Gemini model version < `gemini-1.5-pro`, it might not support `system_instruction`. *Fallback*: Prepend system text to the first user message.

### 1.2 OpenAI -> Anthropic Claude

**Endpoint**: `POST https://api.anthropic.com/v1/messages`

**Field Mapping Table**:

| OpenAI Field | Claude Field | Transformation Logic |
| :--- | :--- | :--- |
| `messages` | `messages` | **Strict Alternation Rule**: Claude requires User/Assistant/User order. If consecutive `user` messages exist, **MERGE** their content. |
| `system` (role) | `system` (top-level) | Extract system message to top-level parameter. |
| `max_tokens` | `max_tokens` | **REQUIRED**. If missing in request, default to 4096 (safe limit). |

### 1.3 OpenAI -> Azure OpenAI

**Endpoint Pattern**: `POST https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={api_version}`

**Transformation Logic**:
1.  **Header Swap**: Change `Authorization: Bearer ...` to `api-key: ...`.
2.  **URL Rewrite**:
    *   Input Config: `base_url` (e.g., `https://my-resource.openai.azure.com`), `deployment_map` (e.g., `gpt-4o -> deployment-v1`).
    *   Construct full URL dynamically based on the requested `model`.
3.  **Body**: Pass through (Standard OpenAI format).

### 1.4 OpenAI -> Ollama (Local)

**Endpoint**: `POST http://localhost:11434/v1/chat/completions`

**Special Handling**:
1.  **Keep-Alive**: Inject `{"keep_alive": "5m"}` to prevent model unloading between turns.
2.  **Context Window**: Map `max_tokens` to Ollama `num_predict`.
3.  **Format**: If user asks for JSON, ensure `format: "json"` is passed (Ollama specific).

---

## 2. Response Normalization (The "Unified Stream")

All adapters must output `async_stream::stream!` yielding `Result<OpenAIStreamChunk, AppError>`.

### 2.1 Error Mapping (Standardization)

Upstream errors must be caught and re-thrown as standard HTTP codes.

| Upstream Error | Raw Body Sample | Mapped MemoryOS Error | User HTTP Code |
| :--- | :--- | :--- | :--- |
| Google 400 | `INVALID_ARGUMENT` | `UpstreamError::BadRequest` | 400 |
| Google 403 | `PERMISSION_DENIED` | `UpstreamError::InvalidKey` | 401 (Masked) |
| Google 429 | `RESOURCE_EXHAUSTED`| `UpstreamError::RateLimited`| 429 |
| OpenAI 500 | `Internal Server Error`| `UpstreamError::ProviderFailed`| 502 Bad Gateway |
| Network | `Connection Refused` | `UpstreamError::Unreachable` | 504 Gateway Timeout |

### 2.2 SSE Stream Normalization (Gemini Example)

Gemini returns a JSON array in SSE (`data: [...]`). We must convert it to OpenAI format (`data: {...}`).

**Gemini Chunk**:
```json
{"candidates": [{"content": {"parts": [{"text": "Hello"}]}}]}
```

**Transformed Output**:
```json
{
  "id": "chatcmpl-generated-uuid",
  "object": "chat.completion.chunk",
  "created": <now>,
  "model": "gemini-3-pro-preview",
  "choices": [{"index": 0, "delta": {"content": "Hello"}, "finish_reason": null}]
}
```

**Boundary Condition**:
*   If Gemini returns `finishReason: SAFETY`, map to `finish_reason: content_filter`.
*   If Gemini returns empty text (keep-alive), skip yielding chunk.
