use async_trait::async_trait;
use memoryos_core::AppError;
use memoryos_ports::{ChatRequest, ChatResponse, ChatStreamChunk, LlmAdapter};
use reqwest::Client;
use serde_json::json;

pub struct MistralAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl MistralAdapter {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
        }
    }
}

#[async_trait]
impl LlmAdapter for MistralAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        let url = format!("{}/chat/completions", self.base_url);
        let payload = json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Mistral request failed: {}", e)))?;

        let chat_response: ChatResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Mistral response parse failed: {}", e))
        })?;

        Ok(chat_response)
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<Vec<ChatStreamChunk>, AppError> {
        let response = self.chat(request).await?;
        Ok(vec![ChatStreamChunk {
            id: response.id,
            object: "chat.completion.chunk".to_string(),
            model: response.model,
            choices: response
                .choices
                .into_iter()
                .map(|c| memoryos_ports::ChatStreamChoice {
                    index: c.index,
                    delta: memoryos_ports::ChatDelta {
                        role: Some(c.message.role),
                        content: Some(c.message.content),
                    },
                    finish_reason: Some(c.finish_reason),
                })
                .collect(),
        }])
    }

    fn name(&self) -> &str {
        "mistral"
    }
}
