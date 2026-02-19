//! OpenRouter Adapter - OpenAI-compatible backend.

use async_trait::async_trait;
use memoryos_core::AppError;
use memoryos_ports::{ChatRequest, ChatResponse, ChatStreamChunk, LlmAdapter};
use reqwest::Client;
use tracing::debug;

pub struct OpenRouterAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenRouterAdapter {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
        }
    }
}

#[async_trait]
impl LlmAdapter for OpenRouterAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        let url = format!("{}/chat/completions", self.base_url);
        debug!("Calling OpenRouter API: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenRouter request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "OpenRouter API error {}: {}",
                status, body
            )));
        }

        response.json::<ChatResponse>().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse OpenRouter response: {}", e))
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<Vec<ChatStreamChunk>, AppError> {
        let url = format!("{}/chat/completions", self.base_url);
        debug!("Calling OpenRouter API (stream)");

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
                AppError::ExternalService(format!("OpenRouter stream request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "OpenRouter API error {}: {}",
                status, body
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to read stream: {}", e)))?;

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
        "openrouter"
    }
}
