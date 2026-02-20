//! FAQ 自动提升服务

use crate::faq::HeatTracker;
use crate::memory::{MemoryType, MidTermSegment};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 提升历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionRecord {
    pub id: Uuid,
    pub memory_id: Uuid,
    pub from_type: MemoryType,
    pub to_type: MemoryType,
    pub reason: String,
    pub heat_score: f32,
    pub access_count: u32,
    pub promoted_at: chrono::DateTime<chrono::Utc>,
}

/// 自动提升配置
#[derive(Debug, Clone)]
pub struct AutoPromotionConfig {
    /// 扫描间隔（秒）
    pub scan_interval_secs: u64,
    /// 每次扫描的最大数量
    pub batch_size: usize,
    /// 是否启用自动提升
    pub enabled: bool,
}

impl Default for AutoPromotionConfig {
    fn default() -> Self {
        Self {
            scan_interval_secs: 3600, // 1 小时
            batch_size: 100,
            enabled: true,
        }
    }
}

/// 自动提升服务
pub struct AutoPromoter {
    config: AutoPromotionConfig,
    heat_tracker: Arc<HeatTracker>,
    history: Arc<RwLock<Vec<PromotionRecord>>>,
}

impl AutoPromoter {
    pub fn new(config: AutoPromotionConfig, heat_tracker: Arc<HeatTracker>) -> Self {
        Self {
            config,
            heat_tracker,
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 扫描并提升候选 FAQ
    pub async fn scan_and_promote(&self, segments: &mut [MidTermSegment]) -> PromotionResult {
        let mut promoted_to_candidate = Vec::new();
        let mut promoted_to_faq = Vec::new();

        for segment in segments.iter_mut() {
            // QA -> FaqCandidate
            if segment.memory_type == MemoryType::QA && self.heat_tracker.should_promote(segment) {
                let record = self.create_record(
                    segment,
                    MemoryType::QA,
                    MemoryType::FaqCandidate,
                    format!(
                        "访问次数: {}, 热度: {:.2}",
                        segment.access_count, segment.heat_score
                    ),
                );

                self.heat_tracker.promote_to_candidate(segment);
                promoted_to_candidate.push(record.clone());
                self.add_history(record).await;
            }
            // FaqCandidate -> Faq (需要更高的阈值或人工审核)
            else if segment.memory_type == MemoryType::FaqCandidate
                && segment.access_count >= 20
                && segment.heat_score >= 100.0
            {
                let record = self.create_record(
                    segment,
                    MemoryType::FaqCandidate,
                    MemoryType::Faq,
                    format!(
                        "高频访问: {}, 热度: {:.2}",
                        segment.access_count, segment.heat_score
                    ),
                );

                self.heat_tracker.promote_to_faq(segment);
                promoted_to_faq.push(record.clone());
                self.add_history(record).await;
            }
        }

        PromotionResult {
            promoted_to_candidate,
            promoted_to_faq,
            scanned_count: segments.len(),
        }
    }

    /// 创建提升记录
    fn create_record(
        &self,
        segment: &MidTermSegment,
        from: MemoryType,
        to: MemoryType,
        reason: String,
    ) -> PromotionRecord {
        PromotionRecord {
            id: Uuid::new_v4(),
            memory_id: segment.id,
            from_type: from,
            to_type: to,
            reason,
            heat_score: segment.heat_score,
            access_count: segment.access_count,
            promoted_at: Utc::now(),
        }
    }

    /// 添加到历史记录
    async fn add_history(&self, record: PromotionRecord) {
        let mut history = self.history.write().await;
        history.push(record);
    }

    /// 获取提升历史
    pub async fn get_history(&self, limit: usize) -> Vec<PromotionRecord> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> PromotionStats {
        let history = self.history.read().await;
        let total = history.len();
        let to_candidate = history
            .iter()
            .filter(|r| r.to_type == MemoryType::FaqCandidate)
            .count();
        let to_faq = history
            .iter()
            .filter(|r| r.to_type == MemoryType::Faq)
            .count();

        PromotionStats {
            total_promotions: total,
            to_candidate_count: to_candidate,
            to_faq_count: to_faq,
        }
    }

    /// 启动后台任务
    pub fn start_background_task<F>(
        self: Arc<Self>,
        mut fetch_segments: F,
    ) -> tokio::task::JoinHandle<()>
    where
        F: FnMut() -> Vec<MidTermSegment> + Send + 'static,
    {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
                self.config.scan_interval_secs,
            ));

            loop {
                interval.tick().await;

                if !self.config.enabled {
                    continue;
                }

                let mut segments = fetch_segments();
                let result = self.scan_and_promote(&mut segments).await;

                tracing::info!(
                    "FAQ 自动提升完成: 扫描 {}, 提升候选 {}, 提升 FAQ {}",
                    result.scanned_count,
                    result.promoted_to_candidate.len(),
                    result.promoted_to_faq.len()
                );
            }
        })
    }
}

/// 提升结果
#[derive(Debug, Clone, Serialize)]
pub struct PromotionResult {
    pub promoted_to_candidate: Vec<PromotionRecord>,
    pub promoted_to_faq: Vec<PromotionRecord>,
    pub scanned_count: usize,
}

/// 提升统计
#[derive(Debug, Clone, Serialize)]
pub struct PromotionStats {
    pub total_promotions: usize,
    pub to_candidate_count: usize,
    pub to_faq_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faq::HeatConfig;

    fn create_test_segment(id: Uuid, access_count: u32, days_old: i64) -> MidTermSegment {
        let mut segment = MidTermSegment {
            id,
            user_id: "test_user".to_string(),
            summary: "Test question".to_string(),
            embedding: vec![0.1; 768],
            heat: 0.0,
            created_at: Utc::now() - chrono::Duration::days(days_old),
            access_count,
            heat_score: 0.0,
            last_accessed: None,
            memory_type: MemoryType::QA,
            version: 1,
            tags: vec![],
            updated_at: None,
            previous_version_id: None,
        };

        let tracker = HeatTracker::new(HeatConfig::default());
        segment.heat_score = tracker.calculate_heat(&segment);
        segment
    }

    #[tokio::test]
    async fn test_auto_promotion() {
        let tracker = Arc::new(HeatTracker::new(HeatConfig::default()));
        let promoter = AutoPromoter::new(AutoPromotionConfig::default(), tracker);

        let mut segments = vec![
            create_test_segment(Uuid::new_v4(), 15, 1), // 应该提升
            create_test_segment(Uuid::new_v4(), 5, 1),  // 不应该提升
        ];

        let result = promoter.scan_and_promote(&mut segments).await;

        assert_eq!(result.promoted_to_candidate.len(), 1);
        assert_eq!(segments[0].memory_type, MemoryType::FaqCandidate);
        assert_eq!(segments[1].memory_type, MemoryType::QA);
    }

    #[tokio::test]
    async fn test_promotion_history() {
        let tracker = Arc::new(HeatTracker::new(HeatConfig::default()));
        let promoter = AutoPromoter::new(AutoPromotionConfig::default(), tracker);

        let mut segments = vec![create_test_segment(Uuid::new_v4(), 15, 1)];

        promoter.scan_and_promote(&mut segments).await;

        let history = promoter.get_history(10).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from_type, MemoryType::QA);
        assert_eq!(history[0].to_type, MemoryType::FaqCandidate);
    }

    #[tokio::test]
    async fn test_promotion_stats() {
        let tracker = Arc::new(HeatTracker::new(HeatConfig::default()));
        let promoter = AutoPromoter::new(AutoPromotionConfig::default(), tracker);

        let mut segments = vec![
            create_test_segment(Uuid::new_v4(), 15, 1),
            create_test_segment(Uuid::new_v4(), 12, 1),
        ];

        promoter.scan_and_promote(&mut segments).await;

        let stats = promoter.get_stats().await;
        assert_eq!(stats.total_promotions, 2);
        assert_eq!(stats.to_candidate_count, 2);
    }
}
