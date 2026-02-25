# 向量存储集成测试指南

**版本**: v1.0.0-rc  
**更新**: 2026-02-19

---

## 📋 概述

本文档描述如何运行 MemoryOS-Rust 的向量存储集成测试，验证短期记忆功能在真实向量数据库上的表现。

## 🎯 测试目标

验证所有向量数据库的短期记忆 CRUD 操作：

1. **Qdrant** - 开源向量数据库
2. **Chroma** - 轻量级向量数据库
3. **Pinecone** - 云端向量数据库

## 🧪 测试内容

每个向量数据库测试以下功能：

### 1. 添加短期记忆 (add_short_term_message)
- 添加多条消息
- 验证存储成功

### 2. 获取短期记忆 (get_short_term_messages)
- 检索所有消息
- 验证内容正确
- 验证顺序正确

### 3. 限制数量
- 测试 `limit` 参数
- 验证返回数量正确

### 4. 用户隔离
- 不同 user_id 的消息互不干扰
- 验证数据隔离

### 5. 清空短期记忆 (clear_short_term)
- 删除用户所有短期记忆
- 验证删除成功

### 6. 并发操作
- 多个任务同时写入
- 验证数据一致性

---

## 🚀 快速开始

### 方法 1: 使用自动化脚本（推荐）

```bash
cd MemoryOS-Rust
./scripts/run_integration_tests.sh
```

脚本会自动：
- 检查 Docker 是否安装
- 检查各向量数据库是否运行
- 运行可用的测试
- 跳过不可用的测试

### 方法 2: 手动运行

#### 1. 启动向量数据库

**Qdrant**:
```bash
docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant
```

**Chroma**:
```bash
docker run -d -p 8000:8000 chromadb/chroma
```

**Pinecone**:
```bash
export PINECONE_API_KEY=your_api_key_here
```

#### 2. 运行测试

**测试 Qdrant**:
```bash
cargo test --package memoryos-adapters \
  --test vector_storage_integration \
  test_qdrant_short_term_memory \
  -- --ignored --nocapture
```

**测试 Chroma**:
```bash
cargo test --package memoryos-adapters \
  --test vector_storage_integration \
  test_chroma_short_term_memory \
  -- --ignored --nocapture
```

**测试 Pinecone**:
```bash
cargo test --package memoryos-adapters \
  --test vector_storage_integration \
  test_pinecone_short_term_memory \
  -- --ignored --nocapture
```

**测试并发操作**:
```bash
cargo test --package memoryos-adapters \
  --test vector_storage_integration \
  test_concurrent_operations \
  -- --ignored --nocapture
```

---

## 📊 预期输出

成功的测试输出示例：

```
🧪 Testing Qdrant - Short-term Memory
  ➤ Adding messages...
  ✅ Added 3 messages
  ➤ Retrieving messages...
  ✅ Retrieved 3 messages correctly
  ➤ Testing limit...
  ✅ Limit works correctly
  ➤ Testing user isolation...
  ✅ User isolation works
  ➤ Clearing messages...
  ✅ Clear works correctly
✅ Qdrant - All tests passed!

test test_qdrant_short_term_memory ... ok
```

---

## 🔧 故障排查

### Qdrant 连接失败

**错误**: `Failed to connect to Qdrant`

**解决**:
```bash
# 检查 Qdrant 是否运行
curl http://localhost:6333/health

# 重启 Qdrant
docker restart <qdrant_container_id>
```

### Chroma 连接失败

**错误**: `Failed to connect to Chroma`

**解决**:
```bash
# 检查 Chroma 是否运行
curl http://localhost:8000/api/v1/heartbeat

# 重启 Chroma
docker restart <chroma_container_id>
```

### Pinecone API Key 错误

**错误**: `PINECONE_API_KEY not set`

**解决**:
```bash
export PINECONE_API_KEY=your_api_key_here
```

### 测试超时

**原因**: 向量数据库响应慢

**解决**:
- 检查网络连接
- 检查 Docker 资源限制
- 增加测试超时时间

---

## 📈 性能基准

预期性能指标（本地 Docker）：

| 操作 | Qdrant | Chroma | Pinecone |
|------|--------|--------|----------|
| add_short_term_message | < 50ms | < 100ms | < 200ms |
| get_short_term_messages | < 30ms | < 50ms | < 150ms |
| clear_short_term | < 100ms | < 150ms | < 300ms |

*注: Pinecone 是云端服务，延迟包含网络传输*

---

## 🎯 下一步

集成测试通过后：

1. ✅ **Performance Benchmarking** - 量化性能提升
2. ✅ **Production Deployment Guide** - 编写部署文档
3. ✅ **Monitoring Setup** - 配置监控系统

---

## 📚 相关文档

- [Architecture Improvement](./ARCHITECTURE_IMPROVEMENT.md) - 架构改进说明
- [Vector Databases Guide](./VECTOR_DATABASES.md) - 向量数据库配置
- [Work Log](./WORK_LOG.md) - 开发日志

---

## 🤝 贡献

发现问题或有改进建议？欢迎提交 Issue 或 PR！
