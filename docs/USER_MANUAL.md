# MemoryOS-Rust 用户手册

**版本**: v1.0.0-rc
**仓库**: [TelivANT/memoryos-rust](https://github.com/TelivANT/memoryos-rust)

---

## 目录

1. [快速开始](#快速开始)
2. [核心概念](#核心概念)
3. [聊天与记忆](#聊天与记忆)
4. [知识图谱](#知识图谱)
5. [记忆管理](#记忆管理)
6. [多模态记忆](#多模态记忆)
7. [FAQ 系统](#faq-系统)
8. [Wiki 生成](#wiki-生成)
9. [存储连接器](#存储连接器)
10. [安全与合规](#安全与合规)
11. [认证配置](#认证配置)
12. [多租户](#多租户)
13. [运维管理](#运维管理)
14. [故障排查](#故障排查)

---

## 快速开始

```bash
# 1. 克隆项目
git clone https://github.com/TelivANT/memoryos-rust.git
cd memoryos-rust

# 2. 启动依赖服务
docker compose up -d redis qdrant

# 3. 配置
cp config.example.toml config.toml
# 编辑 config.toml，填入 LLM API Key

# 4. 启动服务
cargo run --bin memoryos-gateway
```

访问: http://localhost:8080/health/status

详细步骤见 [快速开始文档](QUICKSTART.md)。

---

## 核心概念

### 三层记忆模型

MemoryOS 使用三层记忆架构管理对话历史和用户知识：

| 层 | 名称 | 存储 | 用途 |
|----|------|------|------|
| **STM** | 短期记忆 | Qdrant (short_term_messages) | 最近 N 条对话，按 timestamp 排序，TTL 自动过期 |
| **MTM** | 中期记忆 | Qdrant (mid_term_segments) | 对话片段摘要，向量化存储，热度追踪 |
| **LTM** | 长期记忆 | Qdrant (long_term_memory) | 用户画像 + 知识库 + FAQ |

消息流向：STM → 定期整理 → MTM → 热度提升 → LTM

### 降级模式

当 Redis 或 Qdrant 不可用时，系统自动降级而非完全不可用：

| Redis | Qdrant | 可用功能 |
|-------|--------|---------|
| Up | Up | 全部功能 |
| Up | Down | STM + LLM（无向量检索） |
| Down | Up | MTM/LTM + LLM（无短期记忆） |
| Down | Down | 仅 LLM 透传 |

降级状态通过 `X-MemoryOS-Status: degraded` 响应头标识。

---

## 聊天与记忆

### OpenAI 兼容聊天

MemoryOS Gateway 提供 OpenAI 兼容的聊天接口，自动注入记忆上下文：

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "What do you know about me?"}]
  }'
```

Gateway 会自动：
1. 检索该用户的 STM/MTM/LTM 记忆
2. 将记忆上下文注入到 LLM 请求中
3. 将新对话存入 STM

### 手动添加记忆

```bash
curl -X POST http://localhost:8080/v1/memory/add \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "alice",
    "role": "user",
    "content": "I prefer Python over Java for data analysis"
  }'
```

### 检索记忆上下文

```bash
curl -X POST http://localhost:8080/v1/memory/retrieve \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "query": "programming preferences"}'
```

### 异步记忆管道

默认同步写入记忆。启用异步管道可提升写入吞吐：

```bash
MEMORYOS_ASYNC_MEMORY_PIPELINE=true cargo run --bin memoryos-gateway
```

异步模式下 `/v1/memory/add` 立即返回，后台异步处理记忆存储。

---

## 知识图谱

从文本中自动提取实体和关系，支持 10 种关系模式。

### 提取实体

```bash
curl -X POST http://localhost:8080/v1/graph/extract \
  -H "Content-Type: application/json" \
  -d '{"text": "Alice works at Google as a senior engineer in Mountain View."}'
```

支持的关系类型：works_at、has_role、located_in、part_of、uses、created_by 等。

### LLM 辅助提取

对于复杂文本，可使用 LLM 辅助提取：

```bash
# 1. 生成 LLM prompt
curl -X POST http://localhost:8080/v1/graph/extract/llm-prompt \
  -H "Content-Type: application/json" \
  -d '{"text": "Complex paragraph..."}'

# 2. 将 prompt 发送到 LLM，获取响应后解析
curl -X POST http://localhost:8080/v1/graph/extract/llm-parse \
  -H "Content-Type: application/json" \
  -d '{"response": "LLM output..."}'
```

### 查询图谱

```bash
# 查询实体
curl -X POST http://localhost:8080/v1/graph/query \
  -H "Content-Type: application/json" \
  -d '{"query": "alice"}'

# 路径查询（DFS）
curl -X POST http://localhost:8080/v1/graph/path \
  -H "Content-Type: application/json" \
  -d '{"from": "Alice", "to": "Google", "max_depth": 3}'

# 所有三元组
curl http://localhost:8080/v1/graph/triples

# 统计信息
curl http://localhost:8080/v1/graph/stats
```

---

## 记忆管理

### 标签系统

为记忆片段添加标签进行分类管理：

```bash
# 添加标签
curl -X POST http://localhost:8080/v1/memory/manage/tags \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "segment_id": "uuid", "tags": ["work", "important"]}'

# 按标签搜索
curl -X POST http://localhost:8080/v1/memory/manage/search/tags \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "tags": ["work"], "limit": 50}'
```

### 导出和导入

```bash
# 导出为 JSON
curl -X POST http://localhost:8080/v1/memory/manage/export \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "format": "json"}'

# 导出为 Markdown
curl -X POST http://localhost:8080/v1/memory/manage/export \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "format": "markdown"}'

# 导入
curl -X POST http://localhost:8080/v1/memory/manage/import \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "alice",
    "segments": [
      {"summary": "Alice likes hiking", "tags": ["hobby"]}
    ]
  }'
```

### 版本历史

记忆片段支持版本控制，每次修改（如添加标签）自动创建新版本：

```bash
curl -X POST http://localhost:8080/v1/memory/manage/versions \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "segment_id": "uuid"}'
```

---

## 多模态记忆

支持存储和检索文本、图像、音频等多模态内容。使用 Qdrant 向量存储。

```bash
# 存储多模态消息
curl -X POST http://localhost:8080/v1/multimodal/store \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "alice",
    "role": "user",
    "contents": [
      {"type": "text", "text": "Check this photo"},
      {"type": "image", "embedding": [0.1, 0.2, 0.3]}
    ]
  }'

# 按文本搜索
curl -X POST http://localhost:8080/v1/multimodal/search \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "query": "photo", "limit": 10}'

# 按 embedding 搜索（图像或音频）
curl -X POST http://localhost:8080/v1/multimodal/search/embedding \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "embedding": [0.1, 0.2], "limit": 10, "modality": "image"}'
```

> 当前多模态使用预计算的 embedding 向量输入，CLIP/Whisper 模型集成待后续版本。

---

## FAQ 系统

高频 Q&A 自动识别、分类和管理。

### 工作流程

1. 对话消息存入 MTM 后，系统追踪访问热度
2. 热度达标的片段自动成为 FAQ 候选
3. 管理员审核后手动提升为正式 FAQ
4. FAQ 在后续检索中优先匹配

### 管理操作

```bash
# 查看 FAQ 候选
curl http://localhost:8080/v1/admin/faq/candidates

# 提升为正式 FAQ
curl -X POST http://localhost:8080/v1/admin/faq/promote \
  -H "Content-Type: application/json" \
  -d '{"memory_id": "uuid", "reason": "High frequency question"}'

# LLM FAQ 分类（生成 prompt）
curl -X POST http://localhost:8080/v1/admin/faq/classify \
  -H "Content-Type: application/json" \
  -d '{"question": "How to reset password?", "answer": "Go to settings..."}'

# 降级 FAQ
curl -X DELETE http://localhost:8080/v1/admin/faq/{id}

# 查看提升历史
curl http://localhost:8080/v1/admin/faq/history

# 统计信息
curl http://localhost:8080/v1/admin/faq/stats
```

---

## Wiki 生成

基于 Tree-sitter + LLM 混合管线，从代码仓库自动生成结构化 Markdown Wiki。

### 支持语言

V1 版本支持：Rust、Python、Java、TypeScript/Vue

### 使用方式

**CLI 工具**：

```bash
cargo run --bin wiki-gen -- --repo /path/to/project --output ./wiki-output
```

**API 调用**：

```bash
# 触发 Wiki 生成
curl -X POST http://localhost:8080/v1/wiki/generate \
  -H "Content-Type: application/json" \
  -d '{"repo_path": "/path/to/project"}'

# 查看任务状态
curl http://localhost:8080/v1/wiki/status

# 查看特定任务
curl http://localhost:8080/v1/wiki/jobs/{job_id}
```

### 生成内容

Wiki 生成管线包含 7 个阶段：

1. **Repo Intake** — 文件发现（尊重 .gitignore）、语言检测
2. **Multi-Language Parsing** — Tree-sitter AST 解析
3. **API Endpoint Extraction** — 路由/端点提取
4. **Code Graph** — 文件依赖 + Symbol 关系 + Runtime 图
5. **LLM Documentation** — 基于 Evidence Pack 生成文档
6. **Mermaid Diagrams** — 自动生成架构图
7. **Page Assembly** — Tera 模板组装 Markdown 页面

---

## 存储连接器

Wiki 生成支持 17 种代码仓库连接方式：

| 类型 | 说明 |
|------|------|
| **local** | 本地文件系统 |
| **git** | Git 仓库（支持 token 认证） |
| **s3** | Amazon S3 |
| **gcs** | Google Cloud Storage |
| **azure_blob** | Azure Blob Storage |
| **oss** | 阿里云 OSS |
| **cos** | 腾讯云 COS |
| **obs** | 华为云 OBS |
| **webdav** | WebDAV 服务器 |
| **sftp** | SFTP 服务器 |
| **smb** | SMB/CIFS 网络共享 |
| **nfs** | NFS 网络文件系统 |
| **onedrive** | Microsoft OneDrive |
| **google_drive** | Google Drive |
| **dropbox** | Dropbox |
| **baidu_pan** | 百度网盘 |
| **aliyun_drive** | 阿里云盘 |

### 使用流程

```bash
# 1. 查看支持的连接器类型
curl http://localhost:8080/v1/wiki/connectors

# 2. 测试连接
curl -X POST http://localhost:8080/v1/wiki/connectors/test \
  -H "Content-Type: application/json" \
  -d '{"type": "git", "config": {"url": "https://github.com/example/repo.git"}}'

# 3. 浏览目录
curl -X POST http://localhost:8080/v1/wiki/connectors/browse \
  -H "Content-Type: application/json" \
  -d '{"connector_id": "session-id", "path": "/"}'

# 4. 触发 Wiki 生成
curl -X POST http://localhost:8080/v1/wiki/connectors/generate \
  -H "Content-Type: application/json" \
  -d '{"connector_id": "session-id"}'

# 5. 保存连接器配置（敏感字段 AES-256-GCM 加密）
curl -X POST http://localhost:8080/v1/wiki/connectors \
  -H "Content-Type: application/json" \
  -d '{"name": "My Repo", "connector_type": "git", "config": {"url": "..."}}'
```

---

## 安全与合规

### IP 防御

内置 IP 防御系统，检测 5 种攻击类型：

- 暴力破解
- DDoS
- 扫描探测
- SQL 注入
- XSS

触发防御后自动临时封禁（Redis TTL）或永久封禁（Qdrant 持久化）。

### 审计日志

所有敏感操作自动记录审计日志，持久化到 `~/.memoryos/audit.jsonl`：

```bash
# 查询审计日志
curl -X POST http://localhost:8080/v1/security/audit/logs \
  -H "Content-Type: application/json" \
  -d '{"limit": 50}'

# 审计统计
curl http://localhost:8080/v1/security/audit/stats
```

### GDPR 合规

支持数据可携带权和被遗忘权：

```bash
# 导出用户数据
curl -X POST http://localhost:8080/v1/security/gdpr/export \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice"}'

# 删除用户数据（从所有后端）
curl -X POST http://localhost:8080/v1/security/gdpr/delete \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice"}'

# 记录同意
curl -X POST http://localhost:8080/v1/security/gdpr/consent \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "purpose": "data_processing", "granted": true}'

# 检查同意
curl -X POST http://localhost:8080/v1/security/gdpr/consent/check \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "purpose": "data_processing"}'
```

---

## 认证配置

### 启用认证

```toml
[auth]
enabled = true
admin_keys = ["admin-secret-key"]
api_keys = ["user-key-1", "user-key-2"]
```

### API Key 管理

```bash
# 创建 API Key（需要 admin 权限）
curl -X POST http://localhost:8080/v1/admin/keys \
  -H "Authorization: Bearer admin-secret-key" \
  -H "Content-Type: application/json" \
  -d '{
    "api_key": "new-user-key",
    "user_id": "bob@company.com",
    "description": "Bob dev key",
    "permissions": ["chat", "memory"]
  }'

# 删除 API Key
curl -X DELETE http://localhost:8080/v1/admin/keys/new-user-key \
  -H "Authorization: Bearer admin-secret-key"
```

### 请求认证

所有 `/v1/*` 请求需要携带 API Key：

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer user-key-1" \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4o-mini", "messages": [...]}'
```

---

## 多租户

### 租户隔离

通过 `X-Tenant-ID` 请求头实现数据隔离：

```bash
curl -X POST http://localhost:8080/v1/memory/add \
  -H "X-Tenant-ID: tenant-001" \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice", "role": "user", "content": "Hello"}'
```

不同租户的数据自动隔离，查询时自动按 `tenant_id` 过滤。

### RBAC 权限

| 权限 | SuperAdmin | Admin | User | ReadOnly |
|------|:---------:|:-----:|:----:|:--------:|
| 读取记忆 | Y | Y | Y | Y |
| 写入记忆 | Y | Y | Y | - |
| 管理用户 | Y | Y | - | - |
| 管理租户 | Y | - | - | - |
| 查看审计 | Y | Y | - | - |
| 管理配置 | Y | - | - | - |

---

## 运维管理

### 健康检查

```bash
# 存活检查
curl http://localhost:8080/health

# 详细状态（Redis + Qdrant 实时探测）
curl http://localhost:8080/health/status
```

### Prometheus 指标

```bash
curl http://localhost:8080/metrics
```

### 配置热重载

默认每 5 秒检查 `config.toml` 变更并自动重载，无需重启服务。

禁用热重载：
```bash
MEMORYOS_CONFIG_HOT_RELOAD=false ./target/release/memoryos-gateway
```

### Admin 服务

Admin 服务 (`:9090`) 提供内网管理功能：

```bash
cargo run --bin memoryos-admin
```

---

## 故障排查

### 常见问题

| 问题 | 原因 | 解决方法 |
|------|------|---------|
| `health/status` 返回 `degraded` | Redis 或 Qdrant 不可用 | 检查依赖服务状态 |
| 聊天无记忆上下文 | 向量存储不可用或用户无历史数据 | 先通过 `/v1/memory/add` 添加数据 |
| 401 Unauthorized | API Key 无效或未配置 | 检查 `auth` 配置和请求头 |
| LLM 返回错误 | API Key 无效或余额不足 | 检查 `[llm.providers]` 配置 |
| Wiki 生成失败 | 代码仓库路径无效 | 确认 `repo_path` 存在且可读 |

### 日志调试

```bash
RUST_LOG=debug cargo run --bin memoryos-gateway
```

按模块过滤：
```bash
RUST_LOG=memoryos_gateway=debug,memoryos_core=info cargo run --bin memoryos-gateway
```
