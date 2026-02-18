//! 攻击防御系统 - IP 封禁和限流

use crate::error::{AppError, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// 攻击类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackType {
    /// 认证失败 (5 次/分钟)
    AuthFailure,
    /// API 滥用 (100 次/分钟)
    RateLimit,
    /// Prompt 注入 (3 次/小时)
    PromptInjection,
    /// 数据爬取 (200 次/分钟)
    Scraping,
    /// DDoS (500 次/分钟)
    DDoS,
}

impl AttackType {
    /// 获取检测窗口 (秒)
    pub fn window_seconds(&self) -> u64 {
        match self {
            Self::AuthFailure => 60,
            Self::RateLimit => 60,
            Self::PromptInjection => 3600,
            Self::Scraping => 60,
            Self::DDoS => 60,
        }
    }

    /// 获取阈值
    pub fn threshold(&self) -> usize {
        match self {
            Self::AuthFailure => 5,
            Self::RateLimit => 100,
            Self::PromptInjection => 3,
            Self::Scraping => 200,
            Self::DDoS => 500,
        }
    }

    /// 获取封禁时长 (秒)
    pub fn ban_duration(&self) -> u64 {
        match self {
            Self::AuthFailure => 900,      // 15 分钟
            Self::RateLimit => 300,        // 5 分钟
            Self::PromptInjection => 3600, // 1 小时
            Self::Scraping => 1800,        // 30 分钟
            Self::DDoS => u64::MAX,        // 永久
        }
    }
}

/// 封禁记录
#[derive(Debug, Clone)]
struct BanRecord {
    /// 封禁原因
    reason: AttackType,
    /// 封禁时间
    banned_at: SystemTime,
    /// 封禁次数
    ban_count: u32,
}

/// 请求记录
#[derive(Debug)]
struct RequestRecord {
    timestamps: Vec<SystemTime>,
}

/// IP 防御系统
pub struct IpDefenseSystem {
    /// 黑名单 (永久封禁)
    blacklist: Arc<RwLock<HashMap<IpAddr, BanRecord>>>,
    /// 临时封禁
    temp_bans: Arc<RwLock<HashMap<IpAddr, BanRecord>>>,
    /// 请求记录 (滑动窗口)
    request_records: Arc<RwLock<HashMap<IpAddr, RequestRecord>>>,
    /// 白名单
    whitelist: Arc<RwLock<Vec<IpAddr>>>,
}

impl IpDefenseSystem {
    pub fn new() -> Self {
        Self {
            blacklist: Arc::new(RwLock::new(HashMap::new())),
            temp_bans: Arc::new(RwLock::new(HashMap::new())),
            request_records: Arc::new(RwLock::new(HashMap::new())),
            whitelist: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 检查 IP 是否被封禁
    pub async fn is_banned(&self, ip: IpAddr) -> bool {
        // 检查白名单
        if self.whitelist.read().await.contains(&ip) {
            return false;
        }

        // 检查永久封禁
        if self.blacklist.read().await.contains_key(&ip) {
            return true;
        }

        // 检查临时封禁
        let temp_bans = self.temp_bans.read().await;
        if let Some(record) = temp_bans.get(&ip) {
            let elapsed = SystemTime::now()
                .duration_since(record.banned_at)
                .unwrap_or(Duration::ZERO);

            if elapsed.as_secs() < record.reason.ban_duration() {
                return true;
            }
        }

        false
    }

    /// 记录请求并检查是否超限
    pub async fn check_rate_limit(
        &self,
        ip: IpAddr,
        attack_type: AttackType,
    ) -> Result<()> {
        // 白名单跳过检查
        if self.whitelist.read().await.contains(&ip) {
            return Ok(());
        }

        // 检查是否已封禁
        if self.is_banned(ip).await {
            return Err(AppError::Forbidden("IP banned".to_string()));
        }

        let now = SystemTime::now();
        let window = Duration::from_secs(attack_type.window_seconds());

        let mut records = self.request_records.write().await;
        let record = records.entry(ip).or_insert(RequestRecord {
            timestamps: Vec::new(),
        });

        // 清理过期记录
        record
            .timestamps
            .retain(|t| now.duration_since(*t).unwrap_or(Duration::MAX) < window);

        // 检查是否超限
        if record.timestamps.len() >= attack_type.threshold() {
            // 触发封禁
            self.ban_ip(ip, attack_type).await;
            return Err(AppError::TooManyRequests(format!(
                "Rate limit exceeded: {:?}",
                attack_type
            )));
        }

        // 记录本次请求
        record.timestamps.push(now);

        Ok(())
    }

    /// 封禁 IP
    async fn ban_ip(&self, ip: IpAddr, reason: AttackType) {
        let now = SystemTime::now();

        // 检查是否已有封禁记录
        let mut temp_bans = self.temp_bans.write().await;
        let ban_count = temp_bans
            .get(&ip)
            .map(|r| r.ban_count + 1)
            .unwrap_or(1);

        // 渐进式惩罚: 5 次封禁后永久封禁
        if ban_count >= 5 || reason == AttackType::DDoS {
            drop(temp_bans);
            let mut blacklist = self.blacklist.write().await;
            blacklist.insert(
                ip,
                BanRecord {
                    reason,
                    banned_at: now,
                    ban_count,
                },
            );
            tracing::warn!("IP {} permanently banned (reason: {:?})", ip, reason);
        } else {
            temp_bans.insert(
                ip,
                BanRecord {
                    reason,
                    banned_at: now,
                    ban_count,
                },
            );
            tracing::warn!(
                "IP {} temporarily banned for {} seconds (reason: {:?}, count: {})",
                ip,
                reason.ban_duration(),
                reason,
                ban_count
            );
        }
    }

    /// 添加到白名单
    pub async fn add_whitelist(&self, ip: IpAddr) {
        self.whitelist.write().await.push(ip);
        tracing::info!("IP {} added to whitelist", ip);
    }

    /// 手动解封
    pub async fn unban(&self, ip: IpAddr) -> bool {
        let mut blacklist = self.blacklist.write().await;
        let mut temp_bans = self.temp_bans.write().await;

        let removed = blacklist.remove(&ip).is_some() || temp_bans.remove(&ip).is_some();

        if removed {
            tracing::info!("IP {} unbanned", ip);
        }

        removed
    }

    /// 获取封禁统计
    pub async fn get_stats(&self) -> DefenseStats {
        DefenseStats {
            blacklist_count: self.blacklist.read().await.len(),
            temp_ban_count: self.temp_bans.read().await.len(),
            whitelist_count: self.whitelist.read().await.len(),
        }
    }
}

impl Default for IpDefenseSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// 防御统计
#[derive(Debug, Clone)]
pub struct DefenseStats {
    pub blacklist_count: usize,
    pub temp_ban_count: usize,
    pub whitelist_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_rate_limit() {
        let defense = IpDefenseSystem::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // 前 5 次应该通过
        for _ in 0..5 {
            assert!(defense
                .check_rate_limit(ip, AttackType::AuthFailure)
                .await
                .is_ok());
        }

        // 第 6 次应该被封禁
        assert!(defense
            .check_rate_limit(ip, AttackType::AuthFailure)
            .await
            .is_err());

        // 检查是否被封禁
        assert!(defense.is_banned(ip).await);
    }

    #[tokio::test]
    async fn test_whitelist() {
        let defense = IpDefenseSystem::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        defense.add_whitelist(ip).await;

        // 白名单 IP 不受限制
        for _ in 0..100 {
            assert!(defense
                .check_rate_limit(ip, AttackType::AuthFailure)
                .await
                .is_ok());
        }

        assert!(!defense.is_banned(ip).await);
    }

    #[tokio::test]
    async fn test_ddos_permanent_ban() {
        let defense = IpDefenseSystem::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // DDoS 攻击应该立即永久封禁
        for _ in 0..501 {
            let _ = defense.check_rate_limit(ip, AttackType::DDoS).await;
        }

        let stats = defense.get_stats().await;
        assert_eq!(stats.blacklist_count, 1);
    }
}
