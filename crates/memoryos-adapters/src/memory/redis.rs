//! Redis adapter for short-term memory

use async_trait::async_trait;
use memoryos_core::{AppError, Message};
use memoryos_ports::{ConcurrencyControl, ShortTermStorage};
use redis::{AsyncCommands, Client, Script};
use std::time::Duration;
use tokio::time::timeout;
use tracing::debug;

pub struct RedisStorage {
    client: Client,
    ttl_seconds: usize,
    max_messages: usize,
}

impl RedisStorage {
    pub fn new(redis_url: &str, ttl_seconds: usize, max_messages: usize) -> Result<Self, AppError> {
        let client = Client::open(redis_url)
            .map_err(|e| AppError::Config(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            client,
            ttl_seconds,
            max_messages,
        })
    }

    fn key(&self, user_id: &str) -> String {
        format!("stm:{}", user_id)
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        let mut conn = timeout(
            Duration::from_millis(800),
            self.client.get_multiplexed_async_connection(),
        )
        .await
        .map_err(|_| AppError::ExternalService("Redis connection timeout".to_string()))?
        .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

        timeout(
            Duration::from_millis(800),
            redis::cmd("PING").query_async::<String>(&mut conn),
        )
        .await
        .map_err(|_| AppError::ExternalService("Redis ping timeout".to_string()))?
        .map_err(|e| AppError::ExternalService(format!("Redis ping failed: {}", e)))?;
        Ok(())
    }
}

#[async_trait]
impl ShortTermStorage for RedisStorage {
    async fn add_message(&self, user_id: &str, message: Message) -> Result<(), AppError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

        let key = self.key(user_id);
        let value = serde_json::to_string(&message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;

        debug!("Adding message to Redis: key={}", key);

        // Add to list
        conn.lpush::<_, _, ()>(&key, value)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis lpush failed: {}", e)))?;

        // Trim to max size
        conn.ltrim::<_, ()>(&key, 0, (self.max_messages - 1) as isize)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis ltrim failed: {}", e)))?;

        // Set TTL
        conn.expire::<_, ()>(&key, self.ttl_seconds as i64)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis expire failed: {}", e)))?;

        Ok(())
    }

    async fn get_recent(&self, user_id: &str, limit: usize) -> Result<Vec<Message>, AppError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

        let key = self.key(user_id);
        let values: Vec<String> = conn
            .lrange(&key, 0, (limit - 1) as isize)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis lrange failed: {}", e)))?;

        debug!("Retrieved {} messages from Redis", values.len());

        values
            .into_iter()
            .map(|v| {
                serde_json::from_str(&v).map_err(|e| {
                    AppError::Internal(format!("Failed to deserialize message: {}", e))
                })
            })
            .collect()
    }

    async fn clear(&self, user_id: &str) -> Result<(), AppError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

        let key = self.key(user_id);
        conn.del::<_, ()>(&key)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis del failed: {}", e)))?;

        debug!("Cleared short-term memory for user: {}", user_id);
        Ok(())
    }
}

#[async_trait]
impl ConcurrencyControl for RedisStorage {
    async fn acquire_fencing_lock(
        &self,
        lock_key: &str,
        owner_id: &str,
        ttl_ms: u64,
    ) -> Result<Option<u64>, AppError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

        let fence_key = format!("fence:{}", lock_key);

        let token: u64 = conn
            .incr::<_, _, u64>(&fence_key, 1)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis incr failed: {}", e)))?;
        let lock_value: String = format!("{}:{}", owner_id, token);

        let acquired: bool = conn
            .set_nx(lock_key, &lock_value)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis set_nx failed: {}", e)))?;
        if !acquired {
            return Ok(None);
        }

        conn.pexpire::<_, ()>(lock_key, ttl_ms as i64)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis pexpire failed: {}", e)))?;

        Ok(Some(token))
    }

    async fn renew_fencing_lock(
        &self,
        lock_key: &str,
        owner_id: &str,
        fencing_token: u64,
        ttl_ms: u64,
    ) -> Result<bool, AppError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

        let expected = format!("{}:{}", owner_id, fencing_token);
        let renew_script = Script::new(
            r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
              return redis.call("PEXPIRE", KEYS[1], ARGV[2])
            end
            return 0
            "#,
        );
        let result: i64 = renew_script
            .key(lock_key)
            .arg(expected)
            .arg(ttl_ms as i64)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis lock renew failed: {}", e)))?;
        Ok(result == 1)
    }

    async fn release_fencing_lock(
        &self,
        lock_key: &str,
        owner_id: &str,
        fencing_token: u64,
    ) -> Result<bool, AppError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

        let expected = format!("{}:{}", owner_id, fencing_token);
        let release_script = Script::new(
            r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
              return redis.call("DEL", KEYS[1])
            end
            return 0
            "#,
        );
        let result: i64 = release_script
            .key(lock_key)
            .arg(expected)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis lock release failed: {}", e)))?;
        Ok(result == 1)
    }

    async fn enforce_fencing_version(
        &self,
        version_key: &str,
        fencing_token: u64,
    ) -> Result<bool, AppError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

        let cas_script = Script::new(
            r#"
            local current = redis.call("GET", KEYS[1])
            if (not current) or (tonumber(ARGV[1]) > tonumber(current)) then
              redis.call("SET", KEYS[1], ARGV[1])
              return 1
            end
            return 0
            "#,
        );
        let result: i64 = cas_script
            .key(version_key)
            .arg(fencing_token as i64)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis fencing CAS failed: {}", e)))?;
        Ok(result == 1)
    }

    async fn is_event_processed(&self, event_id: &str) -> Result<bool, AppError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;
        let bucket = format!(
            "processed_events:{}",
            chrono::Utc::now().format("%Y_%m_%d_%H")
        );
        conn.sismember::<_, _, bool>(&bucket, event_id)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis sismember failed: {}", e)))
    }

    async fn mark_event_processed(
        &self,
        event_id: &str,
        ttl_seconds: usize,
    ) -> Result<(), AppError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;
        let bucket = format!(
            "processed_events:{}",
            chrono::Utc::now().format("%Y_%m_%d_%H")
        );
        conn.sadd::<_, _, ()>(&bucket, event_id)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis sadd failed: {}", e)))?;
        conn.expire::<_, ()>(&bucket, ttl_seconds as i64)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis expire failed: {}", e)))?;
        Ok(())
    }
}
