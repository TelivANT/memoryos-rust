pub mod cache;
pub mod config;
pub mod diagram;
pub mod discovery;
pub mod endpoint;
pub mod error;
pub mod evidence;
pub mod export;
pub mod graph;
pub mod ir;
pub mod lang;
pub mod llm_adapter;
pub mod llm_gen;
pub mod manifest;
pub mod page_builder;
pub mod parser;
pub mod storage;
pub mod wiki_index;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use tracing::{debug, info, warn};

use cache::CacheStore;
use config::WikiGenConfig;
use discovery::FileDiscovery;
use endpoint::detect_framework;
use error::WikiGenResult;
use export::ExportOrchestrator;
use graph::CodeGraph;
use ir::RepoIR;
use lang::FileType;
use llm_gen::LlmDocGenerator;
use manifest::create_extractor;
use page_builder::PageBuilder;
use parser::{create_parser, ParseOutput};
use wiki_index::WikiIndex;

pub use llm_adapter::WikiLlmAdapter;

pub struct WikiGenerator {
    config: WikiGenConfig,
    llm_adapter: Option<Arc<dyn WikiLlmAdapter>>,
}

impl WikiGenerator {
    pub fn new(config: WikiGenConfig) -> Self {
        Self {
            config,
            llm_adapter: None,
        }
    }

    pub fn with_llm_adapter(config: WikiGenConfig, adapter: Arc<dyn WikiLlmAdapter>) -> Self {
        Self {
            config,
            llm_adapter: Some(adapter),
        }
    }

    pub async fn generate(&self, repo_root: &Path) -> WikiGenResult<()> {
        let output_dir = self.config.resolved_output_dir(repo_root);
        let cache_dir = self.config.resolved_cache_dir(repo_root);

        info!("Wiki generation starting for {}", repo_root.display());

        let ir = self.phase0_and_phase1(repo_root)?;

        info!(
            "Phase 1 complete: {} files, {} symbols, {} references",
            ir.files.len(),
            ir.symbols.len(),
            ir.references.len()
        );

        let graph = CodeGraph::build_from_ir(&ir);
        info!(
            "Phase 2 complete: {} nodes, {} edges",
            graph.node_count(),
            graph.edge_count()
        );

        let file_contents = self.load_file_contents(&ir, repo_root);

        let mut cache = if self.config.cache.enabled {
            CacheStore::load(&cache_dir).unwrap_or_else(|_| CacheStore::new())
        } else {
            CacheStore::new()
        };

        let llm_gen = match &self.llm_adapter {
            Some(adapter) => {
                LlmDocGenerator::with_adapter(self.config.llm.clone(), adapter.clone())
            }
            None => LlmDocGenerator::new(self.config.llm.clone()),
        };
        let symbol_docs = llm_gen
            .generate_all(&ir, &graph, &file_contents, &mut cache)
            .await?;

        let overview = llm_gen.generate_overview(&ir, &symbol_docs).await?;

        info!(
            "Phase 3 complete: {} symbol docs generated",
            symbol_docs.len()
        );

        let source_commit = self.get_git_commit(repo_root);

        let page_builder = PageBuilder::new()?;
        let pages = page_builder.build_all(&ir, &graph, &symbol_docs, &overview, &source_commit)?;

        info!("Phase 5 complete: {} pages built", pages.len());

        let wiki_index = WikiIndex::build(&pages, &source_commit);

        ExportOrchestrator::export_local(&pages, &wiki_index, &output_dir)?;

        if self.config.cache.enabled {
            cache.save(&cache_dir)?;
        }

        info!(
            "Wiki generation complete: {} pages exported to {}",
            pages.len(),
            output_dir.display()
        );

        Ok(())
    }

    pub fn parse_only(&self, repo_root: &Path) -> WikiGenResult<RepoIR> {
        self.phase0_and_phase1(repo_root)
    }

    fn phase0_and_phase1(&self, repo_root: &Path) -> WikiGenResult<RepoIR> {
        let discovery = FileDiscovery::new(self.config.clone());
        let discovered = discovery.discover(repo_root)?;

        info!("Phase 0 complete: {} files discovered", discovered.len());

        let pb = ProgressBar::new(discovered.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        let mut ir = RepoIR::new(repo_root.to_path_buf());

        let source_files: Vec<_> = discovered
            .iter()
            .filter(|f| matches!(f.file_type, FileType::Source(_)))
            .collect();

        let parse_results: Vec<WikiGenResult<(ParseOutput, PathBuf)>> = source_files
            .par_iter()
            .map(|file| {
                let content = std::fs::read(&file.absolute_path)?;
                let language = match file.file_type {
                    FileType::Source(lang) => lang,
                    _ => unreachable!(),
                };
                let parser = create_parser(language);
                let output = parser.parse(&content, &file.relative_path)?;
                Ok((output, file.absolute_path.clone()))
            })
            .collect();

        for result in parse_results {
            match result {
                Ok((output, _path)) => {
                    ir.files.push(output.file);
                    ir.symbols.extend(output.symbols);
                    ir.references.extend(output.references);
                    ir.diagnostics.extend(output.diagnostics);
                }
                Err(e) => {
                    warn!("Parse error: {}", e);
                }
            }
            pb.inc(1);
        }

        let manifest_files: Vec<_> = discovered
            .iter()
            .filter(|f| matches!(f.file_type, FileType::Manifest(_)))
            .collect();

        for file in manifest_files {
            if let FileType::Manifest(manifest_type) = file.file_type {
                match std::fs::read(&file.absolute_path) {
                    Ok(content) => {
                        let extractor = create_extractor(manifest_type);
                        match extractor.extract(&content, &file.relative_path) {
                            Ok(manifest) => ir.manifests.push(manifest),
                            Err(e) => warn!("Manifest parse error: {}", e),
                        }
                    }
                    Err(e) => warn!("Failed to read {}: {}", file.absolute_path.display(), e),
                }
            }
            pb.inc(1);
        }

        let spec_files: Vec<_> = discovered
            .iter()
            .filter(|f| matches!(f.file_type, FileType::ApiSpec(_)))
            .collect();

        for file in spec_files {
            if let FileType::ApiSpec(spec_type) = file.file_type {
                match std::fs::read(&file.absolute_path) {
                    Ok(content) => {
                        match endpoint::spec_discovery::discover_api_spec(
                            &content,
                            &file.relative_path,
                            spec_type,
                        ) {
                            Ok(spec) => {
                                ir.endpoints.extend(spec.endpoints.clone());
                                ir.specs.push(spec);
                            }
                            Err(e) => warn!("Spec parse error: {}", e),
                        }
                    }
                    Err(e) => warn!("Failed to read {}: {}", file.absolute_path.display(), e),
                }
            }
            pb.inc(1);
        }

        for file in &ir.files.clone() {
            if let Ok(content) = std::fs::read(repo_root.join(&file.path)) {
                let source_str = String::from_utf8_lossy(&content);
                if let Some(extractor) = detect_framework(&source_str, &file.path) {
                    match extractor.extract(&ir.symbols, &content, &file.path) {
                        Ok(endpoints) => ir.endpoints.extend(endpoints),
                        Err(e) => {
                            debug!(
                                "Endpoint extraction failed for {}: {}",
                                file.path.display(),
                                e
                            )
                        }
                    }
                }
            }
        }

        pb.finish_with_message("Parsing complete");

        Ok(ir)
    }

    fn load_file_contents(&self, ir: &RepoIR, repo_root: &Path) -> HashMap<PathBuf, String> {
        let mut contents = HashMap::new();
        for file in &ir.files {
            let abs_path = repo_root.join(&file.path);
            if let Ok(content) = std::fs::read_to_string(&abs_path) {
                contents.insert(file.path.clone(), content);
            }
        }
        contents
    }

    fn get_git_commit(&self, repo_root: &Path) -> String {
        std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_root)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}
