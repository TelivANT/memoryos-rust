//! Memory history tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Memory history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHistoryEntry {
    pub id: String,
    pub memory_id: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub event_type: HistoryEventType,
    pub created_at: DateTime<Utc>,
    pub actor_id: Option<String>,
}

/// History event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HistoryEventType {
    Add,
    Update,
    Delete,
}

impl std::fmt::Display for HistoryEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryEventType::Add => write!(f, "ADD"),
            HistoryEventType::Update => write!(f, "UPDATE"),
            HistoryEventType::Delete => write!(f, "DELETE"),
        }
    }
}
