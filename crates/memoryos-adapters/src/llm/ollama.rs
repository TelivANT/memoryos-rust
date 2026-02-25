//! Ollama Adapter - OpenAI-compatible local backend.

use async_trait::async_trait;
use memoryos_core::AppError;
use memoryos_ports::{ChatRequest, ChatResponse, LlmAdapter};
use reqwest::Client;

pub struct OllamaAdapter {
    client: Client,
    base_url: String,
}

impl OllamaAdapter {
    pub fn new(base_url: String) -> Self {
        Self {
            client: super::build_llm_http_client(),
            base_url,
        }
    }
}

#[async_trait]
impl LlmAdapter for OllamaAdapter {
    async fn chat(&self, mut request: ChatRequest) -> Result<ChatResponse, AppError> {
        // keep OpenAI-compatible payload and endpoint.
        let url = format!("{}/chat/completions", self.base_url);
        request.stream = false;

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Ollama request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Ollama API error {}: {}",
                status, body
            )));
        }

        response.json::<ChatResponse>().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse Ollama response: {}", e))
        })
    }

    fn name(&self) -> &str {
        "ollama"
    }
}
