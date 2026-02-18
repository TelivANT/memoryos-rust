# Issue 追踪规范

> **原则**: 所有异常、问题、优化都在这里追踪，不创建临时文档

---

## 📂 目录结构

```
issues/
├── open/                    # 待处理
├── in-progress/             # 处理中
├── resolved/                # 已解决
└── archived/                # 已归档
```

---

## 📝 命名规范

**格式**: `[优先级]-[简短描述].md`

### 优先级定义
- **P0**: 阻塞性问题，必须立即解决
- **P1**: 重要问题，影响核心功能
- **P2**: 一般问题，可以延后处理
- **P3**: 优化建议，不影响功能

### 命名示例
```
✅ 正确:
- P0-redis-connection-timeout.md
- P1-embedding-cache-miss.md
- P2-performance-optimization.md

❌ 错误:
- bug-fix.md
- issue-001.md
- 修复Redis问题.md
```

---

## 📋 Issue 模板

```markdown
# [问题标题]

**优先级**: P0/P1/P2/P3  
**状态**: 待处理/处理中/已解决/已归档  
**创建时间**: YYYY-MM-DD  
**负责人**: TBD  
**预计解决时间**: TBD

---

## 问题描述

简要描述问题现象和影响范围。

## 复现步骤

1. 步骤 1
2. 步骤 2
3. 步骤 3

## 预期行为

描述正确的行为应该是什么。

## 实际行为

描述当前的错误行为。

## 根因分析

分析问题的根本原因。

## 解决方案

描述如何解决这个问题。

## 验证方法

如何验证问题已解决。

---

## 变更历史

- YYYY-MM-DD: 创建 issue
- YYYY-MM-DD: 开始处理
- YYYY-MM-DD: 已解决
```

---

## 🔄 状态流转

```
待处理 (open/)
    ↓
处理中 (in-progress/)
    ↓
已解决 (resolved/)
    ↓
已归档 (archived/)
```

### 流转规则

1. **创建 Issue**: 在 `open/` 目录创建文档
2. **开始处理**: 移动到 `in-progress/`，更新状态和负责人
3. **解决完成**: 移动到 `resolved/`，记录解决方案
4. **归档**: 30 天后移动到 `archived/`

---

## 📊 Issue 统计

使用脚本统计 issue 数量：

```bash
# 统计各状态 issue 数量
find issues/open -name "*.md" | wc -l
find issues/in-progress -name "*.md" | wc -l
find issues/resolved -name "*.md" | wc -l
```

---

## 🔍 查找 Issue

### 按优先级查找
```bash
find issues -name "P0-*.md"
find issues -name "P1-*.md"
```

### 按关键词查找
```bash
grep -r "redis" issues/
grep -r "performance" issues/
```

---

## ✅ 最佳实践

### 正确做法
- ✅ 及时创建 issue 记录问题
- ✅ 使用标准模板
- ✅ 详细记录复现步骤
- ✅ 更新状态和变更历史
- ✅ 解决后记录方案

### 错误做法
- ❌ 创建临时文档 (如 FIX_XXX.md)
- ❌ 不记录问题就直接修复
- ❌ 使用非标准命名
- ❌ 不更新状态
- ❌ 不记录解决方案

---

## 📚 相关文档

- [路线图](../roadmap/README.md)
- [开发指南](../docs/DEVELOPMENT.md)

---

**最后更新**: 2026-02-18
