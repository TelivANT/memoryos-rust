//! FAQ Wiki 导出服务

use crate::memory::{MemoryType, MidTermSegment};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Backend trait for remote wiki exports (S3, Confluence, etc.)
/// Implemented in memoryos-adapters to keep core dependency-free.
#[async_trait::async_trait]
pub trait WikiExportBackend: Send + Sync {
    async fn write_content(&self, path: &str, content: &[u8]) -> Result<ExportResult, String>;

    fn backend_name(&self) -> &str;
}

/// Wiki 导出配置
#[derive(Debug, Clone)]
pub struct WikiExportConfig {
    /// 导出间隔（秒）
    pub export_interval_secs: u64,
    /// 最小年龄（天）
    pub min_age_days: i64,
    /// 最小访问次数
    pub min_access_count: u32,
    /// 是否启用
    pub enabled: bool,
    /// 导出目标
    pub target: ExportTarget,
}

impl Default for WikiExportConfig {
    fn default() -> Self {
        Self {
            export_interval_secs: 86400, // 24 小时
            min_age_days: 30,
            min_access_count: 10,
            enabled: false, // 默认关闭
            target: ExportTarget::Local("./wiki_export".to_string()),
        }
    }
}

/// 导出目标
#[derive(Debug, Clone)]
pub enum ExportTarget {
    /// 本地文件系统
    Local(String),
    /// S3/OSS
    S3 {
        bucket: String,
        prefix: String,
        endpoint: Option<String>,
    },
    /// Confluence
    Confluence {
        base_url: String,
        space_key: String,
        parent_page_id: Option<String>,
    },
}

/// Wiki 导出器
pub struct WikiExporter {
    config: WikiExportConfig,
    backend: Option<Arc<dyn WikiExportBackend>>,
}

impl WikiExporter {
    pub fn new(config: WikiExportConfig) -> Self {
        Self {
            config,
            backend: None,
        }
    }

    pub fn with_backend(mut self, backend: Arc<dyn WikiExportBackend>) -> Self {
        self.backend = Some(backend);
        self
    }

    /// 筛选可导出的 FAQ
    pub fn filter_exportable<'a>(&self, segments: &'a [MidTermSegment]) -> Vec<&'a MidTermSegment> {
        let now = Utc::now();
        segments
            .iter()
            .filter(|s| {
                s.memory_type == MemoryType::Faq
                    && s.access_count >= self.config.min_access_count
                    && (now - s.created_at).num_days() >= self.config.min_age_days
            })
            .collect()
    }

    /// 按分类组织 FAQ
    pub fn categorize<'a>(
        &self,
        segments: Vec<&'a MidTermSegment>,
    ) -> HashMap<String, Vec<&'a MidTermSegment>> {
        let mut categories: HashMap<String, Vec<&'a MidTermSegment>> = HashMap::new();

        for segment in segments {
            // 简单分类：根据 user_id 前缀或内容关键词
            let category = self.extract_category(segment);
            categories.entry(category).or_default().push(segment);
        }

        categories
    }

    /// 提取分类
    fn extract_category(&self, segment: &MidTermSegment) -> String {
        // 简单实现：根据内容关键词分类
        let summary_lower = segment.summary.to_lowercase();

        if summary_lower.contains("wifi") || summary_lower.contains("网络") {
            "网络问题".to_string()
        } else if summary_lower.contains("密码") || summary_lower.contains("password") {
            "账号密码".to_string()
        } else if summary_lower.contains("报销") || summary_lower.contains("expense") {
            "财务报销".to_string()
        } else if summary_lower.contains("请假") || summary_lower.contains("leave") {
            "考勤休假".to_string()
        } else {
            "其他".to_string()
        }
    }

    /// 生成 Markdown
    pub fn generate_markdown(&self, categories: HashMap<String, Vec<&MidTermSegment>>) -> String {
        let mut md = String::new();

        // 标题
        md.push_str("# FAQ 知识库\n\n");
        md.push_str(&format!(
            "**生成时间**: {}\n\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));
        md.push_str("---\n\n");

        // 目录
        md.push_str("## 📑 目录\n\n");
        for category in categories.keys() {
            md.push_str(&format!(
                "- [{}](#{})\n",
                category,
                category.replace(' ', "-")
            ));
        }
        md.push_str("\n---\n\n");

        // 各分类内容
        for (category, segments) in categories {
            md.push_str(&format!("## {}\n\n", category));

            for (idx, segment) in segments.iter().enumerate() {
                md.push_str(&format!("### {}. {}\n\n", idx + 1, segment.summary));
                md.push_str(&format!(
                    "**访问次数**: {} | **热度**: {:.2}\n\n",
                    segment.access_count, segment.heat_score
                ));
                md.push_str(&format!(
                    "**创建时间**: {}\n\n",
                    segment.created_at.format("%Y-%m-%d")
                ));

                if let Some(last_accessed) = segment.last_accessed {
                    md.push_str(&format!(
                        "**最后访问**: {}\n\n",
                        last_accessed.format("%Y-%m-%d")
                    ));
                }

                md.push_str("---\n\n");
            }
        }

        md
    }

    /// 导出到目标
    pub async fn export(&self, markdown: String) -> Result<ExportResult, String> {
        match &self.config.target {
            ExportTarget::Local(path) => self.export_to_local(path, markdown).await,
            ExportTarget::S3 {
                bucket,
                prefix,
                endpoint,
            } => {
                self.export_to_s3(bucket, prefix, endpoint.as_deref(), markdown)
                    .await
            }
            ExportTarget::Confluence {
                base_url,
                space_key,
                parent_page_id,
            } => {
                self.export_to_confluence(base_url, space_key, parent_page_id.as_deref(), markdown)
                    .await
            }
        }
    }

    /// 导出到本地文件
    async fn export_to_local(&self, path: &str, markdown: String) -> Result<ExportResult, String> {
        use tokio::fs;

        // 创建目录
        fs::create_dir_all(path)
            .await
            .map_err(|e| format!("创建目录失败: {}", e))?;

        // 生成文件名
        let filename = format!("faq_{}.md", Utc::now().format("%Y%m%d_%H%M%S"));
        let filepath = format!("{}/{}", path, filename);

        // 写入文件
        let line_count = markdown.lines().filter(|l| l.starts_with("### ")).count();
        fs::write(&filepath, markdown)
            .await
            .map_err(|e| format!("写入文件失败: {}", e))?;

        Ok(ExportResult {
            success: true,
            target: format!("local://{}", filepath),
            exported_count: line_count,
            message: format!("成功导出到 {}", filepath),
        })
    }

    /// 导出到 S3/OSS (delegates to WikiExportBackend)
    async fn export_to_s3(
        &self,
        _bucket: &str,
        prefix: &str,
        _endpoint: Option<&str>,
        markdown: String,
    ) -> Result<ExportResult, String> {
        let backend = self
            .backend
            .as_ref()
            .ok_or_else(|| "S3 export backend not configured. Use WikiExporter::with_backend() to set an S3 backend.".to_string())?;
        let filename = format!("faq_{}.md", Utc::now().format("%Y%m%d_%H%M%S"));
        let path = format!("{}{}", prefix, filename);
        backend.write_content(&path, markdown.as_bytes()).await
    }

    /// 导出到 Confluence (delegates to WikiExportBackend)
    async fn export_to_confluence(
        &self,
        _base_url: &str,
        _space_key: &str,
        _parent_page_id: Option<&str>,
        markdown: String,
    ) -> Result<ExportResult, String> {
        let backend = self
            .backend
            .as_ref()
            .ok_or_else(|| "Confluence export backend not configured. Use WikiExporter::with_backend() to set a Confluence backend.".to_string())?;
        let title = format!("FAQ_{}", Utc::now().format("%Y%m%d_%H%M%S"));
        backend.write_content(&title, markdown.as_bytes()).await
    }

    /// 启动后台导出任务
    pub fn start_background_task<F>(self, mut fetch_segments: F) -> tokio::task::JoinHandle<()>
    where
        F: FnMut() -> Vec<MidTermSegment> + Send + 'static,
    {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
                self.config.export_interval_secs,
            ));

            loop {
                interval.tick().await;

                if !self.config.enabled {
                    continue;
                }

                let segments = fetch_segments();
                let exportable = self.filter_exportable(&segments);

                if exportable.is_empty() {
                    tracing::info!("没有可导出的 FAQ");
                    continue;
                }

                let categories = self.categorize(exportable);
                let markdown = self.generate_markdown(categories);

                match self.export(markdown).await {
                    Ok(result) => {
                        tracing::info!(
                            "FAQ Wiki 导出成功: {} 个条目导出到 {}",
                            result.exported_count,
                            result.target
                        );
                    }
                    Err(e) => {
                        tracing::error!("FAQ Wiki 导出失败: {}", e);
                    }
                }
            }
        })
    }
}

/// 导出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub success: bool,
    pub target: String,
    pub exported_count: usize,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_faq(summary: &str, access_count: u32, days_old: i64) -> MidTermSegment {
        MidTermSegment {
            id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            summary: summary.to_string(),
            embedding: vec![0.1; 768],
            heat: 0.0,
            created_at: Utc::now() - chrono::Duration::days(days_old),
            access_count,
            heat_score: 50.0,
            last_accessed: Some(Utc::now()),
            memory_type: MemoryType::Faq,
        }
    }

    #[test]
    fn test_filter_exportable() {
        let exporter = WikiExporter::new(WikiExportConfig::default());

        let segments = vec![
            create_test_faq("WiFi 密码是多少？", 15, 35), // 应该导出
            create_test_faq("今天天气如何？", 5, 35),     // 访问次数不够
            create_test_faq("报销流程是什么？", 15, 10),  // 年龄不够
        ];

        let exportable = exporter.filter_exportable(&segments);
        assert_eq!(exportable.len(), 1);
        assert!(exportable[0].summary.contains("WiFi"));
    }

    #[test]
    fn test_categorize() {
        let exporter = WikiExporter::new(WikiExportConfig::default());

        let segments = [
            create_test_faq("WiFi 密码是多少？", 15, 35),
            create_test_faq("报销流程是什么？", 15, 35),
            create_test_faq("如何请假？", 15, 35),
        ];

        let exportable: Vec<&MidTermSegment> = segments.iter().collect();
        let categories = exporter.categorize(exportable);

        assert!(categories.contains_key("网络问题"));
        assert!(categories.contains_key("财务报销"));
        assert!(categories.contains_key("考勤休假"));
    }

    #[test]
    fn test_generate_markdown() {
        let exporter = WikiExporter::new(WikiExportConfig::default());

        let segments = [create_test_faq("WiFi 密码是多少？", 15, 35)];

        let exportable: Vec<&MidTermSegment> = segments.iter().collect();
        let categories = exporter.categorize(exportable);
        let markdown = exporter.generate_markdown(categories);

        assert!(markdown.contains("# FAQ 知识库"));
        assert!(markdown.contains("WiFi 密码是多少？"));
        assert!(markdown.contains("**访问次数**: 15"));
    }

    #[tokio::test]
    async fn test_export_to_local() {
        let config = WikiExportConfig {
            target: ExportTarget::Local("/tmp/memoryos_test_wiki".to_string()),
            ..Default::default()
        };
        let exporter = WikiExporter::new(config);

        let markdown = "# Test FAQ\n\nTest content".to_string();
        let result = exporter.export(markdown).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert!(result.target.contains("/tmp/memoryos_test_wiki"));
    }
}
