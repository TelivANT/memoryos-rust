# Wiki Generation System Specification

> **Status**: Approved (Design Phase)
> **Priority**: P1
> **Crate**: `memoryos-wiki-gen`
> **Objective**: Multi-language code parsing + LLM documentation generation, producing structured Markdown Wiki from source code and FAQ memory data.

---

## 1. System Overview

### 1.1 Problem

1. Source code documentation is manual, quickly outdated, and scattered across READMEs, inline comments, and external wikis.
2. FAQ knowledge accumulated in MemoryOS remains locked in the vector database, inaccessible to traditional documentation systems.
3. No unified pipeline exists to automatically produce architecture diagrams, API docs, and knowledge base articles from a single codebase.

### 1.2 Solution

A **Tree-sitter + LLM hybrid pipeline** that:
- Parses multi-language codebases (V1: Rust, Python, Java, Vue) into a unified Intermediate Representation (IR)
- Builds a layered Code Graph (File / Symbol / Runtime)
- Uses LLM to generate human-readable documentation with source evidence
- Renders Markdown pages with Mermaid diagrams
- Merges FAQ wiki export into the same output pipeline
- Exports via CLI tool and HTTP API (dual-path)

### 1.3 Design Principles

| Principle | Description |
|-----------|-------------|
| **Symbol-centric IR** | Symbols are first-class citizens; files are containers |
| **Stable IDs** | SymbolId = file_path + span + kind (not just name) |
| **Graceful degradation** | Parse failures produce low-fidelity output, never crash the pipeline |
| **Incremental by default** | Content hashing at file/symbol/prompt level; only regenerate what changed |
| **Evidence-backed** | Every generated doc paragraph links back to source file + line range |
| **80% rule for inference** | Auth/middleware detection outputs signals, not conclusions |

---

## 2. High-Level Architecture

### 2.1 Pipeline Overview

```mermaid
graph TD
    A[Code Repository] --> B[Phase 0: Repo Intake]
    B --> C[Phase 1: Multi-Language Parsing]
    C --> D[Phase 1.5: API Endpoint Extraction]
    D --> E[Phase 2: Code Graph Construction]
    E --> F[Phase 3: LLM Documentation Generation]
    F --> G[Phase 4: Mermaid Diagram Generation]
    G --> H[Phase 5: Page Assembly]
    I[FAQ Memory Data] --> H
    H --> J[Phase 6: Export]
    J --> K[Local FS]
    J --> L[S3 / MinIO]
    J --> M[Confluence]

    style A fill:#e1f5fe
    style I fill:#fff3e0
    style K fill:#e8f5e9
    style L fill:#e8f5e9
    style M fill:#e8f5e9
```

### 2.2 Crate Position in Workspace

```mermaid
graph LR
    subgraph Workspace
        WG[memoryos-wiki-gen]
        CORE[memoryos-core]
        PORTS[memoryos-ports]
        ADAPTERS[memoryos-adapters]
        GW[memoryos-gateway]
    end

    WG -->|uses LlmAdapter trait| PORTS
    WG -->|uses WikiExportBackend| CORE
    WG -->|uses GraphEntity model| CORE
    GW -->|exposes /v1/wiki API| WG
    ADAPTERS -->|implements backends| CORE

    style WG fill:#ffecb3,stroke:#f57c00
```

### 2.3 Dual-Path Access

```mermaid
graph TD
    subgraph CLI Path
        CLI[memoryos-wiki-gen CLI] -->|clap args| PIPE[Pipeline Engine]
    end

    subgraph API Path
        GW[Gateway /v1/wiki/*] -->|axum handler| PIPE
    end

    PIPE --> OUT[Markdown Output]

    style CLI fill:#e3f2fd
    style GW fill:#fce4ec
```

---

## 3. Phase 0: Repo Intake & Runtime Control

### 3.1 Responsibilities

| Component | Tool | Purpose |
|-----------|------|---------|
| File Discovery | `ignore::WalkBuilder` | .gitignore-aware traversal |
| Language Detection | Extension map + fallback | Route files to correct parser |
| Parallel Parse | `rayon` | File-level parallelism for tree-sitter |
| CLI Progress | `indicatif` | MultiProgress bar (discover / parse / graph / llm / render) |
| LLM Throttle | `tokio::sync::Semaphore` | Limit concurrent LLM calls (default: 5) |
| Cache Layer | SHA256 content hash | Skip unchanged files/symbols/prompts |

### 3.2 File Discovery Strategy

```mermaid
flowchart TD
    START[Repo Root] --> WALK[ignore::WalkBuilder]
    WALK --> GITIGNORE{.gitignore rules}
    GITIGNORE -->|excluded| SKIP[Skip file]
    GITIGNORE -->|included| EXT{Extension check}
    EXT -->|.rs| RUST[Rust Parser]
    EXT -->|.py| PYTHON[Python Parser]
    EXT -->|.java| JAVA[Java Parser]
    EXT -->|.vue| VUE[Vue SFC Splitter]
    EXT -->|.ts/.tsx| TS[TypeScript Parser]
    EXT -->|.html| HTML[HTML Parser]
    EXT -->|other| META{Is manifest?}
    META -->|pom.xml/Cargo.toml/...| MANIFEST[Manifest Extractor]
    META -->|openapi.yaml/swagger.json| SPEC[API Spec Discovery]
    META -->|*.proto| PROTO[Proto Spec Discovery]
    META -->|no| IGNORE[Ignore]
```

### 3.3 CLI Override Rules

| Flag | Behavior |
|------|----------|
| `--no-ignore` | Disable .gitignore filtering |
| `--include-dot-github` | Parse `.github/workflows/*.yml` |
| `--include <path>` | Force-include a path even if ignored |
| `--exclude <glob>` | Additional exclusion pattern |
| `--lang <lang>` | Only parse specified languages |

### 3.4 Configuration File (`wiki-gen.toml`)

```toml
[repo]
root = "."
include_patterns = []
exclude_patterns = ["**/generated/**", "**/vendor/**"]

[parse]
languages = ["rust", "python", "java", "vue"]
max_file_size_kb = 512

[llm]
adapter = "openai"
model = "gpt-4o"
max_concurrent = 5
retry_max = 3
retry_backoff_ms = 1000

[cache]
enabled = true
cache_dir = ".wiki-cache"

[output]
format = "markdown"
output_dir = "wiki-output"
include_faq = true

[export]
target = "local"
# target = "s3"
# target = "confluence"
```

---

## 4. Phase 1: Multi-Language Parsing

### 4.1 Parser Architecture

```mermaid
graph TD
    subgraph "Language Parsers (Tree-sitter)"
        RP[RustParser<br>tree-sitter-rust]
        PP[PythonParser<br>tree-sitter-python]
        JP[JavaParser<br>tree-sitter-java]
        TP[TypeScriptParser<br>tree-sitter-typescript]
        HP[HtmlParser<br>tree-sitter-html]
    end

    subgraph "Special Handlers"
        VS[VueSfcSplitter<br>regex section split]
        VS -->|script lang=ts| TP
        VS -->|template| HP
    end

    RP --> IR[Unified RepoIR]
    PP --> IR
    JP --> IR
    TP --> IR
    HP --> IR
    VS --> IR
```

### 4.2 Unified Intermediate Representation (IR)

The IR is **Symbol-centric**, not file-centric. Files are containers; symbols are first-class citizens.

```
RepoIR
  repo_root: PathBuf
  files: Vec<FileIR>
  symbols: Vec<Symbol>
  references: Vec<Reference>
  endpoints: Vec<Endpoint>         // Phase 1.5 output
  diagnostics: Vec<Diagnostic>     // Parse failures, degradation markers
  manifests: Vec<ManifestInfo>     // Dependency metadata
  specs: Vec<ApiSpec>              // OpenAPI/Proto specs (Phase 1.5)
```

#### FileIR

```
FileIR
  path: PathBuf                    // Relative to repo root
  language: Language               // Rust | Python | Java | TypeScript | Vue | Html
  content_hash: String             // SHA256 of file content
  parse_status: ParseStatus        // Success | PartialSuccess | Failed
  byte_count: usize
```

#### Symbol

```
Symbol
  id: SymbolId                     // Globally unique, stable
  kind: SymbolKind                 // Function | Struct | Trait | Class | Interface |
                                   // Method | Module | Enum | Constant | TypeAlias |
                                   // Component (Vue)
  qualified_name: String           // Human-readable: "memoryos_core::memory::GraphManager"
  visibility: Visibility           // Public | Private | Protected | Internal
  file: PathBuf
  span: Span { start_byte, end_byte, start_line, end_line }
  signature: Option<String>        // fn foo(x: i32) -> String
  doc: Option<Doc>                 // Parsed documentation
  parent: Option<SymbolId>         // Containing module/class/impl
  children: Vec<SymbolId>          // Contained symbols
  annotations: Vec<Annotation>     // Java @Override, Python @decorator, Rust #[derive]
  type_params: Vec<String>         // Generics
```

#### SymbolId (Stable, Cross-Language)

```
SymbolId
  repo_root: String                // Anchor to repo
  file_path: String                // Relative path
  start_byte: usize                // AST node start
  end_byte: usize                  // AST node end
  kind: SymbolKind                 // Disambiguate overlapping spans
  _qualified_name: String          // Cached readable name (not used for equality)
```

Equality: `file_path + start_byte + end_byte + kind`

#### Reference

```
Reference
  source: SymbolId                 // Where the reference originates
  target: ReferenceTarget          // Resolved SymbolId or unresolved name
  kind: ReferenceKind              // Import | Call | Extends | Implements |
                                   // UsesType | FieldType | AnnotationUsage
  span: Span                       // Location of the reference
```

#### Doc

```
Doc
  raw: String                      // Original text
  format: DocFormat                // Markdown | JavaDoc | ReST | JSDoc | RustDoc
  summary: Option<String>          // First sentence / @brief (local rule extraction)
```

#### Diagnostic

```
Diagnostic
  file: PathBuf
  severity: DiagSeverity           // Error | Warning | Info
  message: String
  span: Option<Span>
  fallback_data: Option<FallbackData>  // Low-fidelity import lines from token scan
```

### 4.3 Tree-sitter Query Strategy

**Minimum viable queries per language (V1):**

| Language | Declarations | Imports | Doc Comments |
|----------|-------------|---------|-------------|
| Rust | `fn`, `struct`, `enum`, `trait`, `impl`, `mod`, `const`, `type` | `use`, `extern crate` | `///`, `//!`, `#[doc = ""]` |
| Python | `def`, `class`, `async def` | `import`, `from...import` | `"""docstring"""` |
| Java | `class`, `interface`, `enum`, `method`, `constructor` | `import` | `/** JavaDoc */` |
| TypeScript | `function`, `class`, `interface`, `type`, `const`, `export` | `import` | `/** JSDoc */` |
| Vue | Virtual `Component` symbol per `.vue` file | `import` in `<script>` | `<script>` top-level JSDoc |

**Not in V1** (deferred to V2):
- Call graph (`fn_a()` calls `fn_b()`) — high noise, low accuracy
- Data flow analysis
- Macro expansion (Rust `macro_rules!`, Java annotation processors)

### 4.4 Vue SFC Handling

```mermaid
flowchart TD
    VUE[.vue file] --> SPLIT[Regex Section Splitter]
    SPLIT --> SCRIPT["&lt;script setup lang=ts&gt;<br>offset: byte 145"]
    SPLIT --> TEMPLATE["&lt;template&gt;<br>offset: byte 0"]
    SPLIT --> STYLE["&lt;style&gt;<br>(ignored)"]

    SCRIPT --> TS_PARSE[tree-sitter-typescript]
    TEMPLATE --> HTML_PARSE[tree-sitter-html]

    TS_PARSE --> REMAP[Span Remapping<br>+start_byte offset]
    HTML_PARSE --> REMAP

    REMAP --> SYMBOLS[Symbols with correct<br>global file positions]

    SYMBOLS --> VCOMP[Virtual Component Symbol<br>kind=Component<br>children=defineProps/defineEmits/...]
```

**Key implementation details:**
- Record `section_start_byte` before parsing each section
- All tree-sitter spans += `section_start_byte` for global file offset
- `<script setup>` creates a virtual `Component` symbol with `qualified_name = path::filename.vue`
- `defineProps`, `defineEmits`, `withDefaults` detected as child symbols

### 4.5 Degradation Strategy

```mermaid
flowchart TD
    FILE[Source File] --> PARSE{tree-sitter parse}
    PARSE -->|SUCCESS| FULL[Full IR extraction]
    PARSE -->|HAS_ERRORS| PARTIAL[Extract what succeeded<br>+ token-scan imports]
    PARSE -->|TOTAL_FAIL| FALLBACK[Token scan only:<br>import lines + file existence]

    FULL --> DIAG_OK["Diagnostic: severity=Info"]
    PARTIAL --> DIAG_WARN["Diagnostic: severity=Warning<br>fallback_data=Some(imports)"]
    FALLBACK --> DIAG_ERR["Diagnostic: severity=Error<br>fallback_data=Some(imports)"]
```

---

## 5. Phase 1.5: API Endpoint Extraction

### 5.1 API Spec Discovery (Priority Source)

Before code-level extraction, discover authoritative spec files:

| File Pattern | Format | Priority |
|-------------|--------|----------|
| `openapi.yaml`, `openapi.yml`, `openapi.json` | OpenAPI 3.x | Highest |
| `swagger.json`, `swagger.yaml` | Swagger 2.0 | Highest |
| `*.proto` | gRPC / Protocol Buffers | Highest |

When a spec file exists:
1. Parse it into `Vec<Endpoint>` with `source = Spec(OpenAPI)` or `Spec(Proto)`
2. Code-level extraction runs in **alignment mode**: same path+method deduplicates, spec wins for schema/description, code adds handler mapping + middleware signals

### 5.2 Framework Route Extraction

| Framework | Language | Detection Pattern |
|-----------|----------|------------------|
| Axum | Rust | `Router::new().route("/path", get(handler))`, `.layer(...)` |
| Actix-web | Rust | `#[get("/path")]`, `web::resource(...)` |
| FastAPI | Python | `@app.get("/path")`, `@router.post(...)`, `Depends(...)` |
| Flask | Python | `@app.route("/path", methods=[...])` |
| Spring MVC | Java | `@GetMapping`, `@PostMapping`, `@RequestMapping` |
| Express | TypeScript | `app.get("/path", handler)`, `router.use(...)` |

### 5.3 Endpoint IR

```
Endpoint
  method: HttpMethod               // GET | POST | PUT | DELETE | PATCH | ALL
  path: String                     // Normalized: /users/{id} (unified param style)
  handler: Option<SymbolId>        // Resolved handler function
  source: EndpointSource           // CodeExtraction(framework) | Spec(OpenAPI) | Spec(Proto)
  file: PathBuf
  span: Span
  request_type: Option<TypeRef>    // Request body type (SymbolId or string name)
  response_type: Option<TypeRef>   // Response type
  auth: AuthInfo                   // See below
  tags: Vec<String>                // Controller/module grouping
  doc: Option<Doc>                 // From doc comment or spec description
```

### 5.4 Auth Signal Extraction (80% Rule)

```
AuthInfo
  signals: Vec<String>             // Raw evidence strings
  classification: AuthClassification  // Required | Optional | Unknown
```

| Framework | Signal Patterns |
|-----------|----------------|
| Axum | `.layer(middleware::from_fn(auth_check))`, `.route_layer(...)` |
| Spring | `@PreAuthorize("...")`, `@Secured("ROLE_ADMIN")`, `@RolesAllowed(...)` |
| FastAPI | `Depends(get_current_user)`, router-level dependencies |
| Express | `app.use(authMiddleware)`, route middleware array |

Output: **raw signals** + optional classification. LLM uses signals to write docs; no premature conclusion.

### 5.5 Path Normalization

All extracted paths are normalized to a canonical format internally:

| Input | Normalized |
|-------|-----------|
| `/users/:id` (Express) | `/users/{id}` |
| `/users/<id>` (Flask) | `/users/{id}` |
| `/users/{id}` (OpenAPI) | `/users/{id}` |

---

## 6. Manifest Extractors (Dependency Analysis)

### 6.1 Unified Interface

```
trait ManifestExtractor {
  fn detect(&self, repo: &RepoContext) -> bool;
  fn extract(&self, repo: &RepoContext) -> Vec<Dependency>;
}
```

### 6.2 Implementations

| Extractor | File | Parser |
|-----------|------|--------|
| `CargoExtractor` | `Cargo.toml` + `cargo_metadata` | `cargo_metadata` crate |
| `MavenExtractor` | `pom.xml` | `quick-xml` |
| `GradleExtractor` | `build.gradle` / `build.gradle.kts` | Regex (V1) |
| `PythonExtractor` | `pyproject.toml` / `requirements.txt` | `toml` / line parse |
| `NodeExtractor` | `package.json` | `serde_json` |

### 6.3 Dependency Model

```
Dependency
  name: String
  version_req: Option<String>
  scope: DependencyScope           // Runtime | Dev | Build | Optional
  source_file: PathBuf
  ecosystem: Ecosystem             // Cargo | Maven | Npm | Pip | Gradle
```

---

## 7. Phase 2: Code Graph Construction

### 7.1 Three-Layer Graph Design

```mermaid
graph TD
    subgraph "Layer 1: FileGraph (Coarse)"
        F1[src/main.rs] -->|imports| F2[src/lib.rs]
        F2 -->|imports| F3[src/memory/mod.rs]
        F2 -->|imports| F4[src/faq/mod.rs]
    end

    subgraph "Layer 2: SymbolGraph (Fine)"
        S1[GraphManager] -->|implements| S2[trait Display]
        S1 -->|has_field| S3[Vec&lt;GraphEntity&gt;]
        S4[fn extract_entities] -->|uses_type| S3
        S5[MemoryManager] -->|calls| S4
    end

    subgraph "Layer 3: RuntimeGraph (API)"
        R1["GET /v1/graph/stats"] -->|handler| S6[graph_stats_handler]
        S6 -->|calls| S1
        R2["POST /v1/chat"] -->|handler| S7[chat_handler]
        S7 -->|calls| S5
    end

    style F1 fill:#e3f2fd
    style F2 fill:#e3f2fd
    style F3 fill:#e3f2fd
    style F4 fill:#e3f2fd
    style S1 fill:#fff3e0
    style S2 fill:#fff3e0
    style S3 fill:#fff3e0
    style S4 fill:#fff3e0
    style S5 fill:#fff3e0
    style R1 fill:#fce4ec
    style R2 fill:#fce4ec
```

### 7.2 Graph Storage

All three layers live in a single `petgraph::DiGraph` with typed nodes and edges:

```
CodeGraphNode
  | FileNode(FileIR)
  | SymbolNode(Symbol)
  | EndpointNode(Endpoint)
  | ExternalDep(Dependency)
```

```
CodeGraphEdge
  kind: EdgeKind
  // FileGraph edges
  | FileImports            // file A imports from file B
  // SymbolGraph edges
  | Contains               // module/class contains symbol
  | Implements             // impl/class implements trait/interface
  | Extends                // class extends parent
  | UsesType               // field type, return type, parameter type
  | FieldType              // struct field references another type
  // RuntimeGraph edges
  | HandledBy              // endpoint -> handler function
  | MiddlewareApplied      // endpoint -> middleware symbol
```

### 7.3 Graph Queries (for downstream phases)

| Query | Purpose | Used By |
|-------|---------|---------|
| `neighbors(node, direction)` | Get related nodes | LLM context building |
| `subgraph(root, depth)` | Extract local neighborhood | Mermaid diagram scoping |
| `topo_sort()` | Dependency order | Page generation order |
| `modules_at_depth(n)` | Get modules at a nesting level | Architecture diagram |
| `endpoints_by_tag()` | Group endpoints | API doc generation |

---

## 8. Phase 3: LLM Documentation Generation

### 8.1 Generation Strategy

```mermaid
flowchart TD
    SYM[Symbol from Graph] --> BUILD[Build Evidence Pack]
    BUILD --> CHECK{Cache hit?<br>prompt_hash unchanged}
    CHECK -->|hit| CACHED[Use cached description]
    CHECK -->|miss| PROMPT[Build LLM Prompt]
    PROMPT --> CALL[LLM Call<br>via LlmAdapter]
    CALL --> PARSE[Parse Response]
    PARSE --> STORE[Store in Cache<br>with evidence links]

    CACHED --> OUT[Symbol Description<br>+ Crate Overview]
    STORE --> OUT
```

### 8.2 Evidence Pack (per Symbol)

Each LLM call receives a structured **Evidence Pack**:

```
EvidencePack
  symbol: SymbolSummary            // kind, qualified_name, signature, visibility
  doc: Option<String>              // Existing doc comment
  source_snippet: String           // Key source lines (truncated to ~200 lines)
  graph_context: GraphContext      // Neighbors: implements, used_by, contains, calls
  file_context: String             // File-level imports and module path
```

### 8.3 LLM Output Schema

```
LlmDocResult
  summary: String                  // 1-2 sentence description
  detailed: String                 // Multi-paragraph explanation
  usage_example: Option<String>    // Code example if applicable
  sources: Vec<EvidenceRef>        // [{file, start_line, end_line}]
```

### 8.4 Batching Strategy

```mermaid
flowchart TD
    REPO[All Symbols] --> BATCH[Group by Crate/Module]
    BATCH --> SMALL{Module < 30 symbols?}
    SMALL -->|yes| ONE[Single LLM call<br>for entire module]
    SMALL -->|no| SPLIT[Split into chunks<br>of ~20 symbols]
    SPLIT --> MULTI[Multiple LLM calls]

    ONE --> MERGE[Merge Results]
    MULTI --> MERGE
    MERGE --> OVERVIEW[Generate Crate-Level Overview<br>separate LLM call]
```

### 8.5 Concurrency & Cost Control

| Control | Mechanism | Default |
|---------|-----------|---------|
| Concurrent LLM calls | `tokio::sync::Semaphore` | 5 |
| Dedup cache | `prompt_hash` (SHA256 of evidence pack) | Enabled |
| Retry | Exponential backoff + jitter | 3 attempts |
| Token budget | Truncate source snippets to fit context | ~4000 tokens input |
| Rate limit (optional) | Per-minute token counter | Disabled (V1) |

### 8.6 Incremental Update

```mermaid
flowchart TD
    FILE[File Changed] --> HASH[New content_hash]
    HASH --> SYMBOLS[Affected Symbols]
    SYMBOLS --> NEIGHBORS[Graph neighbors<br>of affected symbols]
    NEIGHBORS --> REGEN[Regenerate docs<br>for affected + neighbors]
```

Cache structure (`.wiki-cache/cache.json`):
```json
{
  "version": 1,
  "entries": {
    "<symbol_id_hash>": {
      "prompt_hash": "sha256...",
      "content_hash": "sha256...",
      "result": { "summary": "...", "detailed": "...", "sources": [...] },
      "generated_at": "2026-02-20T10:00:00Z"
    }
  }
}
```

---

## 9. Phase 4: Mermaid Diagram Generation

### 9.1 Diagram Types

| Diagram | Source | Scope |
|---------|--------|-------|
| Module Dependency | FileGraph (aggregated to module level) | `architecture.md` |
| API Router Flow | RuntimeGraph (endpoints -> handlers -> services) | `api.md` |
| Class / Trait Relationship | SymbolGraph (struct/trait/impl edges) | Per-module pages |
| Crate Dependency | ManifestExtractor output | `index.md` |

### 9.2 Module Dependency Diagram

```mermaid
graph TD
    subgraph "Example: memoryos-rust"
        CORE[memoryos-core]
        PORTS[memoryos-ports]
        ADAPTERS[memoryos-adapters]
        GW[memoryos-gateway]
        ADMIN[memoryos-admin]
        WORKER[memoryos-worker]
        WIKI[memoryos-wiki-gen]

        GW --> CORE
        GW --> PORTS
        GW --> ADAPTERS
        ADMIN --> CORE
        WORKER --> CORE
        WIKI --> CORE
        WIKI --> PORTS
        ADAPTERS --> PORTS
        CORE --> PORTS
    end
```

### 9.3 API Router Flow Diagram

```mermaid
graph LR
    REQ[HTTP Request] --> MW1[Auth Middleware]
    MW1 --> MW2[RBAC Middleware]
    MW2 --> ROUTER{Route Match}

    ROUTER -->|"/v1/chat"| H1[chat_handler]
    ROUTER -->|"/v1/memory"| H2[memory_handler]
    ROUTER -->|"/v1/graph"| H3[graph_handler]

    H1 --> SVC1[MemoryManager]
    H2 --> SVC1
    H3 --> SVC2[GraphManager]

    SVC1 --> DB[(Qdrant)]
    SVC2 --> DB
```

### 9.4 Generation Rules

- **Max nodes per diagram**: 30 (beyond this, aggregate into clusters)
- **Directory aggregation**: If a directory has > 10 files, collapse into one node
- **Edge filtering**: Only show direct dependencies (depth=1) unless explicitly requested
- **Layout hint**: Use `graph TD` for architecture, `graph LR` for request flows

---

## 10. Phase 5: Page Assembly

### 10.1 Template Engine

**Engine**: `tera` (Jinja2-style)

Templates stored in `crates/memoryos-wiki-gen/templates/`:

```
templates/
  index.md.tera
  architecture.md.tera
  api.md.tera
  module.md.tera
  symbol.md.tera        (optional, for key public APIs)
  faq_category.md.tera
```

### 10.2 Output Information Architecture

```mermaid
graph TD
    subgraph "Generated Wiki Structure"
        INDEX[index.md<br>Project Overview + Crate Diagram]
        ARCH[architecture.md<br>Module Dependency + Architecture Diagram]
        API[api.md<br>All Endpoints + Router Flow Diagram]

        subgraph "modules/"
            M1[core.md]
            M2[gateway.md]
            M3[adapters.md]
            MN[...]
        end

        subgraph "faq/"
            FAQ1[it-network.md]
            FAQ2[hr-onboarding.md]
            FAQN[...]
        end
    end

    INDEX --> ARCH
    INDEX --> API
    INDEX --> M1
    INDEX --> FAQ1

    style INDEX fill:#e8f5e9
    style ARCH fill:#e3f2fd
    style API fill:#fce4ec
    style FAQ1 fill:#fff3e0
```

### 10.3 Page Content Sources

| Page | Data Source | Diagrams |
|------|-----------|----------|
| `index.md` | Repo metadata + Crate-level LLM overviews | Crate dependency diagram |
| `architecture.md` | FileGraph + SymbolGraph (traits/impls) | Module dependency diagram + Class diagram |
| `api.md` | Endpoint IR + handler docs | API router flow diagram |
| `modules/<name>.md` | SymbolGraph subgraph + LLM descriptions | Per-module class/trait diagram |
| `faq/<category>.md` | FAQ WikiExporter output (LlmClassifier) | None |

### 10.4 Frontmatter (YAML)

Every generated page includes:

```yaml
---
title: "Module: memoryos-core"
generated_at: "2026-02-20T10:00:00Z"
generator: "memoryos-wiki-gen v0.1.0"
source_repo: "TelivANT/memoryos-rust"
source_commit: "abc1234"
---
```

### 10.5 FAQ Integration

The FAQ pipeline remains largely unchanged but routes through the same Page Builder:

```mermaid
flowchart TD
    FAQ_DB[(FAQ HeatTracker<br>Qdrant)] --> FILTER[Filter Qualified FAQs]
    FILTER --> CLASSIFY[LlmClassifier<br>replaces hardcoded keywords]
    CLASSIFY --> TEMPLATE[faq_category.md.tera]
    TEMPLATE --> PAGES[faq/*.md]

    CODE[Code Pipeline<br>Phase 0-4] --> TEMPLATE2[module.md.tera / api.md.tera]
    TEMPLATE2 --> PAGES2[modules/*.md + api.md]

    PAGES --> EXPORT[Phase 6: Unified Export]
    PAGES2 --> EXPORT
```

---

## 11. Phase 6: Export

### 11.1 Unified Export via WikiExportBackend

Reuse existing `WikiExportBackend` trait from `memoryos-core`:

```
trait WikiExportBackend {
    async fn write_content(&self, path: &str, content: &[u8]) -> Result<()>;
}
```

Implementations (existing):
- `LocalFsBackend` — write to directory
- `S3ExportBackend` — OpenDAL S3/MinIO
- `ConfluenceExportBackend` — REST API

V2 planned:
- `GitHubWikiBackend` — `git clone wiki.git` + push

### 11.2 Incremental Export

```mermaid
flowchart TD
    PAGE[Rendered Page] --> HASH[SHA256 content hash]
    HASH --> CHECK{Hash matches<br>last export?}
    CHECK -->|same| SKIP[Skip upload]
    CHECK -->|different| UPLOAD[Upload via Backend]
    UPLOAD --> RECORD[Update wiki_index.json]
```

### 11.3 Evidence Index (`wiki_index.json`)

Stored alongside exported pages:

```json
{
  "version": 1,
  "generated_at": "2026-02-20T10:00:00Z",
  "source_commit": "abc1234",
  "pages": [
    {
      "path": "modules/core.md",
      "content_hash": "sha256...",
      "symbols_referenced": ["memoryos_core::memory::GraphManager", "..."],
      "evidence": [
        {"file": "crates/memoryos-core/src/memory/graph.rs", "lines": "71-96"}
      ]
    }
  ]
}
```

Use cases:
- "Click doc paragraph → jump to source" (future IDE integration)
- "Doc outdated?" detection (compare symbol hashes)
- Audit trail for generated content

---

## 12. Data Model Summary (Entity-Relationship)

```mermaid
erDiagram
    RepoIR ||--o{ FileIR : contains
    RepoIR ||--o{ Symbol : contains
    RepoIR ||--o{ Reference : contains
    RepoIR ||--o{ Endpoint : contains
    RepoIR ||--o{ Diagnostic : contains
    RepoIR ||--o{ ManifestInfo : contains
    RepoIR ||--o{ ApiSpec : contains

    Symbol ||--o{ Reference : "source_of"
    Symbol ||--o| Doc : has
    Symbol }o--o{ Symbol : "parent/child"
    Symbol ||--o{ Annotation : has

    Endpoint }o--|| Symbol : "handler"
    Endpoint ||--|| AuthInfo : has

    CodeGraph ||--o{ CodeGraphNode : contains
    CodeGraph ||--o{ CodeGraphEdge : contains
    CodeGraphNode }o--|| Symbol : wraps
    CodeGraphNode }o--|| Endpoint : wraps
    CodeGraphNode }o--|| FileIR : wraps

    LlmDocResult }o--|| Symbol : "documents"
    LlmDocResult ||--o{ EvidenceRef : "backed_by"

    WikiPage }o--o{ Symbol : "references"
    WikiPage }o--o{ LlmDocResult : "includes"
```

---

## 13. Technology Stack (Final)

### 13.1 New Dependencies

| Category | Crate | Purpose |
|----------|-------|---------|
| Parsing | `tree-sitter` | Core AST parsing engine |
| Parsing | `tree-sitter-rust` | Rust grammar |
| Parsing | `tree-sitter-python` | Python grammar |
| Parsing | `tree-sitter-java` | Java grammar |
| Parsing | `tree-sitter-typescript` | TS/JS grammar (Vue script) |
| Parsing | `tree-sitter-html` | HTML grammar (Vue template) |
| Parsing | `cargo_metadata` | Rust workspace dependency graph |
| Parsing | `quick-xml` | Maven pom.xml parsing |
| Graph | `petgraph` | In-memory directed graph |
| Template | `tera` | Jinja2-style Markdown generation |
| CLI | `clap` | Command-line argument parsing |
| CLI | `indicatif` | Progress bars |
| File | `ignore` | .gitignore-aware file walking |
| Parallel | `rayon` | CPU-parallel tree-sitter parsing |
| Hash | `sha2` | Content hashing for incremental cache |

### 13.2 Reused from Workspace

| Crate | Usage |
|-------|-------|
| `serde` / `serde_json` | Serialization |
| `tokio` | Async runtime |
| `tracing` | Structured logging |
| `reqwest` | HTTP client (LLM API) |
| `LlmAdapter` trait + 10 adapters | LLM calls |
| `WikiExportBackend` trait | Export backends |
| `GraphEntity` / `GraphRelation` model | Graph data model reference |

---

## 14. Crate Structure

```
crates/memoryos-wiki-gen/
  Cargo.toml
  src/
    lib.rs                         # Public API: WikiGenerator
    cli.rs                         # clap CLI entry point (bin target)

    # Phase 0
    config.rs                      # WikiGenConfig, wiki-gen.toml parsing
    discovery.rs                   # File discovery (ignore::WalkBuilder)
    lang.rs                        # Language detection

    # Phase 1
    ir.rs                          # RepoIR, Symbol, Reference, FileIR, etc.
    parser/
      mod.rs                       # LanguageParser trait
      rust.rs                      # RustParser (tree-sitter-rust)
      python.rs                    # PythonParser (tree-sitter-python)
      java.rs                      # JavaParser (tree-sitter-java)
      typescript.rs                # TypeScriptParser (tree-sitter-typescript)
      vue.rs                       # VueSfcSplitter + delegation

    # Phase 1.5
    endpoint/
      mod.rs                       # Endpoint IR, extraction trait
      spec_discovery.rs            # OpenAPI / Proto / Swagger detection
      axum.rs                      # Axum route extractor
      fastapi.rs                   # FastAPI route extractor
      spring.rs                    # Spring MVC route extractor
      express.rs                   # Express.js route extractor

    # Phase 1 (Manifests)
    manifest/
      mod.rs                       # ManifestExtractor trait
      cargo.rs                     # CargoExtractor
      maven.rs                     # MavenExtractor
      python.rs                    # PythonExtractor (pip/poetry)
      node.rs                      # NodeExtractor (npm/yarn)

    # Phase 2
    graph.rs                       # CodeGraph (petgraph), 3-layer, queries

    # Phase 3
    llm_gen.rs                     # LLM documentation generation
    evidence.rs                    # EvidencePack builder
    cache.rs                       # Incremental cache (SHA256)

    # Phase 4
    diagram.rs                     # Mermaid diagram generators

    # Phase 5
    page_builder.rs                # Tera template rendering
    faq_integration.rs             # FAQ WikiExporter bridge

    # Phase 6
    export.rs                      # Export orchestration
    wiki_index.rs                  # wiki_index.json builder

  templates/
    index.md.tera
    architecture.md.tera
    api.md.tera
    module.md.tera
    symbol.md.tera
    faq_category.md.tera
```

---

## 15. CLI Interface

### 15.1 Commands

```bash
# Full generation
memoryos-wiki-gen generate \
  --repo /path/to/repo \
  --output ./wiki-output \
  --config wiki-gen.toml

# Incremental update (only changed files)
memoryos-wiki-gen generate \
  --repo /path/to/repo \
  --incremental

# Parse only (no LLM, no render — for debugging)
memoryos-wiki-gen parse \
  --repo /path/to/repo \
  --output-ir repo_ir.json

# Diagram only (generate Mermaid from existing IR)
memoryos-wiki-gen diagram \
  --ir repo_ir.json \
  --type module-dependency

# Export to remote target
memoryos-wiki-gen export \
  --source ./wiki-output \
  --target s3 \
  --config wiki-gen.toml
```

### 15.2 API Endpoints (Gateway Integration)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v1/wiki/generate` | Trigger full wiki generation |
| `POST` | `/v1/wiki/generate/incremental` | Incremental update |
| `GET` | `/v1/wiki/status` | Generation job status |
| `GET` | `/v1/wiki/pages` | List generated pages |
| `GET` | `/v1/wiki/pages/{path}` | Get specific page content |

---

## 16. Old Wiki System Cleanup

### 16.1 Files to Delete (System A)

| File | Reason |
|------|--------|
| `crates/memoryos-ports/src/wiki.rs` | `WikiAdapter` trait — replaced by `WikiExportBackend` |
| `crates/memoryos-adapters/src/wiki/exporter.rs` | Old exporter with mock data |
| `crates/memoryos-adapters/src/wiki/s3.rs` | Old S3 adapter without SigV4 |

### 16.2 Files to Keep (System B)

| File | Reason |
|------|--------|
| `crates/memoryos-core/src/faq/wiki_exporter.rs` | `WikiExportBackend` trait + `WikiExporter` pipeline |
| `crates/memoryos-adapters/src/wiki/confluence_backend.rs` | Working Confluence backend |
| `crates/memoryos-adapters/src/wiki/s3_backend.rs` | Working S3 backend via OpenDAL |

### 16.3 Modifications to Keep (System B)

| Change | Location |
|--------|----------|
| Replace `extract_category()` hardcoded keywords with `LlmClassifier` | `wiki_exporter.rs` |
| Remove `WikiAdapter` re-export from ports `lib.rs` | `memoryos-ports/src/lib.rs` |

---

## 17. Implementation Plan

### 17.1 Phase Breakdown

```mermaid
gantt
    title Wiki Generation System Implementation
    dateFormat  YYYY-MM-DD
    axisFormat  %m-%d

    section Phase 0
    Crate scaffold + Cargo.toml       :p0a, 2026-02-21, 1d
    Config + Discovery + Lang detect  :p0b, after p0a, 2d
    Rayon + Indicatif infra           :p0c, after p0a, 1d

    section Phase 1
    IR data structures                :p1a, after p0b, 1d
    Rust parser (tree-sitter)         :p1b, after p1a, 3d
    Python parser                     :p1c, after p1b, 2d
    Java parser                       :p1d, after p1c, 2d
    Vue SFC splitter + TS parser      :p1e, after p1d, 2d
    Degradation + diagnostics         :p1f, after p1e, 1d

    section Phase 1.5
    Manifest extractors               :p15a, after p1a, 2d
    API Spec Discovery                :p15b, after p15a, 1d
    Axum route extractor              :p15c, after p15b, 2d
    FastAPI route extractor           :p15d, after p15c, 1d
    Spring route extractor            :p15e, after p15d, 1d

    section Phase 2
    CodeGraph (petgraph)              :p2a, after p1f, 2d
    3-layer edges + queries           :p2b, after p2a, 2d

    section Phase 3
    Evidence pack builder             :p3a, after p2b, 1d
    LLM prompt + response parse       :p3b, after p3a, 2d
    Cache layer (SHA256)              :p3c, after p3b, 1d
    Semaphore + retry                 :p3d, after p3b, 1d

    section Phase 4
    Mermaid generators                :p4a, after p3b, 2d

    section Phase 5
    Tera templates                    :p5a, after p4a, 2d
    FAQ integration bridge            :p5b, after p5a, 1d
    wiki_index.json                   :p5c, after p5a, 1d

    section Phase 6
    Export orchestration              :p6a, after p5b, 1d
    CLI (clap)                        :p6b, after p6a, 1d
    Gateway API endpoints             :p6c, after p6b, 1d

    section Cleanup
    Delete old Wiki System A          :cln, after p6c, 1d
    Integration tests                 :test, after cln, 2d
```

### 17.2 MVP Scope (Week 1-2)

**Goal**: Rust + Python end-to-end, one framework (Axum), generate browsable Markdown wiki.

| Task | Deliverable |
|------|------------|
| Phase 0 scaffold | `memoryos-wiki-gen` crate compiles |
| Phase 1 Rust parser | `RepoIR` with Rust symbols + references + docs |
| Phase 1 Python parser | Python symbols in same `RepoIR` |
| Phase 2 Graph | Module dependency Mermaid diagram |
| Phase 3 LLM (1 call) | Crate-level overview |
| Phase 5 Page builder | `index.md` + `architecture.md` + `modules/*.md` |
| Phase 6 Local export | Files written to disk |
| CLI | `memoryos-wiki-gen generate --repo . --output wiki-out` |

### 17.3 V1 Complete Scope (Week 3-4)

| Task | Deliverable |
|------|------------|
| Phase 1 Java + Vue parsers | All 4 languages parsing |
| Phase 1.5 Endpoint extraction | Axum + FastAPI route extraction |
| Phase 1.5 API Spec Discovery | OpenAPI/Swagger/Proto detection |
| Phase 3 Full LLM generation | All symbol descriptions |
| Phase 4 All diagram types | Module dep + API flow + class diagrams |
| Phase 5 All templates | Complete wiki page set |
| Phase 5 FAQ integration | `faq/*.md` pages |
| Phase 6 S3/Confluence export | Remote export working |
| Gateway API | `/v1/wiki/*` endpoints |
| Cleanup | Old Wiki System A deleted |

---

## 18. Risk Analysis

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| Tree-sitter grammar version mismatch | Medium | Parser crashes | Pin grammar versions; degradation output on failure |
| LLM hallucination in docs | High | Incorrect documentation | Evidence pack + source links; human review flag |
| Large repo performance (>10k files) | Medium | Slow generation | Rayon parallelism + incremental cache + file size limit |
| Vue SFC edge cases (scoped CSS, JSX) | Low | Parse failures | Graceful degradation; only parse script/template |
| Token cost explosion | Medium | High API bills | Prompt hash dedup + batching + token budget per symbol |
| Cross-language symbol resolution | High | Broken references | V1: best-effort name matching; V2: full resolution |

---

## 19. Future Enhancements (V2+)

| Feature | Description |
|---------|------------|
| Call graph analysis | `fn_a()` calls `fn_b()` via tree-sitter + LLM hybrid |
| GitHub Wiki backend | `git clone wiki.git` + push |
| IDE integration | "Click doc → jump to source" via wiki_index.json |
| Doc freshness alerts | Compare symbol_hash vs last generation, flag stale docs |
| Multi-repo support | Aggregate graphs across repositories |
| Additional languages | Go, C/C++, PHP, Kotlin, Swift |
| Interactive diagrams | Mermaid → clickable SVG with source links |
| CI integration | GitHub Action: auto-generate wiki on PR merge |
