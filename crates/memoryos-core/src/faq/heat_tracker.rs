//! FAQ 热度追踪服务

use crate::memory::{MemoryType, MidTermSegment};
use chrono::Utc;

/// 热度计算配置
#[derive(Debug, Clone)]
pub struct HeatConfig {
    /// 访问次数权重
    pub access_weight: f32,
    /// 反馈权重
    pub feedback_weight: f32,
    /// 时间衰减权重
    pub time_decay_weight: f32,
    /// FAQ 提升阈值
    pub promotion_threshold: f32,
    /// 最小访问次数
    pub min_access_count: u32,
}

impl Default for HeatConfig {
    fn default() -> Self {
        Self {
            access_weight: 10.0,
            feedback_weight: 5.0,
            time_decay_weight: 0.5,
            promotion_threshold: 50.0,
            min_access_count: 10,
        }
    }
}

/// 热度追踪服务
pub struct HeatTracker {
    config: HeatConfig,
}

impl HeatTracker {
    pub fn new(config: HeatConfig) -> Self {
        Self {
            config,
        }
    }

    /// 记录访问
    pub fn record_access(&self, segment: &mut MidTermSegment) {
        segment.access_count += 1;
        segment.last_accessed = Some(Utc::now());
        segment.heat_score = self.calculate_heat(segment);
    }

    /// 计算热度分数
    /// heat = (access_count * 10) + (feedback * 5) - (days_since_created * 0.5)
    pub fn calculate_heat(&self, segment: &MidTermSegment) -> f32 {
        let access_score = segment.access_count as f32 * self.config.access_weight;
        
        // 时间衰减
        let days_since_created = (Utc::now() - segment.created_at).num_days() as f32;
        let time_decay = days_since_created * self.config.time_decay_weight;
        
        // 暂时没有反馈系统，feedback = 0
        let feedback_score = 0.0;
        
        (access_score + feedback_score - time_decay).max(0.0)
    }

    /// 检查是否应该提升为 FAQ
    pub fn should_promote(&self, segment: &MidTermSegment) -> bool {
        segment.access_count >= self.config.min_access_count
            && segment.heat_score >= self.config.promotion_threshold
            && segment.memory_type == MemoryType::QA
    }

    /// 提升为 FAQ 候选
    pub fn promote_to_candidate(&self, segment: &mut MidTermSegment) {
        if segment.memory_type == MemoryType::QA {
            segment.memory_type = MemoryType::FaqCandidate;
        }
    }

    /// 提升为正式 FAQ
    pub fn promote_to_faq(&self, segment: &mut MidTermSegment) {
        segment.memory_type = MemoryType::Faq;
    }

    /// 批量检查候选 FAQ
    pub async fn scan_candidates(&self, segments: &mut [MidTermSegment]) -> Vec<uuid::Uuid> {
        let mut promoted = Vec::new();
        
        for segment in segments.iter_mut() {
            if self.should_promote(segment) {
                self.promote_to_candidate(segment);
                promoted.push(segment.id);
            }
        }
        
        promoted
    }

    /// 获取热度统计
    pub fn get_stats(&self, segments: &[MidTermSegment]) -> HeatStats {
        let total = segments.len();
        let qa_count = segments.iter().filter(|s| s.memory_type == MemoryType::QA).count();
        let candidate_count = segments.iter().filter(|s| s.memory_type == MemoryType::FaqCandidate).count();
        let faq_count = segments.iter().filter(|s| s.memory_type == MemoryType::Faq).count();
        
        let avg_heat = if total > 0 {
            segments.iter().map(|s| s.heat_score).sum::<f32>() / total as f32
        } else {
            0.0
        };
        
        let avg_access = if total > 0 {
            segments.iter().map(|s| s.access_count).sum::<u32>() / total as u32
        } else {
            0
        };
        
        HeatStats {
            total,
            qa_count,
            candidate_count,
            faq_count,
            avg_heat_score: avg_heat,
            avg_access_count: avg_access,
        }
    }
}

/// 热度统计
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeatStats {
    pub total: usize,
    pub qa_count: usize,
    pub candidate_count: usize,
    pub faq_count: usize,
    pub avg_heat_score: f32,
    pub avg_access_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_segment(access_count: u32, days_old: i64) -> MidTermSegment {
        MidTermSegment {
            id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            summary: "Test question".to_string(),
            embedding: vec![0.1; 768],
            heat: 0.0,
            created_at: Utc::now() - chrono::Duration::days(days_old),
            access_count,
            heat_score: 0.0,
            last_accessed: None,
            memory_type: MemoryType::QA,
        }
    }

    #[test]
    fn test_heat_calculation() {
        let tracker = HeatTracker::new(HeatConfig::default());
        let segment = create_test_segment(15, 5);
        
        let heat = tracker.calculate_heat(&segment);
        // heat = 15 * 10 - 5 * 0.5 = 150 - 2.5 = 147.5
        assert!((heat - 147.5).abs() < 0.1);
    }

    #[test]
    fn test_should_promote() {
        let tracker = HeatTracker::new(HeatConfig::default());
        let mut segment = create_test_segment(10, 1);
        segment.heat_score = tracker.calculate_heat(&segment);
        
        // 10 * 10 - 1 * 0.5 = 99.5 > 50
        assert!(tracker.should_promote(&segment));
    }

    #[test]
    fn test_promotion() {
        let tracker = HeatTracker::new(HeatConfig::default());
        let mut segment = create_test_segment(10, 1);
        
        assert_eq!(segment.memory_type, MemoryType::QA);
        
        tracker.promote_to_candidate(&mut segment);
        assert_eq!(segment.memory_type, MemoryType::FaqCandidate);
        
        tracker.promote_to_faq(&mut segment);
        assert_eq!(segment.memory_type, MemoryType::Faq);
    }
}
