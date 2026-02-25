pub mod azure_openai;
pub mod claude;
pub mod cohere;
pub mod deepseek;
pub mod gemini;
pub mod groq;
pub mod mistral;
pub mod ollama;
pub mod openai;
pub mod openrouter;

pub use azure_openai::AzureOpenAiAdapter;
pub use claude::ClaudeAdapter;
pub use cohere::CohereAdapter;
pub use deepseek::DeepSeekAdapter;
pub use gemini::GeminiAdapter;
pub use groq::GroqAdapter;
pub use mistral::MistralAdapter;
pub use ollama::OllamaAdapter;
pub use openai::OpenAiAdapter;
pub use openrouter::OpenRouterAdapter;

/// Default HTTP timeout for LLM API calls (120 seconds).
/// Prevents Tokio tasks from blocking indefinitely on slow LLM responses.
const LLM_HTTP_TIMEOUT_SECS: u64 = 120;

/// Build a reqwest::Client with standard LLM timeout settings.
pub(crate) fn build_llm_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(LLM_HTTP_TIMEOUT_SECS))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

#[cfg(test)]
mod tests;
