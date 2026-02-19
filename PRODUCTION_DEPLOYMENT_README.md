# Production Deployment

生产部署指南已创建！

## 📁 文件

- `docs/PRODUCTION_DEPLOYMENT.md` - 完整部署指南
- `docker-compose.yml` - Docker Compose 配置
- `.env.example` - 环境变量示例
- `scripts/deploy.sh` - 快速部署脚本

## 🚀 快速部署

### 方法 1: 使用部署脚本（推荐）

```bash
# 运行部署脚本
./scripts/deploy.sh
```

脚本会自动：
- 检查 Docker 和 Docker Compose
- 创建 .env 文件（如果不存在）
- 启动所有服务
- 检查服务状态
- 显示服务 URL

### 方法 2: 手动部署

```bash
# 1. 配置环境变量
cp .env.example .env
# 编辑 .env 文件，添加 API keys

# 2. 启动服务
docker-compose up -d

# 3. 检查状态
docker-compose ps

# 4. 查看日志
docker-compose logs -f gateway
```

## 📊 服务架构

```
┌─────────────────────────────────────────┐
│         MemoryOS Gateway                │
│         (Port 8080)                     │
└─────────────┬───────────────────────────┘
              │
    ┌─────────┼─────────┐
    ▼         ▼         ▼
┌────────┐ ┌────────┐ ┌────────┐
│ Qdrant │ │ Redis  │ │  NATS  │
│  6333  │ │  6379  │ │  4222  │
└────────┘ └────────┘ └────────┘
```

## 🔧 配置选项

### 向量数据库

**Qdrant**（默认）:
```env
VECTOR_STORAGE_TYPE=qdrant
VECTOR_STORAGE_URL=http://qdrant:6333
```

**Chroma**:
```env
VECTOR_STORAGE_TYPE=chroma
VECTOR_STORAGE_URL=http://chroma:8000
```

**Pinecone**:
```env
VECTOR_STORAGE_TYPE=pinecone
PINECONE_API_KEY=your-key
PINECONE_ENVIRONMENT=us-east-1-aws
```

## 📈 监控

### 健康检查

```bash
# Gateway
curl http://localhost:8080/health

# Qdrant
curl http://localhost:6333/health

# Redis
docker exec memoryos-redis redis-cli ping
```

### 查看日志

```bash
# 所有服务
docker-compose logs -f

# 特定服务
docker-compose logs -f gateway
docker-compose logs -f qdrant
```

## 🔄 迁移指南

### 从 v0.2.x 迁移到 v0.3.0

1. **备份数据**
2. **更新配置**（移除 short_term_storage 配置）
3. **部署新版本**
4. **验证迁移**

详细步骤见 `docs/PRODUCTION_DEPLOYMENT.md`

## 🚨 故障排查

### 服务无法启动

```bash
# 查看日志
docker-compose logs

# 重启服务
docker-compose restart

# 完全重建
docker-compose down
docker-compose up -d --build
```

### 连接失败

```bash
# 检查网络
docker network ls
docker network inspect memoryos-rust_memoryos-backend

# 检查服务状态
docker-compose ps
```

## 📚 详细文档

查看 `docs/PRODUCTION_DEPLOYMENT.md` 获取：
- 完整架构说明
- 详细配置指南
- 迁移步骤
- 监控设置
- 安全最佳实践
- 性能优化建议

## 🎯 下一步

部署完成后：
1. ✅ 运行集成测试验证功能
2. ✅ 运行性能测试验证性能
3. ✅ 配置监控和告警
4. ✅ 设置备份策略

---

**部署愉快！** 🚀
