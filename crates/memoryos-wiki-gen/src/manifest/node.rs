use std::path::Path;

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::lang::ManifestType;
use crate::manifest::ManifestExtractor;

pub struct NodeExtractor;

impl ManifestExtractor for NodeExtractor {
    fn manifest_type(&self) -> ManifestType {
        ManifestType::PackageJson
    }

    fn extract(&self, content: &[u8], file_path: &Path) -> WikiGenResult<ManifestInfo> {
        let text = std::str::from_utf8(content).map_err(|e| WikiGenError::Parse {
            file: file_path.display().to_string(),
            message: format!("Invalid UTF-8 in package.json: {}", e),
        })?;

        let json: serde_json::Value =
            serde_json::from_str(text).map_err(|e| WikiGenError::Parse {
                file: file_path.display().to_string(),
                message: format!("Failed to parse package.json: {}", e),
            })?;

        let mut deps = Vec::new();

        if let Some(obj) = json.get("dependencies").and_then(|v| v.as_object()) {
            for (name, val) in obj {
                deps.push(Dependency {
                    name: name.clone(),
                    version_req: val.as_str().map(|s| s.to_string()),
                    scope: DependencyScope::Runtime,
                    source_file: file_path.to_path_buf(),
                    ecosystem: Ecosystem::Npm,
                });
            }
        }

        if let Some(obj) = json.get("devDependencies").and_then(|v| v.as_object()) {
            for (name, val) in obj {
                deps.push(Dependency {
                    name: name.clone(),
                    version_req: val.as_str().map(|s| s.to_string()),
                    scope: DependencyScope::Dev,
                    source_file: file_path.to_path_buf(),
                    ecosystem: Ecosystem::Npm,
                });
            }
        }

        Ok(ManifestInfo {
            ecosystem: Ecosystem::Npm,
            source_file: file_path.to_path_buf(),
            dependencies: deps,
        })
    }
}
