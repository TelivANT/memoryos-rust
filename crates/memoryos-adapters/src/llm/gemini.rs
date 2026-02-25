//! Gemini Adapter - 原生 REST API 调用

use async_trait::async_trait;
use memoryos_core::AppError;
use memoryos_ports::{ChatMessage, ChatRequest, ChatResponse, LlmAdapter};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

pub struct GeminiAdapter {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GeminiAdapter {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: super::build_llm_http_client(),
            api_key,
            base_url,
        }
    }
}

// Gemini 原生请求格式
#[derive(Serialize)]
struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    contents: Vec<GeminiContent>,
}

#[derive(Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

// Gemini 原生响应格式
#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[async_trait]
impl LlmAdapter for GeminiAdapter {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        let model = request.model.clone();
        let mut system_parts = Vec::new();
        let mut contents = Vec::new();
        for msg in request.messages {
            if msg.role == "system" {
                system_parts.push(msg.content);
                continue;
            }
            contents.push(GeminiContent {
                role: if msg.role == "assistant" {
                    "model".to_string()
                } else {
                    "user".to_string()
                },
                parts: vec![GeminiPart { text: msg.content }],
            });
        }

        let gemini_req = GeminiRequest {
            system_instruction: if system_parts.is_empty() {
                None
            } else {
                Some(GeminiSystemInstruction {
                    parts: vec![GeminiPart {
                        text: system_parts.join("\n"),
                    }],
                })
            },
            contents,
        };

        let url = format!("{}/v1beta/models/{}:generateContent", self.base_url, model);
        debug!("Calling Gemini API endpoint");

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&gemini_req)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Gemini request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Gemini API error {}: {}",
                status, body
            )));
        }

        let gemini_resp: GeminiResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse Gemini response: {}", e))
        })?;

        // 转换为 OpenAI 格式
        let content = gemini_resp
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        Ok(ChatResponse {
            id: format!("gemini-{}", uuid::Uuid::now_v7()),
            object: "chat.completion".to_string(),
            model,
            choices: vec![memoryos_ports::llm::ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content,
                },
                finish_reason: gemini_resp
                    .candidates
                    .first()
                    .and_then(|c| c.finish_reason.clone())
                    .unwrap_or_else(|| "stop".to_string()),
            }],
            usage: None,
        })
    }

    fn name(&self) -> &str {
        "gemini"
    }
}
