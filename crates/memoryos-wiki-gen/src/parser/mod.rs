pub mod java;
pub mod python;
pub mod rust;
pub mod typescript;
pub mod vue;

use std::path::Path;

use crate::error::WikiGenResult;
use crate::ir::{Diagnostic, FileIR, Language, Reference, Symbol};

pub struct ParseOutput {
    pub file: FileIR,
    pub symbols: Vec<Symbol>,
    pub references: Vec<Reference>,
    pub diagnostics: Vec<Diagnostic>,
}

pub trait LanguageParser: Send + Sync {
    fn language(&self) -> Language;
    fn parse(&self, source: &[u8], file_path: &Path) -> WikiGenResult<ParseOutput>;
}

pub fn create_parser(language: Language) -> Box<dyn LanguageParser> {
    match language {
        Language::Rust => Box::new(rust::RustParser::new()),
        Language::Python => Box::new(python::PythonParser::new()),
        Language::Java => Box::new(java::JavaParser::new()),
        Language::TypeScript | Language::JavaScript => {
            Box::new(typescript::TypeScriptParser::new())
        }
        Language::Vue => Box::new(vue::VueSfcParser::new()),
        Language::Html => Box::new(typescript::TypeScriptParser::new()),
    }
}
