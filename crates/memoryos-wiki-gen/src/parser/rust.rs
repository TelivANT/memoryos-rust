use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::parser::{LanguageParser, ParseOutput};

pub struct RustParser {
    _private: (),
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RustParser {
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
            "function_item" => {
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
            "struct_item" => {
                if let Some(sym) = self.extract_struct(node, source, file_path, parent_id) {
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
            "enum_item" => {
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
            "trait_item" => {
                if let Some(sym) = self.extract_trait(node, source, file_path, parent_id) {
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
            "impl_item" => {
                self.extract_impl_methods(
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
            "mod_item" => {
                if let Some(sym) = self.extract_module(node, source, file_path, parent_id) {
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
            "const_item" | "static_item" => {
                if let Some(sym) = self.extract_constant(node, source, file_path, parent_id) {
                    symbols.push(sym);
                }
            }
            "type_item" => {
                if let Some(sym) = self.extract_type_alias(node, source, file_path, parent_id) {
                    symbols.push(sym);
                }
            }
            "use_declaration" => {
                if let Some(name) = self.extract_use_path(node, source) {
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
        let visibility = self.detect_visibility(node, source);
        let doc = self.extract_doc_comment(node, source);
        let signature = self.extract_signature(node, source);
        let type_params = self.extract_type_params(node, source);

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
            signature,
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: self.extract_attributes(node, source),
            type_params,
        })
    }

    fn extract_struct(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let visibility = self.detect_visibility(node, source);
        let doc = self.extract_doc_comment(node, source);
        let type_params = self.extract_type_params(node, source);

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::Struct,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Struct,
            qualified_name: qname,
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: Some(format!("struct {}", name)),
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: self.extract_attributes(node, source),
            type_params,
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
        let doc = self.extract_doc_comment(node, source);
        let type_params = self.extract_type_params(node, source);

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
            annotations: self.extract_attributes(node, source),
            type_params,
        })
    }

    fn extract_trait(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = node_text(name_node, source)?;
        let visibility = self.detect_visibility(node, source);
        let doc = self.extract_doc_comment(node, source);
        let type_params = self.extract_type_params(node, source);

        let qname = build_qualified_name(file_path, &name);
        let id = SymbolId::new(
            file_path.to_string_lossy().to_string(),
            node.start_byte(),
            node.end_byte(),
            SymbolKind::Trait,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Trait,
            qualified_name: qname,
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: Some(format!("trait {}", name)),
            doc,
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: self.extract_attributes(node, source),
            type_params,
        })
    }

    fn extract_impl_methods(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        parent_id: Option<&SymbolId>,
        symbols: &mut Vec<Symbol>,
        references: &mut Vec<Reference>,
        _diagnostics: &mut Vec<Diagnostic>,
    ) {
        let type_name = node
            .child_by_field_name("type")
            .and_then(|n| node_text(n, source));

        let trait_name = node
            .child_by_field_name("trait")
            .and_then(|n| node_text(n, source));

        if let (Some(ref tname), Some(ref _type_n)) = (&trait_name, &type_name) {
            if let Some(p) = parent_id {
                references.push(Reference {
                    source: p.clone(),
                    target: ReferenceTarget::Unresolved(tname.clone()),
                    kind: ReferenceKind::Implements,
                    span: node_span(node),
                });
            }
        }

        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "function_item" {
                    if let Some(mut sym) =
                        self.extract_function(child, source, file_path, parent_id)
                    {
                        sym.kind = SymbolKind::Method;
                        if let Some(ref tn) = type_name {
                            sym.qualified_name = format!(
                                "{}::{}",
                                tn,
                                sym.qualified_name
                                    .split("::")
                                    .last()
                                    .unwrap_or(&sym.qualified_name)
                            );
                        }
                        symbols.push(sym);
                    }
                }
            }
        }
    }

    fn extract_module(
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
            SymbolKind::Module,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Module,
            qualified_name: qname,
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: Some(format!("mod {}", name)),
            doc: self.extract_doc_comment(node, source),
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: Vec::new(),
            type_params: Vec::new(),
        })
    }

    fn extract_constant(
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
            SymbolKind::Constant,
            qname.clone(),
        );

        Some(Symbol {
            id,
            kind: SymbolKind::Constant,
            qualified_name: qname,
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: self.extract_signature(node, source),
            doc: self.extract_doc_comment(node, source),
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
        let visibility = self.detect_visibility(node, source);

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
            visibility,
            file: file_path.to_path_buf(),
            span: node_span(node),
            signature: self.extract_signature(node, source),
            doc: self.extract_doc_comment(node, source),
            parent: parent_id.cloned(),
            children: Vec::new(),
            annotations: Vec::new(),
            type_params: Vec::new(),
        })
    }

    fn detect_visibility(&self, node: tree_sitter::Node, source: &[u8]) -> Visibility {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                let text = node_text(child, source).unwrap_or_default();
                if text.contains("pub") {
                    return Visibility::Public;
                }
            }
        }
        Visibility::Private
    }

    fn extract_doc_comment(&self, node: tree_sitter::Node, source: &[u8]) -> Option<Doc> {
        let mut doc_lines = Vec::new();
        let mut prev = node.prev_sibling();
        while let Some(sibling) = prev {
            match sibling.kind() {
                "line_comment" => {
                    let text = node_text(sibling, source)?;
                    if text.starts_with("///") {
                        doc_lines.push(text.trim_start_matches("///").trim().to_string());
                    } else if text.starts_with("//!") {
                        doc_lines.push(text.trim_start_matches("//!").trim().to_string());
                    } else {
                        break;
                    }
                }
                "block_comment" => {
                    let text = node_text(sibling, source)?;
                    if text.starts_with("/**") || text.starts_with("/*!") {
                        let cleaned = text
                            .trim_start_matches("/**")
                            .trim_start_matches("/*!")
                            .trim_end_matches("*/")
                            .trim();
                        doc_lines.push(cleaned.to_string());
                    }
                    break;
                }
                _ => break,
            }
            prev = sibling.prev_sibling();
        }

        if doc_lines.is_empty() {
            return None;
        }

        doc_lines.reverse();
        let raw = doc_lines.join("\n");
        let summary = doc_lines.first().cloned();

        Some(Doc {
            raw,
            format: DocFormat::RustDoc,
            summary,
        })
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

    fn extract_type_params(&self, node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
        let mut params = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_parameters" {
                let mut param_cursor = child.walk();
                for param in child.children(&mut param_cursor) {
                    if param.kind() == "type_identifier"
                        || param.kind() == "constrained_type_parameter"
                    {
                        if let Some(text) = node_text(param, source) {
                            params.push(text);
                        }
                    }
                }
            }
        }
        params
    }

    fn extract_attributes(&self, node: tree_sitter::Node, source: &[u8]) -> Vec<Annotation> {
        let mut attrs = Vec::new();
        let mut prev = node.prev_sibling();
        while let Some(sibling) = prev {
            if sibling.kind() == "attribute_item" || sibling.kind() == "inner_attribute_item" {
                if let Some(text) = node_text(sibling, source) {
                    let name = text
                        .trim_start_matches("#[")
                        .trim_start_matches("#![")
                        .split('(')
                        .next()
                        .unwrap_or(&text)
                        .trim_end_matches(']')
                        .to_string();
                    attrs.push(Annotation {
                        name,
                        arguments: Vec::new(),
                        span: node_span(sibling),
                    });
                }
            } else {
                break;
            }
            prev = sibling.prev_sibling();
        }
        attrs
    }

    fn extract_use_path(&self, node: tree_sitter::Node, source: &[u8]) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "use_wildcard"
                || child.kind() == "use_list"
                || child.kind() == "scoped_identifier"
                || child.kind() == "identifier"
                || child.kind() == "scoped_use_list"
            {
                return node_text(child, source);
            }
        }
        None
    }
}

impl LanguageParser for RustParser {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn parse(&self, source: &[u8], file_path: &Path) -> WikiGenResult<ParseOutput> {
        let mut parser = tree_sitter::Parser::new();
        let lang = tree_sitter_rust::LANGUAGE;
        parser
            .set_language(&lang.into())
            .map_err(|e| WikiGenError::Parse {
                file: file_path.display().to_string(),
                message: format!("Failed to set Rust language: {}", e),
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
            language: Language::Rust,
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
        .replace(['/', '\\'], "::");
    format!("{}::{}", stem, name)
}
