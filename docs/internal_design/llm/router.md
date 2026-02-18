# Model Router Specification (P0)

> **Status**: Implementation Ready
> **Module**: `crates/memoryos-core/src/llm/router.rs`
> **Objective**: Intelligent traffic steering based on semantic confidence and query complexity.

## 1. Interface Definition

### 1.1 Router Trait
```rust
#[async_trait]
pub trait ModelRouter: Send + Sync {
    /// Decide where to route the user query
    async fn route(&self, query: &str, context: &RouterContext) -> Result<RouteDecision>;
}

pub struct RouterContext {
    pub user_id: String,
    pub session_id: Option<String>,
    pub available_backends: Vec<BackendStatus>,
}

#[derive(Debug)]
pub enum RouteDecision {
    /// Tier 0: Return static text immediately (No LLM)
    DirectHit { content: String, confidence: f32 },
    
    /// Tier 1: Route to Local LLM (Cost-effective)
    Local { model: String, endpoint: String },
    
    /// Tier 2: Route to Cloud LLM (High Intelligence)
    Cloud { provider: String, model: String },
}
```

## 2. Routing Logic (The Algorithm)

The router executes a waterfall logic:

### Step 1: Direct Hit Check (FAQ)
*   **Input**: `vector_store.search(query, filter={type: "faq"})`
*   **Logic**: 
    ```rust
    if top_result.score > config.router.direct_hit_threshold (0.92) {
        return DirectHit(top_result.payload);
    }
    ```

### Step 2: Complexity Analysis (Heuristic)
*   **Input**: Query text.
*   **Logic**:
    ```rust
    let token_count = tokenizer.count(query);
    let has_code = query.contains("```") || query.contains("function");
    let is_complex = token_count > config.router.max_local_tokens (2000) || has_code;
    ```

### Step 3: Hotspot Check (Semantic Confidence)
*   **Input**: `vector_store.search(query, filter={scope: "global"})`
*   **Logic**:
    ```rust
    let hot_score = top_result.score;
    if !is_complex && hot_score > config.router.hot_threshold (0.85) {
        return Local(RoundRobin(local_backends));
    }
    ```

### Step 4: Fallback
*   **Action**: `return Cloud(default_upstream)`;

## 3. Configuration (`config.toml`)

```toml
[router]
enable = true
direct_hit_threshold = 0.92
hot_threshold = 0.85
max_local_tokens = 2000
sensitive_keywords = ["confidential", "secret"]

[[router.local_backends]]
name = "llama-3-8b"
endpoint = "http://192.168.1.10:11434"
```
