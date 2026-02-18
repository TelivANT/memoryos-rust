# MemoryOS-Rust Detailed Backlog (Parity-Oriented)

**Date**: 2026-02-17  
**Use**: Detailed review checklist before coding starts.

## 1. Scope Baseline

This backlog focuses on parity-critical capability gaps while preserving:

1. Rust-first implementation.
2. Existing API stability.
3. Degraded-mode safety.
4. Concurrency/idempotency guarantees.

## 2. Detailed Gap List

| Gap ID | Capability Gap | Current State | Target State | Priority |
|---|---|---|---|---|
| G-01 | Structured fact model | Heuristic string extraction | Typed facts + provenance + confidence | P0 |
| G-02 | Memory update semantics | Append-heavy behavior | Policy-driven merge/replace/conflict resolve | P0 |
| G-03 | Retrieval filter support | Vector search only | Metadata filter + namespace/user scopes | P0 |
| G-04 | Rerank stage | Missing | Candidate reranker with timeout fallback | P0 |
| G-05 | Async ingestion pipeline | Mostly synchronous path | Event-driven worker memory processing | P0 |
| G-06 | Worker production readiness | Skeleton worker | Consumer loop + retry + DLQ + metrics | P0 |
| G-07 | Graph memory | Missing | Entity-relation extraction + retrieval fusion | P1 |
| G-08 | API compatibility breadth | Basic memory add/retrieve | Extended memory CRUD/search variants | P1 |
| G-09 | Observability depth | Basic counters/logging | Stage-level latency/error/quality metrics | P1 |
| G-10 | Quality regression framework | Limited tests | Offline eval + acceptance thresholds | P1 |

## 3. Implementation Tasks by Module

## 3.1 `memoryos-core`

### T-CORE-01 Fact domain model
- Add:
  - `MemoryFact`
  - `FactType`
  - `FactSource`
  - `FactConfidence`
  - `FactVersion`
- Acceptance:
  - Serialization roundtrip tests pass.
  - Unknown enum values fail gracefully.
- Estimate: 0.5 day

### T-CORE-02 Policy model
- Add:
  - `ExtractionPolicy`
  - `UpdatePolicy`
  - `ConflictPolicy`
- Acceptance:
  - Policy validation rejects invalid thresholds.
- Estimate: 0.5 day

## 3.2 `memoryos-ports`

### T-PORT-01 Extraction/Rerank ports
- Add traits:
  - `MemoryExtractor`
  - `MemoryUpdater`
  - `Reranker`
  - `EventBus`
- Acceptance:
  - Trait contracts compile without adapter changes blocked.
- Estimate: 0.5 day

### T-PORT-02 Retrieval request extension
- Extend retrieve request model with optional filters:
  - `tags`
  - `time_range`
  - `fact_types`
  - `debug`
- Acceptance:
  - Existing clients still work with default behavior.
- Estimate: 0.5 day

## 3.3 `memoryos-adapters` (memory)

### T-ADP-01 Extraction pipeline v2
- Replace heuristic-only path with pluggable extractor pipeline.
- Keep existing heuristic as fallback adapter.
- Acceptance:
  - Deterministic extraction snapshots.
  - Duplicate inputs do not produce duplicate facts.
- Estimate: 1.5 days

### T-ADP-02 Update/merge/conflict handling
- Implement policy-driven fact updates with version checks.
- Acceptance:
  - Conflict cases covered in table-driven tests.
- Estimate: 1 day

### T-ADP-03 Retrieval v2 (candidate + filter + rerank)
- Add:
  - Metadata-aware candidate search.
  - Optional rerank stage.
  - Timeout fallback to base ranking.
- Acceptance:
  - Rerank timeout does not fail request path.
  - Filter constraints are enforced.
- Estimate: 1.5 days

### T-ADP-04 Redis Stream EventBus adapter
- Implement producer/consumer helpers for memory events.
- Acceptance:
  - Event publish/consume smoke test passes.
- Estimate: 1 day

### T-ADP-05 Worker adapter logic
- Add idempotent event processing and DLQ handling.
- Acceptance:
  - Duplicate event replay remains exactly-once logically.
- Estimate: 1.5 days

## 3.4 `memoryos-gateway`

### T-GW-01 API compatibility extension
- Add endpoints (phase-in):
  - `POST /v1/memory/search`
  - `POST /v1/memory/update`
  - `POST /v1/memory/delete`
- Keep old endpoints unchanged.
- Acceptance:
  - Contract tests validate response schema.
- Estimate: 1 day

### T-GW-02 Async event emission path
- For add-message path:
  - Synchronous write remains default.
  - Optional async mode emits event for worker enrichment.
- Acceptance:
  - Chat/memory API remains available under worker outage.
- Estimate: 1 day

## 3.5 `memoryos-worker`

### T-WKR-01 Production worker runtime
- Implement:
  - consumer loop
  - retry policy
  - DLQ route
  - worker metrics
- Acceptance:
  - Restart/retry scenario tests pass.
- Estimate: 2 days

## 4. API Delta Checklist (Review Before Implementation)

| API | Change Type | Compatibility |
|---|---|---|
| `/v1/memory/add` | Add optional async behavior flag | Backward compatible |
| `/v1/memory/retrieve` | Add optional filters/debug fields | Backward compatible |
| `/v1/memory/search` | New | Additive |
| `/v1/memory/update` | New | Additive |
| `/v1/memory/delete` | New | Additive |

## 5. Data Contract Delta Checklist

1. Introduce fact-level metadata fields:
   - `source_event_id`
   - `source_message_id`
   - `created_at`
   - `updated_at`
   - `confidence`
2. Ensure storage adapters can persist and restore these fields.
3. Add migration/version marker for long-term payload schema.

## 6. Test Plan (Detailed)

## 6.1 Unit tests
1. Fact extraction determinism.
2. Merge/conflict policy correctness.
3. Rerank fallback behavior.

## 6.2 Integration tests
1. Gateway add -> retrieval includes new facts.
2. Filtered retrieval returns constrained result set.
3. Degraded mode still responds with partial context.

## 6.3 End-to-end tests
1. Gateway -> event bus -> worker -> stores.
2. Worker crash/restart replay handling.
3. Duplicate event ingestion dedup correctness.

## 7. Definition of Done (DoD)

A task is done only if all are true:

1. Code merged with tests.
2. API/documentation updated.
3. Degraded-mode behavior validated.
4. Non-regression checks for concurrency/idempotency passed.
5. Metrics/log fields added for new critical path stages.

## 8. Sequencing Recommendation

Execution order for minimal rework:

1. `T-CORE-01` -> `T-CORE-02`
2. `T-PORT-01` -> `T-PORT-02`
3. `T-ADP-01` -> `T-ADP-02` -> `T-ADP-03`
4. `T-ADP-04` + `T-WKR-01`
5. `T-GW-01` + `T-GW-02`
6. P1 items (`G-07` to `G-10`)

## 9. Open Decisions for Joint Review

1. Reranker default mode:
   - Always on vs feature-flagged.
2. Async memory write mode:
   - Dual-write vs event-only mode.
3. Fact confidence threshold:
   - Conservative (precision first) vs balanced.
4. Graph memory timing:
   - Week 4 core only vs include retrieval fusion same week.

