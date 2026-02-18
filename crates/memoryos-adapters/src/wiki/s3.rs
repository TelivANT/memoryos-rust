use async_trait::async_trait;
use memoryos_core::AppError;
use memoryos_ports::{WikiAdapter, WikiDocument};
use reqwest::Client;
use tracing::info;

pub struct S3Adapter {
    bucket: String,
    region: String,
    prefix: String,
    access_key: String,
    secret_key: String,
    client: Client,
}

impl S3Adapter {
    pub fn new(bucket: String, region: String, prefix: String, access_key: String, secret_key: String) -> Self {
        Self {
            bucket,
            region,
            prefix,
            access_key,
            secret_key,
            client: Client::new(),
        }
    }

    fn generate_key(&self, doc: &WikiDocument) -> String {
        let slug = doc.title.to_lowercase().replace(" ", "-");
        format!("{}{}/{}.md", self.prefix, doc.category, slug)
    }
}

#[async_trait]
impl WikiAdapter for S3Adapter {
    async fn publish(&self, doc: WikiDocument) -> Result<String, AppError> {
        let key = self.generate_key(&doc);
        let url = format!("https://{}.s3.{}.amazonaws.com/{}", self.bucket, self.region, key);
        
        info!("Uploading Wiki Document to S3: {}", url);

        // TODO: Sign request with AWS SigV4
        // For prototype, we assume a pre-signed URL or public bucket (in dev)
        // In prod, use `rust-s3` or `aws-sdk-s3` crate.
        
        // Mock Implementation
        // self.client.put(&url).body(doc.content).send().await...

        Ok(url)
    }

    async fn recall(&self, doc_id: &str) -> Result<(), AppError> {
        info!("Recalling Wiki Document from S3: {}", doc_id);
        // Mock Implementation
        Ok(())
    }

    fn name(&self) -> &str {
        "s3"
    }
}
