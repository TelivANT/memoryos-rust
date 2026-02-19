//! NATS 短期存储适配器
//! 
//! 作为 Redis 的备选方案，使用 NATS JetStream 提供持久化消息存储

use async_trait::async_trait;
use memoryos_core::{AppError, Message};
use memoryos_ports::ShortTermStorage;
use std::time::Duration;

pub struct NatsStorage {
    client: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    bucket_name: String,
    ttl_seconds: u64,
    max_messages: usize,
}

impl NatsStorage {
    pub async fn new(
        nats_url: &str,
        ttl_seconds: u64,
        max_messages: usize,
    ) -> Result<Self, AppError> {
        // 连接 NATS
        let client = async_nats::connect(nats_url)
            .await
            .map_err(|e| AppError::Config(format!("Failed to connect to NATS: {}", e)))?;

        // 获取 JetStream context
        let jetstream = async_nats::jetstream::new(client.clone());

        let bucket_name = "memoryos_short_term".to_string();

        Ok(Self {
            client,
            jetstream,
            bucket_name,
            ttl_seconds,
            max_messages,
        })
    }

    async fn ensure_bucket(&self) -> Result<async_nats::jetstream::kv::Store, AppError> {
        use async_nats::jetstream::kv::Config;

        let config = Config {
            bucket: self.bucket_name.clone(),
            max_age: Duration::from_secs(self.ttl_seconds),
            ..Default::default()
        };

        match self.jetstream.create_key_value(config).await {
            Ok(store) => Ok(store),
            Err(_) => {
                // Bucket 已存在，获取它
                self.jetstream
                    .get_key_value(&self.bucket_name)
                    .await
                    .map_err(|e| AppError::ExternalService(format!("NATS KV error: {}", e)))
            }
        }
    }
}

#[async_trait]
impl ShortTermStorage for NatsStorage {
    async fn add_message(&self, user_id: &str, message: Message) -> Result<(), AppError> {
        let bucket = self.ensure_bucket().await?;

        // 获取现有消息
        let mut messages: Vec<Message> = match bucket.get(user_id).await {
            Ok(Some(entry)) => serde_json::from_slice(&entry)
                .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?,
            Ok(None) => Vec::new(),
            Err(e) => {
                return Err(AppError::ExternalService(format!("NATS get error: {}", e)))
            }
        };

        // 添加新消息
        messages.push(message);

        // 保持最大消息数
        if messages.len() > self.max_messages {
            messages.drain(0..messages.len() - self.max_messages);
        }

        // 存储
        let data = serde_json::to_vec(&messages)
            .map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))?;

        bucket
            .put(user_id, data.into())
            .await
            .map_err(|e| AppError::ExternalService(format!("NATS put error: {}", e)))?;

        Ok(())
    }

    async fn get_recent(&self, user_id: &str, limit: usize) -> Result<Vec<Message>, AppError> {
        let bucket = self.ensure_bucket().await?;

        let messages: Vec<Message> = match bucket.get(user_id).await {
            Ok(Some(entry)) => serde_json::from_slice(&entry)
                .map_err(|e| AppError::Internal(format!("Deserialization error: {}", e)))?,
            Ok(None) => return Ok(Vec::new()),
            Err(e) => {
                return Err(AppError::ExternalService(format!("NATS get error: {}", e)))
            }
        };

        // 返回最近的 N 条消息
        let start = messages.len().saturating_sub(limit);
        Ok(messages[start..].to_vec())
    }

    async fn clear(&self, user_id: &str) -> Result<(), AppError> {
        let bucket = self.ensure_bucket().await?;

        bucket
            .delete(user_id)
            .await
            .map_err(|e| AppError::ExternalService(format!("NATS delete error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memoryos_core::MessageRole;

    #[tokio::test]
    #[ignore] // 需要运行 NATS 服务器
    async fn test_nats_storage() {
        let storage = NatsStorage::new("nats://localhost:4222", 3600, 20)
            .await
            .unwrap();

        let message = Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        storage.add_message("user1", message).await.unwrap();

        let messages = storage.get_recent("user1", 10).await.unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello");
    }
}
