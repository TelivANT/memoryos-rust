# MemoryOS-Rust Project Definition

## 1. Project Overview (Mission)

This project is a high-performance, strictly typed, and locally deployable rewrite of the MemoryOS architecture. It aims to provide persistent, context-aware memory for AI agents by mimicking the human memory process.

## 2. Core Scientific Logic (The Memory Hierarchy)

The system is designed based on the **Operating System Memory Management** metaphor and human cognitive psychology (Atkinson-Shiffrin model).

### 2.1 Short-Term Memory (STM)
*   **Concept**: L1 Cache / Working Memory.
*   **Logic**: First-In-First-Out (FIFO) Deque. Stores the immediate conversation context (last N turns).
*   **Lifecycle**: Volatile. Contents are either discarded or promoted to Mid-Term Memory upon capacity overflow.

### 2.2 Mid-Term Memory (MTM)
*   **Concept**: RAM / Episodic Memory.
*   **Logic**: Vector-indexed segments of conversation history.
*   **Key Algorithm**: **Heat-based Retention**.
    *   $H = f(Frequency, Duration, Recency)$
    *   Segments with $H > H_{threshold}$ are considered "hot" and trigger analysis for Long-Term promotion.
    *   Segments with low $H$ are eventually evicted (LRU) or archived.
*   **Storage**: Embedding vectors + Metadata (timestamps, access counts).

### 2.3 Long-Term Memory (LTM)
*   **Concept**: Disk / Semantic Memory & Procedural Memory.
*   **Logic**: Structured knowledge base.
    *   **User Profile**: Synthesized persona traits (e.g., "User is a Rust developer").
    *   **Fact Knowledge**: Discrete facts extracted from conversations.
*   **Lifecycle**: Persistent. Updated via background LLM tasks (Updater).

## 3. Architecture Definition (Hexagonal)

The codebase strictly follows the **Ports and Adapters (Hexagonal)** architecture to ensure testability and flexibility.

*   **Core (Domain)**: Pure Rust logic defining `Memory`, `Message`, `Segment`. No external dependencies like HTTP or specific DB drivers leak here.
*   **Ports (Traits)**: Interface definitions (`LLMClient`, `Storage`, `Embedder`).
*   **Adapters (Infrastructure)**: Concrete implementations (`OpenAIAdapter`, `QdrantAdapter`, `AxumHandler`).

## 4. Key Improvements over Legacy Python Version

1.  **Concurrency**: Rust's ownership model and `tokio` runtime eliminate GIL issues, allowing massive concurrency for retrieval and updates.
2.  **Safety**: Compile-time checks prevent `NoneType` errors common in the Python codebase.
3.  **Deployment**: Single binary executable with zero runtime dependencies.
4.  **Network Resilience**: Custom `Reqwest` middleware to handle complex proxy/gateway scenarios (e.g., Google Gemini non-standard paths).

## 5. References

*   *Original Implementation*: `https://github.com/BAI-LAB/MemoryOS`
*   *Cognitive Science*: "The Magical Number Seven, Plus or Minus Two" (Miller, 1956) - Basis for STM capacity.
*   *Vector Search*: HNSW (Hierarchical Navigable Small World) graphs for MTM retrieval.
