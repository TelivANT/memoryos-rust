use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::info;

pub trait AuditStorageBackend: Send + Sync {
    fn append(&self, event: &AuditEvent);
    fn load_recent(&self, limit: usize) -> Vec<AuditEvent>;
}

pub struct FileAuditBackend {
    path: PathBuf,
    writer: Mutex<Option<std::fs::File>>,
}

impl FileAuditBackend {
    pub fn new(path: &str) -> Self {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let writer = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();
        Self {
            path,
            writer: Mutex::new(writer),
        }
    }
}

impl AuditStorageBackend for FileAuditBackend {
    fn append(&self, event: &AuditEvent) {
        if let Ok(mut writer) = self.writer.lock() {
            if let Some(ref mut file) = *writer {
                if let Ok(json) = serde_json::to_string(event) {
                    let _ = writeln!(file, "{}", json);
                    let _ = file.flush();
                }
            }
        }
    }

    fn load_recent(&self, limit: usize) -> Vec<AuditEvent> {
        std::fs::read_to_string(&self.path)
            .ok()
            .map(|data| {
                let all: Vec<AuditEvent> = data
                    .lines()
                    .filter_map(|line| serde_json::from_str(line).ok())
                    .collect();
                all.into_iter().rev().take(limit).collect()
            })
            .unwrap_or_default()
    }
}

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
    #[serde(default)]
    pub persist_path: Option<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_buffer_size: 10000,
            persist_path: None,
        }
    }
}

pub struct AuditLogger {
    config: AuditConfig,
    buffer: Mutex<VecDeque<AuditEvent>>,
    backend: Option<Box<dyn AuditStorageBackend>>,
}

impl AuditLogger {
    pub fn new(config: AuditConfig) -> Self {
        let backend: Option<Box<dyn AuditStorageBackend>> = config
            .persist_path
            .as_ref()
            .map(|path| Box::new(FileAuditBackend::new(path)) as Box<dyn AuditStorageBackend>);

        let mut buffer = VecDeque::new();
        if let Some(ref b) = backend {
            for event in b.load_recent(config.max_buffer_size) {
                buffer.push_front(event);
            }
        }

        Self {
            config,
            buffer: Mutex::new(buffer),
            backend,
        }
    }

    pub fn with_backend(config: AuditConfig, backend: Box<dyn AuditStorageBackend>) -> Self {
        let mut buffer = VecDeque::new();
        for event in backend.load_recent(config.max_buffer_size) {
            buffer.push_front(event);
        }
        Self {
            config,
            buffer: Mutex::new(buffer),
            backend: Some(backend),
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

        if let Some(ref backend) = self.backend {
            backend.append(&event);
        }

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
            persist_path: None,
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
            persist_path: None,
        };
        let logger = AuditLogger::new(config);
        logger.log_auth(Some("user1"), true, None);
        assert_eq!(logger.event_count(), 0);
    }

    #[test]
    fn test_audit_file_persistence() {
        let dir = std::env::temp_dir().join("memoryos_audit_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("audit.jsonl");
        let _ = std::fs::remove_file(&path);

        let config = AuditConfig {
            enabled: true,
            max_buffer_size: 10000,
            persist_path: Some(path.to_string_lossy().to_string()),
        };
        let logger = AuditLogger::new(config);
        logger.log_data_access("user1", "memory", "read");
        logger.log_auth(Some("user2"), true, None);
        drop(logger);

        let config2 = AuditConfig {
            enabled: true,
            max_buffer_size: 10000,
            persist_path: Some(path.to_string_lossy().to_string()),
        };
        let logger2 = AuditLogger::new(config2);
        assert_eq!(logger2.event_count(), 2);
        let events = logger2.get_recent(10);
        assert_eq!(events[0].resource, "api");
        assert_eq!(events[1].resource, "memory");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
