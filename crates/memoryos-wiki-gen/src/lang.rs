use std::path::PathBuf;

use crate::config::ParseConfig;
use crate::ir::Language;

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub absolute_path: PathBuf,
    pub relative_path: PathBuf,
    pub file_type: FileType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Source(Language),
    Manifest(ManifestType),
    ApiSpec(ApiSpecType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestType {
    CargoToml,
    PomXml,
    BuildGradle,
    PackageJson,
    PyprojectToml,
    RequirementsTxt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiSpecType {
    OpenApi,
    Swagger,
    Proto,
}

pub fn detect_file(
    relative_path: &std::path::Path,
    config: &ParseConfig,
) -> Option<DiscoveredFile> {
    let ext = relative_path.extension().and_then(|e| e.to_str());
    let file_name = relative_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let file_type = match file_name {
        "Cargo.toml" => Some(FileType::Manifest(ManifestType::CargoToml)),
        "pom.xml" => Some(FileType::Manifest(ManifestType::PomXml)),
        "build.gradle" | "build.gradle.kts" => Some(FileType::Manifest(ManifestType::BuildGradle)),
        "package.json" => Some(FileType::Manifest(ManifestType::PackageJson)),
        "pyproject.toml" => Some(FileType::Manifest(ManifestType::PyprojectToml)),
        "requirements.txt" => Some(FileType::Manifest(ManifestType::RequirementsTxt)),
        "openapi.yaml" | "openapi.yml" | "openapi.json" => {
            Some(FileType::ApiSpec(ApiSpecType::OpenApi))
        }
        "swagger.json" | "swagger.yaml" | "swagger.yml" => {
            Some(FileType::ApiSpec(ApiSpecType::Swagger))
        }
        _ => match ext {
            Some("proto") => Some(FileType::ApiSpec(ApiSpecType::Proto)),
            Some("rs") if config.language_enabled(Language::Rust) => {
                Some(FileType::Source(Language::Rust))
            }
            Some("py") if config.language_enabled(Language::Python) => {
                Some(FileType::Source(Language::Python))
            }
            Some("java") if config.language_enabled(Language::Java) => {
                Some(FileType::Source(Language::Java))
            }
            Some("vue") if config.language_enabled(Language::Vue) => {
                Some(FileType::Source(Language::Vue))
            }
            Some("ts" | "tsx") if config.language_enabled(Language::TypeScript) => {
                Some(FileType::Source(Language::TypeScript))
            }
            Some("js" | "jsx") if config.language_enabled(Language::JavaScript) => {
                Some(FileType::Source(Language::JavaScript))
            }
            Some("html" | "htm") if config.language_enabled(Language::Html) => {
                Some(FileType::Source(Language::Html))
            }
            _ => None,
        },
    };

    file_type.map(|ft| DiscoveredFile {
        absolute_path: PathBuf::new(),
        relative_path: relative_path.to_path_buf(),
        file_type: ft,
    })
}
