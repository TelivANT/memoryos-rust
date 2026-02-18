# Phase 6 实施计划

**版本**: v1.0  
**创建时间**: 2026-02-17  
**预计周期**: 2-3 周

---

## 📅 时间规划

### Week 1: 功能完善 (7 天)

#### Day 1-2: LLM 功能增强
**负责人**: 核心开发  
**工作量**: 2 天

**任务**:
- [ ] 实现 `LlmAdapter::summarize()` 方法
- [ ] 实现 `LlmAdapter::extract_profile()` 方法
- [ ] 更新 `consolidate_to_mid_term_internal()`
- [ ] 添加 JSON Schema 约束
- [ ] 编写单元测试

**交付物**:
- `crates/memoryos-ports/src/llm.rs` (更新)
- `crates/memoryos-adapters/src/memory/manager.rs` (更新)
- 测试用例

---

#### Day 3-4: 本地 Embedding
**负责人**: 核心开发  
**工作量**: 2 天

**任务**:
- [ ] 创建 `memoryos-embedding` crate
- [ ] 实现 ONNX provider
- [ ] 实现 Fallback 机制
- [ ] 集成到 MemoryManager
- [ ] 性能测试

**交付物**:
- `crates/memoryos-embedding/` (新建)
- 模型下载脚本
- 性能基准报告

---

#### Day 5-7: Worker 异步任务
**负责人**: 核心开发  
**工作量**: 3 天

**任务**:
- [ ] 选择消息队列（Redis Stream）
- [ ] 实现 Task Producer
- [ ] 实现 Task Consumer
- [ ] 实现任务类型：
  - consolidate_memory
  - extract_profile
  - export_knowledge
- [ ] 添加任务监控

**交付物**:
- `crates/memoryos-tasks/` (新建)
- `crates/memoryos-worker/src/main.rs` (完善)
- Worker 部署文档

---

### Week 2: 性能优化 (7 天)

#### Day 1-2: 压测和优化
**负责人**: 性能工程师  
**工作量**: 2 天

**任务**:
- [ ] 编写 `wrk` 压测脚本
- [ ] 编写 `k6` 压测脚本
- [ ] 执行压测（4 个场景）
- [ ] 生成压测报告
- [ ] 识别性能瓶颈

**交付物**:
- `scripts/perf/wrk_test.sh`
- `scripts/perf/k6_test.js`
- `docs/PERFORMANCE_REPORT.md`

---

#### Day 3-4: 缓存和连接池
**负责人**: 核心开发  
**工作量**: 2 天

**任务**:
- [ ] 使用 `lru` crate 实现真正的 LRU
- [ ] 配置 Redis 连接池
- [ ] 配置 Qdrant 连接池
- [ ] 添加连接池监控
- [ ] 缓存统计和分析

**交付物**:
- `crates/memoryos-adapters/src/memory/manager.rs` (优化)
- `config.production.toml` (更新)
- 缓存性能报告

---

#### Day 5-7: 性能调优
**负责人**: 全员  
**工作量**: 3 天

**任务**:
- [ ] 根据压测结果优化瓶颈
- [ ] 调整配置参数
- [ ] 重新压测验证
- [ ] 达到目标 QPS

**交付物**:
- 优化后的代码
- 最终性能报告
- 调优文档

---

### Week 3: 商业化准备 (7 天)

#### Day 1-2: API Key 认证
**负责人**: 后端开发  
**工作量**: 2 天

**任务**:
- [ ] 创建 `memoryos-auth` crate
- [ ] 实现 API Key 生成和管理
- [ ] 实现认证中间件
- [ ] 实现权限控制
- [ ] 添加 Key 管理 API

**交付物**:
- `crates/memoryos-auth/` (新建)
- API 文档更新
- 认证测试用例

---

#### Day 3-4: 配额和多租户
**负责人**: 后端开发  
**工作量**: 2 天

**任务**:
- [ ] 实现配额检查中间件
- [ ] 实现 Token 计数
- [ ] 添加 `tenant_id` 支持
- [ ] 实现租户数据隔离
- [ ] 添加租户管理 API

**交付物**:
- `crates/memoryos-auth/src/quota.rs`
- 多租户设计文档
- 测试用例

---

#### Day 5-7: 使用量统计
**负责人**: 数据工程师  
**工作量**: 3 天

**任务**:
- [ ] 创建 `memoryos-analytics` crate
- [ ] 实现请求日志记录
- [ ] 实现使用量聚合
- [ ] 实现统计查询 API
- [ ] 添加导出功能

**交付物**:
- `crates/memoryos-analytics/` (新建)
- 统计 API 文档
- Dashboard 原型

---

## 🎯 里程碑

### Milestone 1: 功能完整 (Week 1 结束)
**验收标准**:
- [ ] LLM 总结和 Profile 提取正常工作
- [ ] 本地 Embedding 可用
- [ ] Worker 服务可以处理异步任务
- [ ] 所有单元测试通过

### Milestone 2: 性能达标 (Week 2 结束)
**验收标准**:
- [ ] 压测 QPS > 10,000
- [ ] P99 延迟 < 200ms
- [ ] 缓存命中率 > 80%
- [ ] 无明显性能瓶颈

### Milestone 3: 商业就绪 (Week 3 结束)
**验收标准**:
- [ ] API Key 认证正常工作
- [ ] 配额限制正常工作
- [ ] 多租户隔离正常工作
- [ ] 使用量统计准确

---

## 📊 资源需求

### 人力
- 核心开发: 1 人 (全职 3 周)
- 性能工程师: 1 人 (Week 2)
- 后端开发: 1 人 (Week 3)
- 数据工程师: 1 人 (Week 3)

### 基础设施
- 开发环境: 本地 Mac/Linux
- 测试环境: 
  - Redis (单机)
  - Qdrant (单机)
  - Postgres (新增)
- 压测环境:
  - 4C8G 服务器 x 2
  - Redis Cluster
  - Qdrant Cluster

### 第三方服务
- OpenAI API (用于 LLM 调用和 Fallback)
- Hugging Face (模型下载)

---

## 🚨 风险管理

### 技术风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| ONNX 模型兼容性问题 | 中 | 高 | 提前测试多个模型，准备 Fallback |
| 压测无法达到目标 QPS | 中 | 高 | 预留调优时间，必要时降低目标 |
| Worker 消息队列选型错误 | 低 | 中 | 先用 Redis Stream，后续可切换 |
| LLM 总结质量不达标 | 低 | 中 | 优化 Prompt，使用更好的模型 |

### 进度风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| Week 1 延期 | 中 | 高 | 砍掉 Worker 部分功能 |
| Week 2 压测时间不足 | 中 | 中 | 提前准备压测环境 |
| Week 3 商业化功能复杂 | 低 | 低 | 简化 MVP 功能 |

---

## ✅ 验收清单

### 功能验收
- [ ] LLM 总结压缩比 < 0.5
- [ ] Profile 提取准确率 > 90%
- [ ] 本地 Embedding 延迟 < 50ms
- [ ] Worker 任务处理延迟 < 5s
- [ ] 所有单元测试通过
- [ ] 所有集成测试通过

### 性能验收
- [ ] 纯聊天 QPS > 10,000
- [ ] 聊天+记忆 QPS > 5,000
- [ ] P99 延迟 < 200ms
- [ ] 错误率 < 0.5%
- [ ] 缓存命中率 > 80%

### 商业验收
- [ ] API Key 认证正常
- [ ] 配额限制正常
- [ ] 多租户隔离正常
- [ ] 使用量统计准确
- [ ] 文档完善

---

## 📚 交付物清单

### 代码
- [ ] `crates/memoryos-embedding/` - 本地 Embedding
- [ ] `crates/memoryos-tasks/` - 异步任务
- [ ] `crates/memoryos-auth/` - 认证鉴权
- [ ] `crates/memoryos-analytics/` - 统计分析
- [ ] 更新的 `memoryos-worker/` - Worker 服务
- [ ] 更新的 `memoryos-gateway/` - Gateway 服务

### 文档
- [ ] `docs/specs/PHASE6_REQUIREMENTS.md` - 需求文档
- [ ] `docs/specs/phase6_llm_enhancement.md` - LLM 技术方案
- [ ] `docs/specs/phase6_local_embedding.md` - Embedding 技术方案
- [ ] `docs/PERFORMANCE_REPORT.md` - 性能报告
- [ ] `docs/API.md` - API 文档更新
- [ ] `docs/DEPLOYMENT.md` - 部署文档更新

### 脚本
- [ ] `scripts/perf/wrk_test.sh` - wrk 压测
- [ ] `scripts/perf/k6_test.js` - k6 压测
- [ ] `scripts/download_models.sh` - 模型下载
- [ ] `scripts/setup_postgres.sh` - Postgres 初始化

---

## 🚀 下一步

1. **Review 需求文档** - 确认优先级和范围
2. **技术选型确认** - 消息队列、Embedding 模型
3. **环境准备** - 安装依赖、准备测试环境
4. **开始 Week 1** - LLM 功能增强

**准备好开始了吗？** 🎯
