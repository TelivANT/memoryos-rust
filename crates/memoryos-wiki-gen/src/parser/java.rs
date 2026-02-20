use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::parser::{LanguageParser, ParseOutput};

pub struct JavaParser {
    _private: (),
}

impl Default for JavaParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaParser {
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
            "method_declaration" => {
                if let Some(sym) = self.extract_method(node, source, file_path, parent_id) {
                    symbols.push(sym);
                }
            }
            "constructor_declaration" => {
                if let Some(sym) = self.extract_constructor(node, source, file_path, parent_id) {
                    symbols.push(sym);
                }
            }
            "field_declaration" => {
                if let Some(sym) = self.extract_field(node, source, file_path, parent_id) {
                    symbols.push(sym);
                }
            }
            "import_declaration" => {
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

    fn extract_class(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let visibility = self.detect_visibility(node, source);
        let doc = self.extract_javadoc(node, source);
        let annotations = self.extract_annotations(node, source);
        let type_params = self.extract_type_params(node, source);

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
            annotations,
            type_params,
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
        let visibility = self.detect_visibility(node, source);
        let doc = self.extract_javadoc(node, source);
        let annotations = self.extract_annotations(node, source);

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
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: Some(format!("interface {}", name)),
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations,
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
        let visibility = self.detect_visibility(node, source);
        let doc = self.extract_javadoc(node, source);

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
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: Some(format!("enum {}", name)),
            doc,
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
        let visibility = self.detect_visibility(node, source);
        let doc = self.extract_javadoc(node, source);
        let annotations = self.extract_annotations(node, source);

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::Method,
            qname.clone(),
        );

        let signature = self.extract_signature(node, source);

        Some(Symbol {
            id,
            kind: SymbolKind::Method,
            qualified_name: qname,
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature,
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations,
            type_params: Vec::new(),
        })
    }

    fn extract_constructor(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let visibility = self.detect_visibility(node, source);

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::Constructor,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Constructor,
            qualified_name: qname,
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: self.extract_signature(node, source),
            doc: self.extract_javadoc(node, source),
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: Vec::new(),
            type_params: Vec::new(),
        })
    }

    fn extract_field(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                let name_node = child.child_by_field_name("name")?;
                let name = node_text(name_node, source)?;
                let visibility = self.detect_visibility(node, source);

                let qname = build_qualified_name(file_path, &name);
                let id = SymbolId::new(
                    file_path.to_string_lossy().to_string(),
                    node.start_byte(),
                    node.end_byte(),
                    SymbolKind::Field,
                    qname.clone(),
                );

                return Some(Symbol {
                    id,
                    kind: SymbolKind::Field,
                    qualified_name: qname,
                    visibility,
                    file: file_path.to_path_buf(),
                    span: node_span(node),
                    signature: self.extract_signature(node, source),
                    doc: None,
                    parent: parent_id.cloned(),
                    children: Vec::new(),
                    annotations: Vec::new(),
                    type_params: Vec::new(),
                });
            }
        }
        None
    }

    fn detect_visibility(&self, node: tree_sitter::Node, source: &[u8]) -> Visibility {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                let text = node_text(child, source).unwrap_or_default();
                if text.contains("public") {
                    return Visibility::Public;
                } else if text.contains("protected") {
                    return Visibility::Protected;
                } else if text.contains("private") {
                    return Visibility::Private;
                }
            }
        }
        Visibility::Internal
    }

    fn extract_javadoc(&self, node: tree_sitter::Node, source: &[u8]) -> Option<Doc> {
        let prev = node.prev_sibling()?;
        if prev.kind() == "block_comment" {
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
                    format: DocFormat::JavaDoc,
                    summary,
                });
            }
        }
        None
    }

    fn extract_annotations(&self, node: tree_sitter::Node, source: &[u8]) -> Vec<Annotation> {
        let mut annotations = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                let mut mod_cursor = child.walk();
                for mod_child in child.children(&mut mod_cursor) {
                    if mod_child.kind() == "marker_annotation" || mod_child.kind() == "annotation" {
                        if let Some(text) = node_text(mod_child, source) {
                            let name = text.trim_start_matches('@').to_string();
                            annotations.push(Annotation {
                                name,
                                arguments: Vec::new(),
                                span: node_span(mod_child),
                            });
                        }
                    }
                }
            }
        }
        annotations
    }

    fn extract_type_params(&self, node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
        let mut params = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_parameters" {
                let mut param_cursor = child.walk();
                for param in child.children(&mut param_cursor) {
                    if param.kind() == "type_parameter" {
                        if let Some(text) = node_text(param, source) {
                            params.push(text);
                        }
                    }
                }
            }
        }
        params
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

impl LanguageParser for JavaParser {
    fn language(&self) -> Language {
        Language::Java
    }

    fn parse(&self, source: &[u8], file_path: &Path) -> WikiGenResult<ParseOutput> {
        let mut parser = tree_sitter::Parser::new();
        let lang = tree_sitter_java::LANGUAGE;
        parser
            .set_language(&lang.into())
            .map_err(|e| WikiGenError::Parse {
                file: file_path.display().to_string(),
                message: format!("Failed to set Java language: {}", e),
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
            language: Language::Java,
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
