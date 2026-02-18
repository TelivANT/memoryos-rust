# Observability & Telemetry Specification (P0)

> **Status**: Approved
> **Target**: Global Implementation
> **Objective**: Ensure 100% visibility into the request lifecycle.

## 1. Distributed Tracing (Trace ID)

**Requirement**: Every log line, metric, and error must be correlatable to a single user request.

### 1.1 Trace ID Propagation
*   **Ingress**: Nginx/Client sends `x-request-id` (UUID).
*   **Gateway**:
    *   If header exists: Use it as `trace_id`.
    *   If missing: Generate new UUIDv7.
    *   **Action**: Inject `trace_id` into `tracing::Span`.
*   **Async Boundary**:
    *   When publishing to NATS/Kafka: Put `trace_id` in message header.
*   **Worker**:
    *   Extract `trace_id` from message header.
    *   Start new Span with `parent_id = trace_id`.

### 1.2 Event Identity (Normative)
*   `request_id`: per HTTP request boundary.
*   `trace_id`: cross-service correlation id.
*   `event_id`: unique id per async message (dedup key).
*   `task_id`: consumer execution id (retries may create new `task_id` for same `event_id`).
*   **Rule**: `event_id` MUST be used for idempotency checks. `trace_id` MUST NOT be used for dedup.

---

## 2. Structured Logging (Schema)

**Format**: JSON Lines (NDJSON).
**Library**: `tracing-subscriber` with `json` formatter.

### 2.1 Standard Fields
Every log entry MUST contain:
```json
{
  "timestamp": "2026-02-17T10:00:00Z",
  "level": "INFO",
  "target": "memoryos::gateway::router",
  "trace_id": "018d... (UUIDv7)",
  "user_id": "user_123",
  "message": "Routing request to Local Llama",
  "fields": { ...context_specific... }
}
```

### 2.2 Critical Log Events
| Event | Level | Required Fields | Description |
| :--- | :--- | :--- | :--- |
| `request_in` | INFO | `method`, `path`, `ip` | HTTP Request received. |
| `router_decision` | INFO | `tier` (0/1/2), `hot_score` | Router logic outcome. |
| `upstream_call` | DEBUG | `provider`, `model`, `latency_ms` | External API call. |
| `memory_injected` | INFO | `stm_count`, `mtm_count` | How many memories used. |
| `security_block` | WARN | `reason` (PII/Injection/Compliance) | Request blocked by shield. |

---

## 3. Metrics (Prometheus)

Exposed at `/metrics`.

### 3.1 RED Method (Rate, Errors, Duration)
*   `http_requests_total{status="200", route="/chat"}`
*   `http_request_duration_seconds_bucket{route="/chat"}`

### 3.2 Business Metrics
*   **Cost Control**:
    *   `llm_token_usage_total{model="gpt-4o", type="input"}`
    *   `llm_token_usage_total{model="gpt-4o", type="output"}`
*   **Memory Health**:
    *   `vector_db_latency_seconds_bucket{operation="search"}`
    *   `worker_queue_depth{queue="memory_updates"}` (Critical for scaling workers)
*   **Routing Stats**:
    *   `router_decisions_total{tier="0_faq"}`
    *   `router_decisions_total{tier="1_local"}`
    *   `router_decisions_total{tier="2_cloud"}`

---

## 4. Health Checks

*   **Liveness**: `/health/live` -> Returns 200 if process is running.
*   **Readiness**: `/health/ready`
    *   **Logic**:
        *   Full dependencies healthy -> **200 OK**.
        *   Any dependency bypassed by approved fallback -> **200 OK** with `X-MemoryOS-Status: degraded`.
        *   No safe fallback available -> **503 Service Unavailable**.
    *   **Purpose**: K8s should keep serving Pods in degraded mode.
*   **Status**: `/health/status` -> dependency matrix (redis/qdrant/upstream/auth-cache) + current mode (`ready`, `degraded_ready`, `not_ready`).
*   **Runtime Update Rule**:
    *   Health status MUST be refreshed periodically during runtime.
    *   Memory capability (full/partial/noop) MUST hot-switch when dependency health state changes.
    *   Observability dashboards SHOULD treat health state as dynamic, not startup-static.

## 5. Alerting Rules (Prometheus)

Recommended thresholds for paging DevOps.

| Alert Name | Condition | Severity | Description |
| :--- | :--- | :--- | :--- |
| **HighErrorRate** | `rate(http_requests_total{status=~"5.."}[1m]) > 1` | 🔴 Critical | >1 error/sec implies systematic failure. |
| **HighLatency** | `histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 5.0` | 🟡 Warning | P99 Latency > 5s. LLM or DB is slow. |
| **QueueJam** | `worker_queue_depth > 1000` | 🟡 Warning | Async memory processing is lagging behind. Scale up Workers. |
| **MemoryFull** | `process_resident_memory_bytes > 80% limit` | 🔴 Critical | OOM Killer risk. Check ONNX model usage. |
