//! Confluence WikiExportBackend implementation using REST API

use async_trait::async_trait;
use memoryos_core::faq::wiki_exporter::{ExportResult, WikiExportBackend};
use reqwest::Client;
use serde::Serialize;
use tracing::info;

pub struct ConfluenceExportBackend {
    client: Client,
    base_url: String,
    space_key: String,
    parent_page_id: Option<String>,
    username: String,
    api_token: String,
}

#[derive(Serialize)]
struct CreatePageRequest {
    r#type: String,
    title: String,
    space: SpaceRef,
    body: PageBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    ancestors: Option<Vec<AncestorRef>>,
}

#[derive(Serialize)]
struct SpaceRef {
    key: String,
}

#[derive(Serialize)]
struct PageBody {
    storage: StorageBody,
}

#[derive(Serialize)]
struct StorageBody {
    value: String,
    representation: String,
}

#[derive(Serialize)]
struct AncestorRef {
    id: String,
}

impl ConfluenceExportBackend {
    pub fn new(
        base_url: String,
        space_key: String,
        parent_page_id: Option<String>,
        username: String,
        api_token: String,
    ) -> Self {
        Self {
            client: Client::new(),
            base_url,
            space_key,
            parent_page_id,
            username,
            api_token,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let base_url = std::env::var("CONFLUENCE_BASE_URL")
            .map_err(|_| "CONFLUENCE_BASE_URL not set".to_string())?;
        let space_key = std::env::var("CONFLUENCE_SPACE_KEY")
            .map_err(|_| "CONFLUENCE_SPACE_KEY not set".to_string())?;
        let username = std::env::var("CONFLUENCE_USERNAME")
            .map_err(|_| "CONFLUENCE_USERNAME not set".to_string())?;
        let api_token = std::env::var("CONFLUENCE_API_TOKEN")
            .map_err(|_| "CONFLUENCE_API_TOKEN not set".to_string())?;
        let parent_page_id = std::env::var("CONFLUENCE_PARENT_PAGE_ID").ok();

        Ok(Self::new(
            base_url,
            space_key,
            parent_page_id,
            username,
            api_token,
        ))
    }

    fn markdown_to_confluence_storage(markdown: &str) -> String {
        let mut html = String::new();
        for line in markdown.lines() {
            if let Some(h1) = line.strip_prefix("# ") {
                html.push_str(&format!("<h1>{}</h1>\n", h1));
            } else if let Some(h2) = line.strip_prefix("## ") {
                html.push_str(&format!("<h2>{}</h2>\n", h2));
            } else if let Some(h3) = line.strip_prefix("### ") {
                html.push_str(&format!("<h3>{}</h3>\n", h3));
            } else if line == "---" {
                html.push_str("<hr/>\n");
            } else if line.starts_with("- ") {
                html.push_str(&format!("<li>{}</li>\n", &line[2..]));
            } else if line.starts_with("**") && line.ends_with("**") {
                html.push_str(&format!(
                    "<p><strong>{}</strong></p>\n",
                    line.trim_matches('*')
                ));
            } else if !line.is_empty() {
                html.push_str(&format!("<p>{}</p>\n", line));
            }
        }
        html
    }
}

#[async_trait]
impl WikiExportBackend for ConfluenceExportBackend {
    async fn write_content(&self, title: &str, content: &[u8]) -> Result<ExportResult, String> {
        info!("Publishing FAQ wiki to Confluence: {}", title);

        let markdown =
            std::str::from_utf8(content).map_err(|e| format!("Invalid UTF-8 content: {}", e))?;

        let storage_content = Self::markdown_to_confluence_storage(markdown);

        let ancestors = self
            .parent_page_id
            .as_ref()
            .map(|id| vec![AncestorRef { id: id.clone() }]);

        let request = CreatePageRequest {
            r#type: "page".to_string(),
            title: title.to_string(),
            space: SpaceRef {
                key: self.space_key.clone(),
            },
            body: PageBody {
                storage: StorageBody {
                    value: storage_content,
                    representation: "storage".to_string(),
                },
            },
            ancestors,
        };

        let url = format!("{}/rest/api/content", self.base_url);
        let response = self
            .client
            .post(&url)
            .basic_auth(&self.username, Some(&self.api_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Confluence API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            return Err(format!("Confluence API returned {}: {}", status, body));
        }

        let line_count = markdown.lines().filter(|l| l.starts_with("### ")).count();

        Ok(ExportResult {
            success: true,
            target: format!("confluence://{}/{}", self.space_key, title),
            exported_count: line_count,
            message: format!(
                "Successfully exported to Confluence space '{}': {}",
                self.space_key, title
            ),
        })
    }

    fn backend_name(&self) -> &str {
        "confluence"
    }
}
