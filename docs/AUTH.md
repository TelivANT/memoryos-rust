# API Key 认证系统

## 两种模式

### 1. 静态模式（小规模，<100 用户）

```toml
[auth]
enabled = true
use_redis_store = false
admin_key = "admin-secret-key"
api_keys = [
    "user-key-1",
    "user-key-2"
]
```

### 2. Qdrant 动态模式（大规模，10万+ 用户）⭐ 推荐

```toml
[auth]
enabled = true
use_redis_store = true  # 实际使用 Qdrant 存储（名字保留兼容性）
admin_key = "admin-secret-key-12345"
```

**为什么用 Qdrant？**
- ✅ **持久化** - 数据存储在磁盘，重启不丢失
- ✅ **高性能** - 向量数据库，查询速度快
- ✅ **可扩展** - 支持 10万+ API Key
- ✅ **零成本** - 复用现有 Qdrant，无需额外部署
- ✅ **生产级** - Qdrant 本身就是生产级数据库

## 管理 API Key

### 创建 API Key

```bash
curl -X POST http://localhost:8080/admin/keys \
  -H "Authorization: Bearer admin-secret-key-12345" \
  -H "Content-Type: application/json" \
  -d '{
    "api_key": "user-alice-key-abc123",
    "user_id": "alice@company.com",
    "description": "Alice的开发环境 Key",
    "permissions": ["chat", "memory"]
  }'
```

### 删除 API Key

```bash
curl -X DELETE http://localhost:8080/admin/keys \
  -H "Authorization: Bearer admin-secret-key-12345" \
  -H "Content-Type: application/json" \
  -d '{
    "api_key": "user-alice-key-abc123"
  }'
```

### 使用 API Key

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer user-alice-key-abc123" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-3-pro-preview",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

## 优势

1. **可扩展** - Qdrant 存储支持 10万+ 用户
2. **持久化** - 数据存储在磁盘，重启不丢失（Redis 会丢失！）
3. **动态管理** - 无需重启服务即可添加/删除 Key
4. **用户关联** - 每个 Key 关联用户信息和权限
5. **审计追踪** - 记录创建时间、描述等元数据
6. **权限控制** - 可扩展支持细粒度权限
7. **零成本** - 复用现有 Qdrant，无需额外部署数据库

## 技术实现

### Qdrant 存储结构

```
Collection: api_keys
├── Point ID: hash(api_key)
└── Payload:
    ├── api_key: "user-alice-key-abc123"
    ├── user_id: "alice@company.com"
    ├── description: "Alice的开发环境 Key"
    ├── created_at: "2026-02-18T08:00:00Z"
    ├── is_active: true
    ├── permissions: ["chat", "memory"]
    └── expires_at: null (可选)
```

### 为什么不用 Redis？

| 特性 | Redis | Qdrant |
|------|-------|--------|
| 持久化 | ❌ 默认内存，重启丢失 | ✅ 磁盘存储 |
| 性能 | ✅ 极快 | ✅ 快 |
| 扩展性 | ✅ 好 | ✅ 好 |
| 额外部署 | ⚠️ 需要 | ✅ 复用现有 |
| 生产级 | ✅ | ✅ |

**结论**: Qdrant 更适合存储 API Key 这种关键数据！

## 批量导入

可以编写脚本批量创建 Key：

```bash
#!/bin/bash
ADMIN_KEY="admin-secret-key-12345"
API_URL="http://localhost:8080"

# 从 CSV 读取用户列表
while IFS=, read -r user_id email; do
  api_key="key-$(uuidgen)"
  curl -X POST $API_URL/admin/keys \
    -H "Authorization: Bearer $ADMIN_KEY" \
    -H "Content-Type: application/json" \
    -d "{
      \"api_key\": \"$api_key\",
      \"user_id\": \"$email\",
      \"description\": \"Auto-generated for $user_id\",
      \"permissions\": [\"chat\", \"memory\"]
    }"
  echo "$email,$api_key" >> keys.csv
done < users.csv
```

## 安全建议

1. **管理员 Key** - 妥善保管，定期轮换
2. **HTTPS** - 生产环境必须使用 HTTPS
3. **Key 格式** - 使用随机生成的长字符串（如 UUID）
4. **过期时间** - 可设置 Key 过期时间（TODO）
5. **速率限制** - 配合速率限制使用（TODO）
