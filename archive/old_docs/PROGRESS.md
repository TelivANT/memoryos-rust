# MemoryOS-Rust Progress

## Overall
- Current status: Phase 1 and Phase 2 are aligned with implementation baseline; Phase 3 hardening is actively implemented.
- Validation: `cargo test --workspace` passes.
- Next target: finish Phase 3 quality gates (rule-driven consolidation policy, live outage integration tests, streaming transport parity).

## Phase 1 (Foundation)
- `AppError` implemented in core and converted to HTTP response via `IntoResponse`.
- `AppConfig` loads from `config.toml` + `MEMORYOS__` env overrides.
- `ConfigManager` added with `ArcSwap` and hot-reload change detection.
- Gateway boots with structured JSON logs.
- Health endpoints available: `/health`, `/health/live`, `/health/ready`, `/health/status`.

## Phase 2 (Universal Gateway)
- `POST /v1/chat/completions` route is active.
- Upstream contracts include `send_request` and `stream_response` methods.
- Implemented adapters:
  - OpenAI
  - Gemini (native REST mapping with `system_instruction` and `x-goog-api-key`)
  - Claude (Anthropic messages API mapping)
  - Ollama (OpenAI-compatible local endpoint)
- Tiered router exists and is wired.

## Important Notes
- Memory backends now support partial degraded operation:
  - Redis down + Qdrant up: keep vector retrieval path.
  - Qdrant down + Redis up: keep STM path.
  - Both down: fallback to `NoopMemoryManager`.
- Qdrant adapter migrated to modern `qdrant_client::Qdrant` API with builder-based requests.
- Degraded mode is surfaced by `/health/ready`, `/health/status`, and business endpoints via `X-MemoryOS-Status: degraded`.
- Health status is refreshed periodically at runtime instead of being fixed at startup.
- Memory manager now hot-switches at runtime when Redis/Qdrant health state changes, keeping actual capability aligned with health status.
- `memory/add` now supports optional `event_id`; duplicate `event_id` is skipped via Redis dedup set.
- Default memory write path now uses Redis fencing lock (`lock:profile:{user_id}`) to prevent concurrent write races.
- Fencing lock lease-renewal heartbeat is now active during write processing.
- Fencing token CAS check is now enforced before write (`version:profile:{user_id}`).
- Vector storage now exposes fenced long-term write API (`store_long_term_with_fencing`) and Qdrant adapter enforces `lock_version` monotonicity when token is provided.
- Default memory write path now performs minimal consolidation and calls fenced long-term write when user message is added.
- Consolidation path now has explicit unit test coverage for fencing token propagation.
- Embedding generation now falls back safely when OpenAI key/upstream is unavailable.
- Consolidation extraction now supports rule-driven policy (`ExtractionPolicy`) with env override (`MEMORYOS_EXTRACTION_POLICY_JSON`) and default rules.
- Added extraction evaluation seed dataset at `docs/references/extraction_eval_dataset.jsonl` for quality benchmarking.
- Added automated extraction scorer test + runner script (`./scripts/eval_extraction.sh`) with pass/fail threshold.
- Added live outage/recovery regression script (`./scripts/test_outage_recovery.sh`) for Redis/Qdrant pause/unpause matrix validation.
- Remote execution verified on server `104.194.91.83:26974`: outage script passed with adaptive baseline path (`qdrant=down`, Redis pause/unpause recovery validated).
- Redis health check now enforces timeout to avoid stuck `up` state under paused/unresponsive Redis.
- Compose scenarios split into dedicated files: `docker-compose.standalone.yml`, `docker-compose.cluster.yml`, `docker-compose.middleware-demo.yml`.
- K8s deployment modes split by environment: `k8s/deployment.yaml` (app-only/prod) and `k8s/middleware-demo.yaml` (all-in-one demo), with `k8s/README.md` usage guide.
- Qdrant long-term point id now uses stable UUID mapping from arbitrary `user_id`, fixing non-UUID user failures in vector path.
- Remote middleware+vector demo verified on `104.194.91.83:26974`: `/v1/memory/add` and `/v1/memory/retrieve` succeeded with `mode=ready`.
- Router no longer overrides client-provided `model`; preserves provider-specific models such as `gpt-oss:20b`.
- Gateway route tests now use local stub adapters (instead of `reqwest` clients) for deterministic macOS/sandbox stability.
- Local Ollama end-to-end path is verified on macOS: `POST /v1/chat/completions` with `model=gpt-oss:20b` returns 200 through gateway.
- Qdrant client startup/health probe now skips compatibility pre-check to avoid repetitive down-state noise logs.
- Streaming chat transport is not yet wired end-to-end at gateway response layer (Phase 4 item).

## Phase 3 Focus (Next)
1. Implement real embedding generation and remove dummy vectors.
2. Add dedicated cluster load-balancer entrypoint (Nginx/Traefik) for docker-compose and K8s multi-replica traffic fan-in.
3. Implement end-to-end streaming transport.
4. Expand evaluation dataset coverage and add per-rule metrics breakdown.
