//! LLM Adapter Port - 定义 LLM 调用的统一接口

use async_trait::async_trait;
use memoryos_core::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// OpenAI 格式的聊天请求
/// 支持透传：保留所有未知字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    /// 保留所有未知字段（如 top_p, frequency_penalty 等）
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI 格式的聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

/// OpenAI 格式的流式响应块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamChunk {
    pub id: String,
    pub object: String,
    pub model: String,
    pub choices: Vec<ChatStreamChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamChoice {
    pub index: u32,
    pub delta: ChatDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// LLM Adapter trait - 所有 LLM 提供商必须实现此接口
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// 发送聊天请求，返回 OpenAI 格式响应
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError>;

    /// 流式聊天请求，返回 SSE 流
    async fn chat_stream(&self, _request: ChatRequest) -> Result<Vec<ChatStreamChunk>, AppError> {
        // 默认实现：不支持流式
        Err(AppError::BadRequest(format!(
            "{} does not support streaming",
            self.name()
        )))
    }

    /// 获取 Adapter 名称（用于日志和监控）
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_serialization() {
        let req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: false,
            extra: HashMap::new(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("Hello"));
        let deserialized: ChatRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, "gpt-4");
        assert_eq!(deserialized.messages.len(), 1);
    }

    #[test]
    fn chat_request_extra_fields_preserved() {
        let json = r#"{
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": false,
            "top_p": 0.9,
            "frequency_penalty": 0.5
        }"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert!(req.extra.contains_key("top_p"));
        assert!(req.extra.contains_key("frequency_penalty"));
    }

    #[test]
    fn chat_request_optional_fields_skipped() {
        let req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
            extra: HashMap::new(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
    }

    #[test]
    fn chat_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }]
        }"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "chatcmpl-123");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, "Hello!");
        assert_eq!(resp.choices[0].finish_reason, "stop");
    }

    #[test]
    fn chat_stream_chunk_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }]
        }"#;
        let chunk: ChatStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
        assert!(chunk.choices[0].finish_reason.is_none());
    }

    #[test]
    fn chat_delta_optional_fields() {
        let delta = ChatDelta {
            role: None,
            content: Some("test".to_string()),
        };
        let json = serde_json::to_string(&delta).unwrap();
        assert!(!json.contains("role"));
        assert!(json.contains("test"));
    }
}
