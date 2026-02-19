use async_trait::async_trait;
use memoryos_core::AppError;
use memoryos_ports::{WikiAdapter, WikiDocument};
use opendal::{Builder, Operator};
use std::collections::HashMap;
use tracing::info;

pub struct OpenDALAdapter {
    operator: Operator,
    scheme_name: String,
}

impl OpenDALAdapter {
    /// Create adapter from config map
    /// map: { "root": "/tmp", "bucket": "my-wiki", "endpoint": "http://minio:9000" ... }
    pub fn new(scheme: &str, map: HashMap<String, String>) -> Result<Self, AppError> {
        let op = match scheme {
            "s3" | "minio" => {
                let mut builder = opendal::services::S3::default();
                if let Some(root) = map.get("root") {
                    builder.root(root);
                }
                if let Some(bucket) = map.get("bucket") {
                    builder.bucket(bucket);
                }
                if let Some(endpoint) = map.get("endpoint") {
                    builder.endpoint(endpoint);
                }
                if let Some(ak) = map.get("access_key_id") {
                    builder.access_key_id(ak);
                }
                if let Some(sk) = map.get("secret_access_key") {
                    builder.secret_access_key(sk);
                }
                if let Some(region) = map.get("region") {
                    builder.region(region);
                }
                Operator::new(builder)
                    .map_err(|e| AppError::Config(e.to_string()))?
                    .finish()
            }
            "fs" => {
                let mut builder = opendal::services::Fs::default();
                if let Some(root) = map.get("root") {
                    builder.root(root);
                }
                Operator::new(builder)
                    .map_err(|e| AppError::Config(e.to_string()))?
                    .finish()
            }
            _ => {
                return Err(AppError::Config(format!(
                    "Unsupported storage scheme: {}",
                    scheme
                )))
            }
        };

        Ok(Self {
            operator: op,
            scheme_name: scheme.to_string(),
        })
    }

    fn generate_path(&self, doc: &WikiDocument) -> String {
        let slug = doc.title.to_lowercase().replace(" ", "-");
        format!("{}/{}.md", doc.category, slug)
    }
}

#[async_trait]
impl WikiAdapter for OpenDALAdapter {
    async fn publish(&self, doc: WikiDocument) -> Result<String, AppError> {
        let path = self.generate_path(&doc);
        info!("Publishing Wiki to {}: {}", self.scheme_name, path);

        let content = doc.content.into_bytes();
        self.operator
            .write(&path, content)
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenDAL write failed: {}", e)))?;

        // Return pseudo-url (OpenDAL doesn't inherently know public URLs)
        Ok(format!("{}://{}", self.scheme_name, path))
    }

    async fn recall(&self, doc_id: &str) -> Result<(), AppError> {
        // doc_id here is assumed to be path
        self.operator
            .delete(doc_id)
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenDAL delete failed: {}", e)))?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.scheme_name
    }
}
