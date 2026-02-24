# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

- Wiki preview system (built-in HTTP browser + Mermaid rendering)
- Production environment end-to-end validation

---

## [1.0.0-rc] - 2026-02-25

### Summary
All 6 must-complete tasks for 1.0 are done. 17/17 Storage Connectors implemented.
MCP Server operational. 303 unit tests passing. Zero production panics.

### Added
- **MCP Server** (`memoryos-mcp` crate): rmcp v0.3, 7 tools, Gateway proxy pattern, stdio transport (PR #48)
- **9 Storage Connectors**: GCS, Azure Blob, SMB, NFS, OneDrive, Google Drive, Dropbox, Baidu Pan, Aliyun Drive (PR #51)
- **Config Validation**: `AppConfig::validate()` with comprehensive checks for all config sections, 24 unit tests (PR #42)
- **69 new unit tests** across 7 modules (234 → 303 total), covering gateway middleware, core error, memory types, health, history, ports LLM, metrics (PR #47)
- **Real Embedding Integration**: `with_embedding_config()` method, wired into gateway and worker (PR #46)
- **LLM Summary Pipeline**: `consolidate_memory()` calls `summarize_messages_internal()` for real LLM summarization (PR #45)
- **Rate limiting middleware** wired into gateway HTTP pipeline
- **Dockerfile.gateway** and **Dockerfile.worker** for proper container deployment
- **config.docker.toml** for Docker Compose with correct service URLs

### Fixed
- Replaced all production `unwrap()` with `expect()` or graceful error handling across 4 crates (PR #43)
- Docker Compose now uses `depends_on: condition: service_healthy` for reliable startup ordering
- Gateway dead code (`RateLimiter`) now wired into middleware stack
- Version numbers aligned across VERSION, Cargo.toml, and documentation

### Changed
- `docker-compose.yml`: monitoring services (Prometheus/Grafana) moved to `monitoring` profile
- `docker-compose.yml`: worker moved to `full` profile (optional)
- `config.toml`: removed hardcoded example API keys, added generation instructions

### Security
- Removed hardcoded API keys from config.toml (was `memoryos-secret-key-12345`)

### Migration from 0.x
1. **Config**: Run `cargo run --bin memoryos-gateway` — startup validation will report any missing/invalid config fields
2. **Docker**: Use `docker compose up` (core services) or `docker compose --profile full up` (with worker)
3. **API Keys**: Generate new keys with `openssl rand -hex 32`, update `config.toml`
4. **MCP**: Add `memoryos-mcp` to Claude Desktop / Cursor config for AI agent integration

---

## [0.13.0] - 2026-02-24

### Added
- MCP Server design and architecture documentation
- Storage Connector framework (8/17 initial implementations: Local, Git, S3, WebDAV, OSS, COS, OBS, SFTP)

### Changed
- Updated ARCHITECTURE.md, DESIGN.md, ROADMAP.md with MCP Server design
- README.md updated to Phase 16

---

## [0.12.0 ~ 0.12.6] - 2026-02-20

### Added
- **RBAC**: Role-based access control (SuperAdmin/Admin/User/ReadOnly) with 6 permissions
- **Multi-tenant**: TenantManager with enable/disable/quota, data isolation via Qdrant tenant_id filter + Redis key prefix
- **Admin Service** (`memoryos-admin`): independent management service on port 9090
- SQLite persistence for RBAC users and tenant data

### Security
- Constant-time token comparison (`subtle::ConstantTimeEq`)
- Admin CORS restricted to localhost:3000 + `ADMIN_CORS_ORIGINS` env var
- RateLimiter memory leak fix (evicts stale IPs when >1000 entries)
- Nested routes moved under auth middleware
- Fail-closed RBAC (unknown users get 403)

---

## [0.10.0 ~ 0.11.0] - 2026-02-20

### Added
- **Prometheus Observability**: `/metrics` endpoint, HTTP request/duration/router/FAQ/LLM counters
- **LLM FAQ Classifier**: auto-classification with prompt builder + response parser
- Tag search via Qdrant native payload filter
- Memory history wired into gateway
- Graph LLM extraction endpoints
- Audit/GDPR storage backends (file-based persistence)

### Fixed
- Redis upgraded 0.24 → 0.32
- Auth warning when `auth.enabled = false`

---

## [0.7.0 ~ 0.9.0] - 2026-02-20

### Added
- Criterion performance benchmarks (optimization, graph, security)
- AES-256-GCM encryption (upgraded from XOR)
- Audit log persistence (JSONL)
- GDPR record persistence (JSON)
- Security API (`/v1/security/*`)

---

## [0.3.0 ~ 0.6.0] - 2026-02-20

### Added
- FAQ Router Tier 0 direct hit
- Wiki S3/Confluence export
- Knowledge Graph (GraphRAG): entity/relation extraction, graph query API
- Multimodal storage (Qdrant) + HTTP endpoints
- Memory version control + tags + export/import

---

## [0.2.0] - 2026-02-18

### Added
- 3-Tier Memory Architecture (STM → MTM → LTM)
- 10 LLM Adapters (OpenAI/Claude/Gemini/Ollama/Deepseek/OpenRouter/Azure/Groq/Cohere/Mistral)
- Security Shield (PII sanitization, 17 prompt injection patterns)
- 6 optimization modules (Bloom Filter, LRU Cache, Batch, Heat Buffer, Similarity Filter, Incremental Summary)
- Graceful degradation

### Security
- Fixed Admin API authentication bypass (CVSS 9.8 → 0.0)
- Fixed API Key insecure storage (CVSS 8.1 → 0.0)
- Fixed STM memory leak (CVSS 7.5 → 0.0)

---

## [0.1.0] - 2026-02-17
- Initial project skeleton

### Migration from 0.x

1. **Config**: Run `cargo run --bin memoryos-gateway` — startup validation will report any missing/invalid config fields
2. **Docker**: Use `docker compose up` (core services) or `docker compose --profile full up` (with worker)
3. **API Keys**: Generate new keys with `openssl rand -hex 32`, update `config.toml`
4. **MCP**: Add `memoryos-mcp` to Claude Desktop / Cursor config for AI agent integration

---

## [0.13.0] - 2026-02-24

### Added
- MCP Server design and architecture documentation
- Storage Connector framework (8/17 initial implementations: Local, Git, S3, WebDAV, OSS, COS, OBS, SFTP)

---

## [0.12.0 ~ 0.12.6] - 2026-02-20

### Added
- **RBAC**: Role-based access control (SuperAdmin/Admin/User/ReadOnly) with 6 permissions
- **Multi-tenant**: TenantManager with enable/disable/quota, data isolation via Qdrant tenant_id filter + Redis key prefix
- **Admin Service** (`memoryos-admin`): independent management service on port 9090
- SQLite persistence for RBAC users and tenant data

### Security
- Constant-time token comparison (`subtle::ConstantTimeEq`)
- Admin CORS restricted to localhost:3000 + `ADMIN_CORS_ORIGINS` env var
- RateLimiter memory leak fix (evicts stale IPs when >1000 entries)
- Nested routes moved under auth middleware
- Fail-closed RBAC (unknown users get 403)

---

## [0.10.0 ~ 0.11.0] - 2026-02-20

### Added
- **Prometheus Observability**: `/metrics` endpoint, HTTP request/duration/router/FAQ/LLM counters
- **LLM FAQ Classifier**: auto-classification with prompt builder + response parser
- Tag search via Qdrant native payload filter
- Memory history wired into gateway
- Graph LLM extraction endpoints
- Audit/GDPR storage backends (file-based persistence)

### Fixed
- Redis upgraded 0.24 → 0.32
- Auth warning when `auth.enabled = false`

---

## [0.7.0 ~ 0.9.0] - 2026-02-20

### Added
- Criterion performance benchmarks (optimization, graph, security)
- AES-256-GCM encryption (upgraded from XOR)
- Audit log persistence (JSONL)
- GDPR record persistence (JSON)
- Security API (`/v1/security/*`)

---

## [0.3.0 ~ 0.6.0] - 2026-02-20

### Added
- FAQ Router Tier 0 direct hit
- Wiki S3/Confluence export
- Knowledge Graph (GraphRAG): entity/relation extraction, graph query API
- Multimodal storage (Qdrant) + HTTP endpoints
- Memory version control + tags + export/import

---

## [0.2.0] - 2026-02-18

### Added
- 3-Tier Memory Architecture (STM → MTM → LTM)
- 10 LLM Adapters (OpenAI/Claude/Gemini/Ollama/Deepseek/OpenRouter/Azure/Groq/Cohere/Mistral)
- Security Shield (PII sanitization, 17 prompt injection patterns)
- 6 optimization modules (Bloom Filter, LRU Cache, Batch, Heat Buffer, Similarity Filter, Incremental Summary)
- Graceful degradation

### Security
- Fixed Admin API authentication bypass (CVSS 9.8 → 0.0)
- Fixed API Key insecure storage (CVSS 8.1 → 0.0)
- Fixed STM memory leak (CVSS 7.5 → 0.0)

---

## [0.1.0] - 2026-02-17
- Initial project skeleton
