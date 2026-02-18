# Security Shield Specification (P0)

> **Status**: Implementation Ready
> **Module**: `crates/memoryos-core/src/security/shield.rs`
> **Objective**: Input Sanitization, Prompt Injection Defense, and PII Redaction.

## 1. Interface Definition

```rust
pub trait SecurityShield: Send + Sync {
    /// Validate input for malicious patterns (Injection)
    fn validate_input(&self, text: &str) -> Result<(), SecurityError>;
    
    /// Redact sensitive info (PII)
    fn sanitize_pii(&self, text: &str) -> String;
    
    /// Check for compliance keywords
    fn check_compliance(&self, text: &str) -> ComplianceResult;
}

pub enum ComplianceResult {
    Safe,
    RequiresLocal, // Contains "Confidential" -> Route to Local
    Blocked,       // Contains "Ignore previous instructions"
}
```

## 2. Implementation Logic

### 2.1 Prompt Injection Defense
*   **Regex Blocklist**:
    *   `(?i)ignore previous instructions`
    *   `(?i)system override`
*   **Action**: Return `SecurityError::PromptInjection`.

### 2.2 PII Sanitization
*   **Regex Replacement**:
    *   Email: `[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}` -> `<EMAIL>`
    *   API Key (Generic): `sk-[a-zA-Z0-9]{32,}` -> `<API_KEY>`
*   **Note**: This is a regex-based heuristic. Production should use NLP (e.g., Presidio) in Worker, but Gateway needs fast regex.

### 2.3 Compliance Check
*   **Keywords**: Load from `config.router.sensitive_keywords`.
*   **Action**: If match found, return `RequiresLocal`.
