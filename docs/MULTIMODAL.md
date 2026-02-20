# Multi-Modal Support Guide

**版本**: v0.9.0  
**更新**: 2026-02-20  
**状态**: 🟡 Experimental

> **重要说明**: v0.5.0 起已实现多模态存储与检索（QdrantMultiModalStorage）以及 HTTP API（`/v1/multimodal/*`）。
> 尚未实现：
> - CLIP/Whisper 实际模型集成（图像 embedding、音频转录）
> - 跨模态检索（text→image/image→text）
> - 视频帧提取/摘要

---

## 📋 概述

MemoryOS-Rust 支持多模态记忆，包括文本、图像、音频和视频内容的存储和检索。

## 🎯 支持的模态

### 1. 文本 (Text)
- 标准文本内容
- 完全支持 ✅

### 2. 图像 (Image)
- 图像 URL
- 可选标题
- 图像 embedding（用于相似度搜索）
- 状态: 🟡 实验性

### 3. 音频 (Audio)
- 音频 URL
- 可选转录文本
- 音频 embedding
- 状态: 🟡 实验性

### 4. 视频 (Video)
- 视频 URL
- 可选转录文本
- 可选缩略图
- 状态: 🟡 实验性

---

## 🚀 快速开始

### 数据结构

```rust
use memoryos_core::{MultiModalMessage, MultiModalContent};

// 创建多模态消息
let message = MultiModalMessage {
    role: "user".to_string(),
    contents: vec![
        MultiModalContent::Text {
            content: "Look at this image".to_string(),
        },
        MultiModalContent::Image {
            url: "https://example.com/image.jpg".to_string(),
            caption: Some("A beautiful sunset".to_string()),
            embedding: Some(vec![0.1, 0.2, 0.3, /* ... */]),
        },
    ],
    timestamp: chrono::Utc::now(),
};
```

### 存储多模态消息（已实现）

```rust
use memoryos_ports::MultiModalStorage;

storage.store_multimodal_message(user_id, message).await?;
```

### 搜索多模态消息（已实现）

```rust
let results = storage.search_by_text(user_id, "sunset", 10).await?;
let results = storage.search_by_image(user_id, image_embedding, 10).await?;
let results = storage.search_by_audio(user_id, audio_embedding, 10).await?;
let recent = storage.get_recent_multimodal(user_id, 20).await?;
```

---

## 🔧 实现指南

### 1. 图像 Embedding

使用 CLIP 或类似模型生成图像 embedding：

```rust
// 伪代码
async fn generate_image_embedding(image_url: &str) -> Result<Vec<f32>, Error> {
    // 1. 下载图像
    let image = download_image(image_url).await?;
    
    // 2. 使用 CLIP 模型生成 embedding
    let embedding = clip_model.encode_image(image)?;
    
    Ok(embedding)
}
```

### 2. 音频 Embedding

使用 Whisper + 文本 embedding：

```rust
async fn generate_audio_embedding(audio_url: &str) -> Result<(String, Vec<f32>), Error> {
    // 1. 下载音频
    let audio = download_audio(audio_url).await?;
    
    // 2. 使用 Whisper 转录
    let transcript = whisper_model.transcribe(audio)?;
    
    // 3. 生成文本 embedding
    let embedding = text_model.encode(&transcript)?;
    
    Ok((transcript, embedding))
}
```

### 3. 向量数据库存储

扩展现有的 VectorStorage 实现：

```rust
impl MultiModalStorage for QdrantStorage {
    async fn store_multimodal_message(
        &self,
        user_id: &str,
        message: MultiModalMessage,
    ) -> Result<(), AppError> {
        // 1. 提取文本内容
        let text = message.extract_text();
        
        // 2. 生成文本 embedding
        let text_embedding = self.generate_embedding(&text).await?;
        
        // 3. 存储到 Qdrant
        let point = PointStruct::new(
            uuid::Uuid::now_v7().to_string(),
            text_embedding,
            serde_json::to_value(&message)?,
        );
        
        self.client.upsert_points(collection, vec![point]).await?;
        
        // 4. 如果有图像/音频 embedding，也存储
        for embedding in message.get_embeddings() {
            // 存储到专门的多模态 collection
            // ...
        }
        
        Ok(())
    }
}
```

---

## 📊 API 示例

### HTTP API（已实现）

```bash
# 存储多模态消息
POST /v1/multimodal/store
Content-Type: application/json

{
  "user_id": "user123",
  "message": {
    "role": "user",
    "contents": [
      {
        "type": "text",
        "content": "Check out this photo"
      },
      {
        "type": "image",
        "url": "https://example.com/photo.jpg",
        "caption": "My vacation"
      }
    ]
  }
}

# 文本检索
POST /v1/multimodal/search

# 向量检索（图像/音频 embedding）
POST /v1/multimodal/search/embedding
Content-Type: application/json

{
  "user_id": "user123",
  "embedding": [0.1, 0.2, 0.3, ...],
  "limit": 10
}
```

---

## 🎯 使用场景

### 1. 图像记忆

**场景**: 用户分享照片，系统记住照片内容

```rust
// 用户: "这是我的新车"
let message = MultiModalMessage {
    role: "user".to_string(),
    contents: vec![
        MultiModalContent::Text {
            content: "这是我的新车".to_string(),
        },
        MultiModalContent::Image {
            url: "https://example.com/car.jpg".to_string(),
            caption: Some("红色特斯拉 Model 3".to_string()),
            embedding: Some(car_image_embedding),
        },
    ],
    timestamp: chrono::Utc::now(),
};

// 后续查询: "我的车是什么颜色？"
// 系统可以检索到图像记忆并回答
```

### 2. 音频记忆

**场景**: 语音对话记录

```rust
// 用户发送语音消息
let message = MultiModalMessage {
    role: "user".to_string(),
    contents: vec![
        MultiModalContent::Audio {
            url: "https://example.com/voice.mp3".to_string(),
            transcript: Some("明天下午三点开会".to_string()),
            embedding: Some(audio_embedding),
        },
    ],
    timestamp: chrono::Utc::now(),
};

// 后续查询: "我什么时候开会？"
// 系统可以检索到音频转录并回答
```

### 3. 视频记忆

**场景**: 视频内容记录

```rust
let message = MultiModalMessage {
    role: "user".to_string(),
    contents: vec![
        MultiModalContent::Video {
            url: "https://example.com/tutorial.mp4".to_string(),
            transcript: Some("如何使用 Rust 编程...".to_string()),
            thumbnail: Some("https://example.com/thumb.jpg".to_string()),
        },
    ],
    timestamp: chrono::Utc::now(),
};
```

---

## 🔮 未来计划

### Phase 1: 基础支持 ✅
- ✅ 多模态数据结构 (MultiModalContent enum)
- ✅ extract_text() 和 get_embeddings() 方法
- ✅ MultiModalStorage trait + QdrantMultiModalStorage 实现
- ✅ HTTP 端点: /v1/multimodal/store, /v1/multimodal/search, /v1/multimodal/search/embedding, /v1/multimodal/recent
- ✅ 单元测试覆盖 extract_text/get_embeddings（12 tests）

### Phase 2: 图像支持 🟡
- CLIP embedding 集成
- 图像相似度搜索
- 图像标题生成

### Phase 3: 音频支持 🟡
- Whisper 转录集成
- 音频 embedding
- 语音搜索

### Phase 4: 视频支持 ⏳
- 视频帧提取
- 视频转录
- 视频摘要

### Phase 5: 跨模态检索 ⏳
- 文本查询图像
- 图像查询文本
- 音频查询视频

---

## 📚 相关资源

### 模型推荐

- **图像 Embedding**: CLIP, BLIP
- **音频转录**: Whisper
- **文本 Embedding**: BGE-M3, text-embedding-ada-002

### 外部服务

- **OpenAI CLIP API**: https://platform.openai.com/docs/guides/embeddings
- **Hugging Face**: https://huggingface.co/models

---

## 🐛 已知限制

1. **性能**: 多模态 embedding 生成较慢
2. **存储**: 需要更多存储空间
3. **成本**: 外部 API 调用成本较高
4. **实验性**: 功能仍在开发中

---

## 🤝 贡献

欢迎贡献多模态支持的实现！

- 提交 Issue: 报告问题或建议
- 提交 PR: 实现新功能
- 分享经验: 使用案例和最佳实践

---

**多模态记忆，让 AI 更智能！** 🎨🎵📹
