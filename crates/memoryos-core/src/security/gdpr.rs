use crate::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

pub trait GdprStorageBackend: Send + Sync {
    fn save_snapshot(
        &self,
        consents: &std::collections::HashMap<String, Vec<ConsentRecord>>,
        deletion_requests: &[DeletionRequest],
    );
    fn load_snapshot(
        &self,
    ) -> Option<(
        std::collections::HashMap<String, Vec<ConsentRecord>>,
        Vec<DeletionRequest>,
    )>;
}

pub struct FileGdprBackend {
    path: PathBuf,
}

impl FileGdprBackend {
    pub fn new(path: &str) -> Self {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self { path }
    }
}

impl GdprStorageBackend for FileGdprBackend {
    fn save_snapshot(
        &self,
        consents: &std::collections::HashMap<String, Vec<ConsentRecord>>,
        deletion_requests: &[DeletionRequest],
    ) {
        let snapshot = GdprSnapshot {
            consents: consents.clone(),
            deletion_requests: deletion_requests.to_vec(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
            let _ = std::fs::write(&self.path, json);
        }
    }

    fn load_snapshot(
        &self,
    ) -> Option<(
        std::collections::HashMap<String, Vec<ConsentRecord>>,
        Vec<DeletionRequest>,
    )> {
        if self.path.exists() {
            std::fs::read_to_string(&self.path)
                .ok()
                .and_then(|data| serde_json::from_str::<GdprSnapshot>(&data).ok())
                .map(|s| (s.consents, s.deletion_requests))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GdprDataExport {
    pub user_id: String,
    pub exported_at: chrono::DateTime<chrono::Utc>,
    pub data_categories: Vec<DataCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCategory {
    pub category: String,
    pub description: String,
    pub item_count: usize,
    pub retention_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionRequest {
    pub user_id: String,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub status: DeletionStatus,
    pub categories_deleted: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeletionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub user_id: String,
    pub purpose: String,
    pub granted: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GdprSnapshot {
    consents: std::collections::HashMap<String, Vec<ConsentRecord>>,
    deletion_requests: Vec<DeletionRequest>,
}

pub struct GdprManager {
    consents: std::collections::HashMap<String, Vec<ConsentRecord>>,
    deletion_requests: Vec<DeletionRequest>,
    backend: Option<Box<dyn GdprStorageBackend>>,
}

impl GdprManager {
    pub fn new() -> Self {
        Self {
            consents: std::collections::HashMap::new(),
            deletion_requests: Vec::new(),
            backend: None,
        }
    }

    pub fn with_persistence(path: &str) -> Self {
        let backend = Box::new(FileGdprBackend::new(path));
        Self::with_backend(backend)
    }

    pub fn with_backend(backend: Box<dyn GdprStorageBackend>) -> Self {
        let (consents, deletion_requests) = backend.load_snapshot().unwrap_or_default();

        Self {
            consents,
            deletion_requests,
            backend: Some(backend),
        }
    }

    fn save(&self) {
        if let Some(ref backend) = self.backend {
            backend.save_snapshot(&self.consents, &self.deletion_requests);
        }
    }

    pub fn record_consent(&mut self, user_id: &str, purpose: &str, granted: bool) {
        let record = ConsentRecord {
            user_id: user_id.to_string(),
            purpose: purpose.to_string(),
            granted,
            timestamp: chrono::Utc::now(),
        };
        self.consents
            .entry(user_id.to_string())
            .or_default()
            .push(record);
        self.save();
    }

    pub fn has_consent(&self, user_id: &str, purpose: &str) -> bool {
        self.consents
            .get(user_id)
            .and_then(|records| {
                records
                    .iter()
                    .rev()
                    .find(|r| r.purpose == purpose)
                    .map(|r| r.granted)
            })
            .unwrap_or(false)
    }

    pub fn get_consents(&self, user_id: &str) -> Vec<ConsentRecord> {
        self.consents.get(user_id).cloned().unwrap_or_default()
    }

    pub fn prepare_data_export(&self, user_id: &str) -> GdprDataExport {
        info!("Preparing GDPR data export for user: {}", user_id);

        GdprDataExport {
            user_id: user_id.to_string(),
            exported_at: chrono::Utc::now(),
            data_categories: vec![
                DataCategory {
                    category: "short_term_memory".to_string(),
                    description: "Recent conversation messages".to_string(),
                    item_count: 0,
                    retention_days: Some(7),
                },
                DataCategory {
                    category: "mid_term_memory".to_string(),
                    description: "Conversation summaries and segments".to_string(),
                    item_count: 0,
                    retention_days: Some(90),
                },
                DataCategory {
                    category: "long_term_memory".to_string(),
                    description: "User profile and persistent knowledge".to_string(),
                    item_count: 0,
                    retention_days: None,
                },
                DataCategory {
                    category: "graph_memory".to_string(),
                    description: "Knowledge graph entities and relations".to_string(),
                    item_count: 0,
                    retention_days: None,
                },
                DataCategory {
                    category: "multimodal_data".to_string(),
                    description: "Images, audio, and video content".to_string(),
                    item_count: 0,
                    retention_days: Some(30),
                },
                DataCategory {
                    category: "faq_data".to_string(),
                    description: "Frequently asked questions and answers".to_string(),
                    item_count: 0,
                    retention_days: None,
                },
            ],
        }
    }

    pub fn request_deletion(&mut self, user_id: &str) -> Result<DeletionRequest, AppError> {
        info!("GDPR deletion request for user: {}", user_id);

        let request = DeletionRequest {
            user_id: user_id.to_string(),
            requested_at: chrono::Utc::now(),
            status: DeletionStatus::Pending,
            categories_deleted: vec![],
        };

        self.deletion_requests.push(request.clone());
        self.save();
        Ok(request)
    }

    pub fn get_deletion_requests(&self, user_id: &str) -> Vec<&DeletionRequest> {
        self.deletion_requests
            .iter()
            .filter(|r| r.user_id == user_id)
            .collect()
    }

    pub fn complete_deletion(&mut self, user_id: &str) -> Result<(), AppError> {
        let categories = vec![
            "short_term_memory".to_string(),
            "mid_term_memory".to_string(),
            "long_term_memory".to_string(),
            "graph_memory".to_string(),
            "multimodal_data".to_string(),
            "faq_data".to_string(),
        ];

        if let Some(req) = self
            .deletion_requests
            .iter_mut()
            .rev()
            .find(|r| r.user_id == user_id && r.status == DeletionStatus::Pending)
        {
            req.status = DeletionStatus::Completed;
            req.categories_deleted = categories;
        }

        self.consents.remove(user_id);
        self.save();

        Ok(())
    }
}

impl Default for GdprManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consent_record() {
        let mut mgr = GdprManager::new();
        mgr.record_consent("user1", "data_processing", true);
        assert!(mgr.has_consent("user1", "data_processing"));
        assert!(!mgr.has_consent("user1", "marketing"));
        assert!(!mgr.has_consent("user2", "data_processing"));
    }

    #[test]
    fn test_consent_revocation() {
        let mut mgr = GdprManager::new();
        mgr.record_consent("user1", "data_processing", true);
        assert!(mgr.has_consent("user1", "data_processing"));
        mgr.record_consent("user1", "data_processing", false);
        assert!(!mgr.has_consent("user1", "data_processing"));
    }

    #[test]
    fn test_data_export() {
        let mgr = GdprManager::new();
        let export = mgr.prepare_data_export("user1");
        assert_eq!(export.user_id, "user1");
        assert_eq!(export.data_categories.len(), 6);
    }

    #[test]
    fn test_deletion_request() {
        let mut mgr = GdprManager::new();
        let req = mgr.request_deletion("user1").unwrap();
        assert_eq!(req.status, DeletionStatus::Pending);
        assert_eq!(mgr.get_deletion_requests("user1").len(), 1);
    }

    #[test]
    fn test_complete_deletion() {
        let mut mgr = GdprManager::new();
        mgr.record_consent("user1", "data_processing", true);
        mgr.request_deletion("user1").unwrap();
        mgr.complete_deletion("user1").unwrap();

        let reqs = mgr.get_deletion_requests("user1");
        assert_eq!(reqs[0].status, DeletionStatus::Completed);
        assert!(!mgr.has_consent("user1", "data_processing"));
    }

    #[test]
    fn test_gdpr_file_persistence() {
        let dir = std::env::temp_dir().join("memoryos_gdpr_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("gdpr.json");
        let _ = std::fs::remove_file(&path);

        let mut mgr = GdprManager::with_persistence(&path.to_string_lossy());
        mgr.record_consent("user1", "data_processing", true);
        mgr.record_consent("user1", "marketing", false);
        mgr.request_deletion("user2").unwrap();
        drop(mgr);

        let mgr2 = GdprManager::with_persistence(&path.to_string_lossy());
        assert!(mgr2.has_consent("user1", "data_processing"));
        assert!(!mgr2.has_consent("user1", "marketing"));
        assert_eq!(mgr2.get_deletion_requests("user2").len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
