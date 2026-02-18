# 记忆压缩/归档 - 设计文档

**功能**: 自动压缩旧记忆，降低存储成本  
**优先级**: P0  
**工作量**: 3 天  
**成本节省**: 90%

---

## 1. 问题分析

### 1.1 成本问题

```
假设：
- 用户数: 100,000
- 每用户每天: 100 条记忆
- 每条记忆: 1KB 文本 + 1536 维向量 (6KB)
- 总计: 7KB/条

每月成本:
- 存储: 100K × 100 × 30 × 7KB = 2.1TB
- Qdrant 成本: ~$500/月
- 1 年后: ~$6000/月

压缩后:
- 保留 30 天热数据: 2.1TB
- 压缩 30 天前: 10:1 压缩比 → 210GB
- 归档到 S3: $5/月
- 总成本: $505/月 (节省 92%)
```

---

## 2. 架构设计

### 2.1 三层存储

```
┌─────────────────────────────────────────────────────────┐
│                    Hot Storage (0-30 天)                │
│                    Qdrant + Redis                       │
│                    完整数据，快速访问                    │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼ 自动压缩
┌─────────────────────────────────────────────────────────┐
│                   Warm Storage (30-90 天)               │
│                    Qdrant (压缩)                        │
│                    摘要 + 降维向量                       │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼ 自动归档
┌─────────────────────────────────────────────────────────┐
│                   Cold Storage (90+ 天)                 │
│                    S3 / OSS                             │
│                    压缩文件，按需恢复                    │
└─────────────────────────────────────────────────────────┘
```

### 2.2 压缩策略

```rust
pub enum CompressionStrategy {
    /// LLM 摘要（保留关键信息）
    LlmSummary {
        compression_ratio: f32,  // 10:1
    },
    
    /// 向量降维（PCA/UMAP）
    DimensionReduction {
        from_dim: usize,  // 1536
        to_dim: usize,    // 256
    },
    
    /// 去重合并（相似记忆合并）
    Deduplication {
        similarity_threshold: f32,  // 0.95
    },
    
    /// 混合策略
    Hybrid,
}
```

---

## 3. 核心实现

### 3.1 压缩服务

```rust
// crates/memoryos-compression/src/lib.rs

use chrono::{DateTime, Utc, Duration};

pub struct CompressionService {
    qdrant: Arc<QdrantClient>,
    s3: Arc<S3Client>,
    llm: Arc<dyn LlmAdapter>,
    config: CompressionConfig,
}

#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub hot_days: i64,      // 30 天
    pub warm_days: i64,     // 90 天
    pub compression_ratio: f32,  // 10:1
    pub target_dim: usize,  // 256
}

impl CompressionService {
    /// 自动压缩任务（每天运行）
    pub async fn run_daily_compression(&self) -> Result<CompressionReport> {
        let now = Utc::now();
        let hot_cutoff = now - Duration::days(self.config.hot_days);
        let warm_cutoff = now - Duration::days(self.config.warm_days);
        
        // 1. Hot → Warm: 压缩 30 天前的数据
        let compressed = self.compress_to_warm(hot_cutoff).await?;
        
        // 2. Warm → Cold: 归档 90 天前的数据
        let archived = self.archive_to_cold(warm_cutoff).await?;
        
        Ok(CompressionReport {
            compressed_count: compressed,
            archived_count: archived,
            space_saved: self.calculate_space_saved(compressed, archived),
        })
    }
    
    /// 压缩到 Warm 层
    async fn compress_to_warm(&self, cutoff: DateTime<Utc>) -> Result<usize> {
        // 1. 查询需要压缩的记忆
        let memories = self.query_old_memories(cutoff).await?;
        
        let mut compressed_count = 0;
        
        for batch in memories.chunks(100) {
            // 2. LLM 摘要
            let summaries = self.summarize_batch(batch).await?;
            
            // 3. 向量降维
            let reduced_vectors = self.reduce_dimensions(batch).await?;
            
            // 4. 存入 Warm collection
            self.store_compressed(summaries, reduced_vectors).await?;
            
            // 5. 删除原始数据
            self.delete_from_hot(batch).await?;
            
            compressed_count += batch.len();
        }
        
        Ok(compressed_count)
    }
    
    /// LLM 摘要
    async fn summarize_batch(&self, memories: &[Memory]) -> Result<Vec<Summary>> {
        let mut summaries = vec![];
        
        for memory in memories {
            let prompt = format!(
                "Summarize the following conversation in 50 words, keeping key facts:\n\n{}",
                memory.content
            );
            
            let summary = self.llm.chat(ChatRequest {
                messages: vec![Message::user(prompt)],
                model: "gpt-4o-mini".to_string(),
                ..Default::default()
            }).await?;
            
            summaries.push(Summary {
                id: memory.id.clone(),
                original_length: memory.content.len(),
                summary: summary.content,
                compression_ratio: memory.content.len() as f32 / summary.content.len() as f32,
            });
        }
        
        Ok(summaries)
    }
    
    /// 向量降维（PCA）
    async fn reduce_dimensions(&self, memories: &[Memory]) -> Result<Vec<Vec<f32>>> {
        use ndarray::{Array2, s};
        use ndarray_linalg::SVD;
        
        // 1. 构建矩阵 (n × 1536)
        let n = memories.len();
        let mut matrix = Array2::<f32>::zeros((n, 1536));
        for (i, memory) in memories.iter().enumerate() {
            for (j, &val) in memory.embedding.iter().enumerate() {
                matrix[[i, j]] = val;
            }
        }
        
        // 2. SVD 降维
        let (u, s, _vt) = matrix.svd(true, false)?;
        
        // 3. 保留前 256 维
        let reduced = u.unwrap().slice(s![.., ..self.config.target_dim]);
        
        // 4. 转换回 Vec
        let mut result = vec![];
        for i in 0..n {
            let row = reduced.row(i).to_vec();
            result.push(row);
        }
        
        Ok(result)
    }
    
    /// 归档到 S3
    async fn archive_to_cold(&self, cutoff: DateTime<Utc>) -> Result<usize> {
        let memories = self.query_warm_memories(cutoff).await?;
        
        let mut archived_count = 0;
        
        for user_batch in memories.chunks(1000) {
            let user_id = &user_batch[0].user_id;
            
            // 1. 序列化
            let data = serde_json::to_vec(&user_batch)?;
            
            // 2. 压缩（gzip）
            let compressed = self.gzip_compress(&data)?;
            
            // 3. 上传到 S3
            let key = format!(
                "archives/{}/{}.json.gz",
                user_id,
                cutoff.format("%Y%m%d")
            );
            self.s3.put_object(&key, compressed).await?;
            
            // 4. 删除 Warm 数据
            self.delete_from_warm(user_batch).await?;
            
            // 5. 记录归档元数据
            self.record_archive_metadata(user_id, &key, user_batch.len()).await?;
            
            archived_count += user_batch.len();
        }
        
        Ok(archived_count)
    }
    
    /// 恢复归档数据
    pub async fn restore_from_archive(
        &self,
        user_id: &str,
        date_range: (DateTime<Utc>, DateTime<Utc>),
    ) -> Result<Vec<Memory>> {
        // 1. 查询归档元数据
        let archives = self.query_archive_metadata(user_id, date_range).await?;
        
        let mut restored = vec![];
        
        for archive in archives {
            // 2. 从 S3 下载
            let compressed = self.s3.get_object(&archive.key).await?;
            
            // 3. 解压
            let data = self.gzip_decompress(&compressed)?;
            
            // 4. 反序列化
            let memories: Vec<Memory> = serde_json::from_slice(&data)?;
            
            restored.extend(memories);
        }
        
        Ok(restored)
    }
}

#[derive(Debug)]
pub struct CompressionReport {
    pub compressed_count: usize,
    pub archived_count: usize,
    pub space_saved: u64,  // bytes
}

#[derive(Debug)]
pub struct Summary {
    pub id: String,
    pub original_length: usize,
    pub summary: String,
    pub compression_ratio: f32,
}
```

---

## 4. 定时任务

### 4.1 Cron 调度

```rust
// crates/memoryos-compression/src/scheduler.rs

use tokio_cron_scheduler::{JobScheduler, Job};

pub struct CompressionScheduler {
    service: Arc<CompressionService>,
    scheduler: JobScheduler,
}

impl CompressionScheduler {
    pub async fn start(&self) -> Result<()> {
        // 每天凌晨 2 点运行
        let job = Job::new_async("0 0 2 * * *", |_uuid, _l| {
            Box::pin(async move {
                let report = self.service.run_daily_compression().await?;
                info!("Compression completed: {:?}", report);
                Ok(())
            })
        })?;
        
        self.scheduler.add(job).await?;
        self.scheduler.start().await?;
        
        Ok(())
    }
}
```

### 4.2 手动触发

```rust
// API: POST /api/v1/admin/compress
async fn trigger_compression(
    State(service): State<Arc<CompressionService>>,
) -> Result<Json<CompressionReport>> {
    let report = service.run_daily_compression().await?;
    Ok(Json(report))
}
```

---

## 5. 存储结构

### 5.1 Qdrant Collections

```rust
// Hot collection (原始数据)
collection: "memories_hot"
{
    "id": "mem_123",
    "vector": [0.1, 0.2, ...],  // 1536 维
    "payload": {
        "user_id": "user_001",
        "content": "完整对话内容...",  // 1KB
        "timestamp": "2026-02-18T00:00:00Z"
    }
}

// Warm collection (压缩数据)
collection: "memories_warm"
{
    "id": "mem_123",
    "vector": [0.1, 0.2, ...],  // 256 维
    "payload": {
        "user_id": "user_001",
        "summary": "摘要...",  // 100 bytes
        "original_ids": ["mem_123", "mem_124"],  // 合并的原始 ID
        "timestamp": "2026-01-18T00:00:00Z",
        "compression_ratio": 10.5
    }
}
```

### 5.2 S3 结构

```
s3://memoryos-archives/
├── archives/
│   ├── user_001/
│   │   ├── 20260118.json.gz  (1000 条记忆)
│   │   ├── 20260119.json.gz
│   │   └── ...
│   └── user_002/
│       └── ...
└── metadata/
    └── archive_index.json  (归档元数据)
```

---

## 6. 智能压缩

### 6.1 重要性评分

```rust
/// 根据重要性决定是否压缩
async fn calculate_importance(&self, memory: &Memory) -> f32 {
    let mut score = 0.0;
    
    // 1. 访问频率
    score += memory.access_count as f32 * 0.3;
    
    // 2. 情感强度
    if let Some(sentiment) = &memory.sentiment {
        score += sentiment.intensity * 0.2;
    }
    
    // 3. 关联度（被引用次数）
    score += memory.reference_count as f32 * 0.3;
    
    // 4. 用户标记
    if memory.is_pinned {
        score += 10.0;
    }
    
    // 5. 内容长度
    score += (memory.content.len() as f32 / 1000.0) * 0.2;
    
    score
}

async fn compress_to_warm(&self, cutoff: DateTime<Utc>) -> Result<usize> {
    let memories = self.query_old_memories(cutoff).await?;
    
    for memory in memories {
        let importance = self.calculate_importance(&memory).await;
        
        if importance > 5.0 {
            // 重要记忆：保留原始数据，只降维
            self.compress_light(&memory).await?;
        } else {
            // 普通记忆：完整压缩
            self.compress_full(&memory).await?;
        }
    }
    
    Ok(memories.len())
}
```

### 6.2 去重合并

```rust
/// 合并相似记忆
async fn deduplicate(&self, memories: &[Memory]) -> Result<Vec<Memory>> {
    let mut clusters = vec![];
    let mut visited = vec![false; memories.len()];
    
    for i in 0..memories.len() {
        if visited[i] {
            continue;
        }
        
        let mut cluster = vec![memories[i].clone()];
        visited[i] = true;
        
        for j in (i + 1)..memories.len() {
            if visited[j] {
                continue;
            }
            
            let similarity = cosine_similarity(
                &memories[i].embedding,
                &memories[j].embedding,
            );
            
            if similarity > 0.95 {
                cluster.push(memories[j].clone());
                visited[j] = true;
            }
        }
        
        clusters.push(cluster);
    }
    
    // 合并每个 cluster
    let mut merged = vec![];
    for cluster in clusters {
        if cluster.len() == 1 {
            merged.push(cluster[0].clone());
        } else {
            let merged_memory = self.merge_cluster(&cluster).await?;
            merged.push(merged_memory);
        }
    }
    
    Ok(merged)
}
```

---

## 7. 监控和报告

### 7.1 压缩指标

```rust
#[derive(Debug, Serialize)]
pub struct CompressionMetrics {
    pub total_memories: usize,
    pub hot_memories: usize,
    pub warm_memories: usize,
    pub cold_memories: usize,
    pub total_size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub compression_ratio: f32,
    pub cost_savings_usd: f32,
}

impl CompressionService {
    pub async fn get_metrics(&self) -> Result<CompressionMetrics> {
        let hot = self.count_hot_memories().await?;
        let warm = self.count_warm_memories().await?;
        let cold = self.count_cold_memories().await?;
        
        let total_size = hot.size + warm.size + cold.size;
        let compressed_size = warm.compressed_size + cold.compressed_size;
        
        Ok(CompressionMetrics {
            total_memories: hot.count + warm.count + cold.count,
            hot_memories: hot.count,
            warm_memories: warm.count,
            cold_memories: cold.count,
            total_size_bytes: total_size,
            compressed_size_bytes: compressed_size,
            compression_ratio: total_size as f32 / compressed_size as f32,
            cost_savings_usd: self.calculate_cost_savings(total_size, compressed_size),
        })
    }
}
```

### 7.2 Dashboard API

```rust
// GET /api/v1/admin/compression/metrics
async fn get_compression_metrics(
    State(service): State<Arc<CompressionService>>,
) -> Result<Json<CompressionMetrics>> {
    let metrics = service.get_metrics().await?;
    Ok(Json(metrics))
}

// GET /api/v1/admin/compression/history
async fn get_compression_history(
    State(service): State<Arc<CompressionService>>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<Vec<CompressionReport>>> {
    let history = service.get_compression_history(params.days).await?;
    Ok(Json(history))
}
```

---

## 8. 配置

### 8.1 config.toml

```toml
[compression]
enabled = true
hot_days = 30
warm_days = 90
compression_ratio = 10.0
target_dimension = 256

[compression.schedule]
cron = "0 0 2 * * *"  # 每天凌晨 2 点
timezone = "UTC"

[compression.s3]
bucket = "memoryos-archives"
region = "us-east-1"
endpoint = "https://s3.amazonaws.com"

[compression.strategy]
type = "hybrid"  # llm_summary | dimension_reduction | deduplication | hybrid
importance_threshold = 5.0
```

---

## 9. 测试

### 9.1 单元测试

```rust
#[tokio::test]
async fn test_llm_summary() {
    let service = setup_test_service().await;
    let memory = create_test_memory(1000);  // 1KB
    
    let summary = service.summarize_batch(&[memory]).await.unwrap();
    
    assert!(summary[0].summary.len() < 200);  // < 200 bytes
    assert!(summary[0].compression_ratio > 5.0);
}

#[tokio::test]
async fn test_dimension_reduction() {
    let service = setup_test_service().await;
    let memories = create_test_memories(100);
    
    let reduced = service.reduce_dimensions(&memories).await.unwrap();
    
    assert_eq!(reduced[0].len(), 256);  // 1536 → 256
}
```

---

## 10. 部署

### 10.1 环境变量

```bash
# S3 配置
AWS_ACCESS_KEY_ID=xxx
AWS_SECRET_ACCESS_KEY=xxx
AWS_REGION=us-east-1
S3_BUCKET=memoryos-archives

# 压缩配置
COMPRESSION_ENABLED=true
COMPRESSION_HOT_DAYS=30
COMPRESSION_WARM_DAYS=90
```

---

## 11. 成本分析

### 11.1 对比

| 项目 | 无压缩 | 有压缩 | 节省 |
|------|--------|--------|------|
| Qdrant (Hot) | $500/月 | $500/月 | $0 |
| Qdrant (Warm) | - | $50/月 | - |
| S3 (Cold) | - | $5/月 | - |
| **总计** | **$500/月** | **$555/月** | **-$55** |

**等等，怎么更贵了？**

修正：
- 无压缩 1 年后: $500 × 12 = $6000/月
- 有压缩 1 年后: $555/月
- **节省**: $5445/月 (91%)

---

## 12. 下一步

1. ✅ 设计文档完成
2. ⏳ 实现 LLM 摘要
3. ⏳ 实现向量降维
4. ⏳ 实现 S3 归档
5. ⏳ 定时任务

**预计完成时间**: 3 天
