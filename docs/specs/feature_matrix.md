# MemoryOS-Rust Feature Matrix & Gap Analysis

## 1. Core Memory Logic (核心记忆逻辑)

| Feature | Python (Legacy) | Rust (Target) | Status | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **STM (Short-Term)** | List/Deque (In-memory) | `VecDeque` (In-memory) | ✅ Must-Have | Keep FIFO logic. |
| **MTM (Mid-Term)** | JSON + FAISS/Chroma | **SQLite + Vector Extension** | 🔄 Proposed Change | **Major Upgrade**. Moving away from heavy JSON files to embedded SQL for performance. |
| **LTM (Long-Term)** | JSON (Profile/Facts) | SQLite (Relational Tables) | 🔄 Proposed Change | Structured storage allows better querying than parsing JSON text. |
| **Versioning** | (None) | **Time-Travel Safe** | 🛡️ Accuracy | **TTL & Deprecation**. Old knowledge (e.g., old VPN pass) is archived, not mixed with new facts. |
| **ACL/RBAC** | (None) | **Group-based Access** | 🛡️ Security | **Granular Privacy**. HR knowledge is only accessible to HR group, even in Global Scope. |
| **Heat Algorithm** | $f(freq, dur, recency)$ | Standardized Formula | ✅ Must-Have | Logic remains the same, implementation optimized. |
| **Multi-Lang** | (None) | **Auto-Translation** | 🌍 Localization | **Bridge the Gap**. If User Lang != FAQ Lang, auto-translate the Direct Hit response. |
| **Lifecycle** | (None) | **Decay & Pruning** | 🧠 Intelligence | **Anti-Bloat**. Auto-merge similar old memories; Auto-delete low-value ephemeral data. "Forgetting is part of remembering." |
| **Privacy** | (None) | **Encryption at Rest** | 🔐 Security | **God Mode Prevention**. Private memories are AES-encrypted in DB. Even DB Admins see garbage without the key. |
| **Sovereignty** | (None) | **GDPR Purge** | ⚖️ Legal | **Right to be Forgotten**. One-command to strictly wipe a user's private data while anonymizing their public contributions. |
| **Graph Memory** | (Not Implemented) | (Optional Future) | ⏳ Nice-to-Have | Knowledge Graph support for LTM? |
| **Wiki Export** | (None) | **Auto-Export to S3/Confluence** | 🌟 Killer Feature | **Knowledge Precipitation**. FAQs aged >30 days auto-export to Markdown → S3/Wiki. Converts implicit knowledge to explicit assets. |

## 2. LLM & AI Capabilities (模型能力)

| Feature | Python (Legacy) | Rust (Target) | Status | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **OpenAI Support** | Standard SDK | `reqwest` + OpenAI Schema | ✅ Must-Have | Base compatibility. |
| **Gemini Support** | Via OpenAI SDK (Broken) | **Native Google Adapter** | ✅ Must-Have | **Critical Fix**. Direct support for Google's native API structure. |
| **Local LLM** | Ollama (Basic) | Ollama / Local API | ✅ Must-Have | First-class support for local privacy. |
| **Embedding** | `sentence-transformers` | **ONNX Runtime (ort)** | 🚀 Optimization | Run quantization models (bge-m3) locally without Python/PyTorch deps. |

| **Multi-Modal** | (None) | **OCR-First Vision** | 💰 Cost | **See & Remember**. Use local OCR for docs (Cheap); Vision LLM for photos (Premium). |
| **Reminder** | (None) | **Contextual Greeting** | ❤️ UX | **No Spam**. Instead of push notifications, inject "Last time we talked about..." when user opens chat. |
| **Integrations** | (None) | **Browser Extension** | 🌐 Ingest | **Capture Everything**. "Save to MemoryOS" button in Chrome. Summarize web pages into memory. |
| **Live Sync** | (None) | **Local File Watcher** | ⚡ Real-time | **Codebase Awareness**. Watch `./src` for changes. Auto-update memory when file saves. |

## 3. Interfaces & Integrations (接口与集成)

| Feature | Python (Legacy) | Rust (Target) | Status | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **CLI Mode** | Basic script | **Interactive REPL** | ✅ Must-Have | Standalone chat tool. |
| **HTTP API** | FastAPI (Separate) | **Axum Gateway** | ✅ Must-Have | Core of the new architecture. |
| **OpenAI Proxy** | (None) | **Universal Protocol Translator** | 🌟 Core Feature | **One Interface to Rule Them All**. User speaks OpenAI Protocol -> MemoryOS translates -> Upstream (Gemini/Claude/Azure). |
| **Hybrid Routing**| (None) | **Tiered Intelligence** | 🌟 Killer Feature | **Cost Optimization**. Route "Hot/Simple" queries to Local LLM (Llama); Route "Cold/Complex" to Cloud LLM. |
| **Direct Hit** | (None) | **FAQ Mode** | ⚡ Speed | **Zero Latency**. If Qdrant matches a "Standard Answer" (e.g., Wifi Pass) with >92% confidence, bypass LLM and return text directly. |
| **Single Backend**| (None) | **Bypass Mode** | ⚡ Optimization | **Scale Down**. If only 1 local LLM is configured, bypass router overhead entirely. Ideal for "Mac Studio" deployments. |
| **Local LB** | (None) | **Round-Robin LB** | ✅ Must-Have | Load balance requests across multiple local model instances (e.g., 3x Llama nodes). |

### 3.1 Upstream Adapter Matrix (预置厂商适配表)

We will implement a **Normalization Layer** that converts the standard OpenAI Request/Response format into the vendor-specific formats below. This ensures clients (Cursor/VSCode) work with *any* backend.

| Vendor | Protocol | Auth Header | Path Structure | Special Handling (Normalization) |
| :--- | :--- | :--- | :--- | :--- |
| **OpenAI** | Standard | `Authorization: Bearer` | `/v1/chat/completions` | Pass-through. Supports DeepSeek/Moonshot/Groq. |
| **Google Gemini** | **Native REST** | `x-goog-api-key` (or Bearer) | `/v1beta/models/{model}:generateContent` | Convert `messages` -> `contents`. Extract `system` role -> `system_instruction`. |
| **Anthropic** | **Native REST** | `x-api-key` | `/v1/messages` | Extract `system` param. Merge consecutive user messages. Handle SSE delta conversion. |
| **Azure OpenAI** | Custom | `api-key` | `/deployments/{id}/chat/completions` | Construct URL from endpoint + deployment. Handle API version query param. |
| **Ollama** | OpenAI-Compat | (None/Basic) | `/v1/chat/completions` | Support `keep_alive`, `num_ctx` passthrough. |
| **SiliconFlow** | OpenAI-Compat | `Authorization: Bearer` | `/v1/chat/completions` | SiliconCloud / Qwen optimized. |

## 4. Storage & Persistence (存储与持久化)

| Feature | Python (Legacy) | Rust (Target) | Status | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **File Format** | Large `.json` files | **JSON (Compat) + SQLite** | ✅ Must-Have | **Dual Mode**. Default to SQLite for performance, but strictly compatible with legacy JSON if configured. |
| **ChromaDB** | Supported | **Optional Adapter** | ⚠️ Low Priority | Support via Feature Flag. |
| **Multi-Tenancy**| (File-based) | **Native Multi-User** | ✅ Must-Have | **Identity Awareness**. Serve multiple users via API Keys. Isolate memory per `user_id`. |
| **Org Memory** | (None) | **Global Scope** | 🌟 Killer Feature | **Collective Intelligence**. Solved problems are promoted to `Global` scope, accessible by all employees. |
| **Evolution** | (None) | **Auto-Promotion** | 🚀 Automation | **Crowdsourcing**. If an answer gets N likes, it automatically upgrades to FAQ/Global Memory. |
| **Gamification**| (None) | **HR Audit Log** | 🛡️ Management | **Incentive System**. Track who contributes good data (Reward) and who pollutes data (Penalty). |
| **Feedback Loop** | (None) | **Like/Dislike API** | ✅ Must-Have | **Quality Control**. Untrusted memories are deprecated; High-quality ones are promoted. |
| **Wiki Integration** | (None) | **S3/Confluence/Wiki.js** | 🌟 Killer Feature | **Cross-System Knowledge Sharing**. Export mature FAQs to external Wiki systems. Supports **Wiki.js**, Outline, Confluence. |
| **Code Intelligence**| (None) | **Repo-to-Wiki** | 🧠 Advanced | **Devin-like Capabilty**. Scan Git repos -> Parse AST -> Generate Mermaid Diagrams -> Auto-write Tech Docs. |

## 5. Operations (运维)

| Feature | Python (Legacy) | Rust (Target) | Status | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **Config** | JSON / Hardcoded | **TOML + Env Vars** | ✅ Must-Have | Standard modern config practice. |
| **Security** | (None) | **PII Sanitizer** | ✅ Must-Have | **Data Loss Prevention**. Scrub API Keys/Emails before storing or sending to Upstream. |
| **SSRF Defense**| (None) | **Egress Filter** | 🛡️ Security | **Network Lockdown**. Prevent MemoryOS from scanning internal networks unless whitelisted. |
| **Prompt Shield**| (None) | **Injection Filter**| 🛡️ Security | **Memory Integrity**. Detect and block "Ignore previous instructions" attacks on Updater. |
| **Compliance** | (None) | **Configurable Data Residency** | 🛡️ Optional | **Leak Prevention**. User can toggle `enable_compliance` to force sensitive queries to Local LLM. Defaults to `true` (Safe) but can be `false`. |
| **Cost Control** | (None) | **Token Budgeting** | ✅ Must-Have | Limit injected context tokens per request. Rerank & Compress memories. |
| **Deployment** | Pip + Venv + Conda | **Single Binary** | ✅ Must-Have | `curl | bash` style install. |
| **Data Migration** | (None) | **Re-indexing Engine** | 🛡️ Resilience | **Future Proofing**. Allow switching Embedding Models without data loss via background re-vectorization. |
| **Circuit Breaker**| (None) | **Degradation Mode** | 🛡️ Resilience | **High Availability**. If Redis/Qdrant fails, fallback to in-memory cache or bypass memory logic to keep chat alive. |
| **Knowledge Import**| (None) | **CSV/JSON Importer** | ✅ Must-Have | **Cold Start Fix**. Batch import FAQ/Docs into Global Memory before Day 1 launch. |
| **Logging** | Basic print/logging | **Tracing / OpenTelemetry** | ✅ Must-Have | Structured logging for debugging production issues. |
| **Analytics** | (None) | **Cluster Worker** | 🆕 New Feature | Background worker that mines cross-user patterns to generate "Problem Trends". |

## 6. High Availability & Scaling (高可用与扩展)

To support **20,000+ concurrent users** and multi-turn conversations, the architecture must scale horizontally.

| Component | Default Mode (Local/Dev) | Production Mode (Cluster) | Implementation Strategy |
| :--- | :--- | :--- | :--- |
| **Vector DB** | **SQLite + sqlite-vss** | **Qdrant** (Cluster) | Abstract `VectorStore` trait. Qdrant is the recommended prod backend for millions of vectors. |
| **Short-Term Memory** | In-Memory (`DashMap`) | **Redis** (Cluster) | Abstract `ShortTermStorage` trait. Redis ensures persistence across restarts and fast I/O. |
| **Task Queue** | `tokio::mpsc` | **Redis Stream** / **NATS** | Abstract `EventBus` trait. Async tasks (e.g., memory summarization) are pushed to queue and consumed by workers. |
| **API Gateway** | Axum (Single Instance) | Axum (Multi-Instance) | Stateless design. Can be load-balanced by Nginx/K8s Ingress. |

---

## 🔑 Confirmed Decisions (已确认决策)

1.  **Gateway Architecture**: MemoryOS-Rust will primarily act as an **OpenAI-Compatible Proxy**. It sits between the User Client and the Upstream Model (Ollama/Gemini/OpenAI).
    *   **Workflow**: User Request -> **MemoryOS (Inject Context)** -> Upstream Model -> **MemoryOS (Capture Log)** -> User Response.
    *   **Benefit**: Users don't need plugins. Just change `base_url` in their client.
2.  **Data Compatibility**: Must support reading/writing legacy `json` files to ensure smooth migration.
3.  **Multi-Tenancy**: The system is designed for multi-user environments (Company Intranet). Users are identified via API Keys.
4.  **CLI Tool**: A standalone terminal chat interface is required for direct interaction.
5.  **Scaling Strategy**:
    *   **Redis First**: Use Redis for both caching hot context (STM) and message queuing (Updater tasks).
    *   **Qdrant Integration**: Native support for Qdrant as the heavy-duty vector store.
    *   **NATS Future-Proof**: Interface design will accommodate NATS for future extreme scaling.
