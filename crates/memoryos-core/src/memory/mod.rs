//! Memory data structures

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod graph;
pub use graph::{GraphEntity, GraphRelation, GraphManager};

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
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

/// Memory retrieval result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContext {
    pub short_term: Vec<Message>,
    pub mid_term: Vec<MidTermSegment>,
    pub long_term: Option<LongTermMemory>,
}
