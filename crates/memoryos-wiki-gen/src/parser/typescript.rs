use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::parser::{LanguageParser, ParseOutput};

pub struct TypeScriptParser {
    _private: (),
}

impl Default for TypeScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptParser {
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
            "function_declaration" => {
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
            "class_declaration" => {
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
            "interface_declaration" => {
                if let Some(sym) = self.extract_interface(node, source, file_path, parent_id) {
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
            "type_alias_declaration" => {
                if let Some(sym) = self.extract_type_alias(node, source, file_path, parent_id) {
                    symbols.push(sym);
                }
            }
            "enum_declaration" => {
                if let Some(sym) = self.extract_enum(node, source, file_path, parent_id) {
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
            "method_definition" => {
                if let Some(sym) = self.extract_method(node, source, file_path, parent_id) {
                    symbols.push(sym);
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                self.extract_exported_const(node, source, file_path, parent_id, symbols);
            }
            "export_statement" => {
                self.visit_children(
                    node,
                    source,
                    file_path,
                    parent_id,
                    symbols,
                    references,
                    diagnostics,
                );
                return;
            }
            "import_statement" => {
                if let Some(text) = node_text(node, source) {
                    if let Some(parent) = parent_id {
                        references.push(Reference {
                            source: parent.clone(),
                            target: ReferenceTarget::Unresolved(text),
                            kind: ReferenceKind::Import,
                            span: node_span(node),
                        });
                    }
                }
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
        let doc = self.extract_jsdoc(node, source);
        let is_exported = self.is_exported(node);

        let visibility = if is_exported {
            Visibility::Public
        } else {
            Visibility::Private
        };

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::Function,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Function,
            qualified_name: qname,
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: self.extract_signature(node, source),
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: Vec::new(),
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
        let doc = self.extract_jsdoc(node, source);
        let is_exported = self.is_exported(node);

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
            visibility: if is_exported {
                Visibility::Public
            } else {
                Visibility::Private
            },
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: Some(format!("class {}", name)),
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: Vec::new(),
            type_params: Vec::new(),
        })
    }

    fn extract_interface(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let doc = self.extract_jsdoc(node, source);
        let is_exported = self.is_exported(node);

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::Interface,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Interface,
            qualified_name: qname,
            visibility: if is_exported {
                Visibility::Public
            } else {
                Visibility::Private
            },
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: Some(format!("interface {}", name)),
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: Vec::new(),
            type_params: Vec::new(),
        })
    }

    fn extract_type_alias(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let is_exported = self.is_exported(node);

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::TypeAlias,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::TypeAlias,
            qualified_name: qname,
            visibility: if is_exported {
                Visibility::Public
            } else {
                Visibility::Private
            },
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: self.extract_signature(node, source),
            doc: self.extract_jsdoc(node, source),
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: Vec::new(),
            type_params: Vec::new(),
        })
    }

    fn extract_enum(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let is_exported = self.is_exported(node);

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::Enum,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Enum,
            qualified_name: qname,
            visibility: if is_exported {
                Visibility::Public
            } else {
                Visibility::Private
            },
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: Some(format!("enum {}", name)),
            doc: self.extract_jsdoc(node, source),
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: Vec::new(),
            type_params: Vec::new(),
        })
    }

    fn extract_method(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let doc = self.extract_jsdoc(node, source);

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::Method,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Method,
            qualified_name: qname,
            visibility: Visibility::Public,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: self.extract_signature(node, source),
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: Vec::new(),
            type_params: Vec::new(),
        })
    }

    fn extract_exported_const(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
        symbols: &mut Vec<Symbol>,
    ) {
        if !self.is_exported(node) {
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Some(name) = node_text(name_node, source) {
                        let qname = build_qualified_name(file_path, &name);
                        let id = SymbolId::new(
                            file_path.to_string_lossy().to_string(),
                            node.start_byte(),
                            node.end_byte(),
                            SymbolKind::Constant,
                            qname.clone(),
                        );
                        symbols.push(Symbol {
                            id,
                            kind: SymbolKind::Constant,
                            qualified_name: qname,
                            visibility: Visibility::Public,
                            file: file_path.to_path_buf(),
                            span: node_span(node),
                            signature: self.extract_signature(node, source),
                            doc: self.extract_jsdoc(node, source),
                            parent: parent_id.cloned(),
                            children: Vec::new(),
                            annotations: Vec::new(),
                            type_params: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    fn is_exported(&self, node: tree_sitter::Node) -> bool {
        if let Some(parent) = node.parent() {
            parent.kind() == "export_statement"
        } else {
            false
        }
    }

    fn extract_jsdoc(&self, node: tree_sitter::Node, source: &[u8]) -> Option<Doc> {
        let check_node = if let Some(parent) = node.parent() {
            if parent.kind() == "export_statement" {
                parent
            } else {
                node
            }
        } else {
            node
        };

        let prev = check_node.prev_sibling()?;
        if prev.kind() == "comment" {
            let text = node_text(prev, source)?;
            if text.starts_with("/**") {
                let raw = text
                    .trim_start_matches("/**")
                    .trim_end_matches("*/")
                    .lines()
                    .map(|l| l.trim().trim_start_matches('*').trim())
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                let summary = raw.lines().next().map(|l| l.to_string());
                return Some(Doc {
                    raw,
                    format: DocFormat::JSDoc,
                    summary,
                });
            }
        }
        None
    }

    fn extract_signature(&self, node: tree_sitter::Node, source: &[u8]) -> Option<String> {
        let text = node_text(node, source)?;
        let sig = text.lines().next().unwrap_or(&text);
        let sig = sig.trim_end_matches('{').trim();
        if sig.len() > 200 {
            Some(format!("{}...", &sig[..200]))
        } else {
            Some(sig.to_string())
        }
    }
}

impl LanguageParser for TypeScriptParser {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn parse(&self, source: &[u8], file_path: &Path) -> WikiGenResult<ParseOutput> {
        let mut parser = tree_sitter::Parser::new();
        let lang = tree_sitter_typescript::LANGUAGE_TYPESCRIPT;
        parser
            .set_language(&lang.into())
            .map_err(|e| WikiGenError::Parse {
                file: file_path.display().to_string(),
                message: format!("Failed to set TypeScript language: {}", e),
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
            language: Language::TypeScript,
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

pub fn parse_typescript_section(
    source: &[u8],
    file_path: &Path,
    byte_offset: usize,
    line_offset: usize,
) -> WikiGenResult<(Vec<Symbol>, Vec<Reference>, Vec<Diagnostic>)> {
    let parser = TypeScriptParser::new();
    let output = parser.parse(source, file_path)?;
    let symbols: Vec<Symbol> = output
        .symbols
        .into_iter()
        .map(|mut s| {
            s.span = s.span.offset(byte_offset, line_offset);
            s
        })
        .collect();
    let references: Vec<Reference> = output
        .references
        .into_iter()
        .map(|mut r| {
            r.span = r.span.offset(byte_offset, line_offset);
            r
        })
        .collect();
    Ok((symbols, references, output.diagnostics))
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
        .replace(['/', '\\'], "/");
    format!("{}:{}", stem, name)
}
