# Storage Connector Interactive API Design

**Version**: 1.0  
**Date**: 2026-02-21  
**Status**: Planning

---

## 🎯 Overview

为 wiki-gen 提供完整的交互式存储连接器配置流程，支持用户通过 API 选择连接器、配置认证、浏览目录、选择路径并生成 Wiki。

---

## 📋 User Flow

```
1. 用户选择连接器类型 (Git, S3, OSS, etc.)
   ↓
2. 输入认证信息 (Token, SSH Key, Access Key, etc.)
   ↓
3. 测试连接 (验证认证是否有效)
   ↓
4. 浏览目录结构 (返回目录树)
   ↓
5. 选择目标目录 (用户手动选择)
   ↓
6. 开始生成 Wiki
```

---

## 🔧 API Design

### 1. List Available Connectors

**Endpoint**: `GET /v1/wiki/connectors`

**Response**:
```json
{
  "connectors": [
    {
      "type": "local",
      "name": "Local Filesystem",
      "description": "Read from local filesystem",
      "auth_required": false,
      "fields": [
        {
          "name": "path",
          "type": "string",
          "required": true,
          "description": "Local directory path"
        }
      ]
    },
    {
      "type": "git",
      "name": "Git Repository",
      "description": "Clone from Git repository",
      "auth_required": true,
      "fields": [
        {
          "name": "url",
          "type": "string",
          "required": true,
          "description": "Git repository URL"
        },
        {
          "name": "branch",
          "type": "string",
          "required": false,
          "default": "main",
          "description": "Branch name"
        },
        {
          "name": "auth_type",
          "type": "enum",
          "required": true,
          "options": ["token", "ssh_key", "none"],
          "description": "Authentication type"
        },
        {
          "name": "token",
          "type": "string",
          "required": false,
          "description": "Personal access token (if auth_type=token)"
        },
        {
          "name": "ssh_key_path",
          "type": "string",
          "required": false,
          "description": "SSH private key path (if auth_type=ssh_key)"
        }
      ]
    },
    {
      "type": "s3",
      "name": "AWS S3",
      "description": "Read from S3-compatible storage",
      "auth_required": true,
      "fields": [
        {
          "name": "region",
          "type": "string",
          "required": true,
          "description": "AWS region"
        },
        {
          "name": "bucket",
          "type": "string",
          "required": true,
          "description": "S3 bucket name"
        },
        {
          "name": "access_key_id",
          "type": "string",
          "required": true,
          "description": "AWS access key ID"
        },
        {
          "name": "secret_access_key",
          "type": "string",
          "required": true,
          "sensitive": true,
          "description": "AWS secret access key"
        },
        {
          "name": "endpoint",
          "type": "string",
          "required": false,
          "description": "Custom endpoint (for MinIO, etc.)"
        }
      ]
    }
  ]
}
```

---

### 2. Test Connection

**Endpoint**: `POST /v1/wiki/connectors/test`

**Request**:
```json
{
  "type": "git",
  "config": {
    "url": "https://github.com/user/repo.git",
    "branch": "main",
    "auth_type": "token",
    "token": "ghp_xxxxx"
  }
}
```

**Response** (Success):
```json
{
  "success": true,
  "message": "Connection successful",
  "connector_id": "conn_abc123",
  "metadata": {
    "repository": "user/repo",
    "default_branch": "main",
    "last_commit": "abc123"
  }
}
```

**Response** (Failure):
```json
{
  "success": false,
  "error": "Authentication failed: invalid token",
  "error_code": "AUTH_FAILED"
}
```

---

### 3. Browse Directory

**Endpoint**: `POST /v1/wiki/connectors/browse`

**Request**:
```json
{
  "connector_id": "conn_abc123",
  "path": "/src"
}
```

**Response**:
```json
{
  "path": "/src",
  "entries": [
    {
      "name": "main.rs",
      "path": "/src/main.rs",
      "type": "file",
      "size": 1024,
      "modified": "2026-02-21T10:00:00Z"
    },
    {
      "name": "lib",
      "path": "/src/lib",
      "type": "directory",
      "children_count": 5
    },
    {
      "name": "tests",
      "path": "/src/tests",
      "type": "directory",
      "children_count": 3
    }
  ],
  "total": 3
}
```

---

### 4. Save Connector Configuration

**Endpoint**: `POST /v1/wiki/connectors`

**Request**:
```json
{
  "name": "My GitHub Repo",
  "type": "git",
  "config": {
    "url": "https://github.com/user/repo.git",
    "branch": "main",
    "auth_type": "token",
    "token": "ghp_xxxxx"
  }
}
```

**Response**:
```json
{
  "id": "conn_abc123",
  "name": "My GitHub Repo",
  "type": "git",
  "created_at": "2026-02-21T10:00:00Z",
  "status": "active"
}
```

---

### 5. List Saved Connectors

**Endpoint**: `GET /v1/wiki/connectors/saved`

**Response**:
```json
{
  "connectors": [
    {
      "id": "conn_abc123",
      "name": "My GitHub Repo",
      "type": "git",
      "created_at": "2026-02-21T10:00:00Z",
      "last_used": "2026-02-21T11:00:00Z",
      "status": "active"
    },
    {
      "id": "conn_def456",
      "name": "Production S3",
      "type": "s3",
      "created_at": "2026-02-20T15:00:00Z",
      "last_used": null,
      "status": "active"
    }
  ]
}
```

---

### 6. Generate Wiki with Connector

**Endpoint**: `POST /v1/wiki/generate`

**Request**:
```json
{
  "connector_id": "conn_abc123",
  "path": "/src",
  "options": {
    "include_diagrams": true,
    "include_api_docs": true,
    "output_format": "markdown"
  }
}
```

**Response**:
```json
{
  "job_id": "job_xyz789",
  "status": "processing",
  "estimated_time": 120,
  "message": "Wiki generation started"
}
```

---

### 7. Get Generation Status

**Endpoint**: `GET /v1/wiki/jobs/{job_id}`

**Response** (Processing):
```json
{
  "job_id": "job_xyz789",
  "status": "processing",
  "progress": 45,
  "current_step": "Parsing files",
  "files_processed": 23,
  "total_files": 51
}
```

**Response** (Complete):
```json
{
  "job_id": "job_xyz789",
  "status": "completed",
  "progress": 100,
  "result": {
    "pages_generated": 15,
    "diagrams_generated": 3,
    "output_path": "/wiki/output/job_xyz789"
  },
  "completed_at": "2026-02-21T11:05:00Z"
}
```

---

## 🔐 Security Considerations

### 1. Credential Storage
- **加密存储**: 所有敏感信息（Token, SSH Key, Access Key）使用 AES-256-GCM 加密
- **Key Management**: 使用环境变量或 KMS 管理加密密钥
- **Access Control**: 每个用户只能访问自己的连接器配置

### 2. Credential Transmission
- **HTTPS Only**: 所有 API 必须通过 HTTPS
- **Token Masking**: 返回配置时隐藏敏感字段（显示 `***`）
- **Short-lived Sessions**: 连接器会话 1 小时后过期

### 3. SSH Key Handling
- **Upload**: 支持上传私钥文件
- **Storage**: 加密存储在服务器
- **Permissions**: 确保文件权限 600
- **Cleanup**: 使用后立即清理临时文件

---

## 📊 Database Schema

### connectors 表
```sql
CREATE TABLE connectors (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL,
    config_encrypted TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    last_used_at TIMESTAMP,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    INDEX idx_user_id (user_id),
    INDEX idx_type (type)
);
```

### connector_sessions 表
```sql
CREATE TABLE connector_sessions (
    id VARCHAR(36) PRIMARY KEY,
    connector_id VARCHAR(36) NOT NULL,
    user_id VARCHAR(36) NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL,
    INDEX idx_connector_id (connector_id),
    INDEX idx_expires_at (expires_at),
    FOREIGN KEY (connector_id) REFERENCES connectors(id)
);
```

### wiki_jobs 表
```sql
CREATE TABLE wiki_jobs (
    id VARCHAR(36) PRIMARY KEY,
    connector_id VARCHAR(36) NOT NULL,
    user_id VARCHAR(36) NOT NULL,
    path VARCHAR(1024) NOT NULL,
    status VARCHAR(20) NOT NULL,
    progress INT NOT NULL DEFAULT 0,
    result JSON,
    created_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    INDEX idx_user_id (user_id),
    INDEX idx_status (status),
    FOREIGN KEY (connector_id) REFERENCES connectors(id)
);
```

---

## 🚀 Implementation Plan

### Phase 1: Core API (Week 1)
1. ✅ Storage connectors implementation (P0+P1 complete)
2. 🔄 Connector metadata API (`GET /v1/wiki/connectors`)
3. 🔄 Connection test API (`POST /v1/wiki/connectors/test`)
4. 🔄 Directory browse API (`POST /v1/wiki/connectors/browse`)

### Phase 2: Configuration Management (Week 2)
5. 🔄 Save connector API (`POST /v1/wiki/connectors`)
6. 🔄 List connectors API (`GET /v1/wiki/connectors/saved`)
7. 🔄 Credential encryption service
8. 🔄 Database schema migration

### Phase 3: Wiki Generation (Week 2)
9. 🔄 Generate wiki API (`POST /v1/wiki/generate`)
10. 🔄 Job status API (`GET /v1/wiki/jobs/{job_id}`)
11. 🔄 Async job processing
12. 🔄 Progress tracking

### Phase 4: Frontend UI (Week 3-4)
13. 🔄 Connector selection page
14. 🔄 Authentication form
15. 🔄 Directory tree browser
16. 🔄 Wiki generation dashboard
17. 🔄 Job status monitoring

---

## 🎨 UI Mockup (Text)

```
┌─────────────────────────────────────────────────────────────┐
│  Wiki Generator - Connect to Source                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Select Connector Type:                                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐      │
│  │   Git    │ │   S3     │ │   OSS    │ │  WebDAV  │      │
│  │  GitHub  │ │   AWS    │ │  阿里云   │ │  坚果云   │      │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘      │
│                                                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐      │
│  │   COS    │ │   OBS    │ │   SFTP   │ │  Local   │      │
│  │  腾讯云   │ │  华为云   │ │   SSH    │ │  本地     │      │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘      │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│  Git Repository Configuration:                              │
│                                                              │
│  Repository URL: [https://github.com/user/repo.git      ]  │
│  Branch:         [main                                   ]  │
│                                                              │
│  Authentication:                                             │
│  ○ None  ● Token  ○ SSH Key                                │
│                                                              │
│  Personal Access Token: [ghp_****************************]  │
│                                                              │
│  [Test Connection]                                          │
│                                                              │
│  ✓ Connection successful!                                   │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│  Select Directory:                                          │
│                                                              │
│  📁 /                                                        │
│  ├─ 📁 src                                                  │
│  │  ├─ 📄 main.rs                                          │
│  │  ├─ 📁 lib                                              │
│  │  └─ 📁 tests                                            │
│  ├─ 📁 docs                                                 │
│  └─ 📄 README.md                                            │
│                                                              │
│  Selected: /src                                             │
│                                                              │
│  [Generate Wiki]                                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 📝 Notes

- **Credential Security**: 最高优先级，必须加密存储
- **User Experience**: 流程要简单直观
- **Error Handling**: 提供清晰的错误信息
- **Performance**: 目录浏览要快速响应
- **Scalability**: 支持大型仓库（10000+ 文件）

---

## 🔗 Related

- Storage Connectors: `docs/STORAGE_CONNECTORS.md`
- Wiki Gen Spec: `docs/specs/wiki_gen_spec.md`
- Gateway API: `crates/memoryos-gateway/src/routes/wiki.rs`
