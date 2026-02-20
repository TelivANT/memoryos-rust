use std::path::Path;

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::lang::ManifestType;
use crate::manifest::ManifestExtractor;

pub struct PythonExtractor;

impl ManifestExtractor for PythonExtractor {
    fn manifest_type(&self) -> ManifestType {
        ManifestType::PyprojectToml
    }

    fn extract(&self, content: &[u8], file_path: &Path) -> WikiGenResult<ManifestInfo> {
        let text = std::str::from_utf8(content).map_err(|e| WikiGenError::Parse {
            file: file_path.display().to_string(),
            message: format!("Invalid UTF-8: {}", e),
        })?;

        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let deps = if file_name == "pyproject.toml" {
            self.parse_pyproject(text, file_path)
        } else {
            self.parse_requirements(text, file_path)
        };

        Ok(ManifestInfo {
            ecosystem: Ecosystem::Pip,
            source_file: file_path.to_path_buf(),
            dependencies: deps,
        })
    }
}

impl PythonExtractor {
    fn parse_pyproject(&self, text: &str, file_path: &Path) -> Vec<Dependency> {
        let mut deps = Vec::new();

        let toml_val: Result<toml::Value, _> = toml::from_str(text);
        let toml_val = match toml_val {
            Ok(v) => v,
            Err(_) => return deps,
        };

        if let Some(project_deps) = toml_val
            .get("project")
            .and_then(|p| p.get("dependencies"))
            .and_then(|d| d.as_array())
        {
            for dep in project_deps {
                if let Some(dep_str) = dep.as_str() {
                    let (name, version) = parse_pip_requirement(dep_str);
                    deps.push(Dependency {
                        name,
                        version_req: version,
                        scope: DependencyScope::Runtime,
                        source_file: file_path.to_path_buf(),
                        ecosystem: Ecosystem::Pip,
                    });
                }
            }
        }

        if let Some(poetry_deps) = toml_val
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("dependencies"))
            .and_then(|d| d.as_table())
        {
            for (name, val) in poetry_deps {
                if name == "python" {
                    continue;
                }
                let version_req = match val {
                    toml::Value::String(s) => Some(s.clone()),
                    toml::Value::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    _ => None,
                };
                deps.push(Dependency {
                    name: name.clone(),
                    version_req,
                    scope: DependencyScope::Runtime,
                    source_file: file_path.to_path_buf(),
                    ecosystem: Ecosystem::Pip,
                });
            }
        }

        deps
    }

    fn parse_requirements(&self, text: &str, file_path: &Path) -> Vec<Dependency> {
        let mut deps = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
                continue;
            }
            let (name, version) = parse_pip_requirement(trimmed);
            deps.push(Dependency {
                name,
                version_req: version,
                scope: DependencyScope::Runtime,
                source_file: file_path.to_path_buf(),
                ecosystem: Ecosystem::Pip,
            });
        }
        deps
    }
}

fn parse_pip_requirement(req: &str) -> (String, Option<String>) {
    for sep in &[">=", "<=", "==", "~=", "!=", ">", "<"] {
        if let Some(idx) = req.find(sep) {
            let name = req[..idx].trim().to_string();
            let version = req[idx..].trim().to_string();
            return (name, Some(version));
        }
    }
    let name = req.split(';').next().unwrap_or(req).trim().to_string();
    (name, None)
}
