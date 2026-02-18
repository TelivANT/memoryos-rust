# MemoryOS-Rust Execution Master Plan

> **Objective**: Pixel-perfect implementation of the MemoryOS-Rust Gateway.
> **Status**: Living Document. Do not deviate without Tech Committee approval.

## Phase 1: Foundation (基石)
**Goal**: A running Rust binary with Config, Logging, and Error Handling.

| Task ID | Task Name | Specs / Details | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **1.1** | **Project Init** | `cargo new`, Workspace setup. Deps: `tokio`, `tracing`, `config`. | `cargo run` prints JSON log: `{"level":"INFO","msg":"Starting..."}` |
| **1.2** | **Config Engine** | Struct `AppConfig`. Load from `config.toml` + ENV override. Use `ArcSwap` for hot reload safety. | Modify `config.toml` at runtime -> Application detects change (log it). |
| **1.3** | **Error Handling** | Enum `AppError`. Impl `IntoResponse`. Map logical errors to HTTP 4xx/5xx. | `return Err(AppError::ConfigInvalid)` -> HTTP 500 JSON response. |

## Phase 2: Universal Gateway (全能网关)
**Goal**: A transparent proxy that speaks OpenAI, Gemini, and Claude fluently.

| Task ID | Task Name | Specs / Details | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **2.1** | **Axum Server** | Bind port 8080. Route `POST /v1/chat/completions`. | `curl localhost:8080/health` -> 200 OK. |
| **2.2** | **Upstream Trait** | Define `UpstreamClient` trait. Methods: `send_request`, `stream_response`. | Trait compiles. |
| **2.3** | **OpenAI Adapter** | Pass-through adapter. Forward Headers/Body. | Connect standard OpenAI client to MemoryOS -> Chat works. |
| **2.4** | **Gemini Adapter** | **CRITICAL**. Convert OpenAI JSON -> Gemini Native JSON. Handle `x-goog-api-key`. Map `system` role. | **Real Test**: Configure `base_url="${GEMINI_PROXY_BASE_URL}"`, `key="${GEMINI_API_KEY}"`, `model="gemini-3-pro-preview"`. Verify Gateway correctly proxies to configured upstream and returns answer. |
| **2.5** | **Claude Adapter** | Convert JSON -> Anthropic format. Handle SSE delta differences. | Connect OpenAI client -> MemoryOS (Configured with Claude) -> Chat works. |
| **2.6** | **Ollama Adapter** | Pass specific params (`num_ctx`). Handle local connection errors. | **Real Test**: Configure `base_url="http://localhost:11434/v1"`, `model="gpt-oss:20b"`. Connect User Client to MemoryOS, verify MemoryOS successfully calls local Ollama. |

## Phase 3: Storage Layer (海马体)
**Goal**: Connect Redis and Qdrant.

| Task ID | Task Name | Specs / Details | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **3.1** | **Redis Adapter** | Impl `ShortTermStorage`. `rpush`, `ltrim`. Handle connection loss (Circuit Breaker placeholder). | Chat context persists across Gateway restarts. |
| **3.2** | **Concurrency Control** | Impl Redis Dist-Lock with fencing token + lease renewal and Message Deduplication by `event_id` (`SISMEMBER`). | Worker A and B update same profile -> Only one succeeds (Sequential). Duplicate event is skipped exactly once. |
| **3.3** | **Qdrant Adapter** | Impl `VectorStore`. gRPC client. Support `filter` (RBAC). | Insert vector -> Search vector -> Return correct payload. |

### Phase 3 Reality Check (2026-02-17)
- **3.1 Redis Adapter**: Implemented and running in gateway with degraded fallback paths.
- **3.2 Concurrency Control**: Implemented baseline (`event_id` dedup + fencing lock + lease renewal + CAS + fenced long-term write path). Further hardening still needed for broader downstream targets and policy tuning.
- **3.3 Qdrant Adapter**: Implemented on modern client API; supports fenced long-term write checks via `lock_version`.

## Phase 4: Intelligence & Routing (大脑)
**Goal**: Make the Gateway smart.

| Task ID | Task Name | Specs / Details | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **4.1** | **Security Shield** | Impl `InputSanitizer` (Regex/Fuzzy) & `PromptIsolator` (XML tags). | Input "Ignore instructions" -> 400 Error. |
| **4.2** | **Context Injector** | Fetch STM (Redis) + MTM (Qdrant). Prepend to `system` prompt. | Ask "What did I just say?" -> AI answers correctly. |
| **4.3** | **Model Router** | Impl `Router` logic. Calc `hot_score`. Tier 0/1/2 dispatch. | Log: `Routing to Local Llama (Tier 1)` or `Routing to OpenAI (Tier 2)`. |
| **4.4** | **Direct Hit** | If `type=faq` & `score>0.92`, return payload directly. | Response time < 50ms. Upstream not called. |

## Phase 5: Async Evolution (进化)
**Goal**: Background processing.

| Task ID | Task Name | Specs / Details | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **5.1** | **Event Bus** | Trait `EventBus`. Adapter: Redis Stream / NATS. | Gateway emits event -> Worker receives it. |
| **5.2** | **Worker Service** | Standalone binary. Consumes `chat_log`. Calls LLM to summarize. | Send 10 messages -> Wait -> Qdrant shows new summary vector. |
| **5.3** | **Lifecycle** | Cron job. Delete `heat < 0.1` vectors. Move old to Cold Store. | Old data disappears from Hot Store. |

## Phase 6: Knowledge Asset Management (知识资产沉淀)
**Goal**: Export mature FAQs to external Wiki systems.

| Task ID | Task Name | Specs / Details | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **6.1** | **Wiki Exporter Core** | Cron Job. Query FAQs (age > 30d, access > 10). Categorize via LLM. Render Markdown. | Run `admin wiki list-exportable` → Shows qualified FAQs. |
| **6.2** | **S3 Adapter** | Impl `WikiUploader` trait. Upload Markdown to S3. Generate `index.json`. | Export to S3 → Files appear in bucket with correct structure. |
| **6.3** | **Confluence Adapter** | Convert Markdown to Confluence Storage Format. POST to API. Handle page update. | Export to Confluence → Page created/updated in Space. |
| **6.4** | **Admin CLI** | `memoryos-rust admin wiki export`. Support `--target`, `--force`, `--id` flags. | CLI command works. Logs show export progress. |

## Phase 7: Operations & Security (安保)
**Goal**: Production readiness.

| Task ID | Task Name | Specs / Details | Acceptance Criteria |
| :--- | :--- | :--- | :--- |
| **7.1** | **Admin CLI** | `memoryos-rust admin user add`. `import faq.csv`. | Can manage users/data from terminal. |
| **7.2** | **Encryption** | AES-256-GCM for Private Memory payload. | DB dump shows ciphertext. Admin CLI can decrypt. |
| **7.3** | **Docker** | `Dockerfile`, `docker-compose.yml`. Permission fix script. | `docker-compose up` -> Works out of the box. |
