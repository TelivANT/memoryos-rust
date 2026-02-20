use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoIR {
    pub repo_root: PathBuf,
    pub files: Vec<FileIR>,
    pub symbols: Vec<Symbol>,
    pub references: Vec<Reference>,
    pub endpoints: Vec<Endpoint>,
    pub diagnostics: Vec<Diagnostic>,
    pub manifests: Vec<ManifestInfo>,
    pub specs: Vec<ApiSpec>,
}

impl RepoIR {
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            files: Vec::new(),
            symbols: Vec::new(),
            references: Vec::new(),
            endpoints: Vec::new(),
            diagnostics: Vec::new(),
            manifests: Vec::new(),
            specs: Vec::new(),
        }
    }

    pub fn symbols_in_file(&self, path: &PathBuf) -> Vec<&Symbol> {
        self.symbols.iter().filter(|s| &s.file == path).collect()
    }

    pub fn public_symbols(&self) -> Vec<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| matches!(s.visibility, Visibility::Public))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIR {
    pub path: PathBuf,
    pub language: Language,
    pub content_hash: String,
    pub parse_status: ParseStatus,
    pub byte_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub qualified_name: String,
    pub visibility: Visibility,
    pub file: PathBuf,
    pub span: Span,
    pub signature: Option<String>,
    pub doc: Option<Doc>,
    pub parent: Option<SymbolId>,
    pub children: Vec<SymbolId>,
    pub annotations: Vec<Annotation>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId {
    pub file_path: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub kind: SymbolKind,
    qualified_name_cache: String,
}

impl SymbolId {
    pub fn new(
        file_path: String,
        start_byte: usize,
        end_byte: usize,
        kind: SymbolKind,
        qualified_name: String,
    ) -> Self {
        Self {
            file_path,
            start_byte,
            end_byte,
            kind,
            qualified_name_cache: qualified_name,
        }
    }

    pub fn qualified_name(&self) -> &str {
        &self.qualified_name_cache
    }

    pub fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}:{:?}",
            self.file_path, self.start_byte, self.end_byte, self.kind
        )
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.qualified_name_cache)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub source: SymbolId,
    pub target: ReferenceTarget,
    pub kind: ReferenceKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceTarget {
    Resolved(SymbolId),
    Unresolved(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceKind {
    Import,
    Call,
    Extends,
    Implements,
    UsesType,
    FieldType,
    AnnotationUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub method: HttpMethod,
    pub path: String,
    pub handler: Option<SymbolId>,
    pub source: EndpointSource,
    pub file: PathBuf,
    pub span: Span,
    pub request_type: Option<TypeRef>,
    pub response_type: Option<TypeRef>,
    pub auth: AuthInfo,
    pub tags: Vec<String>,
    pub doc: Option<Doc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    All,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Delete => write!(f, "DELETE"),
            Self::Patch => write!(f, "PATCH"),
            Self::All => write!(f, "ALL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EndpointSource {
    CodeExtraction(String),
    Spec(SpecKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecKind {
    OpenApi,
    Proto,
    Swagger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeRef {
    SymbolRef(SymbolId),
    Named(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInfo {
    pub signals: Vec<String>,
    pub classification: AuthClassification,
}

impl Default for AuthInfo {
    fn default() -> Self {
        Self {
            signals: Vec::new(),
            classification: AuthClassification::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthClassification {
    Required,
    Optional,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Doc {
    pub raw: String,
    pub format: DocFormat,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocFormat {
    Markdown,
    JavaDoc,
    ReST,
    JSDoc,
    RustDoc,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

impl Span {
    pub fn new(start_byte: usize, end_byte: usize, start_line: usize, end_line: usize) -> Self {
        Self {
            start_byte,
            end_byte,
            start_line,
            end_line,
        }
    }

    pub fn offset(&self, byte_offset: usize, line_offset: usize) -> Self {
        Self {
            start_byte: self.start_byte + byte_offset,
            end_byte: self.end_byte + byte_offset,
            start_line: self.start_line + line_offset,
            end_line: self.end_line + line_offset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Struct,
    Trait,
    Class,
    Interface,
    Method,
    Module,
    Enum,
    EnumVariant,
    Constant,
    TypeAlias,
    Component,
    Constructor,
    Field,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function => write!(f, "function"),
            Self::Struct => write!(f, "struct"),
            Self::Trait => write!(f, "trait"),
            Self::Class => write!(f, "class"),
            Self::Interface => write!(f, "interface"),
            Self::Method => write!(f, "method"),
            Self::Module => write!(f, "module"),
            Self::Enum => write!(f, "enum"),
            Self::EnumVariant => write!(f, "enum_variant"),
            Self::Constant => write!(f, "constant"),
            Self::TypeAlias => write!(f, "type_alias"),
            Self::Component => write!(f, "component"),
            Self::Constructor => write!(f, "constructor"),
            Self::Field => write!(f, "field"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    Java,
    TypeScript,
    JavaScript,
    Vue,
    Html,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "Rust"),
            Self::Python => write!(f, "Python"),
            Self::Java => write!(f, "Java"),
            Self::TypeScript => write!(f, "TypeScript"),
            Self::JavaScript => write!(f, "JavaScript"),
            Self::Vue => write!(f, "Vue"),
            Self::Html => write!(f, "HTML"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParseStatus {
    Success,
    PartialSuccess,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: PathBuf,
    pub severity: DiagSeverity,
    pub message: String,
    pub span: Option<Span>,
    pub fallback_data: Option<FallbackData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackData {
    pub import_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub name: String,
    pub arguments: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestInfo {
    pub ecosystem: Ecosystem,
    pub source_file: PathBuf,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_req: Option<String>,
    pub scope: DependencyScope,
    pub source_file: PathBuf,
    pub ecosystem: Ecosystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyScope {
    Runtime,
    Dev,
    Build,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ecosystem {
    Cargo,
    Maven,
    Npm,
    Pip,
    Gradle,
}

impl std::fmt::Display for Ecosystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cargo => write!(f, "Cargo"),
            Self::Maven => write!(f, "Maven"),
            Self::Npm => write!(f, "npm"),
            Self::Pip => write!(f, "pip"),
            Self::Gradle => write!(f, "Gradle"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSpec {
    pub kind: SpecKind,
    pub file: PathBuf,
    pub endpoints: Vec<Endpoint>,
}
