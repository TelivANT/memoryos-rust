# Integration Testing

集成测试已创建！

## 📁 文件

- `crates/memoryos-adapters/tests/vector_storage_integration.rs` - 集成测试代码
- `scripts/run_integration_tests.sh` - 自动化测试脚本
- `docs/INTEGRATION_TESTING.md` - 详细文档

## 🚀 快速运行

```bash
# 方法 1: 使用自动化脚本
./scripts/run_integration_tests.sh

# 方法 2: 手动运行单个测试
cargo test --package memoryos-adapters \
  --test vector_storage_integration \
  test_qdrant_short_term_memory \
  -- --ignored --nocapture
```

## 📋 前置条件

启动向量数据库：

```bash
# Qdrant
docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant

# Chroma
docker run -d -p 8000:8000 chromadb/chroma

# Pinecone
export PINECONE_API_KEY=your_key
export PINECONE_ENVIRONMENT=us-east-1-aws
```

## 🧪 测试内容

每个向量数据库测试：
1. ✅ 添加短期记忆 (add_short_term_message)
2. ✅ 获取短期记忆 (get_short_term_messages)
3. ✅ 限制数量测试
4. ✅ 用户隔离测试
5. ✅ 清空短期记忆 (clear_short_term)
6. ✅ 并发操作测试

## 📚 详细文档

查看 `docs/INTEGRATION_TESTING.md` 获取完整指南。
