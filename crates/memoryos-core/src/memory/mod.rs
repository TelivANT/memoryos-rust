//! Memory data structures

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod graph;
pub use graph::{
    EntityType, ExtractedTriple, GraphEntity, GraphManager, GraphQueryResult, GraphRelation,
};

pub mod multimodal;
pub use multimodal::{MultiModalContent, MultiModalMessage};

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Embedding for vector search (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

/// Short-term memory (最近 N 轮对话)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemory {
    pub user_id: String,
    pub messages: Vec<Message>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Mid-term memory segment (对话片段)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidTermSegment {
    pub id: Uuid,
    pub user_id: String,
    pub summary: String,
    pub embedding: Vec<f32>,
    pub heat: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[serde(default)]
    pub tenant_id: Option<String>,

    #[serde(default)]
    pub access_count: u32,
    #[serde(default)]
    pub heat_score: f32,
    #[serde(default)]
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub memory_type: MemoryType,

    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub previous_version_id: Option<Uuid>,
    #[serde(default)]
    pub score: Option<f32>,
}

/// Long-term memory (用户画像、知识、图谱)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemory {
    pub user_id: String,
    pub profile: UserProfile,
    pub knowledge: Vec<KnowledgeItem>,
    /// 新增：图谱记忆
    #[serde(default)]
    pub graph: Option<GraphMemory>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub traits: Vec<String>,
    pub preferences: Vec<String>,
    pub background: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: Uuid,
    pub content: String,
    pub embedding: Vec<f32>,
    pub source: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 图谱记忆 (基于 Mermaid)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMemory {
    /// 原始 Mermaid 代码 (for visualization)
    pub mermaid_code: String,
    /// 解析后的实体列表 (for retrieval)
    pub entities: Vec<GraphEntity>,
    /// 解析后的关系列表 (for retrieval)
    // pub relations: Vec<GraphRelation>, // Relation is now embedded in Entity in graph.rs
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 记忆类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum MemoryType {
    /// 普通问答
    #[serde(rename = "qa")]
    #[default]
    QA,
    /// FAQ 候选
    #[serde(rename = "faq_candidate")]
    FaqCandidate,
    /// 正式 FAQ
    #[serde(rename = "faq")]
    Faq,
}

/// Memory retrieval result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContext {
    pub short_term: Vec<Message>,
    pub mid_term: Vec<MidTermSegment>,
    pub long_term: Option<LongTermMemory>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn memory_type_default_is_qa() {
        assert_eq!(MemoryType::default(), MemoryType::QA);
    }

    #[test]
    fn memory_type_serialization() {
        assert_eq!(serde_json::to_string(&MemoryType::QA).unwrap(), "\"qa\"");
        assert_eq!(
            serde_json::to_string(&MemoryType::FaqCandidate).unwrap(),
            "\"faq_candidate\""
        );
        assert_eq!(serde_json::to_string(&MemoryType::Faq).unwrap(), "\"faq\"");
    }

    #[test]
    fn memory_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<MemoryType>("\"qa\"").unwrap(),
            MemoryType::QA
        );
        assert_eq!(
            serde_json::from_str::<MemoryType>("\"faq_candidate\"").unwrap(),
            MemoryType::FaqCandidate
        );
        assert_eq!(
            serde_json::from_str::<MemoryType>("\"faq\"").unwrap(),
            MemoryType::Faq
        );
    }

    #[test]
    fn message_serialization_roundtrip() {
        let msg = Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
            timestamp: Utc::now(),
            embedding: Some(vec![0.1, 0.2, 0.3]),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role, "user");
        assert_eq!(deserialized.content, "Hello");
        assert_eq!(deserialized.embedding.unwrap().len(), 3);
    }

    #[test]
    fn message_without_embedding() {
        let msg = Message {
            role: "assistant".to_string(),
            content: "Hi there".to_string(),
            timestamp: Utc::now(),
            embedding: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        // embedding should be skipped when None
        assert!(!json.contains("embedding"));
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(deserialized.embedding.is_none());
    }

    #[test]
    fn short_term_memory_serialization() {
        let stm = ShortTermMemory {
            user_id: "user1".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "test".to_string(),
                timestamp: Utc::now(),
                embedding: None,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&stm).unwrap();
        let deserialized: ShortTermMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, "user1");
        assert_eq!(deserialized.messages.len(), 1);
    }

    #[test]
    fn mid_term_segment_defaults() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "user_id": "user1",
            "summary": "test summary",
            "embedding": [0.1, 0.2],
            "heat": 0.5,
            "created_at": "2026-01-01T00:00:00Z"
        }"#;
        let segment: MidTermSegment = serde_json::from_str(json).unwrap();
        assert_eq!(segment.access_count, 0);
        assert_eq!(segment.heat_score, 0.0);
        assert!(segment.last_accessed.is_none());
        assert_eq!(segment.memory_type, MemoryType::QA);
        assert_eq!(segment.version, 0);
        assert!(segment.tags.is_empty());
        assert!(segment.tenant_id.is_none());
    }

    #[test]
    fn memory_context_with_no_long_term() {
        let ctx = MemoryContext {
            short_term: vec![],
            mid_term: vec![],
            long_term: None,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: MemoryContext = serde_json::from_str(&json).unwrap();
        assert!(deserialized.long_term.is_none());
        assert!(deserialized.short_term.is_empty());
    }

    #[test]
    fn user_profile_serialization() {
        let profile = UserProfile {
            traits: vec!["curious".to_string()],
            preferences: vec!["dark mode".to_string()],
            background: "developer".to_string(),
        };
        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: UserProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.traits, vec!["curious"]);
        assert_eq!(deserialized.background, "developer");
    }

    #[test]
    fn knowledge_item_serialization() {
        let item = KnowledgeItem {
            id: uuid::Uuid::new_v4(),
            content: "Rust is a systems language".to_string(),
            embedding: vec![0.1; 10],
            source: "conversation".to_string(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: KnowledgeItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "Rust is a systems language");
        assert_eq!(deserialized.embedding.len(), 10);
    }
}
