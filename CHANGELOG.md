# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-02-18

### 🚀 Major Features
- **3-Tier Architecture**: Full implementation of STM (Redis) -> MTM (Qdrant) -> LTM (SQLite).
- **Intelligent Router V2**: Tiered routing (Direct Hit -> Local Llama -> Cloud GPT) based on semantic confidence and complexity.
- **Universal Gateway**: OpenAI-compatible API proxy supporting Google Gemini (Native), Anthropic Claude, and Ollama.
- **Graph Memory**: Qdrant-native graph storage with Mermaid visualization support.
- **Wiki Export**: Automated knowledge precipitation to S3/Confluence.

### 🛡️ Security & Compliance
- **Security Shield**: PII sanitization, prompt injection defense, and SSRF egress filtering.
- **RBAC**: Token-based blacklisting and real-time permission enforcement.
- **GDPR**: "Right to be Forgotten" with deletion cascade across Vector DB and S3.
- **Encryption**: AES-256-GCM encryption for private memory payloads.

### ⚙️ Operations
- **High Availability**: Redis Sentinel/Cluster support, Qdrant sharding plan.
- **Observability**: Distributed tracing (TraceID), structured JSON logs, and Prometheus metrics.
- **Cost Control**: Token budgeting, global rate limiting, and IP-based abuse detection.

### 🐛 Fixes
- Fixed Gemini Proxy 404/400 errors by implementing a native adapter.
- Fixed Worker race conditions using Redis distributed locks and fencing tokens.
- Fixed Embedding migration risks with dual-write/read architecture.

---

## [0.1.0] - 2026-02-17
- Initial Project Skeleton.
