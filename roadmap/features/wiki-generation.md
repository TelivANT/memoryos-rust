# Wiki 生成系统 (memoryos-wiki-gen)

**状态**: 🚧 开发中 (设计完成)
**完成度**: 10%
**优先级**: P1
**负责人**: Devin
**预计时间**: 4 周

---

## 功能描述

Tree-sitter + LLM 混合管线，从多语言代码仓库自动生成结构化 Markdown Wiki，同时整合 FAQ 知识库导出。

### 目标
- 多语言代码解析 (V1: Rust / Python / Java / Vue)
- Symbol-centric 统一 IR，三层 Code Graph (File / Symbol / Runtime)
- LLM 生成文档 + Mermaid 架构图
- API Endpoint 自动提取 (OpenAPI/Proto 优先级最高)
- CLI 工具 + Gateway API 双路访问
- 增量更新 (SHA256 content/prompt hash)
- 证据追溯 (wiki_index.json)

---

## 技术方案

### Pipeline

| Phase | 描述 | 关键技术 |
|-------|------|---------|
| Phase 0 | Repo Intake | ignore + rayon + indicatif |
| Phase 1 | Multi-Language Parsing | tree-sitter (Rust/Python/Java/TS/HTML) |
| Phase 1.5 | API Endpoint Extraction | OpenAPI/Proto spec + framework route extraction |
| Phase 2 | Code Graph | petgraph (3-layer DiGraph) |
| Phase 3 | LLM Doc Generation | LlmAdapter trait + Evidence Pack + Cache |
| Phase 4 | Diagram Generation | Mermaid (module dep / API flow / class diagram) |
| Phase 5 | Page Assembly | tera templates + FAQ integration |
| Phase 6 | Export | WikiExportBackend (Local/S3/Confluence) |

### 新增依赖

- tree-sitter + language grammars
- petgraph, tera, clap, indicatif, ignore, rayon, sha2
- cargo_metadata, quick-xml

---

## 验收标准

- [ ] Rust 代码解析 → IR → Graph → LLM 文档 → Markdown 输出 (端到端)
- [ ] Python / Java / Vue 解析同样端到端通过
- [ ] API Endpoint 提取: Axum + FastAPI 至少各一个框架
- [ ] Mermaid 图生成: 模块依赖 + API Router Flow
- [ ] 增量更新: 修改一个文件后只重新生成受影响页面
- [ ] CLI: `memoryos-wiki-gen generate --repo . --output wiki-out`
- [ ] Gateway API: `POST /v1/wiki/generate` 触发生成
- [ ] FAQ 页面整合: faq/*.md 通过 LlmClassifier 分类
- [ ] 旧 Wiki System A 清理完成
- [ ] 所有 clippy / lint 通过

---

## 依赖关系

**前置条件**:
- memoryos-ports: LlmAdapter trait (已有)
- memoryos-core: WikiExportBackend trait (已有)
- memoryos-core: GraphEntity/GraphRelation model (已有, 可参考)
- memoryos-core: LlmClassifier (已有)

**阻塞问题**:
- 无

---

## 变更历史

### 2026-02-20
- **创建文档**: 设计文档完成 (docs/specs/wiki_gen_spec.md)
- **状态**: 📋 规划中 → 🚧 开发中 (设计完成)
- **完成度**: 0% → 10%

---

## 参考资料

- [Wiki 生成系统设计文档](../../docs/specs/wiki_gen_spec.md)
- [现有 FAQ Wiki 导出规范](../../docs/specs/wiki_export_spec.md)
- [架构设计](../../docs/ARCHITECTURE.md)

---

**最后更新**: 2026-02-20
