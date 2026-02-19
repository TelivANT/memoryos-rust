//! 攻击防御系统 - IP 封禁和限流
//!
//! 架构:
//! - 临时封禁: Redis (TTL 自动过期)
//! - 永久封禁: Qdrant (持久化存储)
//! - 请求记录: Redis ZSET (滑动窗口)

use crate::error::{AppError, Result};
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

/// 攻击类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackType {
    AuthFailure,     // 5 次/分钟
    RateLimit,       // 100 次/分钟
    PromptInjection, // 3 次/小时
    Scraping,        // 200 次/分钟
    DDoS,            // 500 次/分钟
}

impl AttackType {
    pub fn window_seconds(&self) -> u64 {
        match self {
            Self::AuthFailure => 60,
            Self::RateLimit => 60,
            Self::PromptInjection => 3600,
            Self::Scraping => 60,
            Self::DDoS => 60,
        }
    }

    pub fn threshold(&self) -> usize {
        match self {
            Self::AuthFailure => 5,
            Self::RateLimit => 100,
            Self::PromptInjection => 3,
            Self::Scraping => 200,
            Self::DDoS => 500,
        }
    }

    pub fn ban_duration(&self) -> u64 {
        match self {
            Self::AuthFailure => 900,
            Self::RateLimit => 300,
            Self::PromptInjection => 3600,
            Self::Scraping => 1800,
            Self::DDoS => u64::MAX,
        }
    }
}

/// IP 防御系统 (Redis + Qdrant)
pub struct IpDefenseSystem {
    redis_client: redis::Client,
    qdrant_client: std::sync::Arc<qdrant_client::Qdrant>,
    collection_name: String,
}

impl IpDefenseSystem {
    pub fn new(
        redis_url: &str,
        qdrant_client: std::sync::Arc<qdrant_client::Qdrant>,
    ) -> Result<Self> {
        let redis_client = redis::Client::open(redis_url)
            .map_err(|e| AppError::Config(format!("Redis: {}", e)))?;

        Ok(Self {
            redis_client,
            qdrant_client,
            collection_name: "ip_blacklist".to_string(),
        })
    }

    /// 检查永久封禁 (Qdrant)
    pub async fn is_permanently_banned(&self, ip: IpAddr) -> Result<bool> {
        use qdrant_client::qdrant::{Condition, FieldCondition, Filter, Match, SearchPoints};

        let filter = Filter {
            must: vec![Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "ip".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Keyword(
                                ip.to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            }],
            ..Default::default()
        };

        let search = SearchPoints {
            collection_name: self.collection_name.clone(),
            vector: vec![0.0],
            limit: 1,
            filter: Some(filter),
            with_payload: Some(true.into()),
            ..Default::default()
        };

        let results = self
            .qdrant_client
            .search_points(search)
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant: {}", e)))?;

        Ok(!results.result.is_empty())
    }

    /// 检查限流 (Redis 滑动窗口)
    pub async fn check_rate_limit(&self, ip: IpAddr, attack_type: AttackType) -> Result<()> {
        // 1. 检查白名单
        if self.is_whitelisted(ip).await? {
            return Ok(());
        }

        // 2. 检查永久封禁 -> 直接返回 429
        if self.is_permanently_banned(ip).await? {
            return Err(AppError::Forbidden(format!("IP {} banned", ip)));
        }

        // 3. 检查临时封禁 -> 直接返回 429
        if self.is_temporarily_banned(ip).await? {
            return Err(AppError::TooManyRequests(format!("IP {} banned", ip)));
        }

        // 4. 滑动窗口限流
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let window = attack_type.window_seconds();
        let threshold = attack_type.threshold();
        let key = format!("rate:{}:{:?}", ip, attack_type);

        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        // 删除过期记录
        redis::cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg(0)
            .arg(now - window)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        // 获取计数
        let count: usize = redis::cmd("ZCARD")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        // 超限 -> 封禁
        if count >= threshold {
            self.ban_ip(ip, attack_type).await?;
            return Err(AppError::TooManyRequests(format!("{:?}", attack_type)));
        }

        // 记录请求
        redis::cmd("ZADD")
            .arg(&key)
            .arg(now)
            .arg(format!("{}:{}", now, uuid::Uuid::new_v4()))
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(window)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        Ok(())
    }

    /// 封禁 IP (渐进式)
    async fn ban_ip(&self, ip: IpAddr, reason: AttackType) -> Result<()> {
        let ban_count = self.get_ban_count(ip).await?;

        // 5 次后永久封禁
        if ban_count >= 4 || reason == AttackType::DDoS {
            self.add_to_blacklist(ip, reason).await?;
            tracing::warn!("IP {} permanently banned ({:?})", ip, reason);
        } else {
            self.add_temp_ban(ip, reason).await?;
            tracing::warn!(
                "IP {} temp banned ({:?}, count: {})",
                ip,
                reason,
                ban_count + 1
            );
        }

        Ok(())
    }

    /// 永久封禁 -> Qdrant
    async fn add_to_blacklist(&self, ip: IpAddr, reason: AttackType) -> Result<()> {
        use qdrant_client::qdrant::{PointStruct, UpsertPointsBuilder, Value};
        use std::collections::HashMap;

        let mut payload: HashMap<String, Value> = HashMap::new();
        payload.insert("ip".to_string(), Value::from(ip.to_string()));
        payload.insert("reason".to_string(), Value::from(format!("{:?}", reason)));
        payload.insert(
            "banned_at".to_string(),
            Value::from(chrono::Utc::now().to_rfc3339()),
        );

        let point = PointStruct::new(uuid::Uuid::new_v4().to_string(), vec![0.0], payload);

        self.qdrant_client
            .upsert_points(
                UpsertPointsBuilder::new(self.collection_name.clone(), vec![point]).build(),
            )
            .await
            .map_err(|e| AppError::ExternalService(format!("Qdrant: {}", e)))?;

        Ok(())
    }

    /// 临时封禁 -> Redis (TTL)
    async fn add_temp_ban(&self, ip: IpAddr, reason: AttackType) -> Result<()> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        let key = format!("ban:temp:{}", ip);
        let duration = reason.ban_duration();

        redis::cmd("SETEX")
            .arg(&key)
            .arg(duration)
            .arg(format!("{:?}", reason))
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        // 增加计数
        let count_key = format!("ban:count:{}", ip);
        redis::cmd("INCR")
            .arg(&count_key)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        redis::cmd("EXPIRE")
            .arg(&count_key)
            .arg(86400)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        Ok(())
    }

    async fn is_temporarily_banned(&self, ip: IpAddr) -> Result<bool> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        let key = format!("ban:temp:{}", ip);
        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        Ok(exists)
    }

    async fn is_whitelisted(&self, ip: IpAddr) -> Result<bool> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        let exists: bool = redis::cmd("SISMEMBER")
            .arg("whitelist")
            .arg(ip.to_string())
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        Ok(exists)
    }

    async fn get_ban_count(&self, ip: IpAddr) -> Result<u32> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        let key = format!("ban:count:{}", ip);
        let count: Option<u32> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        Ok(count.unwrap_or(0))
    }

    pub async fn add_whitelist(&self, ip: IpAddr) -> Result<()> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        redis::cmd("SADD")
            .arg("whitelist")
            .arg(ip.to_string())
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        tracing::info!("IP {} whitelisted", ip);
        Ok(())
    }

    pub async fn unban(&self, ip: IpAddr) -> Result<()> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        redis::cmd("DEL")
            .arg(format!("ban:temp:{}", ip))
            .arg(format!("ban:count:{}", ip))
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::ExternalService(format!("Redis: {}", e)))?;

        tracing::info!("IP {} unbanned", ip);
        Ok(())
    }
}

pub struct DefenseStats {
    pub blacklist_count: usize,
    pub temp_ban_count: usize,
    pub whitelist_count: usize,
}
