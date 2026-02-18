use crate::AppError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone)]
pub struct InjectionStats {
    pub stm_count: usize,
    pub mtm_count: usize,
    pub total_tokens: usize,
}

#[async_trait]
pub trait ContextInjector: Send + Sync {
    async fn inject(
        &self,
        request: &mut ChatRequest,
        user_id: &str,
    ) -> Result<InjectionStats, AppError>;
}

/// Minimal placeholder injector.
/// Current implementation keeps request unchanged and reports zero-injection.
pub struct StandardInjector {
    max_context_tokens: usize,
}

impl StandardInjector {
    pub fn new(max_context_tokens: usize) -> Self {
        Self { max_context_tokens }
    }
}

#[async_trait]
impl ContextInjector for StandardInjector {
    async fn inject(
        &self,
        _request: &mut ChatRequest,
        _user_id: &str,
    ) -> Result<InjectionStats, AppError> {
        Ok(InjectionStats {
            stm_count: 0,
            mtm_count: 0,
            total_tokens: self.max_context_tokens.min(0),
        })
    }
}
