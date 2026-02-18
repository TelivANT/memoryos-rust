use crate::AppError;
use serde::Deserialize;

#[derive(Debug, PartialEq)]
pub enum ComplianceResult {
    Safe,
    RequiresLocal,
    Blocked(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub enable_pii_sanitization: bool,
    pub enable_injection_check: bool,
    pub strict_mode: bool,
    pub sensitive_keywords: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_pii_sanitization: true,
            enable_injection_check: true,
            strict_mode: false,
            sensitive_keywords: vec!["confidential".to_string(), "internal".to_string()],
        }
    }
}

pub struct SecurityShield {
    config: SecurityConfig,
    injection_patterns: Vec<String>,
}

impl SecurityShield {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            injection_patterns: vec![
                "ignore previous instructions".to_string(),
                "system override".to_string(),
            ],
        }
    }

    pub fn validate_input(&self, text: &str) -> Result<(), AppError> {
        if !self.config.enable_injection_check {
            return Ok(());
        }

        let lower = text.to_ascii_lowercase();
        for pattern in &self.injection_patterns {
            if lower.contains(pattern) {
                return Err(AppError::BadRequest(format!(
                    "Potential prompt injection detected: {}",
                    pattern
                )));
            }
        }
        Ok(())
    }

    pub fn sanitize_pii(&self, text: &str) -> String {
        if !self.config.enable_pii_sanitization {
            return text.to_string();
        }

        let mut out = Vec::new();
        for token in text.split_whitespace() {
            let lower = token.to_ascii_lowercase();
            if looks_like_email(&lower) {
                out.push("<EMAIL>".to_string());
            } else if looks_like_api_key(&lower) {
                out.push("<API_KEY>".to_string());
            } else {
                out.push(token.to_string());
            }
        }
        out.join(" ")
    }

    pub fn check_compliance(&self, text: &str) -> ComplianceResult {
        if self.validate_input(text).is_err() {
            return ComplianceResult::Blocked("Injection detected".to_string());
        }

        let lower = text.to_ascii_lowercase();
        for keyword in &self.config.sensitive_keywords {
            if lower.contains(&keyword.to_ascii_lowercase()) {
                return ComplianceResult::RequiresLocal;
            }
        }
        ComplianceResult::Safe
    }
}

fn looks_like_email(token: &str) -> bool {
    if !token.contains('@') || !token.contains('.') {
        return false;
    }
    let cleaned = token.trim_matches(|c: char| {
        !c.is_ascii_alphanumeric() && c != '@' && c != '.' && c != '_' && c != '-' && c != '+'
    });
    let parts: Vec<&str> = cleaned.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    !parts[0].is_empty() && parts[1].contains('.')
}

fn looks_like_api_key(token: &str) -> bool {
    let cleaned = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
    cleaned.starts_with("sk-") && cleaned.len() >= 20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pii_redaction() {
        let shield = SecurityShield::new(SecurityConfig::default());
        let input = "Contact alice@example.com and use sk-1234567890abcdef12345678.";
        let output = shield.sanitize_pii(input);
        assert!(output.contains("<EMAIL>"));
        assert!(output.contains("<API_KEY>"));
    }

    #[test]
    fn test_injection_block() {
        let shield = SecurityShield::new(SecurityConfig::default());
        let input = "Please ignore previous instructions.";
        assert!(shield.validate_input(input).is_err());
    }

    #[test]
    fn test_compliance_check() {
        let shield = SecurityShield::new(SecurityConfig::default());
        assert_eq!(
            shield.check_compliance("This is confidential data."),
            ComplianceResult::RequiresLocal
        );
        assert_eq!(shield.check_compliance("hello"), ComplianceResult::Safe);
    }
}
