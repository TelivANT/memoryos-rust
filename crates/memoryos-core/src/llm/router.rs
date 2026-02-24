use crate::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RouteTier {
    /// Tier 0: Direct Hit (FAQ) - No LLM needed
    DirectHit,
    /// Tier 1: Local LLM - Cost effective
    Local,
    /// Tier 2: Cloud LLM - High intelligence
    Cloud,
}

#[derive(Debug, Clone)]
pub struct RouteDecision {
    pub tier: RouteTier,
    pub endpoint: Option<String>,
    pub model: String,
    pub reason: String,
    /// If DirectHit, this contains the answer
    pub direct_response: Option<String>,
}

#[derive(Debug)]
pub struct RouterContext {
    pub query: String,
    pub token_count: usize,
    pub global_similarity: f32, // Max similarity score from Global Memory
    pub is_faq_match: bool,     // Is the top match an FAQ?
    pub has_sensitive_keywords: bool,
    /// Pre-fetched FAQ answer content (populated by chat handler when FAQ match found)
    pub faq_answer: Option<String>,
}

#[async_trait]
pub trait ModelRouter: Send + Sync {
    async fn route(&self, ctx: &RouterContext) -> Result<RouteDecision, AppError>;
}

pub struct TieredRouter {
    config: RouterConfig,
    rr_counter: AtomicUsize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouterConfig {
    pub enable: bool,
    pub direct_hit_threshold: f32,
    pub hot_threshold: f32,
    pub max_local_tokens: usize,
    pub local_backends: Vec<String>,
    pub cloud_model: String,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            enable: true,
            direct_hit_threshold: 0.92,
            hot_threshold: 0.85,
            max_local_tokens: 2000,
            local_backends: vec!["http://localhost:11434".to_string()],
            cloud_model: "gpt-4o".to_string(),
        }
    }
}

impl TieredRouter {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            rr_counter: AtomicUsize::new(0),
        }
    }

    fn select_local_backend(&self) -> String {
        if self.config.local_backends.is_empty() {
            return String::new();
        }
        let idx =
            self.rr_counter.fetch_add(1, Ordering::Relaxed) % self.config.local_backends.len();
        self.config.local_backends[idx].clone()
    }
}

#[async_trait]
impl ModelRouter for TieredRouter {
    async fn route(&self, ctx: &RouterContext) -> Result<RouteDecision, AppError> {
        // 0. Compliance Check (Security Layer)
        if ctx.has_sensitive_keywords {
            return Ok(RouteDecision {
                tier: RouteTier::Local,
                endpoint: Some(self.select_local_backend()),
                model: "local-safe".to_string(),
                reason: "Compliance: Sensitive keywords detected".to_string(),
                direct_response: None,
            });
        }

        if !self.config.enable {
            return Ok(RouteDecision {
                tier: RouteTier::Cloud,
                endpoint: None,
                model: self.config.cloud_model.clone(),
                reason: "Router disabled".to_string(),
                direct_response: None,
            });
        }

        // 1. Tier 0: Direct Hit (FAQ)
        if ctx.is_faq_match && ctx.global_similarity >= self.config.direct_hit_threshold {
            return Ok(RouteDecision {
                tier: RouteTier::DirectHit,
                endpoint: None,
                model: "none".to_string(),
                reason: format!("Direct Hit (Score: {:.2})", ctx.global_similarity),
                direct_response: ctx
                    .faq_answer
                    .clone()
                    .or_else(|| Some("FAQ Content Placeholder".to_string())),
            });
        }

        // 2. Tier 1: Local (Hotspot)
        let is_simple = ctx.token_count <= self.config.max_local_tokens;
        let is_known = ctx.global_similarity >= self.config.hot_threshold;

        if is_known && is_simple {
            return Ok(RouteDecision {
                tier: RouteTier::Local,
                endpoint: Some(self.select_local_backend()),
                model: "local-llama".to_string(), // Should come from config
                reason: format!(
                    "Hotspot Match ({:.2}) + Simple Query",
                    ctx.global_similarity
                ),
                direct_response: None,
            });
        }

        // 3. Tier 2: Cloud (Default)
        let reason = if !is_simple {
            "Complexity high (Token count)"
        } else {
            "Cold query (No global match)"
        };

        Ok(RouteDecision {
            tier: RouteTier::Cloud,
            endpoint: None,
            model: self.config.cloud_model.clone(),
            reason: reason.to_string(),
            direct_response: None,
        })
    }
}
