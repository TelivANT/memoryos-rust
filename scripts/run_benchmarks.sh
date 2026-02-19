#!/bin/bash
# 性能基准测试运行脚本

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 MemoryOS-Rust Performance Benchmarks"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 检查 Qdrant
echo "🔍 Checking Qdrant (localhost:6333)..."
if curl -s http://localhost:6333/health > /dev/null 2>&1; then
    echo "  ✅ Qdrant is running"
else
    echo "  ❌ Qdrant not running!"
    echo "  Please start: docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant"
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Running Benchmarks"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 运行 benchmark
cargo bench --package memoryos-benchmarks

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Benchmarks Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📊 Results saved to: target/criterion/"
echo "📈 View HTML report: target/criterion/report/index.html"
echo ""
