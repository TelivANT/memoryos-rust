# Multilingual Support Specification (P0)

> **Status**: Approved
> **Objective**: Enable seamless cross-language memory retrieval and interaction.

## 1. Language Detection Strategy

### 1.1 Detection Hierarchy
The system determines the `current_language` based on the following priority:
1.  **Explicit User Setting**: `POST /v1/user/settings { "lang": "zh-CN" }`
2.  **Context Detection**: Analyze the last 3 user messages using `lingua` (Rust library).
3.  **HTTP Header**: `Accept-Language` from the request.
4.  **Default**: `en-US`.

### 1.2 Profile Persistence
Once detected, update `user_profile.preferred_language` in Metadata DB.

---

## 2. Cross-Language Retrieval

**Scenario**: User asks in Chinese, but the best answer (FAQ) is in English.

### 2.1 Query Translation (Search Time)
*   **Trigger**: If `current_language` != `vector_store_language` (default English).
*   **Action**: Translate Query -> English before embedding.
*   **Benefit**: Matches semantic meaning accurately in the high-dimensional space of the primary language.

### 2.2 Response Translation (Delivery Time)
*   **Trigger**: If retrieved FAQ language != `current_language`.
*   **Action**: Call LLM (or NMT model) to translate the FAQ content.
*   **Cache**: Store result in Redis `trans:{faq_id}:v{faq_version}:{target_lang}` (TTL: 7 days).

---

## 3. Storage Strategy

### 3.1 Dual Embedding (Optional)
For critical FAQs, store vectors in multiple languages to avoid translation loss.
*   `vectors_en`: Original English content.
*   `vectors_zh`: Translated Chinese content.

### 3.2 Metadata Tagging
All memory segments must have a `lang` field (e.g., `lang: "en"`).

---

## 4. Implementation Plan

*   **Phase 1**: Detect language and tag new memories.
*   **Phase 2**: Query translation logic in Retriever.
*   **Phase 3**: Response translation with Redis caching.
