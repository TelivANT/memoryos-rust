use std::collections::HashMap;

use tokio::sync::Semaphore;
use tracing::{debug, warn};

use crate::cache::{compute_hash, CacheStore};
use crate::config::LlmConfig;
use crate::error::{WikiGenError, WikiGenResult};
use crate::evidence::{build_evidence_pack, format_evidence_prompt, EvidenceRef, LlmDocResult};
use crate::graph::CodeGraph;
use crate::ir::*;

pub struct LlmDocGenerator {
    config: LlmConfig,
    semaphore: Semaphore,
}

impl LlmDocGenerator {
    pub fn new(config: LlmConfig) -> Self {
        let max_concurrent = config.max_concurrent;
        Self {
            config,
            semaphore: Semaphore::new(max_concurrent),
        }
    }

    pub async fn generate_all(
        &self,
        ir: &RepoIR,
        graph: &CodeGraph,
        file_contents: &HashMap<std::path::PathBuf, String>,
        cache: &mut CacheStore,
    ) -> WikiGenResult<HashMap<String, LlmDocResult>> {
        let mut results = HashMap::new();

        let public_symbols: Vec<&Symbol> = ir
            .symbols
            .iter()
            .filter(|s| matches!(s.visibility, Visibility::Public))
            .filter(|s| {
                matches!(
                    s.kind,
                    SymbolKind::Function
                        | SymbolKind::Struct
                        | SymbolKind::Trait
                        | SymbolKind::Class
                        | SymbolKind::Interface
                        | SymbolKind::Enum
                        | SymbolKind::Method
                        | SymbolKind::Component
                )
            })
            .collect();

        debug!(
            "Generating docs for {} public symbols",
            public_symbols.len()
        );

        for symbol in public_symbols {
            let source = file_contents
                .get(&symbol.file)
                .map(|s| s.as_str())
                .unwrap_or("");

            let file_imports = extract_imports(source);

            let pack = build_evidence_pack(symbol, source, graph, &file_imports);
            let prompt = format_evidence_prompt(&pack);
            let prompt_hash = compute_hash(&prompt);
            let symbol_hash = compute_hash(&symbol.id.stable_key());

            if let Some(cached) = cache.lookup(&symbol_hash, &prompt_hash) {
                debug!("Cache hit for {}", symbol.qualified_name);
                results.insert(symbol.id.stable_key(), cached.clone());
                continue;
            }

            let _permit = self
                .semaphore
                .acquire()
                .await
                .map_err(|e| WikiGenError::Llm(format!("Semaphore acquire failed: {}", e)))?;

            match self.call_llm(&prompt, symbol).await {
                Ok(result) => {
                    let content_hash = compute_hash(&result.summary);
                    cache.insert(symbol_hash, prompt_hash, content_hash, result.clone());
                    results.insert(symbol.id.stable_key(), result);
                }
                Err(e) => {
                    warn!("LLM call failed for {}: {}", symbol.qualified_name, e);
                    let fallback = self.generate_fallback(symbol);
                    results.insert(symbol.id.stable_key(), fallback);
                }
            }
        }

        Ok(results)
    }

    async fn call_llm(&self, _prompt: &str, symbol: &Symbol) -> WikiGenResult<LlmDocResult> {
        debug!(
            "LLM call for {} (adapter: {}, model: {})",
            symbol.qualified_name, self.config.adapter, self.config.model
        );

        let result = self.generate_fallback(symbol);
        Ok(result)
    }

    fn generate_fallback(&self, symbol: &Symbol) -> LlmDocResult {
        let summary = if let Some(ref doc) = symbol.doc {
            doc.summary
                .clone()
                .unwrap_or_else(|| format!("{} `{}`", symbol.kind, symbol.qualified_name))
        } else {
            format!("{} `{}`", symbol.kind, symbol.qualified_name)
        };

        let detailed = if let Some(ref doc) = symbol.doc {
            doc.raw.clone()
        } else if let Some(ref sig) = symbol.signature {
            format!("A {} defined as:\n\n```\n{}\n```", symbol.kind, sig)
        } else {
            format!("A {} named `{}`.", symbol.kind, symbol.qualified_name)
        };

        LlmDocResult {
            summary,
            detailed,
            usage_example: None,
            sources: vec![EvidenceRef {
                file: symbol.file.display().to_string(),
                start_line: symbol.span.start_line,
                end_line: symbol.span.end_line,
            }],
        }
    }

    pub async fn generate_overview(
        &self,
        ir: &RepoIR,
        symbol_docs: &HashMap<String, LlmDocResult>,
    ) -> WikiGenResult<String> {
        let mut overview = String::new();

        overview.push_str(&format!(
            "# Project Overview\n\n**Repository**: `{}`\n\n",
            ir.repo_root.display()
        ));

        overview.push_str(&format!(
            "- **Files**: {}\n- **Symbols**: {}\n- **Endpoints**: {}\n\n",
            ir.files.len(),
            ir.symbols.len(),
            ir.endpoints.len(),
        ));

        let mut lang_counts: HashMap<Language, usize> = HashMap::new();
        for file in &ir.files {
            *lang_counts.entry(file.language).or_insert(0) += 1;
        }

        overview.push_str("## Languages\n\n");
        for (lang, count) in &lang_counts {
            overview.push_str(&format!("- **{}**: {} files\n", lang, count));
        }

        overview.push_str("\n## Key Symbols\n\n");
        let key_symbols: Vec<&Symbol> = ir
            .symbols
            .iter()
            .filter(|s| {
                matches!(s.visibility, Visibility::Public)
                    && matches!(
                        s.kind,
                        SymbolKind::Struct
                            | SymbolKind::Trait
                            | SymbolKind::Class
                            | SymbolKind::Interface
                    )
            })
            .take(20)
            .collect();

        for sym in key_symbols {
            let doc_summary = symbol_docs
                .get(&sym.id.stable_key())
                .map(|d| d.summary.as_str())
                .unwrap_or("No description");
            overview.push_str(&format!(
                "- **`{}`** ({}): {}\n",
                sym.qualified_name, sym.kind, doc_summary
            ));
        }

        Ok(overview)
    }
}

fn extract_imports(source: &str) -> String {
    source
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.starts_with("use ")
                || trimmed.starts_with("import ")
                || trimmed.starts_with("from ")
                || trimmed.starts_with("require(")
        })
        .take(30)
        .collect::<Vec<&str>>()
        .join("\n")
}
