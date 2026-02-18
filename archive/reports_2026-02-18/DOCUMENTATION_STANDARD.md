# MemoryOS-Rust 文档标准

> **核心原则**: 标准是稳定的，所有人都要遵循，包括我自己

---

## 📋 标准体系

### 1. 不创建版本文档

❌ **错误做法**:
```
V2_ROADMAP.md
V3_DESIGN.md
V4_PLAN.md
NEW_FEATURE_V2.md
```

✅ **正确做法**:
```
roadmap/enhancements/memory-compression.md
  ↓ 更新状态和完成度
roadmap/enhancements/memory-compression.md (同一个文件)
```

### 2. 路线图持续更新

**位置**: `roadmap/`

**结构**:
```
roadmap/
├── README.md                    # 路线图总览
├── features/                    # 核心功能
│   ├── foundation.md
│   ├── llm-integration.md
│   └── memory-system.md
└── enhancements/                # 增强功能
    ├── memory-compression.md
    ├── multimodal.md
    └── advanced-retrieval.md
```

**更新方式**:
- 只更新文档内的状态字段
- 只更新完成度百分比
- 记录变更历史
- 不创建新版本文件

### 3. Issue 追踪规范

**位置**: `issues/`

**结构**:
```
issues/
├── README.md                    # 追踪规范
├── open/                        # 待处理
│   └── [P0/P1/P2]-[描述].md
├── in-progress/                 # 处理中
│   └── [P0/P1/P2]-[描述].md
├── resolved/                    # 已解决
│   └── [P0/P1/P2]-[描述].md
└── archived/                    # 已归档
    └── [P0/P1/P2]-[描述].md
```

**命名规范**:
- `P0-redis-connection-timeout.md`
- `P1-embedding-cache-miss.md`
- `P2-performance-optimization.md`

**状态流转**:
```
open/ → in-progress/ → resolved/ → archived/
```

---

## 📊 文档分类

### 核心文档 (docs/)
- **设计文档**: `docs/specs/` - 架构、API 标准
- **API 文档**: `docs/API.md` - 接口规范
- **开发文档**: `docs/DEVELOPMENT.md` - 开发指南
- **运维文档**: `docs/ops/` - 部署、配置

### 路线图 (roadmap/)
- **功能列表**: `roadmap/features/` - 核心功能
- **增强功能**: `roadmap/enhancements/` - 扩展功能
- **状态追踪**: 在文档内更新状态

### Issue 追踪 (issues/)
- **问题管理**: 按优先级和状态分类
- **标准模板**: 统一的 issue 格式
- **状态流转**: 明确的流转规则

### 归档 (archive/)
- **历史文档**: 已完成阶段的报告
- **旧版文档**: 被替代的文档
- **仅供参考**: 不再更新

---

## ✅ 最佳实践

### 新增功能
1. 在 `roadmap/enhancements/` 创建文档
2. 使用标准模板
3. 设置状态为 📋 规划中
4. 在 `roadmap/README.md` 添加索引

### 功能开发
1. 更新状态为 🚧 开发中
2. 更新完成度百分比
3. 记录变更历史
4. 遇到问题创建 issue

### 功能完成
1. 更新状态为 ✅ 已完成
2. 设置完成度为 100%
3. 记录完成时间
4. 关闭相关 issue

### 遇到问题
1. 在 `issues/open/` 创建 issue
2. 使用标准命名: `[P0/P1/P2]-[描述].md`
3. 填写完整的 issue 模板
4. 按状态流转移动文件

---

## 🚫 禁止行为

### ❌ 创建版本文档
```
V2_ROADMAP.md          # 错误
V3_DESIGN.md           # 错误
NEW_FEATURE_V2.md      # 错误
```

### ❌ 创建临时文档
```
FIX_REDIS.md           # 错误
TODO_LIST.md           # 错误
TEMP_NOTES.md          # 错误
```

### ❌ 不规范命名
```
bug-fix.md             # 错误
issue-001.md           # 错误
修复问题.md             # 错误
```

---

## 📚 文档模板

### 功能文档模板
```markdown
# 功能名称

**状态**: 📋 规划中 / 🚧 开发中 / ✅ 已完成  
**完成度**: 0% - 100%  
**优先级**: P0/P1/P2/P3  
**负责人**: TBD  
**预计时间**: TBD

## 功能描述
...

## 技术方案
...

## 验收标准
- [ ] 标准 1
- [ ] 标准 2

## 变更历史
- YYYY-MM-DD: 创建文档
- YYYY-MM-DD: 状态更新
```

### Issue 模板
```markdown
# 问题标题

**优先级**: P0/P1/P2/P3  
**状态**: 待处理/处理中/已解决  
**创建时间**: YYYY-MM-DD  
**负责人**: TBD

## 问题描述
...

## 复现步骤
1. 步骤 1
2. 步骤 2

## 解决方案
...

## 变更历史
- YYYY-MM-DD: 创建 issue
- YYYY-MM-DD: 已解决
```

---

## 🔍 检查清单

在提交文档前，检查：

- [ ] 是否创建了版本文档 (V2/V3/V4)？
- [ ] 是否创建了临时文档？
- [ ] 是否使用了标准模板？
- [ ] 是否使用了规范命名？
- [ ] 是否更新了索引文档？
- [ ] 是否记录了变更历史？

---

## 📞 问题反馈

如果对标准有疑问或建议：
1. 在 `issues/open/` 创建 issue
2. 标题: `P2-documentation-standard-improvement.md`
3. 描述具体问题和建议

---

**最后更新**: 2026-02-18
