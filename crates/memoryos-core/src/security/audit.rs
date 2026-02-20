use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: AuditEventType,
    pub user_id: Option<String>,
    pub resource: String,
    pub action: String,
    pub outcome: AuditOutcome,
    pub details: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    DataDeletion,
    DataExport,
    ConfigChange,
    SecurityEvent,
    SystemEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuditOutcome {
    Success,
    Failure,
    Denied,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub max_buffer_size: usize,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_buffer_size: 10000,
        }
    }
}

pub struct AuditLogger {
    config: AuditConfig,
    buffer: Mutex<VecDeque<AuditEvent>>,
}

impl AuditLogger {
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config,
            buffer: Mutex::new(VecDeque::new()),
        }
    }

    pub fn log(&self, event: AuditEvent) {
        if !self.config.enabled {
            return;
        }

        info!(
            audit_type = ?event.event_type,
            user = ?event.user_id,
            resource = %event.resource,
            action = %event.action,
            outcome = ?event.outcome,
            "AUDIT: {} {} -> {:?}",
            event.action,
            event.resource,
            event.outcome
        );

        if let Ok(mut buffer) = self.buffer.lock() {
            if buffer.len() >= self.config.max_buffer_size {
                buffer.pop_front();
            }
            buffer.push_back(event);
        }
    }

    pub fn log_auth(&self, user_id: Option<&str>, success: bool, details: Option<&str>) {
        self.log(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::Authentication,
            user_id: user_id.map(|s| s.to_string()),
            resource: "api".to_string(),
            action: "authenticate".to_string(),
            outcome: if success {
                AuditOutcome::Success
            } else {
                AuditOutcome::Failure
            },
            details: details.map(|s| s.to_string()),
            ip_address: None,
        });
    }

    pub fn log_data_access(&self, user_id: &str, resource: &str, action: &str) {
        self.log(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::DataAccess,
            user_id: Some(user_id.to_string()),
            resource: resource.to_string(),
            action: action.to_string(),
            outcome: AuditOutcome::Success,
            details: None,
            ip_address: None,
        });
    }

    pub fn log_data_modification(&self, user_id: &str, resource: &str, action: &str) {
        self.log(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::DataModification,
            user_id: Some(user_id.to_string()),
            resource: resource.to_string(),
            action: action.to_string(),
            outcome: AuditOutcome::Success,
            details: None,
            ip_address: None,
        });
    }

    pub fn log_data_deletion(&self, user_id: &str, resource: &str) {
        self.log(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::DataDeletion,
            user_id: Some(user_id.to_string()),
            resource: resource.to_string(),
            action: "delete".to_string(),
            outcome: AuditOutcome::Success,
            details: None,
            ip_address: None,
        });
    }

    pub fn log_security_event(&self, event_type: &str, details: &str) {
        self.log(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SecurityEvent,
            user_id: None,
            resource: "security".to_string(),
            action: event_type.to_string(),
            outcome: AuditOutcome::Denied,
            details: Some(details.to_string()),
            ip_address: None,
        });
    }

    pub fn get_recent(&self, limit: usize) -> Vec<AuditEvent> {
        if let Ok(buffer) = self.buffer.lock() {
            buffer.iter().rev().take(limit).cloned().collect()
        } else {
            vec![]
        }
    }

    pub fn get_by_user(&self, user_id: &str, limit: usize) -> Vec<AuditEvent> {
        if let Ok(buffer) = self.buffer.lock() {
            buffer
                .iter()
                .rev()
                .filter(|e| e.user_id.as_deref() == Some(user_id))
                .take(limit)
                .cloned()
                .collect()
        } else {
            vec![]
        }
    }

    pub fn event_count(&self) -> usize {
        self.buffer.lock().map(|b| b.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_auth() {
        let logger = AuditLogger::new(AuditConfig::default());
        logger.log_auth(Some("user1"), true, None);
        logger.log_auth(Some("user2"), false, Some("bad password"));
        assert_eq!(logger.event_count(), 2);
    }

    #[test]
    fn test_audit_get_recent() {
        let logger = AuditLogger::new(AuditConfig::default());
        logger.log_data_access("user1", "memory", "read");
        logger.log_data_modification("user1", "memory", "update");
        let recent = logger.get_recent(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].action, "update");
    }

    #[test]
    fn test_audit_get_by_user() {
        let logger = AuditLogger::new(AuditConfig::default());
        logger.log_data_access("user1", "memory", "read");
        logger.log_data_access("user2", "memory", "read");
        logger.log_data_access("user1", "faq", "read");
        let user1_events = logger.get_by_user("user1", 10);
        assert_eq!(user1_events.len(), 2);
    }

    #[test]
    fn test_audit_buffer_limit() {
        let config = AuditConfig {
            enabled: true,
            max_buffer_size: 3,
        };
        let logger = AuditLogger::new(config);
        for i in 0..5 {
            logger.log_data_access(&format!("user{}", i), "memory", "read");
        }
        assert_eq!(logger.event_count(), 3);
    }

    #[test]
    fn test_audit_disabled() {
        let config = AuditConfig {
            enabled: false,
            max_buffer_size: 100,
        };
        let logger = AuditLogger::new(config);
        logger.log_auth(Some("user1"), true, None);
        assert_eq!(logger.event_count(), 0);
    }
}
