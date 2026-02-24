use std::collections::HashMap;
use std::path::PathBuf;

use memoryos_wiki_gen::cache::{compute_hash, CacheStore};
use memoryos_wiki_gen::config::WikiGenConfig;
use memoryos_wiki_gen::evidence::{
    build_evidence_pack, format_evidence_prompt, EvidenceRef, LlmDocResult,
};
use memoryos_wiki_gen::graph::CodeGraph;
use memoryos_wiki_gen::ir::*;
use memoryos_wiki_gen::page_builder::PageBuilder;
use memoryos_wiki_gen::parser::{create_parser, LanguageParser};

fn make_symbol(name: &str, kind: SymbolKind, vis: Visibility, file: &str) -> Symbol {
    let file_path = PathBuf::from(file);
    Symbol {
        id: SymbolId::new(file.to_string(), 0, 100, kind, name.to_string()),
        kind,
        qualified_name: name.to_string(),
        visibility: vis,
        file: file_path,
        span: Span::new(0, 100, 1, 10),
        signature: Some(format!("{} {}", kind, name)),
        doc: Some(Doc {
            raw: format!("Documentation for {}", name),
            format: DocFormat::RustDoc,
            summary: Some(format!("Summary of {}", name)),
        }),
        parent: None,
        children: Vec::new(),
        annotations: Vec::new(),
        type_params: Vec::new(),
    }
}

fn make_ir() -> RepoIR {
    let mut ir = RepoIR::new(PathBuf::from("/test/repo"));

    ir.files.push(FileIR {
        path: PathBuf::from("src/lib.rs"),
        language: Language::Rust,
        content_hash: "abc123".to_string(),
        parse_status: ParseStatus::Success,
        byte_count: 1000,
    });
    ir.files.push(FileIR {
        path: PathBuf::from("src/main.rs"),
        language: Language::Rust,
        content_hash: "def456".to_string(),
        parse_status: ParseStatus::Success,
        byte_count: 500,
    });

    ir.symbols.push(make_symbol(
        "MyStruct",
        SymbolKind::Struct,
        Visibility::Public,
        "src/lib.rs",
    ));
    ir.symbols.push(make_symbol(
        "MyTrait",
        SymbolKind::Trait,
        Visibility::Public,
        "src/lib.rs",
    ));
    ir.symbols.push(make_symbol(
        "my_function",
        SymbolKind::Function,
        Visibility::Public,
        "src/main.rs",
    ));
    ir.symbols.push(make_symbol(
        "_private_fn",
        SymbolKind::Function,
        Visibility::Private,
        "src/main.rs",
    ));

    ir.references.push(Reference {
        source: ir.symbols[2].id.clone(),
        target: ReferenceTarget::Unresolved("MyStruct".to_string()),
        kind: ReferenceKind::UsesType,
        span: Span::new(10, 20, 2, 2),
    });

    ir
}

// ── IR Tests ─────────────────────────────────────────────────────

#[test]
fn test_repo_ir_new() {
    let ir = RepoIR::new(PathBuf::from("/test"));
    assert!(ir.files.is_empty());
    assert!(ir.symbols.is_empty());
    assert!(ir.references.is_empty());
    assert!(ir.endpoints.is_empty());
    assert!(ir.diagnostics.is_empty());
}

#[test]
fn test_repo_ir_public_symbols() {
    let ir = make_ir();
    let public = ir.public_symbols();
    assert_eq!(public.len(), 3);
    assert!(public
        .iter()
        .all(|s| matches!(s.visibility, Visibility::Public)));
}

#[test]
fn test_repo_ir_symbols_in_file() {
    let ir = make_ir();
    let lib_symbols = ir.symbols_in_file(&PathBuf::from("src/lib.rs"));
    assert_eq!(lib_symbols.len(), 2);
    let main_symbols = ir.symbols_in_file(&PathBuf::from("src/main.rs"));
    assert_eq!(main_symbols.len(), 2);
}

#[test]
fn test_symbol_id_stable_key() {
    let id = SymbolId::new(
        "src/lib.rs".to_string(),
        0,
        100,
        SymbolKind::Struct,
        "MyStruct".to_string(),
    );
    let key = id.stable_key();
    assert!(key.contains("src/lib.rs"));
    assert!(key.contains("0"));
    assert!(key.contains("100"));
    assert!(key.contains("Struct"));
}

#[test]
fn test_symbol_id_qualified_name() {
    let id = SymbolId::new(
        "f.rs".to_string(),
        0,
        10,
        SymbolKind::Function,
        "foo::bar".to_string(),
    );
    assert_eq!(id.qualified_name(), "foo::bar");
}

#[test]
fn test_span_new_and_offset() {
    let span = Span::new(10, 50, 2, 5);
    assert_eq!(span.start_byte, 10);
    assert_eq!(span.end_byte, 50);
    assert_eq!(span.start_line, 2);
    assert_eq!(span.end_line, 5);

    let offset = span.offset(100, 10);
    assert_eq!(offset.start_byte, 110);
    assert_eq!(offset.end_byte, 150);
    assert_eq!(offset.start_line, 12);
    assert_eq!(offset.end_line, 15);
}

#[test]
fn test_http_method_display() {
    assert_eq!(HttpMethod::Get.to_string(), "GET");
    assert_eq!(HttpMethod::Post.to_string(), "POST");
    assert_eq!(HttpMethod::Put.to_string(), "PUT");
    assert_eq!(HttpMethod::Delete.to_string(), "DELETE");
    assert_eq!(HttpMethod::Patch.to_string(), "PATCH");
    assert_eq!(HttpMethod::All.to_string(), "ALL");
}

#[test]
fn test_symbol_kind_display() {
    assert_eq!(SymbolKind::Function.to_string(), "function");
    assert_eq!(SymbolKind::Struct.to_string(), "struct");
    assert_eq!(SymbolKind::Trait.to_string(), "trait");
    assert_eq!(SymbolKind::Class.to_string(), "class");
    assert_eq!(SymbolKind::Enum.to_string(), "enum");
    assert_eq!(SymbolKind::Module.to_string(), "module");
    assert_eq!(SymbolKind::Component.to_string(), "component");
}

// ── Graph Tests ──────────────────────────────────────────────────

#[test]
fn test_code_graph_new() {
    let graph = CodeGraph::new();
    assert_eq!(graph.node_count(), 0);
    assert_eq!(graph.edge_count(), 0);
}

#[test]
fn test_code_graph_build_from_ir() {
    let ir = make_ir();
    let graph = CodeGraph::build_from_ir(&ir);
    assert!(graph.node_count() > 0);
    assert_eq!(graph.file_nodes().len(), 2);
    assert_eq!(graph.symbol_nodes().len(), 4);
}

#[test]
fn test_code_graph_file_contains_symbol_edges() {
    let ir = make_ir();
    let graph = CodeGraph::build_from_ir(&ir);
    assert!(graph.edge_count() > 0);
}

#[test]
fn test_code_graph_endpoint_nodes() {
    let mut ir = make_ir();
    ir.endpoints.push(Endpoint {
        method: HttpMethod::Get,
        path: "/api/test".to_string(),
        handler: Some(ir.symbols[2].id.clone()),
        source: EndpointSource::CodeExtraction("axum".to_string()),
        file: PathBuf::from("src/main.rs"),
        span: Span::new(0, 50, 1, 5),
        request_type: None,
        response_type: None,
        auth: AuthInfo::default(),
        tags: Vec::new(),
        doc: None,
    });
    let graph = CodeGraph::build_from_ir(&ir);
    assert_eq!(graph.endpoint_nodes().len(), 1);
}

#[test]
fn test_code_graph_subgraph() {
    let ir = make_ir();
    let graph = CodeGraph::build_from_ir(&ir);
    let file_nodes = graph.file_nodes();
    if let Some((idx, _)) = file_nodes.first() {
        let sub = graph.subgraph(*idx, 2);
        assert!(!sub.is_empty());
    }
}

#[test]
fn test_code_graph_endpoints_by_tag() {
    let mut ir = make_ir();
    ir.endpoints.push(Endpoint {
        method: HttpMethod::Get,
        path: "/api/users".to_string(),
        handler: None,
        source: EndpointSource::CodeExtraction("test".to_string()),
        file: PathBuf::from("src/main.rs"),
        span: Span::new(0, 50, 1, 5),
        request_type: None,
        response_type: None,
        auth: AuthInfo::default(),
        tags: Vec::new(),
        doc: None,
    });
    ir.endpoints.push(Endpoint {
        method: HttpMethod::Post,
        path: "/api/users".to_string(),
        handler: None,
        source: EndpointSource::CodeExtraction("test".to_string()),
        file: PathBuf::from("src/main.rs"),
        span: Span::new(0, 50, 1, 5),
        request_type: None,
        response_type: None,
        auth: AuthInfo::default(),
        tags: Vec::new(),
        doc: None,
    });
    let graph = CodeGraph::build_from_ir(&ir);
    let by_tag = graph.endpoints_by_tag();
    assert!(by_tag.contains_key("api"));
}

// ── Parser Tests ─────────────────────────────────────────────────

#[test]
fn test_rust_parser_basic() {
    let parser = create_parser(Language::Rust);
    assert_eq!(parser.language(), Language::Rust);

    let source = br#"
/// A test struct
pub struct TestStruct {
    pub field: String,
}

/// A test function
pub fn test_function(x: i32) -> i32 {
    x + 1
}

fn private_function() {}

pub enum MyEnum {
    VariantA,
    VariantB,
}

pub trait MyTrait {
    fn do_something(&self);
}
"#;

    let output = parser
        .parse(source, std::path::Path::new("src/test.rs"))
        .unwrap();
    assert_eq!(output.file.language, Language::Rust);
    assert_eq!(output.file.parse_status, ParseStatus::Success);
    assert!(!output.symbols.is_empty());

    let names: Vec<&str> = output
        .symbols
        .iter()
        .map(|s| s.qualified_name.as_str())
        .collect();
    assert!(names.iter().any(|n| n.contains("TestStruct")));
    assert!(names.iter().any(|n| n.contains("test_function")));
    assert!(names.iter().any(|n| n.contains("MyEnum")));
    assert!(names.iter().any(|n| n.contains("MyTrait")));

    let pub_symbols: Vec<_> = output
        .symbols
        .iter()
        .filter(|s| matches!(s.visibility, Visibility::Public))
        .collect();
    assert!(pub_symbols.len() >= 4);
}

#[test]
fn test_rust_parser_doc_comments() {
    let parser = create_parser(Language::Rust);
    let source = br#"
/// This is a doc comment
/// with multiple lines
pub fn documented_fn() {}
"#;
    let output = parser
        .parse(source, std::path::Path::new("src/test.rs"))
        .unwrap();
    let func = output
        .symbols
        .iter()
        .find(|s| s.qualified_name.contains("documented_fn"))
        .unwrap();
    assert!(func.doc.is_some());
    let doc = func.doc.as_ref().unwrap();
    assert!(doc.raw.contains("doc comment"));
}

#[test]
fn test_rust_parser_impl_methods() {
    let parser = create_parser(Language::Rust);
    let source = br#"
pub struct Foo {}

impl Foo {
    pub fn bar(&self) -> i32 {
        42
    }
    fn private_method(&self) {}
}
"#;
    let output = parser
        .parse(source, std::path::Path::new("src/test.rs"))
        .unwrap();
    let methods: Vec<_> = output
        .symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Method))
        .collect();
    assert!(!methods.is_empty());
}

#[test]
fn test_python_parser_basic() {
    let parser = create_parser(Language::Python);
    assert_eq!(parser.language(), Language::Python);

    let source = br#"
class MyClass:
    """A test class."""

    def method(self):
        """A method."""
        pass

def standalone_function(x: int) -> int:
    """Return x + 1."""
    return x + 1

def _private_function():
    pass
"#;

    let output = parser
        .parse(source, std::path::Path::new("src/test.py"))
        .unwrap();
    assert_eq!(output.file.language, Language::Python);
    assert!(!output.symbols.is_empty());

    let names: Vec<&str> = output
        .symbols
        .iter()
        .map(|s| s.qualified_name.as_str())
        .collect();
    assert!(names.iter().any(|n| n.contains("MyClass")));
    assert!(names.iter().any(|n| n.contains("standalone_function")));

    let private: Vec<_> = output
        .symbols
        .iter()
        .filter(|s| s.qualified_name.contains("_private_function"))
        .collect();
    assert!(!private.is_empty());
    assert!(matches!(private[0].visibility, Visibility::Private));
}

#[test]
fn test_python_parser_docstrings() {
    let parser = create_parser(Language::Python);
    let source = br#"
def documented(x):
    """This function does something."""
    return x
"#;
    let output = parser
        .parse(source, std::path::Path::new("test.py"))
        .unwrap();
    let func = output
        .symbols
        .iter()
        .find(|s| s.qualified_name.contains("documented"))
        .unwrap();
    assert!(func.doc.is_some());
    assert!(func.doc.as_ref().unwrap().raw.contains("does something"));
}

#[test]
fn test_java_parser_basic() {
    let parser = create_parser(Language::Java);
    assert_eq!(parser.language(), Language::Java);

    let source = br#"
package com.example;

public class MyService {
    public void doSomething() {
    }

    private int helper() {
        return 42;
    }
}
"#;
    let output = parser
        .parse(source, std::path::Path::new("src/MyService.java"))
        .unwrap();
    assert_eq!(output.file.language, Language::Java);
    assert!(!output.symbols.is_empty());
    let names: Vec<&str> = output
        .symbols
        .iter()
        .map(|s| s.qualified_name.as_str())
        .collect();
    assert!(names.iter().any(|n| n.contains("MyService")));
}

#[test]
fn test_typescript_parser_basic() {
    let parser = create_parser(Language::TypeScript);
    assert_eq!(parser.language(), Language::TypeScript);

    let source = br#"
export interface UserService {
    getUser(id: string): Promise<User>;
}

export class UserServiceImpl implements UserService {
    async getUser(id: string): Promise<User> {
        return {} as User;
    }
}

export function createUser(name: string): User {
    return { name };
}
"#;
    let output = parser
        .parse(source, std::path::Path::new("src/service.ts"))
        .unwrap();
    assert!(!output.symbols.is_empty());
}

#[test]
fn test_create_parser_for_all_languages() {
    let _rust = create_parser(Language::Rust);
    let _python = create_parser(Language::Python);
    let _java = create_parser(Language::Java);
    let _ts = create_parser(Language::TypeScript);
    let _js = create_parser(Language::JavaScript);
    let _vue = create_parser(Language::Vue);
}

// ── Evidence Tests ───────────────────────────────────────────────

#[test]
fn test_build_evidence_pack() {
    let ir = make_ir();
    let graph = CodeGraph::build_from_ir(&ir);
    let source = "pub struct MyStruct {\n    field: String,\n}\n";
    let imports = "use std::string::String;\n";
    let pack = build_evidence_pack(&ir.symbols[0], source, &graph, imports);
    assert_eq!(pack.symbol.qualified_name, "MyStruct");
    assert_eq!(pack.symbol.kind, "struct");
    assert!(!pack.source_snippet.is_empty());
    assert!(!pack.file_context.is_empty());
}

#[test]
fn test_format_evidence_prompt() {
    let ir = make_ir();
    let graph = CodeGraph::build_from_ir(&ir);
    let source = "pub struct MyStruct {}\n";
    let pack = build_evidence_pack(&ir.symbols[0], source, &graph, "");
    let prompt = format_evidence_prompt(&pack);
    assert!(prompt.contains("MyStruct"));
    assert!(prompt.contains("struct"));
    assert!(prompt.contains("Source Code"));
}

// ── Cache Tests ──────────────────────────────────────────────────

#[test]
fn test_cache_store_new() {
    let cache = CacheStore::new();
    assert_eq!(cache.version, 1);
    assert!(cache.entries.is_empty());
}

#[test]
fn test_cache_store_insert_and_lookup() {
    let mut cache = CacheStore::new();
    let result = LlmDocResult {
        summary: "Test summary".to_string(),
        detailed: "Test detailed".to_string(),
        usage_example: None,
        sources: vec![EvidenceRef {
            file: "test.rs".to_string(),
            start_line: 1,
            end_line: 10,
        }],
    };
    cache.insert(
        "sym1".to_string(),
        "prompt1".to_string(),
        "content1".to_string(),
        result,
    );
    assert!(cache.lookup("sym1", "prompt1").is_some());
    assert!(cache.lookup("sym1", "wrong_prompt").is_none());
    assert!(cache.lookup("wrong_sym", "prompt1").is_none());
}

#[test]
fn test_cache_store_save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let mut cache = CacheStore::new();
    let result = LlmDocResult {
        summary: "Test".to_string(),
        detailed: "Detailed".to_string(),
        usage_example: Some("example()".to_string()),
        sources: Vec::new(),
    };
    cache.insert(
        "key".to_string(),
        "ph".to_string(),
        "ch".to_string(),
        result,
    );
    cache.save(dir.path()).unwrap();

    let loaded = CacheStore::load(dir.path()).unwrap();
    assert_eq!(loaded.entries.len(), 1);
    assert!(loaded.lookup("key", "ph").is_some());
}

#[test]
fn test_compute_hash_deterministic() {
    let h1 = compute_hash("hello world");
    let h2 = compute_hash("hello world");
    let h3 = compute_hash("different");
    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
}

// ── Diagram Tests ────────────────────────────────────────────────

#[test]
fn test_module_dependency_diagram() {
    use memoryos_wiki_gen::diagram::DiagramGenerator;
    let ir = make_ir();
    let graph = CodeGraph::build_from_ir(&ir);
    let diagram = DiagramGenerator::module_dependency_diagram(&graph);
    assert!(diagram.starts_with("graph TD"));
}

#[test]
fn test_class_diagram() {
    use memoryos_wiki_gen::diagram::DiagramGenerator;
    let ir = make_ir();
    let diagram = DiagramGenerator::class_diagram(&ir.symbols, &ir.references);
    assert!(diagram.starts_with("classDiagram"));
    assert!(diagram.contains("MyStruct"));
}

#[test]
fn test_api_flow_diagram_empty() {
    use memoryos_wiki_gen::diagram::DiagramGenerator;
    let graph = CodeGraph::new();
    let diagram = DiagramGenerator::api_flow_diagram(&graph);
    assert!(diagram.starts_with("graph LR"));
}

#[test]
fn test_crate_dependency_diagram() {
    use memoryos_wiki_gen::diagram::DiagramGenerator;
    let manifests = vec![ManifestInfo {
        ecosystem: Ecosystem::Cargo,
        source_file: PathBuf::from("crates/foo/Cargo.toml"),
        dependencies: vec![
            Dependency {
                name: "serde".to_string(),
                version_req: Some("1.0".to_string()),
                scope: DependencyScope::Runtime,
                source_file: PathBuf::from("crates/foo/Cargo.toml"),
                ecosystem: Ecosystem::Cargo,
            },
            Dependency {
                name: "tokio".to_string(),
                version_req: Some("1.0".to_string()),
                scope: DependencyScope::Runtime,
                source_file: PathBuf::from("crates/foo/Cargo.toml"),
                ecosystem: Ecosystem::Cargo,
            },
        ],
    }];
    let diagram = DiagramGenerator::crate_dependency_diagram(&manifests);
    assert!(diagram.contains("serde"));
    assert!(diagram.contains("tokio"));
}

// ── Page Builder Tests ───────────────────────────────────────────

#[test]
fn test_page_builder_new() {
    let builder = PageBuilder::new();
    assert!(builder.is_ok());
}

#[test]
fn test_page_builder_build_all() {
    let ir = make_ir();
    let graph = CodeGraph::build_from_ir(&ir);
    let symbol_docs = HashMap::new();
    let overview = "# Test Overview";
    let commit = "abc123";

    let builder = PageBuilder::new().unwrap();
    let pages = builder
        .build_all(&ir, &graph, &symbol_docs, overview, commit)
        .unwrap();
    assert!(!pages.is_empty());

    let page_paths: Vec<&str> = pages.iter().map(|p| p.path.as_str()).collect();
    assert!(page_paths.contains(&"index.md"));
    assert!(page_paths.contains(&"architecture.md"));
}

#[test]
fn test_page_builder_with_endpoints() {
    let mut ir = make_ir();
    ir.endpoints.push(Endpoint {
        method: HttpMethod::Get,
        path: "/api/test".to_string(),
        handler: None,
        source: EndpointSource::CodeExtraction("test".to_string()),
        file: PathBuf::from("src/main.rs"),
        span: Span::new(0, 50, 1, 5),
        request_type: None,
        response_type: None,
        auth: AuthInfo::default(),
        tags: Vec::new(),
        doc: None,
    });
    let graph = CodeGraph::build_from_ir(&ir);
    let symbol_docs = HashMap::new();

    let builder = PageBuilder::new().unwrap();
    let pages = builder
        .build_all(&ir, &graph, &symbol_docs, "Overview", "abc")
        .unwrap();
    let page_paths: Vec<&str> = pages.iter().map(|p| p.path.as_str()).collect();
    assert!(page_paths.contains(&"api.md"));
}

#[test]
fn test_page_content_contains_expected_sections() {
    let ir = make_ir();
    let graph = CodeGraph::build_from_ir(&ir);
    let symbol_docs = HashMap::new();

    let builder = PageBuilder::new().unwrap();
    let pages = builder
        .build_all(&ir, &graph, &symbol_docs, "Overview text", "commit123")
        .unwrap();

    let index = pages.iter().find(|p| p.path == "index.md").unwrap();
    assert!(index.content.contains("Project Wiki"));
    assert!(index.content.contains("Overview text"));
    assert!(index.content.contains("Rust"));

    let arch = pages.iter().find(|p| p.path == "architecture.md").unwrap();
    assert!(arch.content.contains("Architecture"));
    assert!(arch.content.contains("mermaid"));
}

// ── Export Tests ─────────────────────────────────────────────────

#[test]
fn test_export_local() {
    use memoryos_wiki_gen::export::ExportOrchestrator;
    use memoryos_wiki_gen::page_builder::GeneratedPage;
    use memoryos_wiki_gen::wiki_index::WikiIndex;

    let dir = tempfile::tempdir().unwrap();
    let pages = vec![
        GeneratedPage {
            path: "index.md".to_string(),
            content: "# Index".to_string(),
            symbols_referenced: Vec::new(),
        },
        GeneratedPage {
            path: "modules/test.md".to_string(),
            content: "# Test Module".to_string(),
            symbols_referenced: vec!["test::foo".to_string()],
        },
    ];
    let index = WikiIndex::build(&pages, "abc123");
    ExportOrchestrator::export_local(&pages, &index, dir.path()).unwrap();

    assert!(dir.path().join("index.md").exists());
    assert!(dir.path().join("modules/test.md").exists());
    assert!(dir.path().join("wiki_index.json").exists());
}

#[test]
fn test_export_incremental_skips_unchanged() {
    use memoryos_wiki_gen::export::ExportOrchestrator;
    use memoryos_wiki_gen::page_builder::GeneratedPage;
    use memoryos_wiki_gen::wiki_index::WikiIndex;

    let dir = tempfile::tempdir().unwrap();
    let pages = vec![GeneratedPage {
        path: "index.md".to_string(),
        content: "# Index".to_string(),
        symbols_referenced: Vec::new(),
    }];
    let index = WikiIndex::build(&pages, "abc");

    ExportOrchestrator::export_local(&pages, &index, dir.path()).unwrap();

    let updated = ExportOrchestrator::export_incremental(&pages, &index, dir.path()).unwrap();
    assert_eq!(updated, 0);
}

// ── Config Tests ─────────────────────────────────────────────────

#[test]
fn test_config_defaults() {
    let config = WikiGenConfig::default();
    assert_eq!(config.llm.model, "gpt-4o");
    assert_eq!(config.llm.max_concurrent, 5);
    assert_eq!(config.llm.retry_max, 3);
    assert!(config.cache.enabled);
    assert_eq!(config.output.format, "markdown");
    assert_eq!(config.export.target, "local");
}

#[test]
fn test_config_language_enabled() {
    let config = WikiGenConfig::default();
    assert!(config.parse.language_enabled(Language::Rust));
    assert!(config.parse.language_enabled(Language::Python));
    assert!(config.parse.language_enabled(Language::Java));
    assert!(config.parse.language_enabled(Language::Vue));
}

#[test]
fn test_config_resolved_paths() {
    let config = WikiGenConfig::default();
    let repo = PathBuf::from("/my/repo");
    let output = config.resolved_output_dir(&repo);
    assert_eq!(output, PathBuf::from("/my/repo/wiki-output"));
    let cache = config.resolved_cache_dir(&repo);
    assert_eq!(cache, PathBuf::from("/my/repo/.wiki-cache"));
}

// ── LLM Adapter Tests ───────────────────────────────────────────

#[test]
fn test_wiki_llm_error_display() {
    use memoryos_wiki_gen::llm_adapter::WikiLlmError;
    let err = WikiLlmError::RequestFailed("timeout".to_string());
    assert!(err.to_string().contains("timeout"));
    let err2 = WikiLlmError::ResponseError("bad json".to_string());
    assert!(err2.to_string().contains("bad json"));
}

#[test]
fn test_chat_request_serialization() {
    use memoryos_wiki_gen::llm_adapter::{ChatMessage, ChatRequest};
    let req = ChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        }],
        temperature: Some(0.3),
        max_tokens: Some(1024),
        stream: false,
        extra: HashMap::new(),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("gpt-4o"));
    assert!(json.contains("Hello"));
}

// ── LLM Gen (Fallback) Tests ────────────────────────────────────

#[tokio::test]
async fn test_llm_gen_fallback_mode() {
    use memoryos_wiki_gen::config::LlmConfig;
    use memoryos_wiki_gen::llm_gen::LlmDocGenerator;

    let config = LlmConfig::default();
    let gen = LlmDocGenerator::new(config);
    let ir = make_ir();
    let graph = CodeGraph::build_from_ir(&ir);
    let file_contents = HashMap::new();
    let mut cache = CacheStore::new();

    let results = gen
        .generate_all(&ir, &graph, &file_contents, &mut cache)
        .await
        .unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_llm_gen_overview() {
    use memoryos_wiki_gen::config::LlmConfig;
    use memoryos_wiki_gen::llm_gen::LlmDocGenerator;

    let config = LlmConfig::default();
    let gen = LlmDocGenerator::new(config);
    let ir = make_ir();
    let symbol_docs = HashMap::new();

    let overview = gen.generate_overview(&ir, &symbol_docs).await.unwrap();
    assert!(overview.contains("Project Overview"));
    assert!(overview.contains("Rust"));
}

// ── Wiki Index Tests ─────────────────────────────────────────────

#[test]
fn test_wiki_index_build() {
    use memoryos_wiki_gen::page_builder::GeneratedPage;
    use memoryos_wiki_gen::wiki_index::WikiIndex;

    let pages = vec![
        GeneratedPage {
            path: "index.md".to_string(),
            content: "# Index content".to_string(),
            symbols_referenced: Vec::new(),
        },
        GeneratedPage {
            path: "modules/foo.md".to_string(),
            content: "# Foo module".to_string(),
            symbols_referenced: vec!["foo::bar".to_string()],
        },
    ];

    let index = WikiIndex::build(&pages, "abc123");
    assert_eq!(index.pages.len(), 2);
    assert_eq!(index.source_commit, "abc123");
}

#[test]
fn test_wiki_index_save_and_load() {
    use memoryos_wiki_gen::page_builder::GeneratedPage;
    use memoryos_wiki_gen::wiki_index::WikiIndex;

    let dir = tempfile::tempdir().unwrap();
    let pages = vec![GeneratedPage {
        path: "index.md".to_string(),
        content: "content".to_string(),
        symbols_referenced: Vec::new(),
    }];
    let index = WikiIndex::build(&pages, "commit");
    index.save(dir.path()).unwrap();
    assert!(dir.path().join("wiki_index.json").exists());
}

// ── Discovery Tests ──────────────────────────────────────────────

#[test]
fn test_file_discovery_on_temp_repo() {
    use memoryos_wiki_gen::discovery::FileDiscovery;

    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("lib.py"), "def foo(): pass").unwrap();
    std::fs::write(src_dir.join("readme.txt"), "Not a source file").unwrap();

    let config = WikiGenConfig::default();
    let discovery = FileDiscovery::new(config);
    let files = discovery.discover(dir.path()).unwrap();

    let extensions: Vec<String> = files
        .iter()
        .map(|f| {
            f.relative_path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert!(extensions.contains(&"rs".to_string()));
    assert!(extensions.contains(&"py".to_string()));
    assert!(!extensions.contains(&"txt".to_string()));
}
