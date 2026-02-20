use std::path::Path;

use crate::endpoint::EndpointExtractor;
use crate::error::WikiGenResult;
use crate::ir::*;

pub struct SpringExtractor;

impl EndpointExtractor for SpringExtractor {
    fn framework_name(&self) -> &str {
        "spring"
    }

    fn extract(
        &self,
        symbols: &[Symbol],
        source: &[u8],
        file_path: &Path,
    ) -> WikiGenResult<Vec<Endpoint>> {
        let source_str = std::str::from_utf8(source).unwrap_or("");
        let mut endpoints = Vec::new();

        let class_path = self.extract_class_request_mapping(source_str);

        let mapping_annotations = [
            ("@GetMapping", HttpMethod::Get),
            ("@PostMapping", HttpMethod::Post),
            ("@PutMapping", HttpMethod::Put),
            ("@DeleteMapping", HttpMethod::Delete),
            ("@PatchMapping", HttpMethod::Patch),
        ];

        for symbol in symbols {
            if !matches!(symbol.kind, SymbolKind::Method) {
                continue;
            }

            for annotation in &symbol.annotations {
                for (pattern, method) in &mapping_annotations {
                    if annotation.name.starts_with(&pattern[1..]) {
                        let method_path = self.extract_annotation_path(&annotation.name);
                        let full_path = combine_paths(&class_path, &method_path);

                        let auth = self.detect_auth_signals(source_str, &symbol.annotations);

                        endpoints.push(Endpoint {
                            method: *method,
                            path: normalize_path(&full_path),
                            handler: Some(symbol.id.clone()),
                            source: EndpointSource::CodeExtraction("spring".to_string()),
                            file: file_path.to_path_buf(),
                            span: symbol.span.clone(),
                            request_type: None,
                            response_type: None,
                            auth,
                            tags: Vec::new(),
                            doc: symbol.doc.clone(),
                        });
                        break;
                    }
                }

                if annotation.name.starts_with("RequestMapping") {
                    let method_path = self.extract_annotation_path(&annotation.name);
                    let full_path = combine_paths(&class_path, &method_path);
                    let method = self.detect_request_method(&annotation.name);

                    endpoints.push(Endpoint {
                        method,
                        path: normalize_path(&full_path),
                        handler: Some(symbol.id.clone()),
                        source: EndpointSource::CodeExtraction("spring".to_string()),
                        file: file_path.to_path_buf(),
                        span: symbol.span.clone(),
                        request_type: None,
                        response_type: None,
                        auth: self.detect_auth_signals(source_str, &symbol.annotations),
                        tags: Vec::new(),
                        doc: symbol.doc.clone(),
                    });
                }
            }
        }

        Ok(endpoints)
    }
}

impl SpringExtractor {
    fn extract_class_request_mapping(&self, source: &str) -> String {
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("@RequestMapping") {
                return self.extract_annotation_path(trimmed);
            }
        }
        String::new()
    }

    fn extract_annotation_path(&self, annotation: &str) -> String {
        if let Some(start) = annotation.find('"') {
            let rest = &annotation[start + 1..];
            if let Some(end) = rest.find('"') {
                return rest[..end].to_string();
            }
        }

        if let Some(start) = annotation.find("value") {
            let rest = &annotation[start..];
            if let Some(eq) = rest.find('=') {
                let after = rest[eq + 1..].trim();
                if let Some(start) = after.find('"') {
                    let rest2 = &after[start + 1..];
                    if let Some(end) = rest2.find('"') {
                        return rest2[..end].to_string();
                    }
                }
            }
        }

        String::new()
    }

    fn detect_request_method(&self, annotation: &str) -> HttpMethod {
        if annotation.contains("GET") {
            HttpMethod::Get
        } else if annotation.contains("POST") {
            HttpMethod::Post
        } else if annotation.contains("PUT") {
            HttpMethod::Put
        } else if annotation.contains("DELETE") {
            HttpMethod::Delete
        } else if annotation.contains("PATCH") {
            HttpMethod::Patch
        } else {
            HttpMethod::All
        }
    }

    fn detect_auth_signals(&self, source: &str, annotations: &[Annotation]) -> AuthInfo {
        let mut signals = Vec::new();

        let auth_annotations = ["PreAuthorize", "Secured", "RolesAllowed", "WithMockUser"];

        for ann in annotations {
            for pattern in &auth_annotations {
                if ann.name.contains(pattern) {
                    signals.push(ann.name.clone());
                }
            }
        }

        let auth_patterns = [
            "SecurityConfig",
            "WebSecurityConfigurerAdapter",
            "@EnableWebSecurity",
            "httpSecurity",
        ];

        for pattern in &auth_patterns {
            if source.contains(pattern) {
                signals.push(pattern.to_string());
            }
        }

        let classification = if signals.is_empty() {
            AuthClassification::Unknown
        } else {
            AuthClassification::Required
        };

        AuthInfo {
            signals,
            classification,
        }
    }
}

fn combine_paths(class_path: &str, method_path: &str) -> String {
    let base = class_path.trim_end_matches('/');
    let method = if method_path.starts_with('/') {
        method_path.to_string()
    } else {
        format!("/{}", method_path)
    };

    if base.is_empty() {
        method
    } else {
        format!("{}{}", base, method)
    }
}

fn normalize_path(path: &str) -> String {
    path.replace("{", "{").replace("}", "}")
}
