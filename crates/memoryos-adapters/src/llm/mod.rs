pub mod claude;
pub mod deepseek;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod azure_openai;
pub mod groq;
pub mod cohere;
pub mod mistral;

pub use azure_openai::AzureOpenAiAdapter;
pub use claude::ClaudeAdapter;
pub use deepseek::DeepSeekAdapter;
pub use gemini::GeminiAdapter;
pub use ollama::OllamaAdapter;
pub use openai::OpenAiAdapter;
pub use openrouter::OpenRouterAdapter;
pub use groq::GroqAdapter;
pub use cohere::CohereAdapter;
pub use mistral::MistralAdapter;

#[cfg(test)]
mod tests;
