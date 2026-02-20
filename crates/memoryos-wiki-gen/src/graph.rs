use std::collections::HashMap;
use std::path::PathBuf;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use serde::{Deserialize, Serialize};

use crate::ir::*;

#[derive(Debug)]
pub struct CodeGraph {
    pub graph: DiGraph<CodeGraphNode, CodeGraphEdge>,
    symbol_index: HashMap<String, NodeIndex>,
    file_index: HashMap<PathBuf, NodeIndex>,
    endpoint_index: HashMap<String, NodeIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeGraphNode {
    FileNode(FileNodeData),
    SymbolNode(SymbolNodeData),
    EndpointNode(EndpointNodeData),
    ExternalDep(DepNodeData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNodeData {
    pub path: PathBuf,
    pub language: Language,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNodeData {
    pub symbol_id: String,
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointNodeData {
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepNodeData {
    pub name: String,
    pub ecosystem: Ecosystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraphEdge {
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    FileImports,
    Contains,
    Implements,
    Extends,
    UsesType,
    FieldType,
    Calls,
    HandledBy,
    MiddlewareApplied,
    DependsOn,
}

impl std::fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileImports => write!(f, "imports"),
            Self::Contains => write!(f, "contains"),
            Self::Implements => write!(f, "implements"),
            Self::Extends => write!(f, "extends"),
            Self::UsesType => write!(f, "uses_type"),
            Self::FieldType => write!(f, "field_type"),
            Self::Calls => write!(f, "calls"),
            Self::HandledBy => write!(f, "handled_by"),
            Self::MiddlewareApplied => write!(f, "middleware"),
            Self::DependsOn => write!(f, "depends_on"),
        }
    }
}

impl Default for CodeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            symbol_index: HashMap::new(),
            file_index: HashMap::new(),
            endpoint_index: HashMap::new(),
        }
    }

    pub fn build_from_ir(ir: &RepoIR) -> Self {
        let mut graph = Self::new();

        for file in &ir.files {
            graph.add_file(file);
        }

        for symbol in &ir.symbols {
            graph.add_symbol(symbol);
        }

        for reference in &ir.references {
            graph.add_reference(reference);
        }

        for endpoint in &ir.endpoints {
            graph.add_endpoint(endpoint);
        }

        for manifest in &ir.manifests {
            for dep in &manifest.dependencies {
                graph.add_dependency(dep);
            }
        }

        graph
    }

    fn add_file(&mut self, file: &FileIR) {
        let node = self.graph.add_node(CodeGraphNode::FileNode(FileNodeData {
            path: file.path.clone(),
            language: file.language,
        }));
        self.file_index.insert(file.path.clone(), node);
    }

    fn add_symbol(&mut self, symbol: &Symbol) {
        let key = symbol.id.stable_key();
        let node = self
            .graph
            .add_node(CodeGraphNode::SymbolNode(SymbolNodeData {
                symbol_id: key.clone(),
                qualified_name: symbol.qualified_name.clone(),
                kind: symbol.kind,
                visibility: symbol.visibility,
                file: symbol.file.clone(),
            }));
        self.symbol_index.insert(key, node);

        if let Some(file_node) = self.file_index.get(&symbol.file) {
            self.graph.add_edge(
                *file_node,
                node,
                CodeGraphEdge {
                    kind: EdgeKind::Contains,
                },
            );
        }

        if let Some(ref parent_id) = symbol.parent {
            let parent_key = parent_id.stable_key();
            if let Some(parent_node) = self.symbol_index.get(&parent_key) {
                self.graph.add_edge(
                    *parent_node,
                    node,
                    CodeGraphEdge {
                        kind: EdgeKind::Contains,
                    },
                );
            }
        }
    }

    fn add_reference(&mut self, reference: &Reference) {
        let source_key = reference.source.stable_key();
        let source_node = match self.symbol_index.get(&source_key) {
            Some(n) => *n,
            None => return,
        };

        let edge_kind = match reference.kind {
            ReferenceKind::Import => EdgeKind::FileImports,
            ReferenceKind::Implements => EdgeKind::Implements,
            ReferenceKind::Extends => EdgeKind::Extends,
            ReferenceKind::UsesType => EdgeKind::UsesType,
            ReferenceKind::FieldType => EdgeKind::FieldType,
            ReferenceKind::Call => EdgeKind::Calls,
            ReferenceKind::AnnotationUsage => EdgeKind::UsesType,
        };

        match &reference.target {
            ReferenceTarget::Resolved(target_id) => {
                let target_key = target_id.stable_key();
                if let Some(target_node) = self.symbol_index.get(&target_key) {
                    self.graph.add_edge(
                        source_node,
                        *target_node,
                        CodeGraphEdge { kind: edge_kind },
                    );
                }
            }
            ReferenceTarget::Unresolved(name) => {
                if let Some(target_node) = self.find_symbol_by_name(name) {
                    self.graph.add_edge(
                        source_node,
                        target_node,
                        CodeGraphEdge { kind: edge_kind },
                    );
                }
            }
        }
    }

    fn add_endpoint(&mut self, endpoint: &Endpoint) {
        let key = format!("{} {}", endpoint.method, endpoint.path);
        let node = self
            .graph
            .add_node(CodeGraphNode::EndpointNode(EndpointNodeData {
                method: endpoint.method.to_string(),
                path: endpoint.path.clone(),
            }));
        self.endpoint_index.insert(key, node);

        if let Some(ref handler_id) = endpoint.handler {
            let handler_key = handler_id.stable_key();
            if let Some(handler_node) = self.symbol_index.get(&handler_key) {
                self.graph.add_edge(
                    node,
                    *handler_node,
                    CodeGraphEdge {
                        kind: EdgeKind::HandledBy,
                    },
                );
            }
        }
    }

    fn add_dependency(&mut self, dep: &Dependency) {
        let dep_node = self.graph.add_node(CodeGraphNode::ExternalDep(DepNodeData {
            name: dep.name.clone(),
            ecosystem: dep.ecosystem,
        }));

        if let Some(file_node) = self.file_index.get(&dep.source_file) {
            self.graph.add_edge(
                *file_node,
                dep_node,
                CodeGraphEdge {
                    kind: EdgeKind::DependsOn,
                },
            );
        }
    }

    fn find_symbol_by_name(&self, name: &str) -> Option<NodeIndex> {
        for &node_idx in self.symbol_index.values() {
            if let Some(CodeGraphNode::SymbolNode(data)) = self.graph.node_weight(node_idx) {
                if data.qualified_name.ends_with(name) || data.qualified_name == name {
                    return Some(node_idx);
                }
            }
        }
        None
    }

    pub fn neighbors(&self, node: NodeIndex, direction: Direction) -> Vec<NodeIndex> {
        self.graph.neighbors_directed(node, direction).collect()
    }

    pub fn subgraph(&self, root: NodeIndex, max_depth: usize) -> Vec<NodeIndex> {
        let mut visited = Vec::new();
        let mut queue = vec![(root, 0usize)];

        while let Some((node, depth)) = queue.pop() {
            if depth > max_depth || visited.contains(&node) {
                continue;
            }
            visited.push(node);
            for neighbor in self.graph.neighbors(node) {
                queue.push((neighbor, depth + 1));
            }
        }

        visited
    }

    pub fn symbol_nodes(&self) -> Vec<(NodeIndex, &SymbolNodeData)> {
        self.graph
            .node_indices()
            .filter_map(|idx| match self.graph.node_weight(idx) {
                Some(CodeGraphNode::SymbolNode(data)) => Some((idx, data)),
                _ => None,
            })
            .collect()
    }

    pub fn file_nodes(&self) -> Vec<(NodeIndex, &FileNodeData)> {
        self.graph
            .node_indices()
            .filter_map(|idx| match self.graph.node_weight(idx) {
                Some(CodeGraphNode::FileNode(data)) => Some((idx, data)),
                _ => None,
            })
            .collect()
    }

    pub fn endpoint_nodes(&self) -> Vec<(NodeIndex, &EndpointNodeData)> {
        self.graph
            .node_indices()
            .filter_map(|idx| match self.graph.node_weight(idx) {
                Some(CodeGraphNode::EndpointNode(data)) => Some((idx, data)),
                _ => None,
            })
            .collect()
    }

    pub fn modules_at_depth(&self, target_depth: usize) -> Vec<(NodeIndex, &SymbolNodeData)> {
        self.symbol_nodes()
            .into_iter()
            .filter(|(_, data)| {
                data.kind == SymbolKind::Module
                    && data.qualified_name.matches("::").count() == target_depth
            })
            .collect()
    }

    pub fn endpoints_by_tag(&self) -> HashMap<String, Vec<(NodeIndex, &EndpointNodeData)>> {
        let mut grouped: HashMap<String, Vec<(NodeIndex, &EndpointNodeData)>> = HashMap::new();
        for (idx, data) in self.endpoint_nodes() {
            let parts: Vec<&str> = data.path.split('/').filter(|p| !p.is_empty()).collect();
            let tag = parts.first().unwrap_or(&"default").to_string();
            grouped.entry(tag).or_default().push((idx, data));
        }
        grouped
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}
