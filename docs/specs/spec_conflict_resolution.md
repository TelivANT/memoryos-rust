# Spec Conflict Resolution (P0)

> Status: Approved
> Scope: MemoryOS-Rust docs alignment before coding
> Objective: Eliminate cross-document contradictions that would cause implementation drift.

## 1. Resolution Policy

### 1.1 Source of Truth Priority
When specs conflict, use this precedence:
1. `docs/specs/api_standard.md`
2. `docs/specs/architecture_design.md`
3. Domain-specific specs under `docs/specs/*`
4. Internal design notes under `docs/internal_design/*`
5. Execution sequencing under `docs/plan/execution_master.md`
6. Examples/reference docs under `docs/api_reference/*`, `docs/ops/*`, `docs/business/*`, `docs/legal/*`

### 1.2 ID Semantics (Normative)
- `request_id`: per HTTP request (ingress correlation).
- `trace_id`: distributed tracing chain id (can span multiple components).
- `event_id`: unique id per emitted async event (dedup key).
- `task_id`: queue-consumer execution id (retry attempts share the same `event_id`, may have different `task_id`).

`event_id` MUST be used for deduplication. `trace_id` MUST NOT be used as dedup key.

## 2. Critical Conflicts and Final Decisions

### CF-001: Degraded Mode vs Readiness
- Severity: P0
- Conflict:
  - Degraded mode says service should continue when Qdrant/Redis partial failure occurs.
  - Readiness currently requires Redis and Qdrant both healthy.
- Decision:
  - Introduce three health states: `ready`, `degraded_ready`, `not_ready`.
  - `/health/ready` returns 200 for `ready` and `degraded_ready`.
  - `/health/status` returns detailed dependency matrix.
  - Response header MUST include `X-MemoryOS-Status: degraded` when any critical dependency is bypassed.
- Impact:
  - Gateway health middleware, K8s probes, alert rules.
- Acceptance:
  - Kill Qdrant: chat endpoint still serves proxy mode, returns `X-MemoryOS-Status: degraded`.
  - `/health/ready` remains 200, `/health/status` marks Qdrant unavailable.

### CF-002: Degraded Header Naming Mismatch
- Severity: P0
- Conflict:
  - `X-MemoryOS-Status` vs `X-Status`.
- Decision:
  - Canonical header: `X-MemoryOS-Status`.
  - `X-Status` is deprecated and removed from tests.
- Impact:
  - Integration tests, API docs.
- Acceptance:
  - All tests assert only `X-MemoryOS-Status`.

### CF-003: Dedup Key Misuse
- Severity: P0
- Conflict:
  - Current text implies UUIDv7 `trace_id` is dedup id.
- Decision:
  - Producer emits both `trace_id` and `event_id`.
  - Redis dedup set key format: `processed_events:{yyyy_mm_dd_hh}` with member = `event_id`.
- Impact:
  - Event schema, worker consumer, observability fields.
- Acceptance:
  - Multiple events in same trace are processed exactly once each.

### CF-004: Distributed Lock TTL Race
- Severity: P0
- Conflict:
  - `SET NX EX 10` may expire before long updates finish.
- Decision:
  - Keep lock with lease renewal heartbeat (every 3s).
  - Add fencing token (`lock_version`) persisted on write targets.
  - Writes must include CAS/fencing checks.
- Impact:
  - Redis lock util, profile/fact update path, Qdrant/SQL write APIs.
- Acceptance:
  - Simulated long task > 10s does not allow concurrent writer corruption.

### CF-005: Auth Model Fragmentation
- Severity: P0
- Conflict:
  - API key auth, JWT blacklist, and RBAC are not unified.
- Decision:
  - Define unified principal context:
    - `subject_id`, `tenant_id`, `auth_method`, `scopes`, `token_jti` (optional), `api_key_id` (optional).
  - All middleware and audit logs consume this principal context.
  - Revocation checks apply by method:
    - API key: key status and key revocation list.
    - JWT: blacklist by `jti`.
- Impact:
  - Auth middleware, audit, billing, rate limiter keys.
- Acceptance:
  - Same RBAC policy works regardless of auth method.

### CF-006: GDPR Deletion Does Not Cover External Wiki
- Severity: P0
- Conflict:
  - Internal delete cascade exists, but no external export revocation SLA.
- Decision:
  - Add mandatory deletion propagation:
    - S3: delete object + tombstone record.
    - Confluence/GitBook: delete page or replace with redaction notice.
    - Webhook target: send `wiki_delete` event.
  - Add `deletion_jobs` table with retry and audit trail.
- Impact:
  - Wiki exporter adapter contracts, admin tooling, legal compliance.
- Acceptance:
  - `DELETE /v1/users/{id}` produces auditable external deletion workflow completion.

## 3. High-Risk Logic Gaps and Final Decisions

### CF-007: FAQ Promotion Sybil Risk
- Severity: P1
- Decision:
  - Promotion requires all:
    - minimum unique users,
    - minimum tenant entropy,
    - anti-bot score below threshold,
    - no abnormal like velocity.
  - Likes from same actor within cooldown window count once.
- Acceptance:
  - Simulated bot burst cannot promote Tier-0 FAQ.

### CF-008: Translation Cache Staleness
- Severity: P1
- Decision:
  - Cache key changed to `trans:{faq_id}:{faq_version}:{target_lang}`.
  - Any FAQ update invalidates prior translation keys.
- Acceptance:
  - Updating source FAQ forces new translation output.

### CF-009: Embedding Migration Scoring Incomparability
- Severity: P1
- Decision:
  - During dual-read, rank by per-model normalized score (`z-score` or calibrated sigmoid).
  - Define cutover gates:
    - Recall@3 delta >= 0
    - Hallucination rate not worse than baseline
    - p95 latency within SLO budget
- Acceptance:
  - Cutover cannot proceed without passing gate report.

### CF-010: Silent Parameter Drops in Adapters
- Severity: P1
- Decision:
  - Unsupported fields must be surfaced via response metadata header:
    - `X-MemoryOS-Adapter-Warnings: frequency_penalty,logit_bias`
  - Optional strict mode: reject unsupported params with 400.
- Acceptance:
  - Client can detect that provider ignored unsupported options.

### CF-011: Quota Strategy Conflict
- Severity: P1
- Decision:
  - Runtime enforcement source of truth is `config.toml`.
  - Pricing docs become policy examples and must reference config-backed limits.
  - Default production behavior:
    - 80% warning,
    - 100% soft limit,
    - 200% hard block.
- Acceptance:
  - Behavior and billing docs remain consistent under one config profile.

### CF-012: Duplicate/Drifting Spec Sections
- Severity: P1
- Decision:
  - Remove duplicate sections in `wiki_export_spec.md` and enforce unique section IDs.
  - Add CI lint script to detect duplicated headings and duplicate numeric section labels.
- Acceptance:
  - Doc CI fails on duplicate heading/section numbering.

## 4. Security Hardening Addendum

### CF-013: Secret Exposure in Repository
- Severity: P0
- Decision:
  - Immediate key rotation for all exposed credentials.
  - Remove secrets from tracked config files and replace with env placeholders.
  - Add pre-commit and CI secret scanning (gitleaks/trufflehog).
- Acceptance:
  - No live key patterns in repo history head and current tree.

### CF-014: Dependency Pinning Rigidity
- Severity: P1
- Decision:
  - Pin critical crates; allow patch-range updates for non-critical crates with lockfile.
  - Weekly security update window plus emergency fast-track process.
- Acceptance:
  - `cargo audit` clean; security patch lead time <= 24h for critical CVEs.

## 5. Implementation Checklist (Before Writing Core Code)

1. Freeze this file as `Approved`.
2. Update conflicting docs to align with this resolution file.
3. Add doc CI checks:
   - header consistency,
   - duplicate section detection,
   - required field glossary (`request_id/trace_id/event_id/task_id`).
4. Only then start workspace scaffolding and trait contracts.

## 6. Owner and Change Control

- Owner: Tech Committee
- Change Policy:
  - P0 sections require explicit review approval.
  - Any change impacting API or legal compliance requires version bump in `docs/state.json`.
