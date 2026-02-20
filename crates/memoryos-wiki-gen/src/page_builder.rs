use std::collections::HashMap;

use tera::{Context, Tera};
use tracing::debug;

use crate::diagram::DiagramGenerator;
use crate::error::{WikiGenError, WikiGenResult};
use crate::evidence::LlmDocResult;
use crate::graph::CodeGraph;
use crate::ir::*;

pub struct PageBuilder {
    tera: Tera,
}

#[derive(Debug)]
pub struct GeneratedPage {
    pub path: String,
    pub content: String,
    pub symbols_referenced: Vec<String>,
}

impl PageBuilder {
    pub fn new() -> WikiGenResult<Self> {
        let mut tera = Tera::default();

        tera.add_raw_template("index.md", INDEX_TEMPLATE)
            .map_err(|e| WikiGenError::Template(format!("index template: {}", e)))?;
        tera.add_raw_template("architecture.md", ARCHITECTURE_TEMPLATE)
            .map_err(|e| WikiGenError::Template(format!("architecture template: {}", e)))?;
        tera.add_raw_template("api.md", API_TEMPLATE)
            .map_err(|e| WikiGenError::Template(format!("api template: {}", e)))?;
        tera.add_raw_template("module.md", MODULE_TEMPLATE)
            .map_err(|e| WikiGenError::Template(format!("module template: {}", e)))?;

        Ok(Self { tera })
    }

    pub fn build_all(
        &self,
        ir: &RepoIR,
        graph: &CodeGraph,
        symbol_docs: &HashMap<String, LlmDocResult>,
        overview: &str,
        source_commit: &str,
    ) -> WikiGenResult<Vec<GeneratedPage>> {
        let mut pages = Vec::new();

        pages.push(self.build_index(ir, graph, symbol_docs, overview, source_commit)?);
        pages.push(self.build_architecture(ir, graph, symbol_docs, source_commit)?);

        if !ir.endpoints.is_empty() {
            pages.push(self.build_api(ir, graph, source_commit)?);
        }

        let modules = collect_modules(ir);
        for (module_name, module_symbols) in &modules {
            pages.push(self.build_module(
                module_name,
                module_symbols,
                &ir.references,
                symbol_docs,
                source_commit,
            )?);
        }

        debug!("Built {} wiki pages", pages.len());
        Ok(pages)
    }

    fn build_index(
        &self,
        ir: &RepoIR,
        _graph: &CodeGraph,
        _symbol_docs: &HashMap<String, LlmDocResult>,
        overview: &str,
        source_commit: &str,
    ) -> WikiGenResult<GeneratedPage> {
        let mut ctx = Context::new();
        ctx.insert("repo_root", &ir.repo_root.display().to_string());
        ctx.insert("source_commit", source_commit);
        ctx.insert("generated_at", &chrono::Utc::now().to_rfc3339());
        ctx.insert("overview", overview);
        ctx.insert("file_count", &ir.files.len());
        ctx.insert("symbol_count", &ir.symbols.len());
        ctx.insert("endpoint_count", &ir.endpoints.len());

        let mut lang_stats: Vec<HashMap<String, String>> = Vec::new();
        let mut lang_counts: HashMap<Language, usize> = HashMap::new();
        for file in &ir.files {
            *lang_counts.entry(file.language).or_insert(0) += 1;
        }
        for (lang, count) in &lang_counts {
            let mut m = HashMap::new();
            m.insert("name".to_string(), lang.to_string());
            m.insert("count".to_string(), count.to_string());
            lang_stats.push(m);
        }
        ctx.insert("languages", &lang_stats);

        let crate_diagram = DiagramGenerator::crate_dependency_diagram(&ir.manifests);
        ctx.insert("crate_diagram", &crate_diagram);

        let modules = collect_modules(ir);
        let module_names: Vec<String> = modules.keys().cloned().collect();
        ctx.insert("modules", &module_names);

        let content = self
            .tera
            .render("index.md", &ctx)
            .map_err(|e| WikiGenError::Template(format!("index render: {}", e)))?;

        Ok(GeneratedPage {
            path: "index.md".to_string(),
            content,
            symbols_referenced: Vec::new(),
        })
    }

    fn build_architecture(
        &self,
        ir: &RepoIR,
        graph: &CodeGraph,
        symbol_docs: &HashMap<String, LlmDocResult>,
        source_commit: &str,
    ) -> WikiGenResult<GeneratedPage> {
        let mut ctx = Context::new();
        ctx.insert("source_commit", source_commit);
        ctx.insert("generated_at", &chrono::Utc::now().to_rfc3339());

        let module_diagram = DiagramGenerator::module_dependency_diagram(graph);
        ctx.insert("module_diagram", &module_diagram);

        let class_diagram = DiagramGenerator::class_diagram(&ir.symbols, &ir.references);
        ctx.insert("class_diagram", &class_diagram);

        let key_traits: Vec<HashMap<String, String>> = ir
            .symbols
            .iter()
            .filter(|s| {
                matches!(s.kind, SymbolKind::Trait | SymbolKind::Interface)
                    && matches!(s.visibility, Visibility::Public)
            })
            .take(15)
            .map(|s| {
                let mut m = HashMap::new();
                m.insert("name".to_string(), s.qualified_name.clone());
                m.insert("kind".to_string(), s.kind.to_string());
                let doc = symbol_docs
                    .get(&s.id.stable_key())
                    .map(|d| d.summary.clone())
                    .unwrap_or_default();
                m.insert("description".to_string(), doc);
                m
            })
            .collect();
        ctx.insert("key_traits", &key_traits);

        let content = self
            .tera
            .render("architecture.md", &ctx)
            .map_err(|e| WikiGenError::Template(format!("architecture render: {}", e)))?;

        Ok(GeneratedPage {
            path: "architecture.md".to_string(),
            content,
            symbols_referenced: Vec::new(),
        })
    }

    fn build_api(
        &self,
        ir: &RepoIR,
        graph: &CodeGraph,
        source_commit: &str,
    ) -> WikiGenResult<GeneratedPage> {
        let mut ctx = Context::new();
        ctx.insert("source_commit", source_commit);
        ctx.insert("generated_at", &chrono::Utc::now().to_rfc3339());

        let api_diagram = DiagramGenerator::api_flow_diagram(graph);
        ctx.insert("api_diagram", &api_diagram);

        let endpoints: Vec<HashMap<String, String>> = ir
            .endpoints
            .iter()
            .map(|e| {
                let mut m = HashMap::new();
                m.insert("method".to_string(), e.method.to_string());
                m.insert("path".to_string(), e.path.clone());
                m.insert(
                    "handler".to_string(),
                    e.handler
                        .as_ref()
                        .map(|h| h.qualified_name().to_string())
                        .unwrap_or_default(),
                );
                m.insert("auth".to_string(), format!("{:?}", e.auth.classification));
                m.insert(
                    "description".to_string(),
                    e.doc
                        .as_ref()
                        .and_then(|d| d.summary.clone())
                        .unwrap_or_default(),
                );
                m
            })
            .collect();
        ctx.insert("endpoints", &endpoints);

        let content = self
            .tera
            .render("api.md", &ctx)
            .map_err(|e| WikiGenError::Template(format!("api render: {}", e)))?;

        Ok(GeneratedPage {
            path: "api.md".to_string(),
            content,
            symbols_referenced: Vec::new(),
        })
    }

    fn build_module(
        &self,
        module_name: &str,
        symbols: &[&Symbol],
        _references: &[Reference],
        symbol_docs: &HashMap<String, LlmDocResult>,
        source_commit: &str,
    ) -> WikiGenResult<GeneratedPage> {
        let mut ctx = Context::new();
        ctx.insert("module_name", module_name);
        ctx.insert("source_commit", source_commit);
        ctx.insert("generated_at", &chrono::Utc::now().to_rfc3339());

        let sym_data: Vec<HashMap<String, String>> = symbols
            .iter()
            .map(|s| {
                let mut m = HashMap::new();
                m.insert("name".to_string(), s.qualified_name.clone());
                m.insert("kind".to_string(), s.kind.to_string());
                m.insert("visibility".to_string(), format!("{:?}", s.visibility));
                m.insert(
                    "signature".to_string(),
                    s.signature.clone().unwrap_or_default(),
                );
                let doc = symbol_docs
                    .get(&s.id.stable_key())
                    .map(|d| d.summary.clone())
                    .unwrap_or_default();
                m.insert("description".to_string(), doc);
                let detailed = symbol_docs
                    .get(&s.id.stable_key())
                    .map(|d| d.detailed.clone())
                    .unwrap_or_default();
                m.insert("detailed".to_string(), detailed);
                m.insert("file".to_string(), s.file.display().to_string());
                m.insert("line".to_string(), s.span.start_line.to_string());
                m
            })
            .collect();
        ctx.insert("symbols", &sym_data);

        let symbols_referenced: Vec<String> =
            symbols.iter().map(|s| s.qualified_name.clone()).collect();

        let content = self
            .tera
            .render("module.md", &ctx)
            .map_err(|e| WikiGenError::Template(format!("module render: {}", e)))?;

        let safe_name =
            module_name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
        Ok(GeneratedPage {
            path: format!("modules/{}.md", safe_name),
            content,
            symbols_referenced,
        })
    }
}

fn collect_modules(ir: &RepoIR) -> HashMap<String, Vec<&Symbol>> {
    let mut modules: HashMap<String, Vec<&Symbol>> = HashMap::new();

    for symbol in &ir.symbols {
        let module = symbol
            .file
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("root")
            .to_string();

        modules.entry(module).or_default().push(symbol);
    }

    modules
}

const INDEX_TEMPLATE: &str = r#"---
title: "Project Wiki"
generated_at: "{{ generated_at }}"
generator: "memoryos-wiki-gen"
source_repo: "{{ repo_root }}"
source_commit: "{{ source_commit }}"
---

# Project Wiki

{{ overview }}

## Statistics

| Metric | Value |
|--------|-------|
| Files | {{ file_count }} |
| Symbols | {{ symbol_count }} |
| Endpoints | {{ endpoint_count }} |

## Languages

{% for lang in languages %}
- **{{ lang.name }}**: {{ lang.count }} files
{% endfor %}

## Modules

{% for module in modules %}
- [{{ module }}](modules/{{ module }}.md)
{% endfor %}

{% if endpoint_count > 0 %}
## API Documentation

See [API Reference](api.md) for endpoint details.
{% endif %}

## Architecture

See [Architecture](architecture.md) for module dependency diagrams.

## Dependency Graph

```mermaid
{{ crate_diagram }}
```
"#;

const ARCHITECTURE_TEMPLATE: &str = r#"---
title: "Architecture"
generated_at: "{{ generated_at }}"
source_commit: "{{ source_commit }}"
---

# Architecture

## Module Dependencies

```mermaid
{{ module_diagram }}
```

## Key Traits & Interfaces

{% for trait in key_traits %}
### `{{ trait.name }}` ({{ trait.kind }})

{{ trait.description }}

{% endfor %}

## Class Diagram

```mermaid
{{ class_diagram }}
```
"#;

const API_TEMPLATE: &str = r#"---
title: "API Reference"
generated_at: "{{ generated_at }}"
source_commit: "{{ source_commit }}"
---

# API Reference

## Router Flow

```mermaid
{{ api_diagram }}
```

## Endpoints

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
{% for ep in endpoints %}| `{{ ep.method }}` | `{{ ep.path }}` | `{{ ep.handler }}` | {{ ep.auth }} | {{ ep.description }} |
{% endfor %}

{% for ep in endpoints %}
### {{ ep.method }} {{ ep.path }}

{% if ep.description %}{{ ep.description }}{% endif %}

- **Handler**: `{{ ep.handler }}`
- **Auth**: {{ ep.auth }}

{% endfor %}
"#;

const MODULE_TEMPLATE: &str = r#"---
title: "Module: {{ module_name }}"
generated_at: "{{ generated_at }}"
source_commit: "{{ source_commit }}"
---

# Module: {{ module_name }}

## Symbols

| Name | Kind | Visibility | Description |
|------|------|------------|-------------|
{% for sym in symbols %}| `{{ sym.name }}` | {{ sym.kind }} | {{ sym.visibility }} | {{ sym.description }} |
{% endfor %}

{% for sym in symbols %}
### `{{ sym.name }}`

**Kind**: {{ sym.kind }} | **Visibility**: {{ sym.visibility }} | **File**: `{{ sym.file }}:{{ sym.line }}`

{% if sym.signature %}
```
{{ sym.signature }}
```
{% endif %}

{{ sym.detailed }}

---

{% endfor %}
"#;
