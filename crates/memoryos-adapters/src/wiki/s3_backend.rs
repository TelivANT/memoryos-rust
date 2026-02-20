//! S3 WikiExportBackend implementation using OpenDAL

use async_trait::async_trait;
use memoryos_core::faq::wiki_exporter::{ExportResult, WikiExportBackend};
use opendal::Operator;
use std::collections::HashMap;
use tracing::info;

pub struct S3ExportBackend {
    operator: Operator,
}

impl S3ExportBackend {
    pub fn new(config: HashMap<String, String>) -> Result<Self, String> {
        let mut builder = opendal::services::S3::default();
        if let Some(bucket) = config.get("bucket") {
            builder.bucket(bucket);
        }
        if let Some(root) = config.get("root") {
            builder.root(root);
        }
        if let Some(endpoint) = config.get("endpoint") {
            builder.endpoint(endpoint);
        }
        if let Some(ak) = config.get("access_key_id") {
            builder.access_key_id(ak);
        }
        if let Some(sk) = config.get("secret_access_key") {
            builder.secret_access_key(sk);
        }
        if let Some(region) = config.get("region") {
            builder.region(region);
        }

        let operator = Operator::new(builder)
            .map_err(|e| format!("Failed to create S3 operator: {}", e))?
            .finish();

        Ok(Self { operator })
    }

    pub fn from_env() -> Result<Self, String> {
        let mut config = HashMap::new();
        if let Ok(bucket) = std::env::var("WIKI_S3_BUCKET") {
            config.insert("bucket".to_string(), bucket);
        }
        if let Ok(endpoint) = std::env::var("WIKI_S3_ENDPOINT") {
            config.insert("endpoint".to_string(), endpoint);
        }
        if let Ok(region) = std::env::var("WIKI_S3_REGION") {
            config.insert("region".to_string(), region);
        } else {
            config.insert("region".to_string(), "us-east-1".to_string());
        }
        if let Ok(ak) = std::env::var("AWS_ACCESS_KEY_ID") {
            config.insert("access_key_id".to_string(), ak);
        }
        if let Ok(sk) = std::env::var("AWS_SECRET_ACCESS_KEY") {
            config.insert("secret_access_key".to_string(), sk);
        }
        if let Ok(root) = std::env::var("WIKI_S3_PREFIX") {
            config.insert("root".to_string(), root);
        }

        Self::new(config)
    }
}

#[async_trait]
impl WikiExportBackend for S3ExportBackend {
    async fn write_content(&self, path: &str, content: &[u8]) -> Result<ExportResult, String> {
        info!("Uploading FAQ wiki to S3: {}", path);

        self.operator
            .write(path, content.to_vec())
            .await
            .map_err(|e| format!("S3 upload failed: {}", e))?;

        let line_count = std::str::from_utf8(content)
            .unwrap_or("")
            .lines()
            .filter(|l| l.starts_with("### "))
            .count();

        Ok(ExportResult {
            success: true,
            target: format!("s3://{}", path),
            exported_count: line_count,
            message: format!("Successfully exported to S3: {}", path),
        })
    }

    fn backend_name(&self) -> &str {
        "s3"
    }
}
