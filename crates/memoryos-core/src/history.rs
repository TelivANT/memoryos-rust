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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_event_type_display() {
        assert_eq!(format!("{}", HistoryEventType::Add), "ADD");
        assert_eq!(format!("{}", HistoryEventType::Update), "UPDATE");
        assert_eq!(format!("{}", HistoryEventType::Delete), "DELETE");
    }

    #[test]
    fn history_event_type_serialization() {
        assert_eq!(
            serde_json::to_string(&HistoryEventType::Add).unwrap(),
            "\"ADD\""
        );
        assert_eq!(
            serde_json::to_string(&HistoryEventType::Update).unwrap(),
            "\"UPDATE\""
        );
        assert_eq!(
            serde_json::to_string(&HistoryEventType::Delete).unwrap(),
            "\"DELETE\""
        );
    }

    #[test]
    fn history_event_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<HistoryEventType>("\"ADD\"").unwrap() as u8,
            HistoryEventType::Add as u8
        );
    }

    #[test]
    fn history_entry_serialization_roundtrip() {
        let entry = MemoryHistoryEntry {
            id: "entry-1".to_string(),
            memory_id: "mem-1".to_string(),
            old_content: Some("old text".to_string()),
            new_content: Some("new text".to_string()),
            event_type: HistoryEventType::Update,
            created_at: Utc::now(),
            actor_id: Some("user-1".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: MemoryHistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "entry-1");
        assert_eq!(deserialized.memory_id, "mem-1");
        assert_eq!(deserialized.old_content, Some("old text".to_string()));
        assert_eq!(deserialized.new_content, Some("new text".to_string()));
    }

    #[test]
    fn history_entry_with_none_fields() {
        let entry = MemoryHistoryEntry {
            id: "entry-2".to_string(),
            memory_id: "mem-2".to_string(),
            old_content: None,
            new_content: Some("created".to_string()),
            event_type: HistoryEventType::Add,
            created_at: Utc::now(),
            actor_id: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: MemoryHistoryEntry = serde_json::from_str(&json).unwrap();
        assert!(deserialized.old_content.is_none());
        assert!(deserialized.actor_id.is_none());
    }
}
