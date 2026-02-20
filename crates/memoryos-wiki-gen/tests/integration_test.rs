use memoryos_wiki_gen::config::WikiGenConfig;
use memoryos_wiki_gen::WikiGenerator;

fn setup_test_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("main.rs"),
        r#"
use crate::lib::MyService;

/// Entry point
fn main() {
    let svc = MyService::new();
    svc.run();
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("lib.rs"),
        r#"
/// Core service module
pub mod service;

/// A public struct for the service
pub struct MyService {
    name: String,
}

impl MyService {
    /// Create a new service
    pub fn new() -> Self {
        Self { name: "default".to_string() }
    }

    /// Run the service
    pub fn run(&self) {
        println!("Running {}", self.name);
    }
}

pub trait Handler {
    fn handle(&self, input: &str) -> String;
}

pub enum Status {
    Active,
    Inactive,
}

const VERSION: &str = "1.0.0";
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("helper.py"),
        r#"
"""Helper utilities module."""

class DataProcessor:
    """Processes incoming data."""

    def __init__(self, config: dict):
        """Initialize with config."""
        self.config = config

    def process(self, data: list) -> list:
        """Process a list of data items."""
        return [self._transform(d) for d in data]

    def _transform(self, item):
        """Internal transform."""
        return item

def create_processor(config: dict) -> DataProcessor:
    """Factory function."""
    return DataProcessor(config)
"#,
    )
    .unwrap();

    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }
"#,
    )
    .unwrap();

    dir
}

#[tokio::test]
async fn test_full_pipeline_generate() {
    let repo = setup_test_repo();
    let config = WikiGenConfig::default();
    let generator = WikiGenerator::new(config);

    generator.generate(repo.path()).await.unwrap();

    let output = repo.path().join("wiki-output");
    assert!(output.exists());
    assert!(output.join("index.md").exists());
    assert!(output.join("architecture.md").exists());
    assert!(output.join("wiki_index.json").exists());
    assert!(output.join("modules").exists());

    let index_content = std::fs::read_to_string(output.join("index.md")).unwrap();
    assert!(index_content.contains("Project Wiki"));
    assert!(index_content.contains("Rust"));

    let arch_content = std::fs::read_to_string(output.join("architecture.md")).unwrap();
    assert!(arch_content.contains("Architecture"));
    assert!(arch_content.contains("mermaid"));

    let wiki_index = std::fs::read_to_string(output.join("wiki_index.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&wiki_index).unwrap();
    assert!(parsed["pages"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn test_parse_only() {
    let repo = setup_test_repo();
    let config = WikiGenConfig::default();
    let generator = WikiGenerator::new(config);

    let ir = generator.parse_only(repo.path()).unwrap();

    assert!(!ir.files.is_empty());
    assert!(!ir.symbols.is_empty());

    let rust_files: Vec<_> = ir
        .files
        .iter()
        .filter(|f| matches!(f.language, memoryos_wiki_gen::ir::Language::Rust))
        .collect();
    assert!(!rust_files.is_empty());

    let python_files: Vec<_> = ir
        .files
        .iter()
        .filter(|f| matches!(f.language, memoryos_wiki_gen::ir::Language::Python))
        .collect();
    assert!(!python_files.is_empty());

    let names: Vec<&str> = ir
        .symbols
        .iter()
        .map(|s| s.qualified_name.as_str())
        .collect();
    assert!(names.iter().any(|n| n.contains("MyService")));
    assert!(names.iter().any(|n| n.contains("DataProcessor")));
}

#[tokio::test]
async fn test_incremental_generation() {
    let repo = setup_test_repo();
    let config = WikiGenConfig::default();
    let generator = WikiGenerator::new(config);

    generator.generate(repo.path()).await.unwrap();

    let output = repo.path().join("wiki-output");
    let first_index = std::fs::read_to_string(output.join("index.md")).unwrap();

    generator.generate(repo.path()).await.unwrap();

    let second_index = std::fs::read_to_string(output.join("index.md")).unwrap();
    assert!(!first_index.is_empty());
    assert!(!second_index.is_empty());
}

#[tokio::test]
async fn test_cache_persistence() {
    let repo = setup_test_repo();
    let config = WikiGenConfig::default();
    let generator = WikiGenerator::new(config);

    generator.generate(repo.path()).await.unwrap();

    let cache_dir = repo.path().join(".wiki-cache");
    assert!(cache_dir.exists());
    assert!(cache_dir.join("cache.json").exists());

    let cache_content = std::fs::read_to_string(cache_dir.join("cache.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&cache_content).unwrap();
    assert_eq!(parsed["version"], 1);
}

#[tokio::test]
async fn test_empty_repo() {
    let dir = tempfile::tempdir().unwrap();
    let config = WikiGenConfig::default();
    let generator = WikiGenerator::new(config);

    generator.generate(dir.path()).await.unwrap();

    let output = dir.path().join("wiki-output");
    assert!(output.join("index.md").exists());
}

#[tokio::test]
async fn test_graph_construction_from_parsed_ir() {
    let repo = setup_test_repo();
    let config = WikiGenConfig::default();
    let generator = WikiGenerator::new(config);

    let ir = generator.parse_only(repo.path()).unwrap();
    let graph = memoryos_wiki_gen::graph::CodeGraph::build_from_ir(&ir);

    assert!(graph.node_count() > 0);
    assert!(graph.edge_count() > 0);
    assert!(!graph.file_nodes().is_empty());
    assert!(!graph.symbol_nodes().is_empty());
}

#[tokio::test]
async fn test_manifest_parsing() {
    let repo = setup_test_repo();
    let config = WikiGenConfig::default();
    let generator = WikiGenerator::new(config);

    let ir = generator.parse_only(repo.path()).unwrap();

    assert!(!ir.manifests.is_empty());
    let cargo_manifest = ir
        .manifests
        .iter()
        .find(|m| matches!(m.ecosystem, memoryos_wiki_gen::ir::Ecosystem::Cargo));
    assert!(cargo_manifest.is_some());
    let manifest = cargo_manifest.unwrap();
    let dep_names: Vec<&str> = manifest
        .dependencies
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    assert!(dep_names.contains(&"serde"));
    assert!(dep_names.contains(&"tokio"));
}

#[tokio::test]
async fn test_custom_output_dir() {
    let repo = setup_test_repo();
    let mut config = WikiGenConfig::default();
    config.output.output_dir = "custom-wiki".to_string();
    let generator = WikiGenerator::new(config);

    generator.generate(repo.path()).await.unwrap();

    let output = repo.path().join("custom-wiki");
    assert!(output.exists());
    assert!(output.join("index.md").exists());
}

#[tokio::test]
async fn test_page_builder_generates_module_pages() {
    use memoryos_wiki_gen::graph::CodeGraph;
    use memoryos_wiki_gen::page_builder::PageBuilder;
    use std::collections::HashMap;

    let repo = setup_test_repo();
    let config = WikiGenConfig::default();
    let generator = WikiGenerator::new(config);

    let ir = generator.parse_only(repo.path()).unwrap();
    let graph = CodeGraph::build_from_ir(&ir);
    let symbol_docs = HashMap::new();

    let builder = PageBuilder::new().unwrap();
    let pages = builder
        .build_all(&ir, &graph, &symbol_docs, "Test overview", "test-commit")
        .unwrap();

    let module_pages: Vec<_> = pages
        .iter()
        .filter(|p| p.path.starts_with("modules/"))
        .collect();
    assert!(!module_pages.is_empty());

    for page in &module_pages {
        assert!(page.content.contains("Module:"));
        assert!(page.content.contains("Symbols"));
    }
}

#[tokio::test]
async fn test_wiki_index_contains_all_pages() {
    use memoryos_wiki_gen::wiki_index::WikiIndex;

    let repo = setup_test_repo();
    let config = WikiGenConfig::default();
    let generator = WikiGenerator::new(config);

    generator.generate(repo.path()).await.unwrap();

    let wiki_index_path = repo.path().join("wiki-output/wiki_index.json");
    let content = std::fs::read_to_string(wiki_index_path).unwrap();
    let index: WikiIndex = serde_json::from_str(&content).unwrap();

    assert!(index.pages.iter().any(|p| p.path == "index.md"));
    assert!(index.pages.iter().any(|p| p.path == "architecture.md"));

    for page in &index.pages {
        assert!(!page.content_hash.is_empty());
    }
}
