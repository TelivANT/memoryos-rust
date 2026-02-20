use std::path::Path;

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::lang::ManifestType;
use crate::manifest::ManifestExtractor;

pub struct CargoExtractor;

impl ManifestExtractor for CargoExtractor {
    fn manifest_type(&self) -> ManifestType {
        ManifestType::CargoToml
    }

    fn extract(&self, content: &[u8], file_path: &Path) -> WikiGenResult<ManifestInfo> {
        let text = std::str::from_utf8(content).map_err(|e| WikiGenError::Parse {
            file: file_path.display().to_string(),
            message: format!("Invalid UTF-8 in Cargo.toml: {}", e),
        })?;

        let toml_val: toml::Value = toml::from_str(text).map_err(|e| WikiGenError::Parse {
            file: file_path.display().to_string(),
            message: format!("Failed to parse Cargo.toml: {}", e),
        })?;

        let mut deps = Vec::new();

        if let Some(table) = toml_val.get("dependencies").and_then(|v| v.as_table()) {
            for (name, val) in table {
                let version_req = extract_version(val);
                deps.push(Dependency {
                    name: name.clone(),
                    version_req,
                    scope: DependencyScope::Runtime,
                    source_file: file_path.to_path_buf(),
                    ecosystem: Ecosystem::Cargo,
                });
            }
        }

        if let Some(table) = toml_val.get("dev-dependencies").and_then(|v| v.as_table()) {
            for (name, val) in table {
                let version_req = extract_version(val);
                deps.push(Dependency {
                    name: name.clone(),
                    version_req,
                    scope: DependencyScope::Dev,
                    source_file: file_path.to_path_buf(),
                    ecosystem: Ecosystem::Cargo,
                });
            }
        }

        if let Some(table) = toml_val
            .get("build-dependencies")
            .and_then(|v| v.as_table())
        {
            for (name, val) in table {
                let version_req = extract_version(val);
                deps.push(Dependency {
                    name: name.clone(),
                    version_req,
                    scope: DependencyScope::Build,
                    source_file: file_path.to_path_buf(),
                    ecosystem: Ecosystem::Cargo,
                });
            }
        }

        Ok(ManifestInfo {
            ecosystem: Ecosystem::Cargo,
            source_file: file_path.to_path_buf(),
            dependencies: deps,
        })
    }
}

fn extract_version(val: &toml::Value) -> Option<String> {
    match val {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Table(t) => t
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}
