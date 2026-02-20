use std::collections::HashMap;

use crate::graph::{CodeGraph, CodeGraphNode};
use crate::ir::*;

pub struct DiagramGenerator;

impl DiagramGenerator {
    pub fn module_dependency_diagram(graph: &CodeGraph) -> String {
        let mut mermaid = String::from("graph TD\n");

        let file_nodes = graph.file_nodes();
        let mut module_map: HashMap<String, Vec<String>> = HashMap::new();

        for (_, data) in &file_nodes {
            let path_str = data.path.to_string_lossy();
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() >= 2 {
                let module = parts[..parts.len() - 1].join("/");
                let file = parts.last().unwrap_or(&"").to_string();
                module_map.entry(module).or_default().push(file);
            }
        }

        let mut modules: Vec<String> = module_map.keys().cloned().collect();
        modules.sort();

        let max_nodes = 30;
        if modules.len() > max_nodes {
            modules.truncate(max_nodes);
        }

        for module in &modules {
            let safe_id = sanitize_mermaid_id(module);
            let file_count = module_map.get(module).map(|f| f.len()).unwrap_or(0);
            mermaid.push_str(&format!(
                "    {}[\"{} ({} files)\"]\n",
                safe_id, module, file_count
            ));
        }

        for (idx, data) in graph.file_nodes() {
            let edges = graph.neighbors(idx, petgraph::Direction::Outgoing);
            for target_idx in edges {
                if let Some(CodeGraphNode::FileNode(target_data)) =
                    graph.graph.node_weight(target_idx)
                {
                    let source_module = get_module(&data.path.to_string_lossy());
                    let target_module = get_module(&target_data.path.to_string_lossy());
                    if source_module != target_module
                        && modules.contains(&source_module)
                        && modules.contains(&target_module)
                    {
                        let src_id = sanitize_mermaid_id(&source_module);
                        let tgt_id = sanitize_mermaid_id(&target_module);
                        mermaid.push_str(&format!("    {} --> {}\n", src_id, tgt_id));
                    }
                }
            }
        }

        mermaid
    }

    pub fn api_flow_diagram(graph: &CodeGraph) -> String {
        let mut mermaid = String::from("graph LR\n");
        mermaid.push_str("    REQ[HTTP Request] --> ROUTER{Route Match}\n");

        let endpoint_nodes = graph.endpoint_nodes();

        let max_endpoints = 20;
        let endpoints: Vec<_> = endpoint_nodes.into_iter().take(max_endpoints).collect();

        for (idx, data) in &endpoints {
            let safe_id = sanitize_mermaid_id(&format!("{}_{}", data.method, data.path));
            mermaid.push_str(&format!(
                "    ROUTER -->|\"{} {}\"| {}[\"{} {}\"]\n",
                data.method, data.path, safe_id, data.method, data.path
            ));

            let handlers = graph.neighbors(*idx, petgraph::Direction::Outgoing);
            for handler_idx in handlers {
                if let Some(CodeGraphNode::SymbolNode(handler_data)) =
                    graph.graph.node_weight(handler_idx)
                {
                    let handler_id = sanitize_mermaid_id(&handler_data.qualified_name);
                    let short_name = handler_data
                        .qualified_name
                        .split("::")
                        .last()
                        .unwrap_or(&handler_data.qualified_name);
                    mermaid.push_str(&format!(
                        "    {} --> {}[{}]\n",
                        safe_id, handler_id, short_name
                    ));
                }
            }
        }

        mermaid
    }

    pub fn class_diagram(symbols: &[Symbol], references: &[Reference]) -> String {
        let mut mermaid = String::from("classDiagram\n");

        let class_like: Vec<&Symbol> = symbols
            .iter()
            .filter(|s| {
                matches!(
                    s.kind,
                    SymbolKind::Struct
                        | SymbolKind::Trait
                        | SymbolKind::Class
                        | SymbolKind::Interface
                        | SymbolKind::Enum
                )
            })
            .take(30)
            .collect();

        for sym in &class_like {
            let short_name = sym
                .qualified_name
                .split("::")
                .last()
                .unwrap_or(&sym.qualified_name);

            let stereotype = match sym.kind {
                SymbolKind::Trait | SymbolKind::Interface => "<<interface>>",
                SymbolKind::Enum => "<<enumeration>>",
                _ => "",
            };

            if !stereotype.is_empty() {
                mermaid.push_str(&format!("    class {} {{\n", short_name));
                mermaid.push_str(&format!("        {}\n", stereotype));
                mermaid.push_str("    }\n");
            } else {
                mermaid.push_str(&format!("    class {}\n", short_name));
            }

            let children: Vec<&Symbol> = symbols
                .iter()
                .filter(|s| {
                    s.parent
                        .as_ref()
                        .map(|p| p.stable_key() == sym.id.stable_key())
                        .unwrap_or(false)
                        && matches!(s.kind, SymbolKind::Method | SymbolKind::Field)
                })
                .take(10)
                .collect();

            if !children.is_empty() {
                mermaid.push_str(&format!("    class {} {{\n", short_name));
                for child in children {
                    let vis = match child.visibility {
                        Visibility::Public => "+",
                        Visibility::Private => "-",
                        Visibility::Protected => "#",
                        Visibility::Internal => "~",
                    };
                    let child_name = child
                        .qualified_name
                        .split("::")
                        .last()
                        .unwrap_or(&child.qualified_name);
                    mermaid.push_str(&format!("        {}{}\n", vis, child_name));
                }
                mermaid.push_str("    }\n");
            }
        }

        for reference in references {
            if matches!(
                reference.kind,
                ReferenceKind::Implements | ReferenceKind::Extends
            ) {
                let source_name = reference
                    .source
                    .qualified_name()
                    .split("::")
                    .last()
                    .unwrap_or(reference.source.qualified_name());

                let target_name = match &reference.target {
                    ReferenceTarget::Resolved(id) => id
                        .qualified_name()
                        .split("::")
                        .last()
                        .unwrap_or(id.qualified_name())
                        .to_string(),
                    ReferenceTarget::Unresolved(name) => {
                        name.split("::").last().unwrap_or(name).to_string()
                    }
                };

                let arrow = match reference.kind {
                    ReferenceKind::Implements => "..|>",
                    ReferenceKind::Extends => "--|>",
                    _ => "-->",
                };

                mermaid.push_str(&format!("    {} {} {}\n", source_name, arrow, target_name));
            }
        }

        mermaid
    }

    pub fn crate_dependency_diagram(manifests: &[ManifestInfo]) -> String {
        let mut mermaid = String::from("graph TD\n");

        for manifest in manifests {
            let crate_name = manifest
                .source_file
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            let safe_id = sanitize_mermaid_id(crate_name);

            let runtime_deps: Vec<&Dependency> = manifest
                .dependencies
                .iter()
                .filter(|d| matches!(d.scope, DependencyScope::Runtime))
                .take(15)
                .collect();

            for dep in runtime_deps {
                let dep_id = sanitize_mermaid_id(&dep.name);
                mermaid.push_str(&format!("    {} --> {}[{}]\n", safe_id, dep_id, dep.name));
            }
        }

        mermaid
    }
}

fn sanitize_mermaid_id(id: &str) -> String {
    id.replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
        .trim_matches('_')
        .to_string()
}

fn get_module(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 {
        parts[..parts.len() - 1].join("/")
    } else {
        path.to_string()
    }
}
