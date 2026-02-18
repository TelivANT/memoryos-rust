# Context Injector Specification (P0)

> **Status**: Implementation Ready
> **Module**: `crates/memoryos-core/src/llm/context.rs`
> **Objective**: Retrieve relevant memories and inject them into the System Prompt without breaking the conversation flow.

## 1. Interface Definition

```rust
pub trait ContextInjector: Send + Sync {
    /// Retrieve and inject context into the request
    async fn inject(&self, request: &mut ChatRequest, user_id: &str) -> Result<InjectionStats, AppError>;
}

pub struct InjectionStats {
    pub stm_count: usize,
    pub mtm_count: usize,
    pub total_tokens: usize,
}
```

## 2. Formatting Strategy (Prompt Engineering)

We use a structured format to help the LLM distinguish between "System Instructions" and "Retrieved Context".

**Template**:
```text
System Instruction: You are a helpful assistant.

[MEMORY CONTEXT BEGIN]
The following facts are retrieved from the user's long-term memory. Use them to answer if relevant.

<fact id="1" type="profile" confidence="0.95">
User prefers Rust over Python.
</fact>

<fact id="2" type="knowledge" confidence="0.88">
The company wifi password is 'SecurePass123'.
</fact>
[MEMORY CONTEXT END]
```

## 3. Token Budgeting

*   **Limit**: `config.memory.max_context_tokens` (e.g., 2000).
*   **Priority**:
    1.  Direct Hit / FAQ (Highest)
    2.  Recent STM (Last 5 turns)
    3.  High Confidence MTM (> 0.85)
    4.  General MTM (Lowest)
*   **Action**: Truncate low-priority items if budget exceeded.
