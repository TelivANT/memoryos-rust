#!/bin/bash
# MemoryOS-Rust 快速部署脚本

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 MemoryOS-Rust Quick Deploy"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 检查 Docker
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose not found. Please install Docker Compose first."
    exit 1
fi

echo "✅ Docker and Docker Compose found"
echo ""

# 检查 .env 文件
if [ ! -f .env ]; then
    echo "⚠️  .env file not found. Creating from .env.example..."
    cp .env.example .env
    echo "📝 Please edit .env file and add your API keys:"
    echo "   - OPENAI_API_KEY"
    echo ""
    read -p "Press Enter to continue after editing .env..."
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📦 Starting Services"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 启动服务
docker-compose up -d

echo ""
echo "⏳ Waiting for services to be ready..."
sleep 10

# 检查服务状态
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 Checking Service Status"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 检查 Qdrant
if curl -s http://localhost:6333/health > /dev/null 2>&1; then
    echo "✅ Qdrant is running (http://localhost:6333)"
else
    echo "❌ Qdrant is not responding"
fi

# 检查 Redis
if docker exec memoryos-redis redis-cli ping > /dev/null 2>&1; then
    echo "✅ Redis is running (localhost:6379)"
else
    echo "⚠️  Redis is not responding"
fi

# 检查 NATS
if curl -s http://localhost:8222/healthz > /dev/null 2>&1; then
    echo "✅ NATS is running (localhost:4222)"
else
    echo "⚠️  NATS is not responding"
fi

# 检查 Gateway
if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Gateway is running (http://localhost:8080)"
else
    echo "⚠️  Gateway is not responding yet (may still be starting)"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Deployment Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📊 Service URLs:"
echo "  • Gateway:  http://localhost:8080"
echo "  • Qdrant:   http://localhost:6333"
echo "  • Redis:    localhost:6379"
echo "  • NATS:     localhost:4222"
echo ""
echo "📝 Useful Commands:"
echo "  • View logs:    docker-compose logs -f"
echo "  • Stop:         docker-compose down"
echo "  • Restart:      docker-compose restart"
echo "  • Status:       docker-compose ps"
echo ""
echo "📚 Documentation: docs/PRODUCTION_DEPLOYMENT.md"
echo ""
