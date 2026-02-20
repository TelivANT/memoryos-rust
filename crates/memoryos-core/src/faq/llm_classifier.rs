//! LLM-based FAQ classification
//!
//! Uses an LLM adapter to automatically classify FAQ content into categories
//! and generate improved question/answer pairs for better matching.

use serde::{Deserialize, Serialize};

/// A single FAQ classification result produced by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaqClassification {
    pub category: String,
    pub subcategory: Option<String>,
    pub confidence: f32,
    pub suggested_tags: Vec<String>,
    pub canonical_question: Option<String>,
}

/// Configuration for the LLM FAQ classifier.
#[derive(Debug, Clone)]
pub struct LlmClassifierConfig {
    pub categories: Vec<String>,
    pub system_prompt: String,
    pub confidence_threshold: f32,
    pub max_retries: u32,
}

impl Default for LlmClassifierConfig {
    fn default() -> Self {
        Self {
            categories: vec![
                "account".to_string(),
                "billing".to_string(),
                "technical".to_string(),
                "product".to_string(),
                "integration".to_string(),
                "security".to_string(),
                "general".to_string(),
            ],
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            confidence_threshold: 0.6,
            max_retries: 2,
        }
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are an FAQ classifier. Given a question-answer pair, respond with ONLY a JSON object (no markdown fences) containing:
- "category": one of the provided categories
- "subcategory": a more specific label (optional, null if unclear)
- "confidence": 0.0 to 1.0
- "suggested_tags": array of 1-3 short keyword tags
- "canonical_question": a cleaner/canonical version of the question (optional, null if the original is already good)"#;

/// Build the classification prompt for the LLM.
pub fn build_classification_prompt(
    config: &LlmClassifierConfig,
    question: &str,
    answer: &str,
) -> Vec<PromptMessage> {
    let categories_str = config.categories.join(", ");
    let system = format!(
        "{}\n\nAvailable categories: [{}]",
        config.system_prompt, categories_str
    );
    let user = format!("Question: {}\nAnswer: {}", question, answer);
    vec![
        PromptMessage {
            role: "system".to_string(),
            content: system,
        },
        PromptMessage {
            role: "user".to_string(),
            content: user,
        },
    ]
}

/// A chat message used by the classifier prompt builder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}

/// Parse the LLM response text into an `FaqClassification`.
///
/// Tries JSON first, then falls back to a simple heuristic parse.
pub fn parse_classification_response(
    response_text: &str,
    config: &LlmClassifierConfig,
) -> Result<FaqClassification, String> {
    let trimmed = response_text.trim();

    let json_str = if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            &trimmed[start..=end]
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    if let Ok(classification) = serde_json::from_str::<FaqClassification>(json_str) {
        if classification.confidence >= config.confidence_threshold {
            return Ok(classification);
        }
        return Ok(classification);
    }

    let lower = trimmed.to_lowercase();
    for cat in &config.categories {
        if lower.contains(&cat.to_lowercase()) {
            return Ok(FaqClassification {
                category: cat.clone(),
                subcategory: None,
                confidence: 0.5,
                suggested_tags: vec![cat.clone()],
                canonical_question: None,
            });
        }
    }

    Err(format!(
        "Failed to parse classification from LLM response: {}",
        &trimmed[..trimmed.len().min(200)]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LlmClassifierConfig::default();
        assert_eq!(config.categories.len(), 7);
        assert!(config.confidence_threshold > 0.0);
    }

    #[test]
    fn test_build_prompt() {
        let config = LlmClassifierConfig::default();
        let messages = build_classification_prompt(
            &config,
            "How do I reset my password?",
            "Go to settings...",
        );
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("account"));
        assert!(messages[1].content.contains("reset my password"));
    }

    #[test]
    fn test_parse_valid_json() {
        let config = LlmClassifierConfig::default();
        let json = r#"{"category":"account","subcategory":"password","confidence":0.95,"suggested_tags":["password","reset"],"canonical_question":"How do I reset my password?"}"#;
        let result = parse_classification_response(json, &config).unwrap();
        assert_eq!(result.category, "account");
        assert_eq!(result.confidence, 0.95);
        assert_eq!(result.suggested_tags.len(), 2);
    }

    #[test]
    fn test_parse_json_with_markdown_fences() {
        let config = LlmClassifierConfig::default();
        let response = "```json\n{\"category\":\"billing\",\"subcategory\":null,\"confidence\":0.8,\"suggested_tags\":[\"billing\"],\"canonical_question\":null}\n```";
        let result = parse_classification_response(response, &config).unwrap();
        assert_eq!(result.category, "billing");
    }

    #[test]
    fn test_parse_fallback_keyword() {
        let config = LlmClassifierConfig::default();
        let response = "This question is about security and access control.";
        let result = parse_classification_response(response, &config).unwrap();
        assert_eq!(result.category, "security");
        assert_eq!(result.confidence, 0.5);
    }

    #[test]
    fn test_parse_unrecognized() {
        let config = LlmClassifierConfig::default();
        let response = "I cannot determine the category.";
        let result = parse_classification_response(response, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_json_with_extra_text() {
        let config = LlmClassifierConfig::default();
        let response = "Here is the classification:\n{\"category\":\"technical\",\"subcategory\":\"api\",\"confidence\":0.88,\"suggested_tags\":[\"api\",\"integration\"],\"canonical_question\":\"How to use the API?\"}\nHope this helps!";
        let result = parse_classification_response(response, &config).unwrap();
        assert_eq!(result.category, "technical");
        assert_eq!(result.subcategory, Some("api".to_string()));
    }

    #[test]
    fn test_classification_serialization() {
        let c = FaqClassification {
            category: "product".to_string(),
            subcategory: Some("features".to_string()),
            confidence: 0.9,
            suggested_tags: vec!["feature".to_string(), "product".to_string()],
            canonical_question: Some("What features does the product have?".to_string()),
        };
        let json = serde_json::to_string(&c).unwrap();
        let deserialized: FaqClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.category, "product");
        assert_eq!(deserialized.suggested_tags.len(), 2);
    }
}
