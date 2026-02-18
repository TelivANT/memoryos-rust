# MemoryOS-Rust 项目文档索引

**最后更新**: 2026-02-18

---

## 📚 核心文档

### 1. [README.md](./README.md)
项目主页，包含：
- 项目简介和特性
- 快速开始指南
- 架构概览
- 当前状态（85% 完成）

### 2. [QUICK_REFERENCE.md](./QUICK_REFERENCE.md)
快速参考指南：
- 常用命令
- API 端点
- 配置示例
- 故障排查

### 3. [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)
部署指南：
- Docker 部署
- 生产环境配置
- 监控和日志
- 性能调优

---

## 🏗️ 架构文档

### 4. [ARCHITECTURE_DIAGRAMS.md](./ARCHITECTURE_DIAGRAMS.md)
完整架构图：
- 系统架构
- 数据流图
- 模块依赖
- 六边形架构

### 5. [COMPARISON_WITH_MEM0.md](./COMPARISON_WITH_MEM0.md)
与 Mem0 对比：
- 功能对比（85% vs 88%）
- 架构差异
- 性能对比
- 路线图

---

## 🔬 技术分析

### 6. [MEM0_ANALYSIS_AND_PLAN.md](./MEM0_ANALYSIS_AND_PLAN.md)
Mem0 源码分析：
- 记忆历史追踪
- 知识图谱
- 多语言 SDK
- 实现计划

### 7. [HISTORY_STORAGE_COMPARISON.md](./HISTORY_STORAGE_COMPARISON.md)
历史存储方案对比：
- Redis vs Qdrant
- 性能分析
- 迁移方案
- 最佳实践

---

## 📝 开发文档

### 8. [CHANGELOG.md](./CHANGELOG.md)
变更日志：
- 版本历史
- 新增功能
- Bug 修复
- 破坏性变更

### 9. [DOCUMENTATION_STATUS.md](./DOCUMENTATION_STATUS.md)
文档完整性报告：
- 文档覆盖率
- 缺失文档
- 改进建议

---

## 🎯 功能文档

### 10. [HISTORY_FEATURE_COMPLETE.md](./archive/phase_reports/HISTORY_FEATURE_COMPLETE.md)
记忆历史追踪完成报告：
- 实现细节
- API 使用
- 性能特点
- 与 Mem0 对比

### 11. [WIKI_WORKER_FIX_COMPLETE.md](./archive/phase_reports/WIKI_WORKER_FIX_COMPLETE.md)
Wiki 和 Worker 修复报告：
- 架构调整
- 依赖修复
- 编译验证

---

## 📦 归档文档

### Phase 报告
位置: `./archive/phase_reports/`

包含所有 Phase 完成报告：
- P0-P2 修复报告
- Phase 1-6 完成报告
- 各阶段总结

### 历史文档
位置: `./archive/old_docs/`

包含过时的文档：
- 旧的状态报告
- 临时分析文档
- 开发笔记

---

## 🔧 脚本

### 1. [sync.sh](./sync.sh)
代码同步脚本：
- Git 提交和推送
- 自动化部署准备

### 2. [deploy.sh](./deploy.sh)
部署脚本：
- Docker 构建
- 服务启动
- 健康检查

---

## 📊 文档统计

| 类型 | 数量 | 位置 |
|------|------|------|
| **核心文档** | 3 | 根目录 |
| **架构文档** | 2 | 根目录 |
| **技术分析** | 2 | 根目录 |
| **开发文档** | 2 | 根目录 |
| **功能文档** | 2 | archive/ |
| **归档文档** | 40+ | archive/ |
| **脚本** | 2 | 根目录 |

**总计**: 12 个活跃文档 + 40+ 归档文档

---

## 🎯 文档使用指南

### 新手入门
1. 阅读 [README.md](./README.md)
2. 查看 [QUICK_REFERENCE.md](./QUICK_REFERENCE.md)
3. 参考 [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)

### 架构理解
1. 查看 [ARCHITECTURE_DIAGRAMS.md](./ARCHITECTURE_DIAGRAMS.md)
2. 对比 [COMPARISON_WITH_MEM0.md](./COMPARISON_WITH_MEM0.md)
3. 深入 [MEM0_ANALYSIS_AND_PLAN.md](./MEM0_ANALYSIS_AND_PLAN.md)

### 功能开发
1. 参考 [HISTORY_STORAGE_COMPARISON.md](./HISTORY_STORAGE_COMPARISON.md)
2. 查看归档的功能报告
3. 更新 [CHANGELOG.md](./CHANGELOG.md)

---

## 📝 文档维护

### 更新频率
- **README.md**: 每次重大更新
- **CHANGELOG.md**: 每次发布
- **QUICK_REFERENCE.md**: 按需更新
- **其他文档**: 功能变更时更新

### 归档规则
- Phase 报告 → `archive/phase_reports/`
- 过时文档 → `archive/old_docs/`
- 保留最近 3 个月的活跃文档

---

**维护者**: MemoryOS-Rust Team  
**最后清理**: 2026-02-18
