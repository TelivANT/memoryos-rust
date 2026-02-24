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

pub struct StandardInjector {
    max_context_tokens: usize,
}

impl StandardInjector {
    pub fn new(max_context_tokens: usize) -> Self {
        Self { max_context_tokens }
    }

    fn estimate_tokens(text: &str) -> usize {
        text.len() / 4
    }
}

#[async_trait]
impl ContextInjector for StandardInjector {
    async fn inject(
        &self,
        request: &mut ChatRequest,
        _user_id: &str,
    ) -> Result<InjectionStats, AppError> {
        let mut total_tokens: usize = 0;
        let mut kept_messages = Vec::new();

        for msg in request.messages.iter().rev() {
            let msg_tokens = Self::estimate_tokens(&msg.content);
            if total_tokens + msg_tokens > self.max_context_tokens {
                break;
            }
            total_tokens += msg_tokens;
            kept_messages.push(msg.clone());
        }

        kept_messages.reverse();
        let stm_count = kept_messages.len();
        request.messages = kept_messages;

        Ok(InjectionStats {
            stm_count,
            mtm_count: 0,
            total_tokens,
        })
    }
}
