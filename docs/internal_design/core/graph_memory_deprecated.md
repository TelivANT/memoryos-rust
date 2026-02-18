# Graph Memory Specification (P1)

> **Status**: Approved
> **Target**: Phase 7 (V1-Final)
> **Architecture**: Qdrant-Native (No SQL/Neo4j)

## 1. Data Structure (Qdrant Payload)

We reuse Qdrant to store graph nodes. Each point is an **Entity**.

### 1.1 Entity Point
*   **Vector**: Embedding of entity name + description.
*   **Payload**:
    ```json
    {
      "type": "entity",
      "name": "MemoryOS Gateway",
      "description": "The main entry point for API requests.",
      "relations": [
        { "predicate": "connects_to", "target": "Redis", "target_id": "uuid-redis" },
        { "predicate": "connects_to", "target": "Qdrant", "target_id": "uuid-qdrant" }
      ]
    }
    ```

## 2. Extraction Pipeline (Worker)

1.  **Trigger**: Chat Log contains high-density information (or `type=playbook`).
2.  **LLM Call**: "Extract entities and relations from this text. Output Mermaid syntax."
3.  **Parsing**: Convert Mermaid -> JSON Graph.
4.  **Storage**: Upsert Points to Qdrant `graph_collection`.

## 3. Retrieval (GraphRAG)

1.  **Search**: `qdrant.search("Gateway architecture")`.
2.  **Expand**: Fetch Top-K entities.
3.  **Format**: Convert Entity + Relations back to **Mermaid**.
4.  **Inject**: `Context: [Mermaid Graph] ...`.

## 4. Visualization

*   **Wiki Export**: Embed the Mermaid code block directly into Markdown.
*   **API**: `GET /v1/graph?query=...` returns Mermaid text.
