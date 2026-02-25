# API 文档

**版本**: v1.0.0-rc
**仓库**: [TelivANT/memoryos-rust](https://github.com/TelivANT/memoryos-rust)
**基础 URL**: `http://localhost:8080`

---

## 目录

- [公开端点](#公开端点无需认证)
- [聊天 API](#聊天-api)
- [记忆 API](#记忆-api)
- [知识图谱 API](#知识图谱-api)
- [记忆管理 API](#记忆管理-api)
- [多模态 API](#多模态-api)
- [安全与合规 API](#安全与合规-api)
- [Wiki 生成 API](#wiki-生成-api)
- [存储连接器 API](#存储连接器-api)
- [管理 API](#管理-api)
- [FAQ 管理 API](#faq-管理-api)
- [错误处理](#错误处理)
- [降级模式](#降级模式)

---

## 认证

当 `auth.enabled = true` 时，所有 `/v1/*` 端点需要在请求头中携带 API Key：

```
Authorization: Bearer <your-api-key>
```

管理端点 (`/v1/admin/*`) 需要 admin 权限的 API Key。

---

## 公开端点（无需认证）

### GET /health

存活检查。

**响应**:
```json
{"status": "ok"}
```

---

### GET /health/status

详细健康状态（实时探测 Redis 和 Qdrant）。

**响应**:
```json
{
  "mode": "ready",
  "redis": "up",
  "qdrant": "up"
}
```

`mode` 取值：`ready` / `degraded_ready` / `not_ready`

**状态码**: 200 OK / 503 Service Unavailable

---

### GET /health/live

Kubernetes Liveness Probe。进程存活即返回 200。

**响应**:
```json
{
  "status": "ok",
  "timestamp": "2026-02-25T12:00:00Z"
}
```

---

### GET /health/ready

Kubernetes Readiness Probe。检查 Redis 和 Qdrant 是否可用。

**响应** (200 OK):
```json
{
  "status": "ready",
  "timestamp": "2026-02-25T12:00:00Z"
}
```

**响应** (503 Service Unavailable):
```json
{
  "status": "not_ready",
  "timestamp": "2026-02-25T12:00:00Z"
}
```

降级模式下响应头包含 `X-MemoryOS-Status: degraded`。

---

### GET /metrics

Prometheus 格式指标端点。

---

## 聊天 API

### POST /v1/chat/completions

OpenAI 兼容的聊天补全接口。MemoryOS 会自动注入记忆上下文到 LLM 请求中。

**请求体**:
```json
{
  "model": "gpt-4o-mini",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello"}
  ],
  "temperature": 0.7,
  "max_tokens": 1000,
  "stream": false
}
```

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| model | string | 是 | 模型名称（由 LLM Router 路由到对应 provider） |
| messages | array | 是 | 消息列表 |
| temperature | float | 否 | 温度 (0-2) |
| max_tokens | int | 否 | 最大 token 数 |
| stream | bool | 否 | 是否流式返回 |

所有未列出的参数会自动透传到上游 LLM API（如 `top_p`、`tools`、`response_format` 等）。

**非流式响应** (stream=false):
```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1708156800,
  "model": "gpt-4o-mini",
  "choices": [{
    "index": 0,
    "message": {"role": "assistant", "content": "Hello!"},
    "finish_reason": "stop"
  }],
  "usage": {"prompt_tokens": 20, "completion_tokens": 15, "total_tokens": 35}
}
```

**流式响应** (stream=true):
```
Content-Type: text/event-stream

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: [DONE]
```

降级模式下响应头包含 `X-MemoryOS-Status: degraded`。

---

## 记忆 API

### POST /v1/memory/add

添加记忆消息。支持同步写入和异步管道两种模式。

**请求体**:
```json
{
  "user_id": "alice_001",
  "role": "user",
  "content": "My name is Alice and I work as a data scientist",
  "event_id": "evt-optional-idempotency-key"
}
```

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| user_id | string | 是 | 用户 ID |
| role | string | 是 | 消息角色 (user/assistant/system) |
| content | string | 是 | 消息内容 |
| event_id | string | 否 | 幂等 key，不填则自动生成 UUIDv7 |

**响应**:
```json
{"status": "ok"}
```

---

### POST /v1/memory/retrieve

检索用户记忆上下文（STM + MTM + LTM）。

**请求体**:
```json
{
  "user_id": "alice_001",
  "query": "What do you know about me?"
}
```

**响应**:
```json
{
  "short_term": [
    {"role": "user", "content": "My name is Alice...", "timestamp": "2026-02-18T11:00:00Z"}
  ],
  "mid_term": [
    {"id": "...", "summary": "Alice is a data scientist", "score": 0.92}
  ],
  "long_term": {
    "profile": "User profile summary...",
    "knowledge": ["Works as a data scientist"]
  }
}
```

---

### GET /v1/memory/{memory_id}/history

获取单条记忆的版本历史。

**路径参数**: `memory_id` — 记忆 ID

**响应**:
```json
[
  {"version": 2, "content": "updated content", "changed_at": "..."},
  {"version": 1, "content": "original content", "changed_at": "..."}
]
```

---

## 知识图谱 API

所有图谱端点挂载在 `/v1/graph` 下。

### POST /v1/graph/extract

从文本中提取实体和关系（规则引擎，10 种关系模式）。

**请求体**:
```json
{"text": "Alice works at Google as a senior engineer."}
```

**响应**:
```json
{
  "entities_added": 2,
  "triples": [
    {"subject": "alice", "predicate": "works_at", "object": "google"},
    {"subject": "alice", "predicate": "has_role", "object": "senior_engineer"}
  ]
}
```

---

### POST /v1/graph/extract/llm-prompt

生成用于 LLM 实体提取的 prompt（不调用 LLM）。

**请求体**:
```json
{"text": "Alice works at Google."}
```

**响应**:
```json
{"prompt": "Extract entities and relations from the following text..."}
```

---

### POST /v1/graph/extract/llm-parse

解析 LLM 返回的实体提取结果并合并到图谱中。

**请求体**:
```json
{"response": "Entity: Alice (Person)\nRelation: Alice works_at Google"}
```

**响应**:
```json
{
  "entities_added": 2,
  "triples": [...],
  "total_entities": 5,
  "total_relations": 3
}
```

---

### POST /v1/graph/query

按标签查询实体及其关系。

**请求体**:
```json
{"query": "alice"}
```

**响应**:
```json
{
  "entities": [{"id": "alice", "label": "Alice", "entity_type": "Person"}],
  "triples": [{"subject": "alice", "predicate": "works_at", "object": "google"}]
}
```

---

### POST /v1/graph/path

查询两个实体之间的路径（DFS）。

**请求体**:
```json
{"from": "Alice", "to": "Google", "max_depth": 3}
```

**响应**:
```json
{"paths": [["alice", "works_at", "google"]], "count": 1}
```

---

### GET /v1/graph/triples

获取图谱中所有三元组。

**响应**:
```json
{"triples": [...], "count": 10}
```

---

### GET /v1/graph/stats

获取图谱统计信息。

**响应**:
```json
{"entity_count": 15, "relation_count": 10}
```

---

## 记忆管理 API

所有端点挂载在 `/v1/memory/manage` 下。支持 `X-Tenant-ID` 请求头进行多租户数据隔离。

### POST /v1/memory/manage/tags

为记忆片段添加标签。

**请求体**:
```json
{
  "user_id": "alice_001",
  "segment_id": "uuid-of-segment",
  "tags": ["important", "work"]
}
```

**响应**:
```json
{"status": "ok", "message": "Tags added successfully"}
```

---

### POST /v1/memory/manage/search/tags

按标签搜索记忆片段。

**请求体**:
```json
{"user_id": "alice_001", "tags": ["work"], "limit": 50}
```

**响应**:
```json
{
  "status": "ok",
  "count": 3,
  "segments": [
    {"id": "...", "summary": "...", "tags": ["work"], "memory_type": "QA", "heat_score": 1.5, "version": 1}
  ]
}
```

---

### POST /v1/memory/manage/export

导出用户记忆数据（JSON 或 Markdown 格式）。

**请求体**:
```json
{"user_id": "alice_001", "format": "json"}
```

`format` 可选值：`json`（默认）、`markdown`

**响应** (JSON 格式):
```json
{
  "status": "ok",
  "format": "json",
  "data": {
    "user_id": "alice_001",
    "exported_at": "2026-02-20T10:00:00Z",
    "segment_count": 5,
    "segments": [...]
  }
}
```

---

### POST /v1/memory/manage/import

导入记忆数据。

**请求体**:
```json
{
  "user_id": "alice_001",
  "segments": [
    {"summary": "Alice likes hiking", "tags": ["hobby"], "memory_type": "qa"}
  ]
}
```

`memory_type` 可选值：`qa`（默认）、`faq_candidate`、`faq`

**响应**:
```json
{"status": "ok", "imported": 1}
```

---

### POST /v1/memory/manage/versions

获取记忆片段的版本历史链。

**请求体**:
```json
{"user_id": "alice_001", "segment_id": "uuid-of-segment"}
```

**响应**:
```json
{
  "status": "ok",
  "segment_id": "...",
  "version_count": 3,
  "history": [
    {"id": "...", "version": 3, "summary": "...", "tags": [...], "created_at": "...", "updated_at": "..."}
  ]
}
```

---

## 多模态 API

所有端点挂载在 `/v1/multimodal` 下。使用 Qdrant 向量存储，支持文本/图像/音频 embedding 输入。

### POST /v1/multimodal/store

存储多模态消息。

**请求体**:
```json
{
  "user_id": "alice_001",
  "role": "user",
  "contents": [
    {"type": "text", "text": "Check this photo"},
    {"type": "image", "embedding": [0.1, 0.2, ...]}
  ]
}
```

**响应**:
```json
{"status": "ok"}
```

---

### POST /v1/multimodal/search

按文本搜索多模态消息。

**请求体**:
```json
{"user_id": "alice_001", "query": "photo of sunset", "limit": 10}
```

---

### POST /v1/multimodal/search/embedding

按 embedding 向量搜索（支持图像/音频模态）。

**请求体**:
```json
{
  "user_id": "alice_001",
  "embedding": [0.1, 0.2, ...],
  "limit": 10,
  "modality": "image"
}
```

`modality`：`image`（默认）或 `audio`

---

### POST /v1/multimodal/recent

获取最近的多模态消息。

**请求体**:
```json
{"user_id": "alice_001", "limit": 10}
```

---

## 安全与合规 API

所有端点挂载在 `/v1/security` 下。

### POST /v1/security/audit/logs

查询审计日志。

**请求体**:
```json
{"user_id": "alice_001", "limit": 50}
```

`user_id` 可选，不填则返回全局最近日志。

**响应**:
```json
{"status": "ok", "count": 10, "events": [...]}
```

---

### GET /v1/security/audit/stats

获取审计统计。

**响应**:
```json
{"status": "ok", "total_events": 150}
```

---

### POST /v1/security/gdpr/export

导出用户数据（GDPR 数据可携带权）。

**请求体**:
```json
{"user_id": "alice_001"}
```

**响应**:
```json
{"status": "ok", "export": {...}}
```

---

### POST /v1/security/gdpr/delete

请求删除用户数据（GDPR 被遗忘权）。会从 Redis 和 Qdrant 中删除该用户的所有数据。

**请求体**:
```json
{"user_id": "alice_001"}
```

**响应**:
```json
{"status": "ok", "message": "User data deleted from all backends"}
```

如果部分后端删除失败：
```json
{"status": "partial", "message": "Some backends failed", "errors": ["qdrant: ..."]}
```

---

### POST /v1/security/gdpr/consent

记录用户同意/撤回。

**请求体**:
```json
{"user_id": "alice_001", "purpose": "data_processing", "granted": true}
```

---

### POST /v1/security/gdpr/consent/check

检查用户是否已同意特定用途。

**请求体**:
```json
{"user_id": "alice_001", "purpose": "data_processing"}
```

**响应**:
```json
{"status": "ok", "user_id": "alice_001", "purpose": "data_processing", "has_consent": true}
```

---

## Wiki 生成 API

所有端点挂载在 `/v1/wiki` 下。基于 Tree-sitter + LLM 混合管线，从代码仓库自动生成 Markdown Wiki。

### POST /v1/wiki/generate

触发 Wiki 生成任务（异步，返回 job_id）。

**请求体**:
```json
{
  "repo_path": "/path/to/local/repo",
  "config": {
    "output_dir": "./wiki-output",
    "languages": ["rust", "python"]
  }
}
```

`config` 可选，不填使用默认配置。

**响应** (202 Accepted):
```json
{"job_id": "wiki-1708156800000", "status": "pending", "message": "Wiki generation started"}
```

---

### POST /v1/wiki/parse

仅解析代码仓库（不生成文档），返回 IR 统计。

**请求体**:
```json
{"repo_path": "/path/to/local/repo"}
```

**响应**:
```json
{
  "files": 42,
  "symbols": 350,
  "references": 120,
  "endpoints": 47,
  "diagnostics": 3,
  "manifests": 2
}
```

---

### GET /v1/wiki/status

获取所有 Wiki 生成任务状态。

**响应**:
```json
{
  "jobs": [
    {"id": "wiki-...", "repo_path": "...", "status": "completed", "pages_generated": 15, "started_at": "...", "completed_at": "..."}
  ]
}
```

---

### GET /v1/wiki/jobs/{job_id}

获取单个任务状态。

**响应**:
```json
{"id": "wiki-...", "status": "running", "pages_generated": 0, "error": null}
```

---

## 存储连接器 API

Wiki 生成支持多种远程代码仓库连接方式。端点挂载在 `/v1/wiki/connectors` 下。

### GET /v1/wiki/connectors

列出所有支持的连接器类型及其配置字段。

**响应**:
```json
{
  "connectors": [
    {
      "type": "local",
      "name": "Local Filesystem",
      "description": "Browse local directories",
      "auth_required": false,
      "fields": [{"name": "path", "type": "string", "required": true, "description": "Local directory path"}]
    },
    {"type": "git", "name": "Git Repository", ...},
    {"type": "s3", "name": "Amazon S3", ...},
    {"type": "webdav", "name": "WebDAV", ...},
    {"type": "sftp", "name": "SFTP", ...},
    {"type": "oss", "name": "Alibaba Cloud OSS", ...},
    {"type": "cos", "name": "Tencent Cloud COS", ...},
    {"type": "obs", "name": "Huawei Cloud OBS", ...}
  ]
}
```

---

### POST /v1/wiki/connectors/test

测试连接器连接，成功后创建会话（1 小时 TTL）。

**请求体**:
```json
{
  "type": "git",
  "config": {
    "url": "https://github.com/example/repo.git",
    "branch": "main",
    "token": "ghp_xxx"
  }
}
```

**响应**:
```json
{
  "success": true,
  "connector_id": "uuid-session-id",
  "metadata": {"files_count": 42}
}
```

---

### POST /v1/wiki/connectors/browse

浏览连接器中的目录结构。

**请求体**:
```json
{"connector_id": "uuid-session-id", "path": "/src"}
```

**响应**:
```json
{
  "success": true,
  "path": "/src",
  "entries": [
    {"name": "main.rs", "path": "/src/main.rs", "type": "file", "size": 1024}
  ],
  "total": 5
}
```

---

### POST /v1/wiki/connectors/generate

使用已连接的连接器触发 Wiki 生成。

**请求体**:
```json
{"connector_id": "uuid-session-id"}
```

**响应**:
```json
{"job_id": "wiki-...", "status": "pending", "message": "Wiki generation started"}
```

---

### POST /v1/wiki/connectors

保存连接器配置（敏感字段 AES-256-GCM 加密存储）。

**请求体**:
```json
{
  "name": "My S3 Repo",
  "connector_type": "s3",
  "config": {"bucket": "my-bucket", "region": "us-east-1", "access_key": "...", "secret_key": "..."}
}
```

---

### GET /v1/wiki/connectors/saved

列出已保存的连接器（敏感字段脱敏显示）。

---

## 管理 API

需要 admin 权限的 API Key。

### POST /v1/admin/keys

创建新的 API Key。

**请求体**:
```json
{
  "api_key": "user-alice-key-abc123",
  "user_id": "alice@company.com",
  "description": "Alice dev key",
  "permissions": ["chat", "memory"]
}
```

**响应**:
```json
{"api_key": "user-alice-key-abc123", "message": "API Key created successfully"}
```

---

### DELETE /v1/admin/keys/{key}

删除指定 API Key。

**路径参数**: `key` — 要删除的 API Key

**响应**:
```json
{"message": "API Key deleted successfully"}
```

---

## FAQ 管理 API

挂载在 `/v1/admin/faq` 下。支持 `X-Tenant-ID` 请求头。

### GET /v1/admin/faq/candidates

获取 FAQ 候选列表（热度达标的高频 Q&A）。

**响应**:
```json
{
  "total": 5,
  "candidates": [
    {
      "id": "uuid",
      "user_id": "...",
      "summary": "How to reset password?",
      "access_count": 15,
      "heat_score": 8.5,
      "memory_type": "faq_candidate",
      "created_at": "...",
      "last_accessed": "..."
    }
  ]
}
```

---

### POST /v1/admin/faq/promote

手动提升候选为正式 FAQ。

**请求体**:
```json
{"memory_id": "uuid-of-candidate", "reason": "High frequency question"}
```

**响应**:
```json
{
  "success": true,
  "message": "Memory uuid promoted to FAQ",
  "record": {"id": "...", "memory_id": "...", "from_type": "FaqCandidate", "to_type": "Faq", ...}
}
```

---

### POST /v1/admin/faq/classify

FAQ 分类（离线/Dry-run）。生成分类 prompt 或解析 LLM 分类结果。

**请求体** (生成 prompt):
```json
{"question": "How to reset password?", "answer": "Go to settings..."}
```

**请求体** (解析结果):
```json
{"question": "...", "answer": "...", "response_text": "category: account\nconfidence: 0.95"}
```

---

### DELETE /v1/admin/faq/{id}

将 FAQ 降级回普通 QA。

**路径参数**: `id` — FAQ 的 UUID

---

### GET /v1/admin/faq/history

获取 FAQ 提升历史记录。

**响应**:
```json
{"history": [...], "total": 10}
```

---

### GET /v1/admin/faq/stats

获取 FAQ 统计信息。

---

## 错误处理

### 错误响应格式
```json
{
  "error": {
    "type": "BadRequest",
    "message": "Invalid model name",
    "details": "Model 'invalid-model' is not supported"
  }
}
```

### 错误类型

| 类型 | HTTP 状态码 | 说明 |
|------|-------------|------|
| BadRequest | 400 | 请求参数错误 |
| Unauthorized | 401 | 未授权 |
| NotFound | 404 | 资源不存在 |
| InternalError | 500 | 内部错误 |
| ExternalServiceError | 502 | 外部服务错误（LLM API） |
| ServiceUnavailable | 503 | 服务不可用 |

---

## 降级模式

当 Redis 或 Qdrant 不可用时，系统自动降级，保证核心功能可用。

| Redis | Qdrant | 模式 | 可用功能 |
|-------|--------|------|---------|
| Up | Up | **Full** | 全部功能 |
| Up | Down | **Degraded** | STM + LLM（无向量检索） |
| Down | Up | **Degraded** | MTM + LTM + LLM（无短期记忆） |
| Down | Down | **Noop** | 仅 LLM 透传（无记忆功能） |

检测方式：
- 响应头 `X-MemoryOS-Status: degraded`
- `GET /health/status` 返回 `mode` 字段

---

## 端点汇总表

| # | 方法 | 路径 | 说明 | 认证 |
|---|------|------|------|------|
| 1 | GET | /health | 存活检查 | 否 |
| 2 | GET | /health/status | 详细健康状态 | 否 |
| 3 | GET | /health/live | K8s Liveness Probe | 否 |
| 4 | GET | /health/ready | K8s Readiness Probe | 否 |
| 5 | GET | /metrics | Prometheus 指标 | 否 |
| 6 | POST | /v1/chat/completions | 聊天补全 (OpenAI 兼容) | 是 |
| 7 | POST | /v1/memory/add | 添加记忆 | 是 |
| 8 | POST | /v1/memory/retrieve | 检索记忆上下文 | 是 |
| 9 | GET | /v1/memory/{id}/history | 记忆版本历史 | 是 |
| 10 | POST | /v1/graph/extract | 实体提取 | 是 |
| 11 | POST | /v1/graph/extract/llm-prompt | LLM 提取 prompt | 是 |
| 12 | POST | /v1/graph/extract/llm-parse | LLM 提取结果解析 | 是 |
| 13 | POST | /v1/graph/query | 实体查询 | 是 |
| 14 | POST | /v1/graph/path | 路径查询 | 是 |
| 15 | GET | /v1/graph/triples | 所有三元组 | 是 |
| 16 | GET | /v1/graph/stats | 图谱统计 | 是 |
| 17 | POST | /v1/memory/manage/tags | 添加标签 | 是 |
| 18 | POST | /v1/memory/manage/search/tags | 按标签搜索 | 是 |
| 19 | POST | /v1/memory/manage/export | 导出记忆 | 是 |
| 20 | POST | /v1/memory/manage/import | 导入记忆 | 是 |
| 21 | POST | /v1/memory/manage/versions | 版本历史 | 是 |
| 22 | POST | /v1/multimodal/store | 存储多模态消息 | 是 |
| 23 | POST | /v1/multimodal/search | 文本搜索多模态 | 是 |
| 24 | POST | /v1/multimodal/search/embedding | 向量搜索多模态 | 是 |
| 25 | POST | /v1/multimodal/recent | 最近多模态消息 | 是 |
| 26 | POST | /v1/security/audit/logs | 查询审计日志 | 是 |
| 27 | GET | /v1/security/audit/stats | 审计统计 | 是 |
| 28 | POST | /v1/security/gdpr/export | GDPR 数据导出 | 是 |
| 29 | POST | /v1/security/gdpr/delete | GDPR 数据删除 | 是 |
| 30 | POST | /v1/security/gdpr/consent | 记录同意 | 是 |
| 31 | POST | /v1/security/gdpr/consent/check | 检查同意 | 是 |
| 32 | POST | /v1/wiki/generate | 触发 Wiki 生成 | 是 |
| 33 | POST | /v1/wiki/parse | 解析代码仓库 | 是 |
| 34 | GET | /v1/wiki/status | Wiki 任务列表 | 是 |
| 35 | GET | /v1/wiki/jobs/{job_id} | Wiki 任务详情 | 是 |
| 36 | GET | /v1/wiki/connectors | 连接器类型列表 | 是 |
| 37 | POST | /v1/wiki/connectors | 保存连接器配置 | 是 |
| 38 | GET | /v1/wiki/connectors/saved | 已保存连接器 | 是 |
| 39 | POST | /v1/wiki/connectors/test | 测试连接 | 是 |
| 40 | POST | /v1/wiki/connectors/browse | 浏览目录 | 是 |
| 41 | POST | /v1/wiki/connectors/generate | 连接器 Wiki 生成 | 是 |
| 42 | POST | /v1/admin/keys | 创建 API Key | Admin |
| 43 | DELETE | /v1/admin/keys/{key} | 删除 API Key | Admin |
| 44 | GET | /v1/admin/faq/candidates | FAQ 候选列表 | 是 |
| 45 | POST | /v1/admin/faq/promote | 提升为 FAQ | 是 |
| 46 | POST | /v1/admin/faq/classify | FAQ 分类 | 是 |
| 47 | DELETE | /v1/admin/faq/{id} | 降级 FAQ | 是 |
| 48 | GET | /v1/admin/faq/history | 提升历史 | 是 |
| 49 | GET | /v1/admin/faq/stats | FAQ 统计 | 是 |
