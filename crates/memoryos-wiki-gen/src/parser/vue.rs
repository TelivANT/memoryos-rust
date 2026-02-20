use std::path::Path;

use sha2::{Digest, Sha256};
use tracing::debug;

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::parser::typescript::parse_typescript_section;
use crate::parser::{LanguageParser, ParseOutput};

pub struct VueSfcParser {
    _private: (),
}

impl Default for VueSfcParser {
    fn default() -> Self {
        Self::new()
    }
}

impl VueSfcParser {
    pub fn new() -> Self {
        Self { _private: () }
    }

    fn split_sections(source: &str) -> Vec<SfcSection> {
        let mut sections = Vec::new();
        let mut current_tag: Option<String> = None;
        let mut current_start: usize = 0;
        let mut content_start: usize = 0;

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            if let Some(tag) = Self::parse_opening_tag(trimmed) {
                current_tag = Some(tag);
                let byte_pos = source.lines().take(i).map(|l| l.len() + 1).sum::<usize>();
                current_start = i;
                content_start = byte_pos + line.len() + 1;
            } else if let Some(ref tag) = current_tag {
                let closing = format!("</{}>", tag);
                if trimmed.starts_with(&closing) {
                    let byte_pos = source.lines().take(i).map(|l| l.len() + 1).sum::<usize>();
                    let content = &source[content_start..byte_pos.min(source.len())];

                    sections.push(SfcSection {
                        tag: tag.clone(),
                        content: content.to_string(),
                        byte_offset: content_start,
                        line_offset: current_start + 1,
                        lang: Self::detect_lang(trimmed, tag),
                    });
                    current_tag = None;
                }
            }
        }

        sections
    }

    fn parse_opening_tag(line: &str) -> Option<String> {
        if line.starts_with("<script") && (line.contains('>') || line.ends_with('>')) {
            Some("script".to_string())
        } else if line.starts_with("<template") && (line.contains('>') || line.ends_with('>')) {
            Some("template".to_string())
        } else if line.starts_with("<style") && (line.contains('>') || line.ends_with('>')) {
            Some("style".to_string())
        } else {
            None
        }
    }

    fn detect_lang(_opening_tag: &str, tag: &str) -> SectionLang {
        if tag == "script" {
            SectionLang::TypeScript
        } else if tag == "template" {
            SectionLang::Html
        } else {
            SectionLang::Css
        }
    }
}

impl LanguageParser for VueSfcParser {
    fn language(&self) -> Language {
        Language::Vue
    }

    fn parse(&self, source: &[u8], file_path: &Path) -> WikiGenResult<ParseOutput> {
        let source_str = std::str::from_utf8(source).map_err(|e| WikiGenError::Parse {
            file: file_path.display().to_string(),
            message: format!("Invalid UTF-8: {}", e),
        })?;

        let sections = Self::split_sections(source_str);

        let mut all_symbols = Vec::new();
        let mut all_references = Vec::new();
        let mut all_diagnostics = Vec::new();
        let mut parse_status = ParseStatus::Success;

        for section in &sections {
            match section.lang {
                SectionLang::TypeScript => {
                    match parse_typescript_section(
                        section.content.as_bytes(),
                        file_path,
                        section.byte_offset,
                        section.line_offset,
                    ) {
                        Ok((syms, refs, diags)) => {
                            all_symbols.extend(syms);
                            all_references.extend(refs);
                            all_diagnostics.extend(diags);
                        }
                        Err(e) => {
                            debug!(
                                "Vue script parse degraded for {}: {}",
                                file_path.display(),
                                e
                            );
                            parse_status = ParseStatus::PartialSuccess;
                            all_diagnostics.push(Diagnostic {
                                file: file_path.to_path_buf(),
                                severity: DiagSeverity::Warning,
                                message: format!("Vue <script> parse failed: {}", e),
                                span: None,
                                fallback_data: None,
                            });
                        }
                    }
                }
                SectionLang::Html | SectionLang::Css => {}
            }
        }

        let component_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("UnknownComponent");

        let qname = format!("{}:{}", file_path.display(), component_name);
        let component_symbol = Symbol {
            id: SymbolId::new(
                file_path.to_string_lossy().to_string(),
                0,
                source.len(),
                SymbolKind::Component,
                qname.clone(),
            ),
            kind: SymbolKind::Component,
            qualified_name: qname,
            visibility: Visibility::Public,
            file: file_path.to_path_buf(),
            span: Span::new(0, source.len(), 1, source_str.lines().count()),
            signature: Some(format!("<{}>", component_name)),
            doc: None,
            parent: None,
            children: all_symbols.iter().map(|s| s.id.clone()).collect(),
            annotations: Vec::new(),
            type_params: Vec::new(),
        };

        let mut final_symbols = vec![component_symbol];
        for sym in &mut all_symbols {
            sym.parent = Some(final_symbols[0].id.clone());
        }
        final_symbols.extend(all_symbols);

        let mut hasher = Sha256::new();
        hasher.update(source);
        let content_hash = format!("{:x}", hasher.finalize());

        let file_ir = FileIR {
            path: file_path.to_path_buf(),
            language: Language::Vue,
            content_hash,
            parse_status,
            byte_count: source.len(),
        };

        Ok(ParseOutput {
            file: file_ir,
            symbols: final_symbols,
            references: all_references,
            diagnostics: all_diagnostics,
        })
    }
}

#[derive(Debug)]
struct SfcSection {
    #[allow(dead_code)]
    tag: String,
    content: String,
    byte_offset: usize,
    line_offset: usize,
    lang: SectionLang,
}

#[derive(Debug, Clone, Copy)]
enum SectionLang {
    TypeScript,
    Html,
    Css,
}
