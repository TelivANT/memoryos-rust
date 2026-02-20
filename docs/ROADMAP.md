# 产品路线图

**版本**: 0.12.0  
**更新**: 2026-02-20

---

## 🎯 项目愿景

打造高性能的 AI Agent 记忆管理系统，提供完整的 3-Tier 记忆架构 + 企业级 RBAC/多租户支持。

---

## 📊 当前状态 (v0.12.0)

**完成度**: ~99%  
**状态**: Enterprise Ready  
**发布日期**: 2026-02-20

### 已完成功能

#### 核心功能
- ✅ 3-Tier 记忆架构（STM/MTM/LTM）
- ✅ 六边形架构
- ✅ 配置热更新（5 秒自动生效）
- ✅ 实时健康检查（运行时动态探测）
- ✅ 优雅降级（Full/Degraded/Noop 三层）
- ✅ FAQ 热度追踪/自动提升已实现
- ✅ Router Tier 0 FAQ 直接命中（v0.3.0 实现）

#### LLM 集成
- ✅ 10 种 LLM 适配器（OpenAI, Gemini, Claude, Ollama, DeepSeek, OpenRouter, Azure, Groq, Cohere, Mistral）
- ✅ 流式响应（SSE）
- ✅ 参数透传（所有 OpenAI 参数）
- ✅ 3-Tier 路由（基于消息长度的启发式分类）

#### 存储
- ✅ Redis 短期存储
- ✅ NATS 短期存储（备选）
- ✅ 3 种向量数据库（Qdrant, Chroma, Pinecone）
- ✅ 真实 Embedding（OpenAI API）

#### 安全
- ✅ PII 脱敏（email/phone/credit_card/SSN/API_key）
- ✅ Prompt 注入检测（17 种模式）
- ✅ IP 防御系统

#### 部署
- ✅ Docker 部署
- ✅ Kubernetes 部署
- ✅ K3s 自动化部署
- ✅ API Key 认证（Qdrant 存储）

#### SDK
- ✅ Python SDK（HTTP 封装 + 自动重试 + SSE 流式 + 异常处理）

---

## 🚧 待实现功能

### FAQ 系统 - Router 集成

**目标**: 将已实现的 FAQ 热度追踪集成到路由器

**已完成**:
- [x] **热度追踪** - HeatTracker 已实现，支持访问计数和热度计算
- [x] **自动提升** - AutoPromoter 已实现，支持 QA → Candidate → FAQ 提升
- [x] **Wiki 导出** - WikiExporter 本地导出已实现

**待实现**:
- [x] **Router Tier 0** - FAQ 直接命中，绕过 LLM 返回 ✅ (v0.3.0)
- [x] **Wiki S3 导出** - 通过 WikiExportBackend + OpenDAL 实现 ✅ (v0.3.0)
- [x] **Wiki Confluence 导出** - 通过 WikiExportBackend + REST API 实现 ✅ (v0.3.0)

**优先级**: 高  
**预计完成**: 2026-03-10  
**详细文档**: [FAQ_SYSTEM.md](./FAQ_SYSTEM.md)

### 知识图谱升级

**目标**: 从 Mermaid 解析升级为真正的 GraphRAG

**已完成**:
- [x] 实体自动提取（正则模式匹配，支持人名/组织/地点识别）
- [x] 关系提取（10 种关系模式：works_at, located_in, related_to 等）
- [x] 图查询 API（HTTP 端点 /v1/graph）
- [x] 图查询方法（query_entity, query_by_label, query_relations, query_path DFS）
- [x] extract_and_merge 自动合并实体和关系

**优先级**: 中  
**状态**: ✅ 完成 (v0.4.0)

### 多模态存储

**目标**: 实现多模态内容的存储和检索

**已完成**:
- [x] MultiModalStorage trait 实现（Qdrant-backed QdrantMultiModalStorage）
- [x] HTTP 端点（/v1/multimodal: store, search, search/embedding, recent）
- [x] Python SDK 异步支持（async_client.py with aiohttp）

**待实现**:
- [ ] CLIP/Whisper 实际集成（当前使用 embedding 向量输入）

**优先级**: 中  
**状态**: ✅ 核心完成 (v0.5.0)

---

## 🗓️ 版本规划

### v0.3.0 - FAQ 路由集成 + 导出完善 (2-3 周)

**目标**: 将 FAQ 集成到路由器，完成 Wiki 导出

**功能**:
- [x] Router Tier 0: FAQ 直接命中 ✅
- [x] Wiki S3 导出实现（OpenDAL S3ExportBackend） ✅
- [x] Wiki Confluence 导出实现（REST API ConfluenceExportBackend） ✅
- [x] FAQ 管理 API（get_candidates, promote, delete, history, stats） ✅
- [x] 清理重复的 wiki exporter 实现（core/wiki 委托给 core/faq） ✅

**优先级**: 高  
**状态**: ✅ 完成  
**完成日期**: 2026-02-20

---

### v0.4.0 - 知识图谱升级

**目标**: 从 Mermaid 解析升级为真正的 GraphRAG

**功能**:
- [x] 实体自动提取（正则模式匹配） ✅
- [x] 关系提取（10 种关系模式） ✅
- [x] 图查询 API（/v1/graph 端点） ✅
- [x] 图查询方法（entity, label, relations, path） ✅

**优先级**: 中  
**状态**: ✅ 完成  
**完成日期**: 2026-02-20

---

### v0.5.0 - 多模态存储 + Python SDK 增强

**目标**: 实现多模态存储，增强 Python SDK

**功能**:
- [x] MultiModalStorage trait 实现（Qdrant-backed） ✅
- [x] 多模态 HTTP 端点（/v1/multimodal） ✅
- [x] Python SDK 异步支持（async_client.py） ✅

**优先级**: 中  
**状态**: ✅ 完成  
**完成日期**: 2026-02-20

---

### v0.6.0 - 记忆增强

**目标**: 增强记忆管理功能

**功能**:
- [x] 记忆版本控制（version + previous_version_id） ✅
- [x] 记忆标签和分类（tags 字段） ✅
- [x] 记忆搜索增强（/v1/memory/manage/search/tags） ✅
- [x] 记忆导出/导入（JSON + Markdown） ✅

**优先级**: 中  
**状态**: ✅ 完成  
**完成日期**: 2026-02-20

---

### v0.7.0 - 性能基准测试

**目标**: 完成性能基准测试，验证所有优化模块效果

**功能**:
- [x] 优化模块基准测试（BloomFilter, EmbeddingCache, SimilarityFilter） ✅
- [x] 图模块基准测试（entity/relation extraction, query） ✅
- [x] 安全模块基准测试（injection, PII, encryption, audit） ✅
- [ ] 发布基准测试报告

**优先级**: 高  
**状态**: ✅ 核心完成  
**完成日期**: 2026-02-20

---

### v0.8.0 - 安全增强

**目标**: 安全加固

**功能**:
- [x] API Key Hash 存储（已在 store.rs 实现） ✅
- [x] 数据加密（AES-256-GCM，DataEncryptor） ✅
- [x] 结构化审计日志（AuditLogger + 文件持久化） ✅
- [x] GDPR 完整合规（GdprManager: consent + export + deletion + 文件持久化） ✅
- [x] 安全 API 端点（/v1/security/audit + /v1/security/gdpr） ✅

**优先级**: 高  
**状态**: ✅ 完成  
**完成日期**: 2026-02-20

---

### v0.9.0 - 技术债清理 + v1.0.0 准备

**目标**: 清理技术债，准备正式发布

**功能**:
- [x] 加密升级：XOR → AES-256-GCM ✅
- [x] 审计日志持久化（JSONL 文件） ✅
- [x] GDPR 记录持久化（JSON 文件） ✅
- [x] 多模态路由接入 main.rs ✅
- [x] vector_storage_benchmark 优雅降级（Qdrant 不可用时跳过） ✅
- [x] 功能对比表更新 ✅
- [x] 性能基准测试报告 ✅
- [x] 安全审计报告 ✅

**优先级**: 最高  
**状态**: ✅ 完成  
**完成日期**: 2026-02-20

---

### v0.10.0 - 可观测性 + LLM FAQ 分类

**目标**: 生产级可观测性集成 + LLM 自动分类 FAQ

**功能**:
- [x] Prometheus 指标集成（HTTP/Router/FAQ/LLM 全链路） ✅
- [x] /metrics 端点（Prometheus text exposition format） ✅
- [x] 请求路径归一化（避免高基数标签爆炸） ✅
- [x] LLM FAQ 分类器（FaqClassification + LlmClassifierConfig） ✅
- [x] /v1/admin/faq/classify API 端点 ✅
- [x] 分类提示构建 + 响应解析（JSON 优先 + 关键词兜底） ✅

**优先级**: 高  
**状态**: ✅ 完成  
**完成日期**: 2026-02-20

---

### v0.11.0 - 剩余问题全面修复

**目标**: 修复本地端到端测试发现的所有问题

**功能**:
- [x] Tag 搜索: Qdrant 原生 payload filter ✅
- [x] Memory History: QdrantHistoryStorage 接入 gateway ✅
- [x] Redis 升级: 0.24 → 0.32 ✅
- [x] Graph LLM 提取: build_llm_extraction_prompt + parse ✅
- [x] Auth 警告增强 ✅
- [x] Audit/GDPR 可插拔存储后端 ✅

**优先级**: 高  
**状态**: ✅ 完成  
**完成日期**: 2026-02-20

---

### v0.12.0 - 企业级功能

**目标**: RBAC + 多租户 + 独立管理服务

**功能**:
- [x] RBAC 权限模型（SuperAdmin/Admin/User/ReadOnly + 6 种权限） ✅
- [x] 多租户数据隔离（TenantContext + 自动 tenant_id 过滤） ✅
- [x] memoryos-admin 独立管理服务（port 9090，内网部署） ✅
- [x] 管理 API: 用户/租户/RBAC/审计/系统统计 ✅
- [x] Gateway RBAC 中间件集成 ✅

**优先级**: 最高  
**状态**: ✅ 完成  
**完成日期**: 2026-02-20

---

### v1.0.0 - 正式版

**目标**: 正式发布

**功能**:
- [x] 完整的文档 ✅
- [ ] 用户案例
- [x] 性能基准测试报告 ✅
- [x] 安全审计报告 ✅
- [ ] 生产环境验证

**优先级**: 最高  
**预计发布**: 2026-07-01

---

## 🎯 长期规划 (v2.0+)

### 多模态增强
- CLIP/Whisper 实际模型集成（当前使用 embedding 向量输入）
- 跨模态检索（text→image / image→text）
- 视频帧提取与摘要

### 可观测性增强
- OpenTelemetry 分布式链路追踪（当前仅 Prometheus 指标）
- Grafana 仪表盘模板

### FAQ 增强
- 多语言 FAQ 自动翻译
- FAQ 提升阈值 A/B 测试

### 分布式增强
- 多区域部署
- 数据同步
- 灾难恢复

### AI 增强
- 自动记忆压缩
- 智能记忆推荐
- 记忆质量评分

### 企业增强
- 计费系统
- SLA 保证
- LDAP/SSO 集成
- 管理 Web UI

---

## 📊 功能对比

### 当前 vs Mem0

| 功能 | MemoryOS-Rust v0.9.0 | Mem0 | 状态 |
|------|---------------------|------|------|
| **3-Tier Memory** | ✅ | ✅ | 对等 |
| **LLM 支持** | 10 种 | 10+ 种 | ✅ 对等 |
| **向量数据库** | 3 种 (Qdrant/Chroma/Pinecone) | 5+ 种 | ⚠️ 接近 |
| **知识图谱** | ✅ GraphRAG (v0.4.0) | ✅ | ✅ 对等 |
| **Python SDK** | ✅ 同步+异步+流式 | ✅ | ✅ 对等 |
| **记忆版本控制** | ✅ (v0.6.0) | ✅ | ✅ 对等 |
| **多模态存储** | ✅ (v0.5.0) | ✅ | ✅ 对等 |
| **数据加密** | ✅ AES-256-GCM | ⚠️ | ✅ 领先 |
| **审计日志** | ✅ 持久化 | ❌ | ✅ 领先 |
| **GDPR 合规** | ✅ 完整 | ⚠️ | ✅ 领先 |
| **配置热更新** | ✅ | ❌ | ✅ 领先 |
| **实时健康检查** | ✅ | ❌ | ✅ 领先 |
| **优雅降级** | ✅ | ⚠️ | ✅ 领先 |
| **性能** | 高（Rust） | 中（Python） | ✅ 领先 |

**目标**: v1.0.0 达到功能对等，性能超越

---

## 🚀 里程碑

| 版本 | 日期 | 状态 | 重点功能 |
|------|------|------|---------|
| **v0.1.0** | 2026-02-16 | ✅ 完成 | 基础功能 |
| **v0.2.0-alpha** | 2026-02-18 | ✅ 完成 | MVP (早期开发) |
| **v0.3.0** | 2026-02-20 | ✅ 完成 | FAQ 路由集成 + 导出完善 |
| **v0.4.0** | 2026-02-20 | ✅ 完成 | 知识图谱升级 (GraphRAG) |
| **v0.5.0** | 2026-02-20 | ✅ 完成 | 多模态存储 + SDK 增强 |
| **v0.6.0** | 2026-02-20 | ✅ 完成 | 记忆增强 |
| **v0.7.0** | 2026-02-20 | ✅ 完成 | 性能基准测试 |
| **v0.8.0** | 2026-02-20 | ✅ 完成 | 安全增强 |
| **v0.9.0** | 2026-02-20 | ✅ 完成 | 技术债清理 + v1.0.0 准备 |
| **v0.10.0** | 2026-02-20 | ✅ 完成 | Prometheus 可观测性 + LLM FAQ 分类 |
| **v0.11.0** | 2026-02-20 | ✅ 完成 | 剩余问题修复 |
| **v0.12.0** | 2026-02-20 | ✅ 完成 | 企业级: RBAC + 多租户 + Admin 服务 |
| **v1.0.0** | 2026-07-01 | 📅 计划 | 正式发布 |

---

## 📈 进度跟踪

### 总体进度

```
v0.1.0       ████████████████████ 100%
v0.2.0-alpha ██████████████░░░░░░  72%
v0.3.0       ████████████████████ 100%
v0.4.0       ████████████████████ 100%
v0.5.0       ████████████████████ 100%
v0.6.0       ████████████████████ 100%
v0.7.0       ██████████████████░░  90%
v0.8.0       ████████████████████ 100%
v0.9.0       ████████████████████ 100%
v1.0.0       ██████████████████░░  90%
```

### 功能完成度

| 类别 | 完成度 |
|------|--------|
| **核心功能** | 98% |
| **LLM 集成** | 95% |
| **存储** | 95% |
| **运维** | 90% |
| **安全** | 95% |
| **文档** | 95% |
| **测试** | 85% |
| **总体** | **~98%** |

---

## 🎯 优先级

### P0 - 必须完成（v1.0.0 前）
1. ✅ 配置热更新
2. ✅ 实时健康检查
3. ✅ 优雅降级
4. ✅ FAQ Router Tier 0 集成（v0.3.0）
5. ✅ 知识图谱升级（v0.4.0）
6. ✅ 性能基准测试（v0.7.0）
7. ✅ 安全增强（v0.8.0）

### P1 - 重要功能
1. ✅ Wiki S3/Confluence 导出（v0.3.0）
2. ✅ 多模态存储实现（v0.5.0）
3. ✅ 记忆版本控制（v0.6.0）
4. ✅ Python SDK 异步支持（v0.5.0）

### P2 - 可选功能
1. [ ] 分布式增强（v2.0+）
2. ✅ 企业功能（v0.12.0 完成: RBAC + 多租户 + Admin 服务）

---

## 📚 相关文档

- [CHANGELOG.md](../CHANGELOG.md) - 版本变更历史
- [COMPARISON.md](./COMPARISON.md) - 与 Mem0 对比
- [PERFORMANCE_REPORT.md](./PERFORMANCE_REPORT.md) - 性能基准测试报告
- [../SECURITY_AUDIT.md](../SECURITY_AUDIT.md) - 安全审计报告
- [plan/ROADMAP_4_WEEKS.md](./plan/ROADMAP_4_WEEKS.md) - 4 周详细计划
- [plan/GAP_ANALYSIS.md](./plan/GAP_ANALYSIS.md) - 差距分析

---

**版本**: 0.12.0  
**更新**: 2026-02-20  
**下一个版本**: v1.0.0
