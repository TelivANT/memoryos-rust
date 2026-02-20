pub mod axum_extractor;
pub mod fastapi_extractor;
pub mod spec_discovery;
pub mod spring_extractor;

use std::path::Path;

use crate::error::WikiGenResult;
use crate::ir::{Endpoint, Symbol};

pub trait EndpointExtractor: Send + Sync {
    fn framework_name(&self) -> &str;
    fn extract(
        &self,
        symbols: &[Symbol],
        source: &[u8],
        file_path: &Path,
    ) -> WikiGenResult<Vec<Endpoint>>;
}

pub fn detect_framework(source: &str, file_path: &Path) -> Option<Box<dyn EndpointExtractor>> {
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "rs" => {
            if source.contains("axum::") || source.contains("use axum") {
                Some(Box::new(axum_extractor::AxumExtractor))
            } else {
                None
            }
        }
        "py" => {
            if source.contains("FastAPI") || source.contains("fastapi") {
                Some(Box::new(fastapi_extractor::FastApiExtractor))
            } else {
                None
            }
        }
        "java" => {
            if source.contains("@RequestMapping")
                || source.contains("@GetMapping")
                || source.contains("@PostMapping")
                || source.contains("@RestController")
            {
                Some(Box::new(spring_extractor::SpringExtractor))
            } else {
                None
            }
        }
        _ => None,
    }
}
