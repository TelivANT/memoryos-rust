use std::path::Path;

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::lang::ManifestType;
use crate::manifest::ManifestExtractor;

pub struct MavenExtractor;

impl ManifestExtractor for MavenExtractor {
    fn manifest_type(&self) -> ManifestType {
        ManifestType::PomXml
    }

    fn extract(&self, content: &[u8], file_path: &Path) -> WikiGenResult<ManifestInfo> {
        let text = std::str::from_utf8(content).map_err(|e| WikiGenError::Parse {
            file: file_path.display().to_string(),
            message: format!("Invalid UTF-8 in pom.xml: {}", e),
        })?;

        let mut deps = Vec::new();
        let mut in_dependencies = false;
        let mut in_dependency = false;
        let mut current_group = String::new();
        let mut current_artifact = String::new();
        let mut current_version = String::new();
        let mut current_scope = String::new();

        for line in text.lines() {
            let trimmed = line.trim();

            if trimmed == "<dependencies>" {
                in_dependencies = true;
            } else if trimmed == "</dependencies>" {
                in_dependencies = false;
            } else if in_dependencies && trimmed == "<dependency>" {
                in_dependency = true;
                current_group.clear();
                current_artifact.clear();
                current_version.clear();
                current_scope.clear();
            } else if in_dependency && trimmed == "</dependency>" {
                in_dependency = false;
                let name = if current_group.is_empty() {
                    current_artifact.clone()
                } else {
                    format!("{}:{}", current_group, current_artifact)
                };

                let scope = match current_scope.as_str() {
                    "test" => DependencyScope::Dev,
                    "provided" | "compile" | "runtime" | "" => DependencyScope::Runtime,
                    _ => DependencyScope::Runtime,
                };

                if !name.is_empty() {
                    deps.push(Dependency {
                        name,
                        version_req: if current_version.is_empty() {
                            None
                        } else {
                            Some(current_version.clone())
                        },
                        scope,
                        source_file: file_path.to_path_buf(),
                        ecosystem: Ecosystem::Maven,
                    });
                }
            } else if in_dependency {
                if let Some(val) = extract_xml_value(trimmed, "groupId") {
                    current_group = val;
                } else if let Some(val) = extract_xml_value(trimmed, "artifactId") {
                    current_artifact = val;
                } else if let Some(val) = extract_xml_value(trimmed, "version") {
                    current_version = val;
                } else if let Some(val) = extract_xml_value(trimmed, "scope") {
                    current_scope = val;
                }
            }
        }

        Ok(ManifestInfo {
            ecosystem: Ecosystem::Maven,
            source_file: file_path.to_path_buf(),
            dependencies: deps,
        })
    }
}

fn extract_xml_value(line: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = line.find(&open) {
        if let Some(end) = line.find(&close) {
            let val_start = start + open.len();
            if val_start < end {
                return Some(line[val_start..end].to_string());
            }
        }
    }
    None
}
