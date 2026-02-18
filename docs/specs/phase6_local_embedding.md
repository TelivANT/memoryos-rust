# Phase 6 技术方案 - 本地 Embedding

**版本**: v1.0  
**创建时间**: 2026-02-17

---

## 1. 需求分析

### 1.1 当前问题
- 依赖 OpenAI API（成本高、延迟高、需要网络）
- 无法离线使用
- 无法自定义模型

### 1.2 目标
- 支持本地 ONNX 模型
- 支持多种 Embedding 模型
- 自动 fallback 到 OpenAI
- 性能优于 OpenAI API

---

## 2. 技术选型

### 2.1 ONNX Runtime
使用 `ort` crate（ONNX Runtime 的 Rust 绑定）

**优势**:
- 跨平台（CPU/GPU）
- 高性能（优化的推理引擎）
- 支持多种模型格式

**依赖**:
```toml
[dependencies]
ort = "2.0"
tokenizers = "0.15"
ndarray = "0.15"
```

### 2.2 模型选择

| 模型 | 语言 | 维度 | 大小 | 性能 |
|------|------|------|------|------|
| **BAAI/bge-m3** | 中英文 | 1024 | 560MB | ⭐⭐⭐⭐⭐ |
| all-MiniLM-L6-v2 | 英文 | 384 | 90MB | ⭐⭐⭐⭐ |
| text-embedding-3-small | API | 1536 | - | ⭐⭐⭐ |

**推荐**: BGE-M3（中英文支持 + 高质量）

---

## 3. 架构设计

### 3.1 Crate 结构
```
crates/memoryos-embedding/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── provider.rs      # Trait 定义
│   ├── onnx.rs          # ONNX 实现
│   ├── openai.rs        # OpenAI 实现
│   ├── cache.rs         # 缓存层
│   ├── models.rs        # 模型管理
│   └── tokenizer.rs     # 分词器
└── models/              # 模型文件目录
    ├── bge-m3/
    │   ├── model.onnx
    │   └── tokenizer.json
    └── all-minilm-l6-v2/
        ├── model.onnx
        └── tokenizer.json
```

### 3.2 Trait 定义
```rust
// crates/memoryos-embedding/src/provider.rs
use async_trait::async_trait;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// 生成单个文本的 embedding
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    
    /// 批量生成 embeddings
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
    
    /// 获取 embedding 维度
    fn dimension(&self) -> usize;
    
    /// 获取 provider 名称
    fn name(&self) -> &str;
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    
    #[error("Tokenization failed: {0}")]
    TokenizationFailed(String),
}
```

---

## 4. ONNX 实现

### 4.1 核心代码
```rust
// crates/memoryos-embedding/src/onnx.rs
use ort::{Environment, Session, SessionBuilder, Value};
use tokenizers::Tokenizer;
use ndarray::{Array2, ArrayView2};

pub struct OnnxEmbeddingProvider {
    session: Session,
    tokenizer: Tokenizer,
    dimension: usize,
    max_length: usize,
}

impl OnnxEmbeddingProvider {
    pub fn new(model_path: &str, tokenizer_path: &str) -> Result<Self, EmbeddingError> {
        // 初始化 ONNX Runtime
        let environment = Environment::builder()
            .with_name("memoryos-embedding")
            .build()
            .map_err(|e| EmbeddingError::InferenceFailed(e.to_string()))?;
        
        // 加载模型
        let session = SessionBuilder::new(&environment)
            .map_err(|e| EmbeddingError::InferenceFailed(e.to_string()))?
            .with_model_from_file(model_path)
            .map_err(|e| EmbeddingError::ModelNotFound(e.to_string()))?;
        
        // 加载分词器
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| EmbeddingError::TokenizationFailed(e.to_string()))?;
        
        // 获取模型输出维度
        let dimension = 1024; // BGE-M3
        
        Ok(Self {
            session,
            tokenizer,
            dimension,
            max_length: 512,
        })
    }
    
    fn tokenize(&self, text: &str) -> Result<(Vec<i64>, Vec<i64>), EmbeddingError> {
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| EmbeddingError::TokenizationFailed(e.to_string()))?;
        
        let input_ids = encoding.get_ids()
            .iter()
            .map(|&id| id as i64)
            .collect::<Vec<_>>();
        
        let attention_mask = encoding.get_attention_mask()
            .iter()
            .map(|&mask| mask as i64)
            .collect::<Vec<_>>();
        
        Ok((input_ids, attention_mask))
    }
    
    fn run_inference(
        &self,
        input_ids: &[i64],
        attention_mask: &[i64],
    ) -> Result<Vec<f32>, EmbeddingError> {
        let batch_size = 1;
        let seq_length = input_ids.len();
        
        // 创建输入张量
        let input_ids_array = Array2::from_shape_vec(
            (batch_size, seq_length),
            input_ids.to_vec(),
        ).map_err(|e| EmbeddingError::InferenceFailed(e.to_string()))?;
        
        let attention_mask_array = Array2::from_shape_vec(
            (batch_size, seq_length),
            attention_mask.to_vec(),
        ).map_err(|e| EmbeddingError::InferenceFailed(e.to_string()))?;
        
        // 运行推理
        let outputs = self.session
            .run(vec![
                Value::from_array(self.session.allocator(), &input_ids_array)?,
                Value::from_array(self.session.allocator(), &attention_mask_array)?,
            ])
            .map_err(|e| EmbeddingError::InferenceFailed(e.to_string()))?;
        
        // 提取 embedding（取 [CLS] token 的输出）
        let embedding_tensor = outputs[0]
            .try_extract::<f32>()
            .map_err(|e| EmbeddingError::InferenceFailed(e.to_string()))?;
        
        let embedding = embedding_tensor
            .view()
            .slice(s![0, 0, ..])
            .to_vec();
        
        Ok(embedding)
    }
}

#[async_trait]
impl EmbeddingProvider for OnnxEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        // 分词
        let (input_ids, attention_mask) = self.tokenize(text)?;
        
        // 推理（在 blocking 线程池中运行）
        let session = self.session.clone();
        let embedding = tokio::task::spawn_blocking(move || {
            self.run_inference(&input_ids, &attention_mask)
        })
        .await
        .map_err(|e| EmbeddingError::InferenceFailed(e.to_string()))??;
        
        Ok(embedding)
    }
    
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        // 简化版：逐个处理（TODO: 真正的批处理）
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }
    
    fn dimension(&self) -> usize {
        self.dimension
    }
    
    fn name(&self) -> &str {
        "onnx-bge-m3"
    }
}
```

### 4.2 模型下载和缓存
```rust
// crates/memoryos-embedding/src/models.rs
use std::path::PathBuf;

pub struct ModelManager {
    cache_dir: PathBuf,
}

impl ModelManager {
    pub fn new(cache_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&cache_dir).ok();
        Self { cache_dir }
    }
    
    pub async fn download_model(&self, model_name: &str) -> Result<PathBuf, EmbeddingError> {
        let model_dir = self.cache_dir.join(model_name);
        
        // 如果已存在，直接返回
        if model_dir.exists() {
            return Ok(model_dir);
        }
        
        // 下载模型（从 Hugging Face）
        let url = format!(
            "https://huggingface.co/{}/resolve/main/onnx/model.onnx",
            model_name
        );
        
        info!("Downloading model from {}", url);
        
        let response = reqwest::get(&url)
            .await
            .map_err(|e| EmbeddingError::ModelNotFound(e.to_string()))?;
        
        let bytes = response.bytes()
            .await
            .map_err(|e| EmbeddingError::ModelNotFound(e.to_string()))?;
        
        // 保存到缓存目录
        std::fs::create_dir_all(&model_dir)?;
        let model_path = model_dir.join("model.onnx");
        std::fs::write(&model_path, bytes)?;
        
        info!("Model downloaded to {:?}", model_path);
        
        Ok(model_dir)
    }
}
```

---

## 5. Fallback 机制

### 5.1 智能 Provider
```rust
// crates/memoryos-embedding/src/lib.rs
pub struct SmartEmbeddingProvider {
    primary: Box<dyn EmbeddingProvider>,
    fallback: Option<Box<dyn EmbeddingProvider>>,
}

impl SmartEmbeddingProvider {
    pub fn new(
        primary: Box<dyn EmbeddingProvider>,
        fallback: Option<Box<dyn EmbeddingProvider>>,
    ) -> Self {
        Self { primary, fallback }
    }
}

#[async_trait]
impl EmbeddingProvider for SmartEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        // 尝试主 provider
        match self.primary.embed(text).await {
            Ok(embedding) => Ok(embedding),
            Err(e) => {
                warn!("Primary embedding failed: {}, trying fallback", e);
                
                // 尝试 fallback
                if let Some(fallback) = &self.fallback {
                    fallback.embed(text).await
                } else {
                    Err(e)
                }
            }
        }
    }
    
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        match self.primary.embed_batch(texts).await {
            Ok(embeddings) => Ok(embeddings),
            Err(e) => {
                warn!("Primary batch embedding failed: {}, trying fallback", e);
                
                if let Some(fallback) = &self.fallback {
                    fallback.embed_batch(texts).await
                } else {
                    Err(e)
                }
            }
        }
    }
    
    fn dimension(&self) -> usize {
        self.primary.dimension()
    }
    
    fn name(&self) -> &str {
        self.primary.name()
    }
}
```

### 5.2 配置
```toml
[embedding]
# 主 provider
provider = "onnx"  # onnx | openai
model = "BAAI/bge-m3"
model_cache_dir = "./models"

# Fallback provider
fallback_enabled = true
fallback_provider = "openai"
fallback_model = "text-embedding-3-small"

# 性能配置
batch_size = 32
max_length = 512
```

---

## 6. 性能优化

### 6.1 批处理
```rust
async fn embed_batch_optimized(
    &self,
    texts: &[&str],
) -> Result<Vec<Vec<f32>>, EmbeddingError> {
    let batch_size = 32;
    let mut all_embeddings = Vec::new();
    
    for chunk in texts.chunks(batch_size) {
        // 并行分词
        let tokenized: Vec<_> = chunk.iter()
            .map(|text| self.tokenize(text))
            .collect::<Result<_, _>>()?;
        
        // 批量推理
        let embeddings = self.run_batch_inference(&tokenized).await?;
        all_embeddings.extend(embeddings);
    }
    
    Ok(all_embeddings)
}
```

### 6.2 GPU 加速
```rust
let session = SessionBuilder::new(&environment)?
    .with_execution_providers([
        ExecutionProvider::CUDA(CUDAExecutionProvider::default()),
        ExecutionProvider::CPU(CPUExecutionProvider::default()),
    ])?
    .with_model_from_file(model_path)?;
```

---

## 7. 集成到 MemoryManager

```rust
// crates/memoryos-adapters/src/memory/manager.rs
use memoryos_embedding::{EmbeddingProvider, SmartEmbeddingProvider, OnnxEmbeddingProvider, OpenAiEmbeddingProvider};

pub struct DefaultMemoryManager {
    // ...
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl DefaultMemoryManager {
    pub fn new_with_embedding(
        short_term: Arc<dyn ShortTermStorage>,
        vector_store: Arc<dyn VectorStorage>,
        llm: Arc<dyn LlmAdapter>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            short_term,
            vector_store,
            _llm: llm,
            embedding_provider,
            // ...
        }
    }
    
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AppError> {
        // 使用新的 embedding provider
        self.embedding_provider
            .embed(text)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }
}
```

---

## 8. 测试和验证

### 8.1 单元测试
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_onnx_embedding() {
        let provider = OnnxEmbeddingProvider::new(
            "models/bge-m3/model.onnx",
            "models/bge-m3/tokenizer.json",
        ).unwrap();
        
        let embedding = provider.embed("Hello world").await.unwrap();
        
        assert_eq!(embedding.len(), 1024);
        assert!(embedding.iter().any(|&x| x != 0.0));
    }
    
    #[tokio::test]
    async fn test_fallback() {
        let primary = Box::new(FailingProvider);
        let fallback = Box::new(OpenAiEmbeddingProvider::new(...));
        
        let provider = SmartEmbeddingProvider::new(primary, Some(fallback));
        
        let embedding = provider.embed("test").await.unwrap();
        assert!(!embedding.is_empty());
    }
}
```

### 8.2 性能基准
```rust
#[bench]
fn bench_onnx_embedding(b: &mut Bencher) {
    let provider = OnnxEmbeddingProvider::new(...).unwrap();
    
    b.iter(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(provider.embed("test text"))
    });
}
```

**目标**:
- ONNX 延迟 < 50ms
- OpenAI 延迟 ~200ms
- 批处理吞吐 > 100 texts/s

---

## 9. 实施计划

### Day 1: 基础架构
- [ ] 创建 `memoryos-embedding` crate
- [ ] 定义 `EmbeddingProvider` trait
- [ ] 实现 OpenAI provider（迁移现有代码）

### Day 2: ONNX 集成
- [ ] 添加 `ort` 依赖
- [ ] 实现 `OnnxEmbeddingProvider`
- [ ] 实现模型下载和缓存

### Day 3: Fallback 和优化
- [ ] 实现 `SmartEmbeddingProvider`
- [ ] 添加批处理支持
- [ ] 性能优化

### Day 4: 集成和测试
- [ ] 集成到 `DefaultMemoryManager`
- [ ] 编写单元测试
- [ ] 性能基准测试

---

## 10. 验收标准

- [ ] ONNX embedding 延迟 < 50ms
- [ ] Fallback 机制正常工作
- [ ] 批处理性能 > 100 texts/s
- [ ] 所有测试通过
- [ ] 文档完善
