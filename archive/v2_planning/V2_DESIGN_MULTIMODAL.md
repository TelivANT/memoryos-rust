# 多模态记忆 - 设计文档

**功能**: 支持图片、音频、视频、文档记忆  
**优先级**: P0  
**工作量**: 5 天  
**状态**: 设计中

---

## 1. 架构设计

### 1.1 整体架构

```
┌─────────────────────────────────────────────────────────┐
│                    API Gateway                          │
│  POST /memory/image, /memory/audio, /memory/video      │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│              Multimodal Memory Service                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐             │
│  │  Image   │  │  Audio   │  │  Video   │             │
│  │ Processor│  │ Processor│  │ Processor│             │
│  └──────────┘  └──────────┘  └──────────┘             │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│              Embedding & Storage                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐             │
│  │ Vision   │  │  Audio   │  │  Text    │             │
│  │Embedding │  │Embedding │  │Embedding │             │
│  └──────────┘  └──────────┘  └──────────┘             │
│                                                         │
│  ┌──────────────────────────────────────┐             │
│  │         Qdrant Vector DB             │             │
│  │  Collections: images, audio, video   │             │
│  └──────────────────────────────────────┘             │
└─────────────────────────────────────────────────────────┘
```

### 1.2 数据流

```
用户上传 → 格式检测 → 内容提取 → 多模态理解 → 向量化 → 存储
                                    ↓
                              关联文本记忆
```

---

## 2. 核心模块

### 2.1 图片记忆

#### 功能
- OCR 文字提取
- 视觉内容理解（物体、场景、人物）
- 图片描述生成
- 相似图片检索

#### 技术实现

```rust
// crates/memoryos-multimodal/src/image.rs

use image::{DynamicImage, ImageFormat};
use tesseract::Tesseract;

pub struct ImageProcessor {
    ocr: Tesseract,
    vision_api: VisionAPI,
}

impl ImageProcessor {
    /// 处理图片并提取信息
    pub async fn process(&self, image: Vec<u8>) -> Result<ImageMemory> {
        // 1. 解码图片
        let img = image::load_from_memory(&image)?;
        
        // 2. OCR 提取文字
        let text = self.extract_text(&img).await?;
        
        // 3. 视觉理解
        let description = self.vision_api.describe(&image).await?;
        let objects = self.vision_api.detect_objects(&image).await?;
        
        // 4. 生成 embedding
        let embedding = self.vision_api.embed(&image).await?;
        
        Ok(ImageMemory {
            text,
            description,
            objects,
            embedding,
            raw_data: image,
        })
    }
    
    /// OCR 文字提取
    async fn extract_text(&self, img: &DynamicImage) -> Result<String> {
        // 预处理：灰度化、二值化
        let gray = img.grayscale();
        
        // Tesseract OCR
        let text = self.ocr.recognize(&gray)?;
        
        Ok(text)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageMemory {
    pub text: String,              // OCR 提取的文字
    pub description: String,       // 图片描述
    pub objects: Vec<DetectedObject>, // 检测到的物体
    pub embedding: Vec<f32>,       // 视觉 embedding
    pub raw_data: Vec<u8>,         // 原始图片数据
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetectedObject {
    pub label: String,
    pub confidence: f32,
    pub bbox: BoundingBox,
}
```

#### API 集成

**选项 1: OpenAI Vision API**
```rust
async fn call_openai_vision(&self, image: &[u8]) -> Result<VisionResponse> {
    let base64 = base64::encode(image);
    
    let response = self.client
        .post("https://api.openai.com/v1/chat/completions")
        .json(&json!({
            "model": "gpt-4-vision-preview",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "Describe this image in detail"},
                    {"type": "image_url", "image_url": {"url": format!("data:image/jpeg;base64,{}", base64)}}
                ]
            }]
        }))
        .send()
        .await?;
    
    Ok(response.json().await?)
}
```

**选项 2: Claude Vision API**
```rust
async fn call_claude_vision(&self, image: &[u8]) -> Result<VisionResponse> {
    let base64 = base64::encode(image);
    
    let response = self.client
        .post("https://api.anthropic.com/v1/messages")
        .json(&json!({
            "model": "claude-3-opus-20240229",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": base64}},
                    {"type": "text", "text": "What's in this image?"}
                ]
            }]
        }))
        .send()
        .await?;
    
    Ok(response.json().await?)
}
```

#### 存储结构

```rust
// Qdrant collection: "images"
{
    "id": "img_123456",
    "vector": [0.1, 0.2, ...],  // 视觉 embedding
    "payload": {
        "user_id": "user_001",
        "timestamp": "2026-02-18T05:00:00Z",
        "text": "OCR 提取的文字",
        "description": "A cat sitting on a laptop",
        "objects": ["cat", "laptop", "desk"],
        "url": "s3://bucket/images/img_123456.jpg"  // 原图存 S3
    }
}
```

---

### 2.2 音频记忆

#### 功能
- 语音转文字（ASR）
- 说话人识别
- 情感分析
- 音频摘要

#### 技术实现

```rust
// crates/memoryos-multimodal/src/audio.rs

pub struct AudioProcessor {
    whisper: WhisperModel,
    sentiment: SentimentAnalyzer,
}

impl AudioProcessor {
    /// 处理音频
    pub async fn process(&self, audio: Vec<u8>) -> Result<AudioMemory> {
        // 1. 语音转文字
        let transcript = self.transcribe(&audio).await?;
        
        // 2. 说话人识别
        let speakers = self.diarize(&audio).await?;
        
        // 3. 情感分析
        let sentiment = self.sentiment.analyze(&transcript).await?;
        
        // 4. 生成摘要
        let summary = self.summarize(&transcript).await?;
        
        Ok(AudioMemory {
            transcript,
            speakers,
            sentiment,
            summary,
            duration: self.get_duration(&audio)?,
        })
    }
    
    /// Whisper ASR
    async fn transcribe(&self, audio: &[u8]) -> Result<Transcript> {
        // 使用 whisper-rs 或 API
        let result = self.whisper.transcribe(audio)?;
        
        Ok(Transcript {
            text: result.text,
            segments: result.segments,
            language: result.language,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioMemory {
    pub transcript: Transcript,
    pub speakers: Vec<Speaker>,
    pub sentiment: Sentiment,
    pub summary: String,
    pub duration: f32,
}
```

#### API 集成

**选项 1: OpenAI Whisper API**
```rust
async fn call_whisper_api(&self, audio: &[u8]) -> Result<Transcript> {
    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(audio.to_vec()))
        .part("model", multipart::Part::text("whisper-1"));
    
    let response = self.client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .multipart(form)
        .send()
        .await?;
    
    Ok(response.json().await?)
}
```

**选项 2: 本地 Whisper**
```rust
use whisper_rs::{WhisperContext, FullParams};

async fn transcribe_local(&self, audio: &[u8]) -> Result<Transcript> {
    let ctx = WhisperContext::new("models/ggml-base.bin")?;
    let params = FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    
    let result = ctx.full(params, audio)?;
    
    Ok(Transcript {
        text: result.text(),
        segments: result.segments(),
        language: result.language(),
    })
}
```

---

### 2.3 视频记忆

#### 功能
- 关键帧提取
- 字幕提取/生成
- 场景分割
- 视频摘要

#### 技术实现

```rust
// crates/memoryos-multimodal/src/video.rs

pub struct VideoProcessor {
    ffmpeg: FFmpegWrapper,
    image_processor: ImageProcessor,
    audio_processor: AudioProcessor,
}

impl VideoProcessor {
    /// 处理视频
    pub async fn process(&self, video: Vec<u8>) -> Result<VideoMemory> {
        // 1. 提取关键帧
        let keyframes = self.extract_keyframes(&video).await?;
        
        // 2. 提取音频
        let audio = self.extract_audio(&video).await?;
        
        // 3. 处理关键帧（图片）
        let frame_memories = self.process_frames(keyframes).await?;
        
        // 4. 处理音频
        let audio_memory = self.audio_processor.process(audio).await?;
        
        // 5. 生成视频摘要
        let summary = self.generate_summary(&frame_memories, &audio_memory).await?;
        
        Ok(VideoMemory {
            keyframes: frame_memories,
            audio: audio_memory,
            summary,
            duration: self.get_duration(&video)?,
        })
    }
    
    /// FFmpeg 提取关键帧
    async fn extract_keyframes(&self, video: &[u8]) -> Result<Vec<Vec<u8>>> {
        // ffmpeg -i input.mp4 -vf "select='eq(pict_type,I)'" -vsync 0 frame%d.jpg
        self.ffmpeg.run(&[
            "-i", "pipe:0",
            "-vf", "select='eq(pict_type,I)'",
            "-vsync", "0",
            "-f", "image2pipe",
            "pipe:1"
        ], video).await
    }
}
```

---

### 2.4 文档记忆

#### 功能
- PDF/Word/PPT 解析
- 表格提取
- 图表识别
- 文档结构化

#### 技术实现

```rust
// crates/memoryos-multimodal/src/document.rs

pub struct DocumentProcessor {
    pdf_parser: PdfParser,
    docx_parser: DocxParser,
}

impl DocumentProcessor {
    pub async fn process(&self, doc: Vec<u8>, format: DocFormat) -> Result<DocumentMemory> {
        match format {
            DocFormat::PDF => self.process_pdf(doc).await,
            DocFormat::DOCX => self.process_docx(doc).await,
            DocFormat::PPTX => self.process_pptx(doc).await,
        }
    }
    
    async fn process_pdf(&self, pdf: Vec<u8>) -> Result<DocumentMemory> {
        use pdf_extract::extract_text;
        
        let text = extract_text(&pdf)?;
        let images = self.extract_images_from_pdf(&pdf)?;
        let tables = self.extract_tables(&pdf)?;
        
        Ok(DocumentMemory {
            text,
            images,
            tables,
            metadata: self.extract_metadata(&pdf)?,
        })
    }
}
```

---

## 3. 存储设计

### 3.1 Qdrant Collections

```rust
// 为每种模态创建独立 collection
collections = [
    "images",      // 图片记忆
    "audio",       // 音频记忆
    "video",       // 视频记忆
    "documents",   // 文档记忆
]

// 统一检索时跨 collection 搜索
async fn search_multimodal(&self, query: &str) -> Result<Vec<Memory>> {
    let query_embedding = self.embed_text(query).await?;
    
    let mut results = vec![];
    for collection in &self.collections {
        let hits = self.qdrant.search(collection, query_embedding.clone(), 10).await?;
        results.extend(hits);
    }
    
    // 按相似度排序
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    Ok(results)
}
```

### 3.2 对象存储（S3/OSS）

```rust
// 原始文件存 S3，向量存 Qdrant
pub struct MultimodalStorage {
    qdrant: QdrantClient,
    s3: S3Client,
}

impl MultimodalStorage {
    async fn store_image(&self, user_id: &str, image: Vec<u8>) -> Result<String> {
        // 1. 上传到 S3
        let key = format!("images/{}/{}.jpg", user_id, Uuid::new_v4());
        self.s3.put_object(&key, image.clone()).await?;
        
        // 2. 处理并向量化
        let memory = self.image_processor.process(image).await?;
        
        // 3. 存入 Qdrant
        self.qdrant.upsert("images", vec![
            PointStruct {
                id: Uuid::new_v4().to_string(),
                vector: memory.embedding,
                payload: json!({
                    "user_id": user_id,
                    "url": format!("s3://{}", key),
                    "description": memory.description,
                }),
            }
        ]).await?;
        
        Ok(key)
    }
}
```

---

## 4. API 设计

### 4.1 REST API

```rust
// POST /api/v1/memory/image
#[derive(Deserialize)]
struct AddImageRequest {
    user_id: String,
    image: String,  // base64
    description: Option<String>,
}

async fn add_image_memory(
    State(service): State<Arc<MultimodalService>>,
    Json(req): Json<AddImageRequest>,
) -> Result<Json<AddImageResponse>> {
    let image = base64::decode(&req.image)?;
    let memory_id = service.add_image(req.user_id, image).await?;
    
    Ok(Json(AddImageResponse { memory_id }))
}

// POST /api/v1/memory/search
#[derive(Deserialize)]
struct SearchRequest {
    user_id: String,
    query: String,
    modalities: Vec<String>,  // ["image", "audio", "video"]
    limit: usize,
}

async fn search_multimodal(
    State(service): State<Arc<MultimodalService>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>> {
    let results = service.search(
        &req.user_id,
        &req.query,
        &req.modalities,
        req.limit,
    ).await?;
    
    Ok(Json(SearchResponse { results }))
}
```

---

## 5. 依赖清单

### 5.1 Cargo.toml

```toml
[dependencies]
# 图片处理
image = "0.24"
tesseract = "0.13"
base64 = "0.21"

# 音频处理
whisper-rs = "0.10"
hound = "3.5"  # WAV 解析

# 视频处理
ffmpeg-next = "6.0"

# 文档处理
pdf-extract = "0.7"
docx-rs = "0.4"

# HTTP 客户端
reqwest = { version = "0.11", features = ["multipart"] }

# 异步
tokio = { version = "1", features = ["full"] }
```

### 5.2 外部依赖

```bash
# FFmpeg
brew install ffmpeg  # macOS
apt install ffmpeg   # Ubuntu

# Tesseract OCR
brew install tesseract
apt install tesseract-ocr

# Whisper 模型
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
```

---

## 6. 测试计划

### 6.1 单元测试

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_image_ocr() {
        let processor = ImageProcessor::new();
        let image = load_test_image("test_ocr.jpg");
        let result = processor.extract_text(&image).await.unwrap();
        assert!(result.contains("Hello World"));
    }
    
    #[tokio::test]
    async fn test_audio_transcribe() {
        let processor = AudioProcessor::new();
        let audio = load_test_audio("test.wav");
        let result = processor.transcribe(&audio).await.unwrap();
        assert!(!result.text.is_empty());
    }
}
```

### 6.2 集成测试

```rust
#[tokio::test]
async fn test_multimodal_search() {
    let service = setup_test_service().await;
    
    // 添加图片记忆
    let image = load_test_image("cat.jpg");
    service.add_image("user1", image).await.unwrap();
    
    // 搜索
    let results = service.search("user1", "cat", &["image"], 10).await.unwrap();
    assert!(!results.is_empty());
}
```

---

## 7. 性能优化

### 7.1 异步处理

```rust
// 并行处理多个模态
async fn process_video_parallel(&self, video: Vec<u8>) -> Result<VideoMemory> {
    let (keyframes, audio) = tokio::join!(
        self.extract_keyframes(&video),
        self.extract_audio(&video),
    );
    
    // ...
}
```

### 7.2 缓存

```rust
use moka::future::Cache;

pub struct CachedImageProcessor {
    processor: ImageProcessor,
    cache: Cache<String, ImageMemory>,
}

impl CachedImageProcessor {
    async fn process(&self, image: Vec<u8>) -> Result<ImageMemory> {
        let hash = sha256(&image);
        
        if let Some(cached) = self.cache.get(&hash).await {
            return Ok(cached);
        }
        
        let result = self.processor.process(image).await?;
        self.cache.insert(hash, result.clone()).await;
        Ok(result)
    }
}
```

---

## 8. 部署

### 8.1 Docker

```dockerfile
FROM rust:1.93 as builder

# 安装依赖
RUN apt-get update && apt-get install -y \
    ffmpeg \
    tesseract-ocr \
    && rm -rf /var/lib/apt/lists/*

# 构建
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ffmpeg \
    tesseract-ocr \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/memoryos /usr/local/bin/
CMD ["memoryos"]
```

---

## 9. 下一步

1. ✅ 设计文档完成
2. ⏳ 创建 `crates/memoryos-multimodal`
3. ⏳ 实现图片处理器
4. ⏳ 实现音频处理器
5. ⏳ 集成测试

**预计完成时间**: 5 天
