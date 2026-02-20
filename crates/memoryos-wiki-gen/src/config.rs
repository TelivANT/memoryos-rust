use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ir::Language;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WikiGenConfig {
    #[serde(default)]
    pub repo: RepoConfig,
    #[serde(default)]
    pub parse: ParseConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub export: ExportConfig,
}

impl WikiGenConfig {
    pub fn from_file(path: &Path) -> Result<Self, crate::error::WikiGenError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::WikiGenError::Config(format!("Failed to read config file: {}", e))
        })?;
        toml::from_str(&content).map_err(|e| {
            crate::error::WikiGenError::Config(format!("Failed to parse config file: {}", e))
        })
    }

    pub fn resolved_output_dir(&self, repo_root: &Path) -> PathBuf {
        let output = Path::new(&self.output.output_dir);
        if output.is_absolute() {
            output.to_path_buf()
        } else {
            repo_root.join(output)
        }
    }

    pub fn resolved_cache_dir(&self, repo_root: &Path) -> PathBuf {
        let cache = Path::new(&self.cache.cache_dir);
        if cache.is_absolute() {
            cache.to_path_buf()
        } else {
            repo_root.join(cache)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    #[serde(default = "default_repo_root")]
    pub root: String,
    #[serde(default)]
    pub include_patterns: Vec<String>,
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            root: default_repo_root(),
            include_patterns: Vec::new(),
            exclude_patterns: default_exclude_patterns(),
        }
    }
}

fn default_repo_root() -> String {
    ".".to_string()
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "**/generated/**".to_string(),
        "**/vendor/**".to_string(),
        "**/node_modules/**".to_string(),
        "**/target/**".to_string(),
        "**/__pycache__/**".to_string(),
        "**/.git/**".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseConfig {
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
    #[serde(default = "default_max_file_size")]
    pub max_file_size_kb: usize,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            languages: default_languages(),
            max_file_size_kb: default_max_file_size(),
        }
    }
}

impl ParseConfig {
    pub fn language_enabled(&self, lang: Language) -> bool {
        let name = match lang {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::Java => "java",
            Language::TypeScript | Language::JavaScript => "typescript",
            Language::Vue => "vue",
            Language::Html => "html",
        };
        self.languages.iter().any(|l| l.eq_ignore_ascii_case(name))
    }
}

fn default_languages() -> Vec<String> {
    vec![
        "rust".to_string(),
        "python".to_string(),
        "java".to_string(),
        "vue".to_string(),
    ]
}

fn default_max_file_size() -> usize {
    512
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_adapter")]
    pub adapter: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    #[serde(default = "default_retry_max")]
    pub retry_max: usize,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            adapter: default_adapter(),
            model: default_model(),
            max_concurrent: default_max_concurrent(),
            retry_max: default_retry_max(),
            retry_backoff_ms: default_retry_backoff_ms(),
        }
    }
}

fn default_adapter() -> String {
    "openai".to_string()
}

fn default_model() -> String {
    "gpt-4o".to_string()
}

fn default_max_concurrent() -> usize {
    5
}

fn default_retry_max() -> usize {
    3
}

fn default_retry_backoff_ms() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: default_cache_enabled(),
            cache_dir: default_cache_dir(),
        }
    }
}

fn default_cache_enabled() -> bool {
    true
}

fn default_cache_dir() -> String {
    ".wiki-cache".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_format")]
    pub format: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_include_faq")]
    pub include_faq: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: default_output_format(),
            output_dir: default_output_dir(),
            include_faq: default_include_faq(),
        }
    }
}

fn default_output_format() -> String {
    "markdown".to_string()
}

fn default_output_dir() -> String {
    "wiki-output".to_string()
}

fn default_include_faq() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    #[serde(default = "default_export_target")]
    pub target: String,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            target: default_export_target(),
        }
    }
}

fn default_export_target() -> String {
    "local".to_string()
}
