# Memory Conflict Resolution Specification (P0)

> **Status**: Approved
> **Objective**: Handle contradictory facts in LTM to ensure AI consistency.

## 1. Conflict Detection Logic

**Scenario**: New fact ("User loves Rust") vs Old fact ("User loves Python").

### 1.1 Semantic Similarity Check
*   **Trigger**: Before inserting any new Fact to LTM.
*   **Action**: Search Qdrant for existing facts with `cosine_similarity > 0.85`.
*   **Result**: If match found, we have a potential conflict.

## 2. Resolution Strategies

### 2.1 Strategy A: Time-Based Supersession (Default)
**Philosophy**: People change. Newest information is truth.

*   **Action**:
    1.  Mark old fact as `is_active: false`.
    2.  Add field `superseded_by: {new_fact_id}` to old fact.
    3.  Insert new fact with `valid_from: <now>`.
*   **Retrieval**: Filter `is_active: true`.

### 2.2 Strategy B: User Confirmation (Interactive)
**Philosophy**: Ask before assuming.

*   **Trigger**: If new fact conflicts with a "High Confidence" old fact (confidence > 0.9).
*   **Action**:
    1.  Do NOT update LTM yet.
    2.  Store candidate fact in `memory_candidates`.
    3.  Next Response: Inject "I recall you loved Python. Do you prefer Rust now?"
*   **Resolution**:
    *   User "Yes": Execute Strategy A.
    *   User "No": Discard candidate.

---

## 3. Data Structure (Fact Schema)

```json
{
  "id": "fact_123",
  "content": "User prefers Rust",
  "confidence": 0.95,
  "created_at": "2026-03-01T10:00:00Z",
  "valid_to": null,
  "superseded_by": null,
  "source_message_id": "msg_abc"
}
```
