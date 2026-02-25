//! Claude Adapter - OpenAI request to Anthropic Messages API.

use async_trait::async_trait;
use memoryos_core::AppError;
use memoryos_ports::{llm::ChatChoice, ChatMessage, ChatRequest, ChatResponse, LlmAdapter};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct ClaudeAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ClaudeAdapter {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: super::build_llm_http_client(),
            api_key,
            base_url,
        }
    }
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    id: String,
    model: String,
    content: Vec<ClaudeContentBlock>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[async_trait]
impl LlmAdapter for ClaudeAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        let mut system_parts = Vec::new();
        let mut messages = Vec::new();

        for msg in request.messages {
            if msg.role == "system" {
                system_parts.push(msg.content);
                continue;
            }
            let role = if msg.role == "assistant" {
                "assistant".to_string()
            } else {
                "user".to_string()
            };
            messages.push(ClaudeMessage {
                role,
                content: msg.content,
            });
        }

        let claude_req = ClaudeRequest {
            model: request.model,
            max_tokens: request.max_tokens.unwrap_or(4096),
            system: if system_parts.is_empty() {
                None
            } else {
                Some(system_parts.join("\n"))
            },
            messages,
        };

        let url = format!("{}/v1/messages", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&claude_req)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Claude request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Claude API error {}: {}",
                status, body
            )));
        }

        let claude_resp: ClaudeResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse Claude response: {}", e))
        })?;

        let content = claude_resp
            .content
            .iter()
            .find(|block| block.block_type == "text")
            .and_then(|block| block.text.clone())
            .unwrap_or_default();

        Ok(ChatResponse {
            id: claude_resp.id,
            object: "chat.completion".to_string(),
            model: claude_resp.model,
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content,
                },
                finish_reason: claude_resp
                    .stop_reason
                    .unwrap_or_else(|| "stop".to_string()),
            }],
            usage: None,
        })
    }

    fn name(&self) -> &str {
        "claude"
    }
}
