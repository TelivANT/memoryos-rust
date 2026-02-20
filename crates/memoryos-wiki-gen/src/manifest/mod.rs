pub mod cargo;
pub mod maven;
pub mod node;
pub mod python;

use std::path::Path;

use crate::error::WikiGenResult;
use crate::ir::ManifestInfo;
use crate::lang::ManifestType;

pub trait ManifestExtractor: Send + Sync {
    fn manifest_type(&self) -> ManifestType;
    fn extract(&self, content: &[u8], file_path: &Path) -> WikiGenResult<ManifestInfo>;
}

pub fn create_extractor(manifest_type: ManifestType) -> Box<dyn ManifestExtractor> {
    match manifest_type {
        ManifestType::CargoToml => Box::new(cargo::CargoExtractor),
        ManifestType::PomXml => Box::new(maven::MavenExtractor),
        ManifestType::BuildGradle => Box::new(maven::MavenExtractor),
        ManifestType::PackageJson => Box::new(node::NodeExtractor),
        ManifestType::PyprojectToml | ManifestType::RequirementsTxt => {
            Box::new(python::PythonExtractor)
        }
    }
}
