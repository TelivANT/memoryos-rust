# Embedding Migration Specification (P0)

> **Status**: Approved
> **Objective**: Zero-downtime migration between incompatible embedding models (e.g., bge-m3 -> qwen-v3).

## 1. Versioning Strategy

Every vector stored in Qdrant MUST carry model metadata.

### Schema Definition
```json
{
  "id": "uuid-v7",
  "vector": [...],
  "payload": {
    "embedding_model": "bge-m3",
    "embedding_version": "1.0",
    "content": "Raw text content..."
  }
}
```

---

## 2. Migration Workflow (The "Dual-Stack" Approach)

**Scenario**: Migrating from Model A (Old) to Model B (New).

### Phase 1: Parallel Write (T+0)
*   **Action**: Gateway loads BOTH Model A and Model B.
*   **Write**: Every new memory is vectorized twice.
    *   Collection A: `vectors_v1` (Model A)
    *   Collection B: `vectors_v2` (Model B)
*   **Read**: Continue searching `vectors_v1`.

### Phase 2: Backfill (Background Worker)
*   **Action**: `Re-indexer Worker` scans `vectors_v1`.
*   **Logic**: Read `payload.content` -> Vectorize with Model B -> Upsert to `vectors_v2`.
*   **Progress**: Tracked in Redis `migration:progress`.

### Phase 3: Dual Retrieval (Verification)
*   **Action**: Gateway searches BOTH collections.
*   **Normalization**: Apply Z-Score normalization or Sigmoid to map scores to [0,1] probability space before merging.
*   **Logic**: Compare results. If Model B returns better semantic matches (Admin review), proceed.

### Phase 4: Cutover (Switch)
*   **Action**: Update `config.toml` -> `current_model = "model_b"`.
*   **Read**: Only search `vectors_v2`.
*   **Write**: Only write `vectors_v2`.

### Phase 5: Sunset
*   **Action**: Archive `vectors_v1` to S3 (Cold Storage). Delete from Qdrant after 60 days.

---

## 3. Rollback Plan

If Model B proves defective in production:
1.  Revert `config.toml` to Model A.
2.  Since `vectors_v1` was never deleted, service restores immediately.
