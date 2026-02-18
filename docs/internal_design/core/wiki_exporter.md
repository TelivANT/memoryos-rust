# Wiki Exporter Specification (P1)

> **Status**: Implementation Ready
> **Module**: `crates/memoryos-worker/src/wiki/`
> **Objective**: Periodically publish mature knowledge to external systems.

## 1. Interface Definition

```rust
#[async_trait]
pub trait WikiAdapter: Send + Sync {
    /// Upload a document (Create or Update)
    async fn publish(&self, doc: WikiDocument) -> Result<String, AppError>;
    
    /// Delete a document (GDPR Recall)
    async fn recall(&self, doc_id: &str) -> Result<(), AppError>;
}

pub struct WikiDocument {
    pub id: String,
    pub title: String,
    pub content: String, // Markdown with Mermaid
    pub category: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}
```

## 2. Export Pipeline (The "Publisher")

**Trigger**: Cron Job (e.g., Daily at 2 AM).

**Logic**:
1.  **Query**: Select Qdrant points where:
    *   `type == "faq"`
    *   `like_count > 5`
    *   `last_exported < last_updated`
2.  **Render**: Convert JSON Payload -> Markdown.
    *   Frontmatter (YAML)
    *   Body
    *   **Mermaid Injection**: If `relations` exist, generate `graph TD` block.
3.  **Sanitize**: Run PII Shield.
4.  **Publish**: Call `WikiAdapter.publish()`.
5.  **Ack**: Update `last_exported` timestamp in Qdrant.

## 3. Supported Adapters

### 3.1 S3 / Object Storage (Default)
*   **Path**: `s3://bucket/{category}/{slug}.md`
*   **Index**: Update `index.json`.

### 3.2 Wiki.js (GraphQL)
*   **API**: `Mutation { pages { create(...) } }`
*   **Format**: Wiki.js native markdown.

### 3.3 Confluence
*   **API**: REST API v2.
*   **Format**: Convert Markdown to Atlasian Document Format (ADF) if possible, or use raw storage format.
