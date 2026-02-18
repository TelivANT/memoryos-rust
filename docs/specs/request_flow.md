# Request Flow: URL to Infrastructure Mapping

> **Objective**: Identify the component chain for a specific URL within 30 seconds.
> **Key**: 🟢 Success Path | 🔴 Failure Point | 🟡 Async Trigger

## 1. Core Chat Endpoint
**URL**: `POST /v1/chat/completions`
**Description**: The main entry point for User Chat (Standard OpenAI Protocol).

### 🔗 Execution Chain (Synchronous Path)

| Step | Component | Infrastructure | Action | Failure Symptom (🔴) |
| :--- | :--- | :--- | :--- | :--- |
| **1** | **Load Balancer** | K8s Ingress (Nginx) | Route traffic to Gateway Service | `502 Bad Gateway` / `Connection Refused` |
| **2** | **API Gateway** | **Pod: gateway-v1** | Auth Check & Request Validation | `401 Unauthorized` / `400 Bad Request` |
| **3** | **Auth Context** | **Redis + SQL** | Validate API Key/JWT, revocation, and RBAC scope | `401 Unauthorized` / `403 Forbidden` |
| **4** | **STM Context** | **Redis Cluster** | `LRANGE user:{id}:stm` (Fetch Short-Term Memory) | Slow Response (High Latency) |
| **5** | **MTM Context** | **Qdrant Cluster** | `SEARCH vectors` (Fetch Mid-Term Memory) | Slow Response / Empty Context |
| **6** | **LLM Proxy** | **Upstream API** | Forward Request to OpenAI/Gemini | `504 Gateway Timeout` / Upstream Error |
| **7** | **Event Emit** | **NATS/Kafka** | 🟡 Publish `chat.log` event | (Invisible to User) |

### 🔍 Quick Troubleshooting
*   **If 401/403**: Check principal resolution (API Key/JWT), Redis revocation set, and SQL key status.
*   **If 502**: Check **Ingress** or **Gateway Pod** status.
*   **If Slow**: Check **Qdrant** latency or **Upstream LLM** status.
*   **If dependency degraded**: Validate `X-MemoryOS-Status: degraded` and confirm fallback path is active.
    *   Redis down + Qdrant up: vector retrieval path remains active, STM path is bypassed.
    *   Qdrant down + Redis up: STM path remains active, vector retrieval path is bypassed.
    *   Redis down + Qdrant down: gateway falls back to pure proxy/noop memory mode.
*   **If dependency recovers**: verify `/health/status` flips to `ready` and memory capability is upgraded without restarting gateway.

---

## 2. Memory Management (Async)
**Trigger**: Event `chat.log` (from Step 7 above)
**Description**: Background processing to update memory (Summarization & Embedding).

### 🔗 Execution Chain (Asynchronous Path)

| Step | Component | Infrastructure | Action | Failure Symptom (🔴) |
| :--- | :--- | :--- | :--- | :--- |
| **1** | **Queue** | **NATS/Kafka** | Buffer messages | Queue Lag spikes (Monitor Dashboard) |
| **2** | **Worker** | **Pod: worker-v1** | Consume message & Update Logic | Worker Pod Restarting / OOM |
| **3** | **STM Update** | **Redis Cluster** | `RPUSH` new message, `LTRIM` old | Recent memory missing in next chat |
| **4** | **LLM Process** | **Upstream API** | Call LLM to summarize (if needed) | Memory not consolidating |
| **5** | **LTM Update** | **Qdrant Cluster** | `UPSERT` new vectors | Long-term memory missing |

### 🔍 Quick Troubleshooting
*   **Queue Lag High**: **Worker Pods** are insufficient or crashing. Scale up Workers.
*   **Memory Not Updating**: Check **Worker Logs** for LLM API errors.

---

## 3. Admin & Profile Endpoints
**URL**: `GET /v1/profile` / `POST /v1/admin/users`

| Step | Component | Infrastructure | Action |
| :--- | :--- | :--- | :--- |
| **1** | **Gateway** | **Pod: gateway-v1** | Route to Admin Handler |
| **2** | **Metadata DB** | **SQL DB (Postgres)** | Read/Write User Profile & Config |

*   **Failure**: `500 Internal Error` usually points to **SQL DB** connectivity.
