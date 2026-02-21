use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use crate::cache::{compute_hash, CacheStore};
use crate::config::LlmConfig;
use crate::error::{WikiGenError, WikiGenResult};
use crate::evidence::{build_evidence_pack, format_evidence_prompt, EvidenceRef, LlmDocResult};
use crate::graph::CodeGraph;
use crate::ir::*;

use crate::llm_adapter::{ChatMessage, ChatRequest, WikiLlmAdapter};

pub struct LlmDocGenerator {
    config: LlmConfig,
    semaphore: Semaphore,
    adapter: Option<Arc<dyn WikiLlmAdapter>>,
}

impl LlmDocGenerator {
    pub fn new(config: LlmConfig) -> Self {
        let max_concurrent = config.max_concurrent;
        Self {
            config,
            semaphore: Semaphore::new(max_concurrent),
            adapter: None,
        }
    }

    pub fn with_adapter(config: LlmConfig, adapter: Arc<dyn WikiLlmAdapter>) -> Self {
        let max_concurrent = config.max_concurrent;
        Self {
            config,
            semaphore: Semaphore::new(max_concurrent),
            adapter: Some(adapter),
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
            "Generating docs for {} public symbols (adapter: {})",
            public_symbols.len(),
            if self.adapter.is_some() {
                "connected"
            } else {
                "fallback"
            }
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

    async fn call_llm(&self, prompt: &str, symbol: &Symbol) -> WikiGenResult<LlmDocResult> {
        let adapter = match &self.adapter {
            Some(a) => a.clone(),
            None => {
                debug!(
                    "No LLM adapter configured, using fallback for {}",
                    symbol.qualified_name
                );
                return Ok(self.generate_fallback(symbol));
            }
        };

        debug!(
            "LLM call for {} (adapter: {}, model: {})",
            symbol.qualified_name,
            adapter.name(),
            self.config.model
        );

        let system_prompt = "You are a technical documentation generator. Given a code symbol with its source code, documentation comments, and graph context, generate clear and concise documentation.\n\nRespond in this exact JSON format:\n{\"summary\": \"one-line summary\", \"detailed\": \"detailed markdown description\", \"usage_example\": \"optional code example or null\"}".to_string();

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: format!(
                        "Generate documentation for the following symbol:\n\n{}",
                        prompt
                    ),
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(1024),
            stream: false,
            extra: HashMap::new(),
        };

        let mut last_error = None;
        for attempt in 0..self.config.retry_max {
            if attempt > 0 {
                let backoff =
                    std::time::Duration::from_millis(self.config.retry_backoff_ms * (1 << attempt));
                debug!(
                    "Retry attempt {} for {} (backoff: {:?})",
                    attempt, symbol.qualified_name, backoff
                );
                tokio::time::sleep(backoff).await;
            }

            match adapter.chat(request.clone()).await {
                Ok(response) => {
                    let content = response
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default();

                    return self.parse_llm_response(&content, symbol);
                }
                Err(e) => {
                    warn!(
                        "LLM attempt {} failed for {}: {}",
                        attempt + 1,
                        symbol.qualified_name,
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(WikiGenError::Llm(format!(
            "All {} retries exhausted for {}: {}",
            self.config.retry_max,
            symbol.qualified_name,
            last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        )))
    }

    fn parse_llm_response(&self, content: &str, symbol: &Symbol) -> WikiGenResult<LlmDocResult> {
        let trimmed = content.trim();
        let json_str = if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                &trimmed[start..=end]
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        #[derive(serde::Deserialize)]
        struct LlmOutput {
            summary: Option<String>,
            detailed: Option<String>,
            usage_example: Option<String>,
        }

        match serde_json::from_str::<LlmOutput>(json_str) {
            Ok(parsed) => Ok(LlmDocResult {
                summary: parsed
                    .summary
                    .unwrap_or_else(|| format!("{} `{}`", symbol.kind, symbol.qualified_name)),
                detailed: parsed.detailed.unwrap_or_else(|| content.to_string()),
                usage_example: parsed.usage_example,
                sources: vec![EvidenceRef {
                    file: symbol.file.display().to_string(),
                    start_line: symbol.span.start_line,
                    end_line: symbol.span.end_line,
                }],
            }),
            Err(_) => {
                debug!(
                    "Failed to parse LLM JSON for {}, using raw content",
                    symbol.qualified_name
                );
                Ok(LlmDocResult {
                    summary: content
                        .lines()
                        .next()
                        .unwrap_or(content)
                        .chars()
                        .take(200)
                        .collect(),
                    detailed: content.to_string(),
                    usage_example: None,
                    sources: vec![EvidenceRef {
                        file: symbol.file.display().to_string(),
                        start_line: symbol.span.start_line,
                        end_line: symbol.span.end_line,
                    }],
                })
            }
        }
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

        if self.adapter.is_some() {
            info!("Overview generated with LLM adapter connected");
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
