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

#[cfg(test)]
mod tests;
