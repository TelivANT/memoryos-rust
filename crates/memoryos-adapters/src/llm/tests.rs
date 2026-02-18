use super::*;
use memoryos_ports::LlmAdapter;

#[test]
fn openai_adapter_has_correct_name() {
    let adapter = OpenAiAdapter::new(
        "test-key".to_string(),
        "https://api.openai.com/v1".to_string(),
    );
    assert_eq!(adapter.name(), "openai");
}

#[test]
fn gemini_adapter_has_correct_name() {
    let adapter = GeminiAdapter::new(
        "test-key".to_string(),
        "https://generativelanguage.googleapis.com/v1beta".to_string(),
    );
    assert_eq!(adapter.name(), "gemini");
}

#[test]
fn claude_adapter_has_correct_name() {
    let adapter = ClaudeAdapter::new(
        "test-key".to_string(),
        "https://api.anthropic.com/v1".to_string(),
    );
    assert_eq!(adapter.name(), "claude");
}

#[test]
fn ollama_adapter_has_correct_name() {
    let adapter = OllamaAdapter::new("http://localhost:11434".to_string());
    assert_eq!(adapter.name(), "ollama");
}

#[test]
fn deepseek_adapter_has_correct_name() {
    let adapter = DeepSeekAdapter::new(
        "test-key".to_string(),
        "https://api.deepseek.com/v1".to_string(),
    );
    assert_eq!(adapter.name(), "deepseek");
}

#[test]
fn openrouter_adapter_has_correct_name() {
    let adapter = OpenRouterAdapter::new(
        "test-key".to_string(),
        "https://openrouter.ai/api/v1".to_string(),
    );
    assert_eq!(adapter.name(), "openrouter");
}

#[test]
fn azure_adapter_has_correct_name() {
    let adapter = AzureOpenAiAdapter::new(
        "test-key".to_string(),
        "https://test.openai.azure.com".to_string(),
    );
    assert_eq!(adapter.name(), "azure-openai");
}

#[test]
fn groq_adapter_has_correct_name() {
    let adapter = GroqAdapter::new(
        "test-key".to_string(),
        "https://api.groq.com/openai/v1".to_string(),
    );
    assert_eq!(adapter.name(), "groq");
}

#[test]
fn cohere_adapter_has_correct_name() {
    let adapter = CohereAdapter::new(
        "test-key".to_string(),
        "https://api.cohere.ai/v1".to_string(),
    );
    assert_eq!(adapter.name(), "cohere");
}

#[test]
fn mistral_adapter_has_correct_name() {
    let adapter = MistralAdapter::new(
        "test-key".to_string(),
        "https://api.mistral.ai/v1".to_string(),
    );
    assert_eq!(adapter.name(), "mistral");
}
