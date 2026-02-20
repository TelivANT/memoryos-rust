use serde::{Deserialize, Serialize};

use crate::graph::{CodeGraph, CodeGraphNode, EdgeKind};
use crate::ir::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePack {
    pub symbol: SymbolSummary,
    pub doc: Option<String>,
    pub source_snippet: String,
    pub graph_context: GraphContext,
    pub file_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSummary {
    pub kind: String,
    pub qualified_name: String,
    pub signature: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphContext {
    pub implements: Vec<String>,
    pub used_by: Vec<String>,
    pub contains: Vec<String>,
    pub calls: Vec<String>,
    pub handled_endpoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmDocResult {
    pub summary: String,
    pub detailed: String,
    pub usage_example: Option<String>,
    pub sources: Vec<EvidenceRef>,
}

pub fn build_evidence_pack(
    symbol: &Symbol,
    source: &str,
    graph: &CodeGraph,
    file_imports: &str,
) -> EvidencePack {
    let symbol_summary = SymbolSummary {
        kind: symbol.kind.to_string(),
        qualified_name: symbol.qualified_name.clone(),
        signature: symbol.signature.clone(),
        visibility: format!("{:?}", symbol.visibility),
    };

    let snippet = extract_snippet(source, &symbol.span, 200);

    let graph_context = build_graph_context(symbol, graph);

    EvidencePack {
        symbol: symbol_summary,
        doc: symbol.doc.as_ref().map(|d| d.raw.clone()),
        source_snippet: snippet,
        graph_context,
        file_context: file_imports.to_string(),
    }
}

fn extract_snippet(source: &str, span: &Span, max_lines: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let start = span.start_line.saturating_sub(1);
    let end = (span.end_line).min(lines.len());
    let end = end.min(start + max_lines);

    lines[start..end].join("\n")
}

fn build_graph_context(symbol: &Symbol, graph: &CodeGraph) -> GraphContext {
    let mut ctx = GraphContext {
        implements: Vec::new(),
        used_by: Vec::new(),
        contains: Vec::new(),
        calls: Vec::new(),
        handled_endpoints: Vec::new(),
    };

    let sym_key = symbol.id.stable_key();
    for (idx, data) in graph.symbol_nodes() {
        if data.symbol_id == sym_key {
            for neighbor in graph.neighbors(idx, petgraph::Direction::Outgoing) {
                if let Some(weight) = graph.graph.node_weight(neighbor) {
                    let edge = graph
                        .graph
                        .edges_connecting(idx, neighbor)
                        .next()
                        .map(|e| e.weight().kind);

                    match (edge, weight) {
                        (Some(EdgeKind::Implements), CodeGraphNode::SymbolNode(n)) => {
                            ctx.implements.push(n.qualified_name.clone());
                        }
                        (Some(EdgeKind::Contains), CodeGraphNode::SymbolNode(n)) => {
                            ctx.contains.push(n.qualified_name.clone());
                        }
                        (Some(EdgeKind::Calls), CodeGraphNode::SymbolNode(n)) => {
                            ctx.calls.push(n.qualified_name.clone());
                        }
                        _ => {}
                    }
                }
            }

            for neighbor in graph.neighbors(idx, petgraph::Direction::Incoming) {
                if let Some(weight) = graph.graph.node_weight(neighbor) {
                    match weight {
                        CodeGraphNode::SymbolNode(n) => {
                            ctx.used_by.push(n.qualified_name.clone());
                        }
                        CodeGraphNode::EndpointNode(e) => {
                            ctx.handled_endpoints
                                .push(format!("{} {}", e.method, e.path));
                        }
                        _ => {}
                    }
                }
            }
            break;
        }
    }

    ctx
}

pub fn format_evidence_prompt(pack: &EvidencePack) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "## Symbol: {} ({})\n",
        pack.symbol.qualified_name, pack.symbol.kind
    ));

    if let Some(ref sig) = pack.symbol.signature {
        prompt.push_str(&format!("**Signature**: `{}`\n", sig));
    }

    prompt.push_str(&format!("**Visibility**: {}\n\n", pack.symbol.visibility));

    if let Some(ref doc) = pack.doc {
        prompt.push_str(&format!("### Existing Documentation\n{}\n\n", doc));
    }

    prompt.push_str("### Source Code\n```\n");
    prompt.push_str(&pack.source_snippet);
    prompt.push_str("\n```\n\n");

    if !pack.graph_context.implements.is_empty() {
        prompt.push_str("### Implements\n");
        for item in &pack.graph_context.implements {
            prompt.push_str(&format!("- `{}`\n", item));
        }
        prompt.push('\n');
    }

    if !pack.graph_context.contains.is_empty() {
        prompt.push_str("### Contains\n");
        for item in &pack.graph_context.contains {
            prompt.push_str(&format!("- `{}`\n", item));
        }
        prompt.push('\n');
    }

    if !pack.graph_context.used_by.is_empty() {
        prompt.push_str("### Used By\n");
        for item in &pack.graph_context.used_by {
            prompt.push_str(&format!("- `{}`\n", item));
        }
        prompt.push('\n');
    }

    if !pack.graph_context.handled_endpoints.is_empty() {
        prompt.push_str("### Handles Endpoints\n");
        for item in &pack.graph_context.handled_endpoints {
            prompt.push_str(&format!("- `{}`\n", item));
        }
        prompt.push('\n');
    }

    if !pack.file_context.is_empty() {
        prompt.push_str("### File Imports\n```\n");
        prompt.push_str(&pack.file_context);
        prompt.push_str("\n```\n");
    }

    prompt
}
