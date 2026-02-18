# Phase 4 剩余任务 (50% → 100%)

**开始时间**: 2026-02-17 18:32  
**目标**: 完成 Phase 4 的剩余 50%  
**预计耗时**: 30-40 分钟

---

## 📋 任务清单

### 1. ✅ 已完成 (50%)
- [x] 速率限制（Rate Limiting）
- [x] Prometheus 指标（Metrics）

### 2. 🔄 待完成 (50%)

#### 2.1 真实流式响应 (20%)
**当前问题**:
- `chat_stream` 实现是收集所有 chunks 后一次性返回
- 没有真正的流式传输（SSE）

**目标**:
- 实现真正的异步流式传输
- 使用 `futures::Stream` 而不是 `Vec<ChatStreamChunk>`
- 支持实时 token-by-token 输出

**文件**:
- `crates/memoryos-adapters/src/llm/openai.rs`
- `crates/memoryos-adapters/src/llm/gemini.rs`
- `crates/memoryos-ports/src/llm.rs`
- `crates/memoryos-gateway/src/routes/chat.rs`

#### 2.2 自动 Consolidation (20%)
**当前问题**:
- STM 满了之后没有自动合并到 MTM
- 代码中有 `TODO: Check if we need to consolidate to mid-term`

**目标**:
- 实现 STM → MTM 自动合并逻辑
- 当 STM 达到容量时触发
- 使用 LLM 总结对话内容
- 存储到 Qdrant 向量数据库

**文件**:
- `crates/memoryos-adapters/src/memory/manager.rs`
- `crates/memoryos-core/src/memory.rs`

#### 2.3 真实 Embedding 生成优化 (10%)
**当前状态**:
- 已实现 OpenAI embeddings API 调用
- 有 fallback 机制

**优化目标**:
- 添加本地 ONNX embedding 支持（可选）
- 添加 embedding 缓存
- 支持批量 embedding 生成

**文件**:
- `crates/memoryos-adapters/src/memory/manager.rs`

---

## 🎯 实现优先级

### P0 - 必须完成
1. **真实流式响应** - 核心功能，用户体验关键
2. **自动 Consolidation** - 记忆系统核心逻辑

### P1 - 建议完成
3. **Embedding 优化** - 性能优化

---

## 📝 实现细节

### 1. 真实流式响应

#### 修改 Trait 定义
```rust
// crates/memoryos-ports/src/llm.rs
use futures::Stream;
use std::pin::Pin;

pub trait LlmAdapter: Send + Sync {
    // 修改返回类型为 Stream
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamChunk, AppError>> + Send>>, AppError>;
}
```

#### OpenAI 实现
```rust
// crates/memoryos-adapters/src/llm/openai.rs
use futures::stream::{Stream, StreamExt};
use eventsource_stream::Eventsource;

async fn chat_stream(&self, request: ChatRequest) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamChunk, AppError>> + Send>>, AppError> {
    let url = format!("{}/chat/completions", self.base_url);
    
    let mut stream_request = request;
    stream_request.stream = true;

    let response = self.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", self.api_key))
        .json(&stream_request)
        .send()
        .await?;

    let stream = response
        .bytes_stream()
        .eventsource()
        .map(|event| {
            match event {
                Ok(event) => {
                    if event.data == "[DONE]" {
                        return None;
                    }
                    serde_json::from_str::<ChatStreamChunk>(&event.data)
                        .map_err(|e| AppError::Internal(e.to_string()))
                        .ok()
                }
                Err(e) => Some(Err(AppError::ExternalService(e.to_string()))),
            }
        })
        .filter_map(|x| async { x });

    Ok(Box::pin(stream))
}
```

#### Gateway 路由
```rust
// crates/memoryos-gateway/src/routes/chat.rs
if request.stream {
    let stream = state.router.route_stream(request).await?;
    
    let sse_stream = stream.map(|result| {
        match result {
            Ok(chunk) => {
                let data = serde_json::to_string(&chunk).unwrap_or_default();
                Ok::<_, Infallible>(Event::default().data(data))
            }
            Err(e) => {
                error!("Stream error: {}", e);
                Ok(Event::default().data("{}"))
            }
        }
    });

    let mut response: Response = Sse::new(sse_stream).into_response();
    apply_degraded_header(&mut response, state.degraded_mode().await);
    Ok(response)
}
```

### 2. 自动 Consolidation

```rust
// crates/memoryos-adapters/src/memory/manager.rs

impl DefaultMemoryManager {
    async fn add_message(&self, user_id: &str, role: &str, content: &str) -> Result<(), AppError> {
        // 1. 添加到 STM
        self.redis.add_to_short_term(user_id, role, content).await?;
        
        // 2. 检查是否需要 consolidate
        let stm_size = self.redis.get_short_term_size(user_id).await?;
        if stm_size >= self.config.short_term_capacity {
            self.consolidate_to_mid_term(user_id).await?;
        }
        
        Ok(())
    }
    
    async fn consolidate_to_mid_term(&self, user_id: &str) -> Result<(), AppError> {
        info!("Consolidating STM to MTM for user: {}", user_id);
        
        // 1. 获取 STM 中的所有消息
        let messages = self.redis.get_short_term_messages(user_id).await?;
        
        // 2. 使用 LLM 总结对话
        let summary = self.summarize_conversation(&messages).await?;
        
        // 3. 生成 embedding
        let embedding = self.generate_embedding(&summary).await?;
        
        // 4. 存储到 Qdrant
        let point_id = uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_DNS,
            format!("{}:mtm:{}", user_id, chrono::Utc::now().timestamp()).as_bytes(),
        );
        
        self.qdrant.store_mid_term(
            user_id,
            &point_id.to_string(),
            &summary,
            embedding,
        ).await?;
        
        // 5. 清空 STM（保留最近 N 条）
        self.redis.trim_short_term(user_id, 5).await?;
        
        info!("Consolidation completed for user: {}", user_id);
        Ok(())
    }
    
    async fn summarize_conversation(&self, messages: &[Message]) -> Result<String, AppError> {
        // 构造总结 prompt
        let conversation = messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        
        let prompt = format!(
            "请总结以下对话的核心内容，提取关键信息和上下文：\n\n{}\n\n总结：",
            conversation
        );
        
        // 调用 LLM（使用简单模型）
        let request = ChatRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "你是一个对话总结助手，擅长提取关键信息。".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(500),
            stream: false,
        };
        
        // 这里需要访问 LLM adapter
        // 简化实现：直接返回拼接的内容
        Ok(conversation)
    }
}
```

### 3. Embedding 优化

```rust
// 添加缓存
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct EmbeddingCache {
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    max_size: usize,
}

impl EmbeddingCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }
    
    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        self.cache.read().await.get(text).cloned()
    }
    
    pub async fn set(&self, text: String, embedding: Vec<f32>) {
        let mut cache = self.cache.write().await;
        if cache.len() >= self.max_size {
            // 简单 LRU：清空一半
            cache.clear();
        }
        cache.insert(text, embedding);
    }
}

// 在 DefaultMemoryManager 中使用
async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AppError> {
    // 1. 检查缓存
    if let Some(cached) = self.embedding_cache.get(text).await {
        return Ok(cached);
    }
    
    // 2. 生成 embedding
    let embedding = self.generate_embedding_impl(text).await?;
    
    // 3. 缓存
    self.embedding_cache.set(text.to_string(), embedding.clone()).await;
    
    Ok(embedding)
}
```

---

## 🧪 测试计划

### 1. 流式响应测试
```bash
curl -N http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Count to 10"}],
    "stream": true
  }'
```

### 2. Consolidation 测试
```bash
# 发送 20+ 条消息，触发 consolidation
for i in {1..25}; do
  curl -X POST http://localhost:8080/v1/memory/add \
    -H "Content-Type: application/json" \
    -d "{\"user_id\": \"test_user\", \"role\": \"user\", \"content\": \"Message $i\"}"
done

# 检查 MTM
curl http://localhost:8080/v1/memory/retrieve?user_id=test_user&query=messages
```

---

## 📊 完成标准

- [ ] 流式响应实时输出（不是批量返回）
- [ ] STM 满时自动触发 consolidation
- [ ] Consolidation 生成有意义的总结
- [ ] Embedding 有缓存机制
- [ ] 所有测试通过
- [ ] 文档更新

---

## 🚀 开始实现

按照以下顺序实现：
1. 流式响应（30 分钟）
2. 自动 Consolidation（20 分钟）
3. Embedding 优化（10 分钟）
4. 测试和文档（10 分钟）

**总计**: 70 分钟
