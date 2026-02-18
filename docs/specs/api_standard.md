# API & Code Standards Specification

## 1. Naming Conventions (Strict)

- **Do Not Use**: `_v1`, `_new`, `_final`, `_temp`.
- **File Names**: `snake_case`. Must reflect the singular responsibility of the module (e.g., `memory_storage.rs`, not `memory_manager_v2.rs`).
- **Structs/Traits**: `UpperCamelCase` (e.g., `MemoryEngine`, `LLMClient`).
- **Functions/Methods**: `snake_case` (e.g., `add_memory`, `retrieve_context`).
- **Variables**: `snake_case` (e.g., `user_input`, `response_text`).
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_CAPACITY`, `MAX_RETRIES`).

## 2. API Design Principles (RESTful / MCP Compatible)

### 2.1 Endpoint Structure

All API endpoints must follow the resource-oriented design pattern.

- **Target (canonical)**:
- `POST /v1/memory`: Add a new memory entry (Conversation turn).
- `POST /v1/memory/search`: Retrieve memory context based on query.
- `GET /v1/profile`: Retrieve the user profile.
- `PUT /v1/profile`: Update the user profile manually.
- `POST /v1/chat/completions`: OpenAI-compatible chat interface (Standard).

- **Current implementation (Phase 3 baseline)**:
- `POST /v1/memory/add`
- `POST /v1/memory/retrieve`
- `POST /v1/chat/completions`
- `GET /health`
- `GET /health/live`
- `GET /health/ready`
- `GET /health/status`

### 2.2 Response Format (JSON)

Every API response must be wrapped in a standard envelope or follow the OpenAI spec strictly for compatibility endpoints.
When degraded fallbacks are used, response header MUST include `X-MemoryOS-Status: degraded`.
Gateway computes degraded mode from runtime health status (not startup-only snapshot).

**Success Response (General):**
```json
{
  "status": "success",
  "data": { ... },
  "meta": {
    "latency_ms": 123,
    "timestamp": 1718888888
  }
}
```

**Error Response:**
```json
{
  "status": "error",
  "error": {
    "code": "resource_not_found",
    "message": "The requested user profile does not exist.",
    "details": "User ID: user_123"
  }
}
```

## 3. Implementation Standards (Rust)

### 3.0 Identity Fields (Normative)
- `request_id`: per HTTP request.
- `trace_id`: distributed tracing correlation id.
- `event_id`: async dedup id (must be unique per emitted event).
- `task_id`: consumer execution id.
- Deduplication MUST use `event_id`, never `trace_id`.

### 3.1 Error Handling
- Use a central `Error` enum in `core::error` deriving `thiserror::Error`.
- All public functions must return `Result<T, crate::core::error::Error>`.
- Do not use `.unwrap()` in production code; use `.expect("reason")` or propagate with `?`.

### 3.2 Asynchronous Runtime
- Use `tokio` for all I/O bound operations.
- Background tasks (Updater/Summarizer) must be spawned as detached tasks: `tokio::spawn(async move { ... })`.

### 3.3 Configuration
- Configuration must be loadable from `config.toml` and Environment Variables.
- No hardcoded paths or API keys in source code.

## 4. Documentation
- All `pub` structs and functions must have doc comments (`///`).
- Complex logic must reference the algorithm in `project_definition.md`.
