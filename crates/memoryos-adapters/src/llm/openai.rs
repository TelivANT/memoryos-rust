//! OpenAI Adapter - 透传模式调用 OpenAI API

use async_trait::async_trait;
use memoryos_core::{
    retry::{retry_with_backoff, RetryConfig},
    AppError,
};
use memoryos_ports::{ChatRequest, ChatResponse, ChatStreamChunk, LlmAdapter};
use reqwest::Client;
use tracing::debug;

pub struct OpenAiAdapter {
    client: Client,
    api_key: String,
    base_url: String,
    retry_config: RetryConfig,
}

impl OpenAiAdapter {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self::with_retry_config(api_key, base_url, RetryConfig::default())
    }

    pub fn with_retry_config(api_key: String, base_url: String, retry_config: RetryConfig) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
            retry_config,
        }
    }
}

#[async_trait]
impl LlmAdapter for OpenAiAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        retry_with_backoff(&self.retry_config, "openai_chat", || async {
            let url = format!("{}/chat/completions", self.base_url);

            debug!("Calling OpenAI API: {}", url);

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("OpenAI request failed: {}", e)))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();

                // 根据状态码返回不同错误类型
                return Err(match status.as_u16() {
                    429 => AppError::RateLimited("OpenAI rate limit exceeded".to_string()),
                    503 => AppError::ServiceUnavailable("OpenAI service unavailable".to_string()),
                    _ if status.is_server_error() => {
                        AppError::Internal(format!("OpenAI API error {}: {}", status, body))
                    }
                    _ => AppError::BadRequest(format!("OpenAI API error {}: {}", status, body)),
                });
            }

            response
                .json::<ChatResponse>()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to parse OpenAI response: {}", e)))
        })
        .await
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<Vec<ChatStreamChunk>, AppError> {
        let url = format!("{}/chat/completions", self.base_url);

        debug!("Calling OpenAI API (stream)");

        let mut stream_request = request;
        stream_request.stream = true;

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&stream_request)
            .send()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("OpenAI stream request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "OpenAI API error {}: {}",
                status, body
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to read stream: {}", e)))?;

        // 解析 SSE 格式
        let mut chunks = Vec::new();
        for line in body.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }
                if let Ok(chunk) = serde_json::from_str::<ChatStreamChunk>(data) {
                    chunks.push(chunk);
                }
            }
        }

        Ok(chunks)
    }

    fn name(&self) -> &str {
        "openai"
    }
}
