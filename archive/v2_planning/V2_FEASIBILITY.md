# V2.0 技术可行性评估

**评估时间**: 2026-02-18  
**评估人**: Kiro AI

---

## 🎯 功能可行性矩阵

| 功能 | 技术难度 | 现有基础 | 依赖风险 | 可行性 | 优先级 |
|------|---------|---------|---------|--------|--------|
| **OCR 集成** | ⭐⭐ | 0% | Low | ✅ 高 | P0 |
| **上下文问候** | ⭐ | 80% | None | ✅ 极高 | P0 |
| **文件监控** | ⭐⭐ | 0% | Low | ✅ 高 | P1 |
| **浏览器插件 API** | ⭐⭐ | 50% | None | ✅ 高 | P1 |
| **SQLite 迁移** | ⭐⭐⭐⭐ | 0% | High | ⚠️ 中 | P2 |
| **直接命中** | ⭐ | 90% | None | ✅ 极高 | P0 |
| **混合路由** | ⭐⭐ | 70% | None | ✅ 高 | P0 |
| **Wiki 导出** | ⭐⭐ | 0% | Low | ✅ 高 | P1 |
| **记忆压缩** | ⭐⭐⭐ | 30% | Medium | ✅ 中 | P1 |
| **多租户** | ⭐⭐ | 60% | None | ✅ 高 | P1 |

---

## ✅ 极易实现 (1-3 天)

### 1. 上下文问候 ⭐
**现有基础**: 
- ✅ 已有 `search_segments` (查询历史)
- ✅ 已有 LLM 适配器

**实现**:
```rust
// 在 chat 接口检测空消息
if messages.is_empty() || is_init_message(&messages) {
    let last_intent = retriever.search_unfinished_intent(user_id).await?;
    if let Some(intent) = last_intent {
        return Ok(format!("欢迎回来！要继续 {} 吗？", intent));
    }
}
```

**依赖**: 无  
**风险**: 无  
**工作量**: 1 天

---

### 2. 直接命中 (FAQ Mode) ⭐
**现有基础**:
- ✅ 已有 Qdrant 搜索
- ✅ 已有相似度计算

**实现**:
```rust
let results = qdrant.search(query, limit=1).await?;
if results[0].score > 0.92 {
    // 跳过 LLM，直接返回
    return Ok(results[0].content.clone());
}
```

**依赖**: 无  
**风险**: 需要调优阈值 (0.92 可能太高)  
**工作量**: 1 天

---

### 3. 混合路由 ⭐⭐
**现有基础**:
- ✅ 已有 10 个 LLM 适配器
- ✅ 已有路由逻辑框架

**实现**:
```rust
let complexity = analyze_query_complexity(query);
let llm = if complexity < 3 {
    router.get_local_llm()  // Ollama
} else {
    router.get_cloud_llm()  // GPT-4
};
```

**依赖**: 需要复杂度评分算法  
**风险**: 评分不准确会影响体验  
**工作量**: 2 天

---

## 🟡 中等难度 (3-7 天)

### 4. OCR 集成 ⭐⭐
**技术栈**:
- `image` crate (图片处理)
- `tesseract-rs` (OCR) 或 `ocrs` (纯 Rust)

**实现**:
```rust
// 1. 接收图片
let image = download_image(url).await?;

// 2. OCR 提取
let text = ocr::extract_text(&image)?;

// 3. 判断文本密度
if text.split_whitespace().count() > 50 {
    // 存为文本记忆
    store_text_memory(text).await?;
} else {
    // 调用 Vision LLM
    let desc = vision_llm.describe(image).await?;
    store_vision_memory(desc).await?;
}
```

**依赖**: 
- ⚠️ `tesseract` 系统依赖 (需要安装)
- ✅ `ocrs` 纯 Rust (无依赖)

**风险**: OCR 准确率依赖图片质量  
**工作量**: 3-5 天

---

### 5. 文件监控 ⭐⭐
**技术栈**: `notify` crate

**实现**:
```rust
use notify::{Watcher, RecursiveMode};

let mut watcher = notify::watcher(tx, Duration::from_secs(2))?;
watcher.watch("./src", RecursiveMode::Recursive)?;

loop {
    match rx.recv() {
        Ok(event) => {
            if event.path.ends_with(".rs") {
                // 重新索引文件
                reindex_file(event.path).await?;
            }
        }
    }
}
```

**依赖**: 无  
**风险**: 高频修改可能导致性能问题 (需要 debounce)  
**工作量**: 3 天

---

### 6. 浏览器插件 API ⭐⭐
**现有基础**:
- ✅ 已有 HTTP Gateway (Axum)

**实现**:
```rust
// 新增端点
#[post("/v1/ingest/webpage")]
async fn ingest_webpage(
    Json(payload): Json<WebpagePayload>
) -> Result<Json<Response>> {
    // 1. 提取正文 (Readability 算法)
    let content = extract_readable_content(&payload.html)?;
    
    // 2. 摘要
    let summary = llm.summarize(&content).await?;
    
    // 3. 存储
    storage.store_long_term(summary).await?;
    
    Ok(Json(Response { success: true }))
}
```

**依赖**: 
- 需要 Readability 算法 (可用 `readability` crate)

**风险**: HTML 解析可能不准确  
**工作量**: 3 天

---

### 7. Wiki 导出 ⭐⭐
**实现**:
```rust
// 定时任务：每天导出
async fn export_faqs() {
    let faqs = storage.get_aged_faqs(days=30).await?;
    
    for faq in faqs {
        let markdown = format!("# {}\n\n{}", faq.question, faq.answer);
        
        // 导出到 S3
        s3_client.put_object(
            bucket="wiki",
            key=format!("faqs/{}.md", faq.id),
            body=markdown
        ).await?;
    }
}
```

**依赖**: 
- `aws-sdk-s3` (S3)
- `reqwest` (Confluence API)

**风险**: 需要配置外部服务  
**工作量**: 3 天

---

### 8. 记忆压缩 ⭐⭐⭐
**实现**:
```rust
// 后台任务：压缩旧记忆
async fn compress_old_memories() {
    let old_segments = storage.get_segments_older_than(days=90).await?;
    
    // 合并相似记忆
    let clusters = cluster_similar_segments(old_segments);
    
    for cluster in clusters {
        let summary = llm.summarize_cluster(&cluster).await?;
        
        // 删除原始记忆，保存摘要
        storage.delete_segments(&cluster.ids).await?;
        storage.store_compressed(summary).await?;
    }
}
```

**依赖**: 
- 需要聚类算法 (HDBSCAN 或简单的相似度聚类)

**风险**: 压缩可能丢失细节  
**工作量**: 5 天

---

### 9. 多租户 ⭐⭐
**现有基础**:
- ✅ 已有 `user_id` 隔离
- ✅ 已有 Qdrant 存储

**实现**:
```rust
// 1. API Key 就是 user_id 的哈希
async fn auth_middleware(
    req: Request,
    next: Next
) -> Result<Response> {
    let api_key = req.headers().get("Authorization")?;
    let user_id = hash_api_key(api_key);  // 无需数据库
    
    req.extensions_mut().insert(user_id);
    Ok(next.run(req).await)
}

// 2. 数据已经按 user_id 隔离在 Qdrant
async fn get_memories(user_id: &str) -> Vec<Memory> {
    qdrant.search_segments(user_id, query, limit).await?
    // Qdrant 已经按 user_id 过滤
}
```

**依赖**: 无！直接用 Qdrant 的 `user_id` 字段过滤

**风险**: 无  
**工作量**: 2 天 (只需加认证中间件)

---

## 🔴 高难度 (7-14 天)

### 10. SQLite 迁移 ⭐⭐⭐⭐
**挑战**:
- 需要重写所有存储层
- 需要数据迁移工具
- 需要保持向后兼容

**实现**:
```rust
// 1. 新存储层
pub struct SqliteStorage {
    conn: SqlitePool,
}

impl VectorStorage for SqliteStorage {
    async fn store_segment(&self, segment: MidTermSegment) -> Result<()> {
        sqlx::query!(
            "INSERT INTO segments (id, user_id, content, embedding) VALUES (?, ?, ?, ?)",
            segment.id, segment.user_id, segment.content, segment.embedding
        ).execute(&self.conn).await?;
        Ok(())
    }
}

// 2. 迁移工具
async fn migrate_from_qdrant() {
    let qdrant_data = qdrant.export_all().await?;
    for segment in qdrant_data {
        sqlite.store_segment(segment).await?;
    }
}
```

**依赖**: 
- `sqlx` (SQL 工具)
- `sqlite-vss` (向量扩展)

**风险**: 
- ⚠️ 向量搜索性能可能不如 Qdrant
- ⚠️ 需要大量测试

**工作量**: 10-14 天

**建议**: ⏸️ **暂缓**，Qdrant 已经很好

---

## 📊 推荐实施顺序

### Sprint 1 (Week 1): 快速胜利
1. ✅ 直接命中 (1 天)
2. ✅ 上下文问候 (1 天)
3. ✅ 混合路由 (2 天)

**产出**: 3 个杀手级功能，立即提升用户体验

---

### Sprint 2 (Week 2): 多模态
4. ✅ OCR 集成 (5 天)

**产出**: 图片记忆能力

---

### Sprint 3 (Week 3): 全域摄取
5. ✅ 浏览器插件 API (3 天)
6. ✅ 文件监控 (3 天)

**产出**: 无缝知识捕获

---

### Sprint 4 (Week 4): 企业功能
7. ✅ 多租户 (5 天)
8. ✅ Wiki 导出 (3 天)

**产出**: 企业级部署能力

---

### Sprint 5 (Week 5): 优化
9. ✅ 记忆压缩 (5 天)

**产出**: 成本优化

---

## ⚠️ 不推荐实施

### SQLite 迁移
**原因**:
- Qdrant 性能已经很好
- 迁移成本高，风险大
- ROI 低

**替代方案**: 
- 保持 Qdrant 作为主力
- SQLite 仅用于元数据 (用户信息、配置)

---

## 🎯 总结

### 高可行性 (立即可做)
- ✅ 直接命中
- ✅ 上下文问候
- ✅ 混合路由
- ✅ OCR 集成
- ✅ 文件监控
- ✅ 浏览器插件 API
- ✅ 多租户 (无需额外数据库)

### 中等可行性 (需要规划)
- ⚠️ 记忆压缩 (需要测试)
- ⚠️ Wiki 导出 (需要外部服务)

### 低可行性 (不推荐)
- ❌ SQLite 迁移 (ROI 低)

**建议**: 按 Sprint 1-5 顺序实施，5 周完成 V2.0 核心功能。
