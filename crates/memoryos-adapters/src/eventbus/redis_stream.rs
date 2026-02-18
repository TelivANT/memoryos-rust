use async_trait::async_trait;
use memoryos_core::AppError;
use memoryos_ports::EventBus;
use redis::{AsyncCommands, Client};

pub struct RedisStreamEventBus {
    client: Client,
    stream_key: String,
}

impl RedisStreamEventBus {
    pub fn new(redis_url: &str, stream_key: &str) -> Result<Self, AppError> {
        let client = Client::open(redis_url).map_err(|e| {
            AppError::Config(format!("Failed to create Redis stream client: {}", e))
        })?;
        Ok(Self {
            client,
            stream_key: stream_key.to_string(),
        })
    }
}

#[async_trait]
impl EventBus for RedisStreamEventBus {
    async fn publish_chat_log(
        &self,
        event_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), AppError> {
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| AppError::Internal(format!("Failed to serialize event payload: {}", e)))?;

        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

        let fields = [("event_id", event_id), ("payload", payload_str.as_str())];
        conn.xadd::<_, _, _, _, String>(&self.stream_key, "*", &fields)
            .await
            .map(|_| ())
            .map_err(|e| AppError::ExternalService(format!("Redis XADD failed: {}", e)))
    }
}
