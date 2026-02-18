# Ollama 本地 Demo 指南

**环境**: macOS + Ollama  
**模型**: gpt-oss:20b  
**时间**: 2026-02-17

---

## 🎯 快速开始

### 前置条件

✅ 已安装：
- Ollama (运行中)
- 模型 `gpt-oss:20b` (已下载)
- Rust 工具链

### 启动 Demo

```bash
# 1. 启动 MemoryOS Gateway
./demo-ollama-simple.sh

# 2. 在另一个终端测试
./test-ollama-simple.sh
```

---

## 📋 Demo 说明

### 模式 1: 简化模式（推荐）

**特点**:
- ✅ 不需要 Redis/Qdrant
- ✅ 只测试 LLM 功能
- ✅ 快速启动

**脚本**: `demo-ollama-simple.sh`

**配置**: `config.ollama.toml`
```toml
[llm]
provider = "ollama"
base_url = "http://localhost:11434/v1"
model = "gpt-oss:20b"
```

### 模式 2: 完整模式

**特点**:
- ✅ 完整功能（LLM + Memory）
- ⚠️ 需要 Redis + Qdrant

**脚本**: `demo-ollama.sh`

**启动依赖**:
```bash
# Redis
docker run -d -p 6379:6379 redis:7-alpine

# Qdrant
docker run -d -p 6333:6333 qdrant/qdrant:latest
```

---

## 🧪 测试用例

### 1. 健康检查
```bash
curl http://localhost:8080/health
```

**预期输出**:
```json
{
  "status": "healthy",
  "mode": "ready"
}
```

### 2. 简单对话
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-oss:20b",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "stream": false
  }'
```

### 3. 流式响应
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-oss:20b",
    "messages": [
      {"role": "user", "content": "Count to 5"}
    ],
    "stream": true
  }'
```

---

## 🐛 当前已知问题（2026-02-17 回真）

1. **本机 Ollama 0.15.6 在 macOS 上启动崩溃（`mlx_random_key` / `NSRangeException`）**
   - 影响: `ollama serve` 无法拉起，`http://localhost:11434` 不可用。
   - 与 MemoryOS 代码关系: **无直接关系**（Ollama 本机二进制问题）。
   - 建议: 升级到 `ollama-app 0.16.2+`（或更新版本）后重试。

2. **路由层覆盖模型名问题已修复**
   - 现状: `LlmRouter` 不再强制覆盖请求中的 `model`，会透传例如 `gpt-oss:20b`。
   - 覆盖: 已新增路由单元测试，验证普通/流式路径均保持请求模型不变。

---

## 📊 性能参考

**测试环境**: MacBook (M1/M2)

| 操作 | 延迟 | 说明 |
|------|------|------|
| 健康检查 | <10ms | 本地检查 |
| 简单对话 | 2-5s | 取决于模型大小 |
| 流式响应 | 首字节 <1s | 逐 token 返回 |

---

## 🔧 故障排查

### Ollama 连接失败

**症状**: `Ollama request failed: connection refused`

**解决**:
```bash
# 检查 Ollama 是否运行
curl http://localhost:11434/api/tags

# 如果没运行，启动
ollama serve
```

### Ollama 启动即崩溃（macOS）

**症状**: `NSRangeException` / `mlx_random_key` / `Attempted to create a NULL object`

**解决**:
```bash
# 1) 卸载旧 formula 版本
brew uninstall ollama

# 2) 安装新版 app/cli
brew install --cask ollama-app

# 3) 启动服务
ollama serve
```

### 模型未找到

**症状**: `model 'gpt-oss:20b' not found`

**解决**:
```bash
# 拉取模型
ollama pull gpt-oss:20b

# 验证
ollama list
```

### 端口被占用

**症状**: `Address already in use (os error 48)`

**解决**:
```bash
# 查找占用进程
lsof -i :8080

# 杀死进程
kill -9 <PID>
```

---

## 📝 下一步

1. **先恢复本机 Ollama 服务可用性**（确认 `curl http://localhost:11434/api/tags` 返回成功）
2. **跑完整模式验证**（Redis + Qdrant + `/v1/memory/*`）
3. **继续 Phase 3 收尾**（规则化 consolidation、故障注入集成测试、流式传输对齐）

---

## 📚 相关文档

- [CODE_REVIEW.md](./CODE_REVIEW.md) - 代码审阅报告
- [REMOTE_DEV.md](./REMOTE_DEV.md) - 远程开发指南
- [API.md](./docs/API.md) - API 文档

---

**Demo 状态**: ⚠️ 受本机 Ollama 版本影响（服务可启动即可用）  
**完整功能**: 🚧 需要 Redis + Qdrant
