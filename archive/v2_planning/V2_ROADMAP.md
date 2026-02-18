# MemoryOS-Rust V2.0 Roadmap: The "Second Brain" Evolution

> **Status**: Planning
> **Target**: Q3 2026
> **Theme**: "See, Read, and Watch"

## 1. Multi-Modal Vision (OCR First)
**Goal**: Handle images/documents cost-effectively.

*   **Strategy**:
    *   **Tier 1**: Local OCR (Tesseract) for high-density text (PDFs, Screenshots). Cost: $0.
    *   **Tier 2**: Vision LLM (GPT-4o) for semantic understanding (Photos, Art). Cost: $$$.
*   **Tech**: `ocrs` (Rust), `image` crate.

## 2. Contextual Greeting (Smart Reminder)
**Goal**: Re-engage users without spamming them.

*   **Logic**:
    *   When user connects (`POST /v1/chat/completions` with empty/init message),
    *   Search: "What was the user's last unfinished intent?"
    *   Response: "Welcome back! Do you want to continue debugging the Redis adapter?"
*   **Anti-Pattern**: NO Push Notifications. Zero Inbox Policy.

## 3. Omnipresent Ingestion
**Goal**: Capture knowledge from everywhere, not just Chat.

### 3.1 Browser Extension API
*   **Endpoint**: `POST /v1/ingest/webpage`
*   **Payload**: `{ "url": "...", "html_content": "..." }`
*   **Processing**: Extract Readability -> Summarize -> Store LTM.

### 3.2 Local File Watcher
*   **Goal**: Keep memory synced with local files.
*   **Tech**: `notify` crate (Rust).
*   **Config**: `watch_paths = ["./src", "./docs"]`
*   **Action**: On File Change -> Debounce -> Re-embed -> Update Qdrant.

## 4. Discarded Features (The "Not-To-Do" List)
*   ❌ **Knowledge Graph**: Too heavy, low ROI. Replaced by GraphRAG (Text-based).
*   ❌ **Versioning**: Too complex. Replaced by simple "Append-Only".
*   ❌ **Cross-Language**: Niche. Replaced by "English-Centric Retrieval".
*   ❌ **Sentiment Analysis**: No clear business case.

---

## 📅 Execution Timeline

| Sprint | Feature | Complexity | Value |
| :--- | :--- | :--- | :--- |
| **Sprint 1** | **OCR Integration** | Medium | High |
| **Sprint 2** | **File Watcher** | Medium | High |
| **Sprint 3** | **Contextual Greeting** | Low | High |
| **Sprint 4** | **Browser Ext API** | Medium | High |
