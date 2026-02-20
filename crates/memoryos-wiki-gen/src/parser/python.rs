use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::parser::{LanguageParser, ParseOutput};

pub struct PythonParser {
    _private: (),
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonParser {
    pub fn new() -> Self {
        Self { _private: () }
    }

    fn extract_symbols(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> (Vec<Symbol>, Vec<Reference>, Vec<Diagnostic>) {
        let mut symbols = Vec::new();
        let mut references = Vec::new();
        let mut diagnostics = Vec::new();

        let root = tree.root_node();
        self.visit_node(
            root,
            source,
            file_path,
            None,
            &mut symbols,
            &mut references,
            &mut diagnostics,
        );
        (symbols, references, diagnostics)
    }

    fn visit_node(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
        symbols: &mut Vec<Symbol>,
        references: &mut Vec<Reference>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match node.kind() {
            "function_definition" => {
                if let Some(sym) = self.extract_function(node, source, file_path, parent_id) {
                    let sym_id = sym.id.clone();
                    symbols.push(sym);
                    self.visit_children(
                        node,
                        source,
                        file_path,
                        Some(&sym_id),
                        symbols,
                        references,
                        diagnostics,
                    );
                    return;
                }
            }
            "class_definition" => {
                if let Some(sym) = self.extract_class(node, source, file_path, parent_id) {
                    let sym_id = sym.id.clone();
                    symbols.push(sym);
                    self.visit_children(
                        node,
                        source,
                        file_path,
                        Some(&sym_id),
                        symbols,
                        references,
                        diagnostics,
                    );
                    return;
                }
            }
            "import_statement" | "import_from_statement" => {
                if let Some(name) = node_text(node, source) {
                    if let Some(parent) = parent_id {
                        references.push(Reference {
                            source: parent.clone(),
                            target: ReferenceTarget::Unresolved(name),
                            kind: ReferenceKind::Import,
                            span: node_span(node),
                        });
                    }
                }
            }
            "decorated_definition" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() != "decorator" {
                        self.visit_node(
                            child,
                            source,
                            file_path,
                            parent_id,
                            symbols,
                            references,
                            diagnostics,
                        );
                    }
                }
                return;
            }
            _ => {}
        }

        self.visit_children(
            node,
            source,
            file_path,
            parent_id,
            symbols,
            references,
            diagnostics,
        );
    }

    fn visit_children(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
        symbols: &mut Vec<Symbol>,
        references: &mut Vec<Reference>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(
                child,
                source,
                file_path,
                parent_id,
                symbols,
                references,
                diagnostics,
            );
        }
    }

    fn extract_function(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let doc = self.extract_docstring(node, source);
        let decorators = self.extract_decorators(node, source);

        let is_method = parent_id.is_some()
            && parent_id
                .map(|p| p.kind == SymbolKind::Class)
                .unwrap_or(false);

        let kind = if is_method {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        };

        let visibility = if name.starts_with('_') && !name.starts_with("__") {
            Visibility::Private
        } else {
            Visibility::Public
        };

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            kind,
            qname.clone(),
        );

        let signature = self.extract_signature(node, source);

        Some(Symbol {
            id,
            kind,
            qualified_name: qname,
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature,
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: decorators,
            type_params: Vec::new(),
        })
    }

    fn extract_class(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let doc = self.extract_docstring(node, source);
        let decorators = self.extract_decorators(node, source);

        let visibility = if name.starts_with('_') {
            Visibility::Private
        } else {
            Visibility::Public
        };

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::Class,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Class,
            qualified_name: qname,
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: Some(format!("class {}", name)),
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: decorators,
            type_params: Vec::new(),
        })
    }

    fn extract_docstring(&self, node: tree_sitter::Node, source: &[u8]) -> Option<Doc> {
        let body = node.child_by_field_name("body")?;
        let first_child = body.child(0)?;
        if first_child.kind() == "expression_statement" {
            let expr = first_child.child(0)?;
            if expr.kind() == "string" {
                let text = node_text(expr, source)?;
                let raw = text
                    .trim_start_matches("\"\"\"")
                    .trim_start_matches("'''")
                    .trim_end_matches("\"\"\"")
                    .trim_end_matches("'''")
                    .trim()
                    .to_string();
                let summary = raw.lines().next().map(|l| l.trim().to_string());
                return Some(Doc {
                    raw,
                    format: DocFormat::ReST,
                    summary,
                });
            }
        }
        None
    }

    fn extract_decorators(&self, node: tree_sitter::Node, source: &[u8]) -> Vec<Annotation> {
        let mut decorators = Vec::new();
        let parent = node.parent();
        if let Some(p) = parent {
            if p.kind() == "decorated_definition" {
                let mut cursor = p.walk();
                for child in p.children(&mut cursor) {
                    if child.kind() == "decorator" {
                        if let Some(text) = node_text(child, source) {
                            let name = text.trim_start_matches('@').to_string();
                            decorators.push(Annotation {
                                name,
                                arguments: Vec::new(),
                                span: node_span(child),
                            });
                        }
                    }
                }
            }
        }
        decorators
    }

    fn extract_signature(&self, node: tree_sitter::Node, source: &[u8]) -> Option<String> {
        let text = node_text(node, source)?;
        let first_line = text.lines().next().unwrap_or(&text);
        let sig = first_line.trim_end_matches(':').trim();
        if sig.len() > 200 {
            Some(format!("{}...", &sig[..200]))
        } else {
            Some(sig.to_string())
        }
    }
}

impl LanguageParser for PythonParser {
    fn language(&self) -> Language {
        Language::Python
    }

    fn parse(&self, source: &[u8], file_path: &Path) -> WikiGenResult<ParseOutput> {
        let mut parser = tree_sitter::Parser::new();
        let lang = tree_sitter_python::LANGUAGE;
        parser
            .set_language(&lang.into())
            .map_err(|e| WikiGenError::Parse {
                file: file_path.display().to_string(),
                message: format!("Failed to set Python language: {}", e),
            })?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| WikiGenError::Parse {
                file: file_path.display().to_string(),
                message: "Tree-sitter parse returned None".to_string(),
            })?;

        let has_errors = tree.root_node().has_error();
        let parse_status = if has_errors {
            ParseStatus::PartialSuccess
        } else {
            ParseStatus::Success
        };

        let mut hasher = Sha256::new();
        hasher.update(source);
        let content_hash = format!("{:x}", hasher.finalize());

        let (symbols, references, diagnostics) = self.extract_symbols(&tree, source, file_path);

        let file_ir = FileIR {
            path: file_path.to_path_buf(),
            language: Language::Python,
            content_hash,
            parse_status,
            byte_count: source.len(),
        };

        Ok(ParseOutput {
            file: file_ir,
            symbols,
            references,
            diagnostics,
        })
    }
}

fn node_text(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(|s| s.to_string())
}

fn node_span(node: tree_sitter::Node) -> Span {
    Span::new(
        node.start_byte(),
        node.end_byte(),
        node.start_position().row + 1,
        node.end_position().row + 1,
    )
}

fn build_qualified_name(file_path: &Path, name: &str) -> String {
    let stem = file_path
        .with_extension("")
        .to_string_lossy()
        .replace(['/', '\\'], ".");
    format!("{}.{}", stem, name)
}
