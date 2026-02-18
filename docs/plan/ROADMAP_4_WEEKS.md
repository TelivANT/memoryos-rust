# MemoryOS-Rust 4-Week Parity Roadmap (Keep Rust, Preserve Strengths)

**Date**: 2026-02-17  
**Goal**: Incrementally approach Mem0-like memory capability while preserving current Rust architecture advantages.

## 1. Working Rules

1. Do not break existing gateway contracts (`/v1/chat/completions`, `/v1/memory/*`).
2. New capabilities must support degraded mode behavior.
3. Concurrency/idempotency protections remain mandatory.
4. Land changes in small PRs; each PR must include tests.

## 2. Week-by-Week Plan

## Week 1: Memory Intelligence Foundation

### Objective
Upgrade from heuristic memory extraction to configurable extraction/update pipeline.

### Deliverables
- Introduce structured fact model (`trait`, `preference`, `background`, `knowledge`, metadata).
- Add extraction policy config file (instead of env-json only).
- Add update strategy (`append`, `merge_unique`, `replace_if_confident`).
- Persist extraction provenance (source message/event timestamp).

### Suggested PR breakdown
1. `PR-W1-01`: Add domain structs and trait interfaces in `memoryos-core` + `memoryos-ports`.
2. `PR-W1-02`: Implement extraction policy loader and validator.
3. `PR-W1-03`: Integrate new pipeline into `DefaultMemoryManager`.
4. `PR-W1-04`: Add unit/integration tests for extraction/update behavior.

### Acceptance criteria
- Same input text always yields deterministic extracted facts.
- Duplicate facts are de-duplicated according to policy.
- Existing memory APIs remain backward compatible.

## Week 2: Retrieval Pipeline v2

### Objective
Increase retrieval precision with metadata filtering and rerank stage.

### Deliverables
- Extend retrieval request with optional metadata filters.
- Add candidate retrieval + rerank pipeline.
- Return retrieval diagnostics under debug flag (scores, filter hits).
- Add benchmark script for retrieval quality and latency.

### Suggested PR breakdown
1. `PR-W2-01`: Extend request/response schema for retrieval filters.
2. `PR-W2-02`: Implement filter-aware Qdrant query composition.
3. `PR-W2-03`: Add reranker adapter abstraction + baseline reranker implementation.
4. `PR-W2-04`: Add offline eval harness in `scripts/` and regression tests.

### Acceptance criteria
- Retrieval quality improves on internal eval dataset.
- P95 retrieval latency stays within agreed budget.
- Degraded mode still returns usable context.

## Week 3: Async Memory Pipeline + Worker Completion

### Objective
Move heavy extraction/consolidation to async path and complete worker service.

### Deliverables
- Define event bus port and Redis Stream implementation.
- Gateway emits memory events; worker consumes and processes.
- Worker applies extraction/update pipeline and writes to stores.
- Add idempotent replay handling and dead-letter strategy.

### Suggested PR breakdown
1. `PR-W3-01`: Event contract + producer in gateway.
2. `PR-W3-02`: Worker runtime loop and consumer adapter.
3. `PR-W3-03`: Idempotency, retry, and dead-letter handling.
4. `PR-W3-04`: End-to-end integration tests (gateway -> stream -> worker -> store).

### Acceptance criteria
- Message ingestion survives worker restart without duplicate writes.
- Backpressure does not break chat endpoint availability.
- `memoryos-worker` is production-usable, not skeleton.

## Week 4: Graph Memory + Compatibility Layer

### Objective
Close major capability gaps and improve ecosystem compatibility.

### Deliverables
- Add graph memory module (entity/relation extraction + retrieval merge).
- Add compatibility endpoints for advanced memory operations.
- Provide thin SDK examples (Python/Node) against existing Rust service.
- Final parity report and benchmark comparison.

### Suggested PR breakdown
1. `PR-W4-01`: Graph domain model + storage adapter abstraction.
2. `PR-W4-02`: Entity/relation extraction pipeline.
3. `PR-W4-03`: Retrieval fusion (vector + graph results).
4. `PR-W4-04`: Compatibility API docs + SDK examples + final report.

### Acceptance criteria
- Graph memory contributes measurable recall gain in eval set.
- API compatibility docs are complete and testable.
- End-to-end load and fault-injection tests pass.

## 3. Optional Source-Comparison Plan

Downloading Mem0 source is optional for weeks 1-2, recommended in weeks 3-4 for edge-case alignment.

1. Week 1-2: Build from public behavior/docs first.
2. Week 3: Clone source for event/idempotency semantics comparison.
3. Week 4: Use source-level diff only to close behavior gaps.

## 4. Tracking Dashboard Template

Use this status template in `PROGRESS.md` updates:

| Item | Status | Owner | ETA | Risk |
|---|---|---|---|---|
| W1 extraction pipeline | Not Started | TBD | YYYY-MM-DD | Low |
| W2 retrieval v2 | Not Started | TBD | YYYY-MM-DD | Medium |
| W3 async worker | Not Started | TBD | YYYY-MM-DD | High |
| W4 graph+compat | Not Started | TBD | YYYY-MM-DD | High |

## 5. Risk Register

1. Quality drift from heuristic to structured extraction:
   - Mitigation: offline eval dataset + deterministic tests.
2. Latency increase from reranking:
   - Mitigation: cap candidate set and add timeout fallback.
3. Async pipeline operational complexity:
   - Mitigation: DLQ + replay tools + strict idempotency keying.
4. Over-coupling to external reference implementation:
   - Mitigation: keep behavior compatibility, not code mirroring.

