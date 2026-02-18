# Multi-Modal Vision Specification (P0)

> **Status**: Approved (High ROI)
> **Objective**: Enable "See and Remember" capability.

## 1. Feature Overview

**Scenario**:
*   User uploads a screenshot of an error log.
*   MemoryOS extracts text (OCR) + semantic meaning (CLIP).
*   User asks: "How to fix the error I sent yesterday?" -> MemoryOS retrieves the image context.

## 2. Technical Architecture

### 2.1 Embedding Strategy
*   **Text**: `bge-m3` (Existing).
*   **Image**: `CLIP-ViT-L-14` (New).
*   **Unified Space**: We assume text and image embeddings are **NOT** aligned (unless using SigLIP).
*   **Storage**: Separate Qdrant Collection `vision_memory`.

### 2.2 Processing Pipeline (Worker)
1.  **Ingest**: Gateway receives `image_url` or base64.
2.  **Download**: Worker fetches image to temp.
3.  **Tier 1: OCR First (Cost Saving)**
    *   Run local `tesseract` (or `ocrs` Rust crate).
    *   If text density > 50 words -> Store as Text Memory. **Stop**.
4.  **Tier 2: Vision LLM (Deep Understanding)**
    *   **Trigger**: If OCR fails OR User explicitly asks "Describe this image".
    *   **Budget Check**: Ensure User has `vision_quota > 0`.
    *   Call `gpt-4o` / `llava` -> Generate Description -> Store.

### 2.3 Cost Control (Vision Budget)
*   **Free**: 5 Vision calls / day (OCR unlimited).
*   **Pro**: 100 calls / day.
*   **Fallback**: If quota exceeded, only run OCR. Return warning: "Image analyzed via OCR only (Quota exceeded)."

## 3. Retrieval
*   User asks text query.
*   System searches `vision_memory` using text vector.
*   Returns: "I found an image where [Description]..."

## 4. Why this wins?
*   **Cheap**: We don't need heavy GPU for CLIP. We use Vision LLM (API) to convert Image -> Text, then use standard Text Search.
*   **Effective**: Text search is more precise for RAG than raw image similarity.
