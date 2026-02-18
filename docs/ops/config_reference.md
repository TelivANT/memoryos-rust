# Configuration Reference (config.toml)

> **Status**: Approved
> **Target**: Phase 1 Implementation
> **File Path**: `config.toml` (Default) or `MEMORYOS_CONFIG=/path/to/config.toml`

This file documents all available configuration options. MemoryOS supports environment variable overrides using `MEMORYOS__` prefix (double underscore).
Example: `MEMORYOS__SERVER__PORT=9090` overrides `[server] port`.

---

## 1. Server Configuration (Core)

```toml
[server]
# Bind address (0.0.0.0 for Docker/K8s)
host = "0.0.0.0"
# Port to listen on
port = 8080
# Number of worker threads for Axum (default: CPU cores)
worker_threads = 4
# Global request timeout (seconds)
timeout_seconds = 60
# Enable CORS for frontend integration
enable_cors = true
```

## 2. LLM Providers (Multi-Backend Registry)

MemoryOS supports defining multiple upstream providers and switching between them.

### 2.1 Global Default
```toml
[llm]
# The name of the provider to use by default
default_provider = "gemini_pro"
# Default model (can be overridden by request)
default_model = "gemini-1.5-pro"
```

### 2.2 Provider Registry
Define your backends here. Keys (e.g., `gemini_pro`, `local`) are user-defined identifiers.

```toml
# Example 1: Google Gemini (Native Protocol)
[llm.providers.gemini_pro]
type = "gemini"  # native gemini protocol
base_url = "http://101.132.72.12:3000/gemini"
api_key_env = "GEMINI_API_KEY" # Reads from env var
max_retries = 3

# Example 2: Custom OpenAI Proxy (e.g. "Goblin")
[llm.providers.goblin]
type = "openai"  # standard openai protocol
base_url = "http://101.132.72.12:8000/openai/v1"
api_key_env = "CODEX_API_KEY"

# Example 3: Local Ollama
[llm.providers.local]
type = "ollama"
base_url = "http://localhost:11434"
# No API key needed for Ollama
```

## 3. Storage Layer (Persistence)

### 3.1 Redis (Short-Term Memory & Cache)
```toml
[storage.redis]
# Connection string (supports redis:// and rediss://)
url = "redis://localhost:6379/0"
# Connection pool size
pool_size = 10
# STM Capacity (Number of turns to keep)
stm_capacity = 20
```

### 3.2 Vector Database (Mid-Term Memory)
```toml
[storage.vector]
# Type: "qdrant" (Recommended)
type = "qdrant"
# Qdrant gRPC endpoint
url = "http://localhost:6334"
# Collection name
collection = "memoryos_v1"
# API Key (if Qdrant is secured)
api_key = "${QDRANT_API_KEY}"
```

## 4. Router & Intelligence (The Brain)

```toml
[router]
# Enable intelligent routing (Local vs Cloud)
enable = true
# Compliance Toggle: Force local route for sensitive keywords
enable_compliance = true
# Keywords that trigger local-only route
sensitive_keywords = ["confidential", "internal", "secret"]

# Hotspot Logic
# If Global Memory similarity > 0.85, route to Local
hot_threshold = 0.85
# If Input Tokens > 2000, force Cloud (Local model might be too weak)
max_local_tokens = 2000

# Local Model Pool (Round-Robin)
local_backends = [
    "http://192.168.1.10:11434/v1",
    "http://192.168.1.11:11434/v1"
]
```

## 5. Security & Privacy

```toml
[security]
# Master key for encrypting Private Memory in DB (AES-256-GCM)
# Generate with `openssl rand -hex 32`
encryption_key = "${MEMORYOS_MASTER_KEY}"

# Rate Limiting (per user IP)
rate_limit_per_min = 60
```

## 6. Cost Control & Sync

```toml
[cost_control]
# Monthly Token Quota (Input+Output) per user
user_monthly_token_limit = 1_000_000
# Action when quota exceeded: "readonly" | "block"
over_quota_action = "readonly"

# Abuse Prevention (IP based)
ip_daily_token_limit = 5_000_000
ip_daily_account_creation_limit = 3
enable_abuse_detection = true

[memory]
# Sync Mode: "async" (Default) | "sync" (Wait for write)
update_mode = "async"
# If hybrid, important memories > 0.9 confidence are synced immediately
sync_threshold_importance = 0.9
```

## 7. Observability

```toml
[observability]
# Log level: trace, debug, info, warn, error
log_level = "info"
# Output format: "json" (Production) or "pretty" (Dev)
log_format = "json"
# Enable Prometheus metrics endpoint /metrics
enable_metrics = true
```

## 8. Wiki Export (Knowledge Asset Precipitation)

```toml
[wiki_export]
# Enable automatic export of mature FAQs
enable = true
min_age_days = 30
min_access_count = 10
min_like_count = 5
schedule = "0 2 * * 0"
target = "s3"

[wiki_export.s3]
bucket = "company-wiki"
region = "us-west-2"
prefix = "memoryos-export/"
access_key = "${AWS_ACCESS_KEY}"
secret_key = "${AWS_SECRET_KEY}"

[wiki_export.categorizer]
llm_model = "gpt-4o-mini"
categories = ["IT/network", "HR/benefits"]
```
