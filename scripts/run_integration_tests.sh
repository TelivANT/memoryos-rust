#!/bin/bash
# 向量存储集成测试运行脚本

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🧪 MemoryOS-Rust Vector Storage Integration Tests"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 检查 Docker
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker first."
    exit 1
fi

echo "📋 Prerequisites:"
echo "  1. Qdrant: docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant"
echo "  2. Chroma: docker run -d -p 8000:8000 chromadb/chroma"
echo "  3. Pinecone: export PINECONE_API_KEY=your_key"
echo ""

# 检查 Qdrant
echo "🔍 Checking Qdrant (localhost:6333)..."
if curl -s http://localhost:6333/health > /dev/null 2>&1; then
    echo "  ✅ Qdrant is running"
    RUN_QDRANT=true
else
    echo "  ⚠️  Qdrant not running (skipping Qdrant tests)"
    RUN_QDRANT=false
fi

# 检查 Chroma
echo "🔍 Checking Chroma (localhost:8000)..."
if curl -s http://localhost:8000/api/v1/heartbeat > /dev/null 2>&1; then
    echo "  ✅ Chroma is running"
    RUN_CHROMA=true
else
    echo "  ⚠️  Chroma not running (skipping Chroma tests)"
    RUN_CHROMA=false
fi

# 检查 Pinecone
echo "🔍 Checking Pinecone API key..."
if [ -n "$PINECONE_API_KEY" ]; then
    echo "  ✅ PINECONE_API_KEY is set"
    RUN_PINECONE=true
else
    echo "  ⚠️  PINECONE_API_KEY not set (skipping Pinecone tests)"
    RUN_PINECONE=false
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Running Tests"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 运行 Qdrant 测试
if [ "$RUN_QDRANT" = true ]; then
    echo "🧪 Testing Qdrant..."
    cargo test --package memoryos-adapters --test vector_storage_integration test_qdrant_short_term_memory -- --ignored --nocapture
    echo ""
    
    echo "🧪 Testing Concurrent Operations (Qdrant)..."
    cargo test --package memoryos-adapters --test vector_storage_integration test_concurrent_operations -- --ignored --nocapture
    echo ""
fi

# 运行 Chroma 测试
if [ "$RUN_CHROMA" = true ]; then
    echo "🧪 Testing Chroma..."
    cargo test --package memoryos-adapters --test vector_storage_integration test_chroma_short_term_memory -- --ignored --nocapture
    echo ""
fi

# 运行 Pinecone 测试
if [ "$RUN_PINECONE" = true ]; then
    echo "🧪 Testing Pinecone..."
    cargo test --package memoryos-adapters --test vector_storage_integration test_pinecone_short_term_memory -- --ignored --nocapture
    echo ""
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Integration Tests Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
