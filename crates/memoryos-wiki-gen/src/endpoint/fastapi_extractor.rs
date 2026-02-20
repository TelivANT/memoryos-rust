use std::path::Path;

use crate::endpoint::EndpointExtractor;
use crate::error::WikiGenResult;
use crate::ir::*;

pub struct FastApiExtractor;

impl EndpointExtractor for FastApiExtractor {
    fn framework_name(&self) -> &str {
        "fastapi"
    }

    fn extract(
        &self,
        symbols: &[Symbol],
        source: &[u8],
        file_path: &Path,
    ) -> WikiGenResult<Vec<Endpoint>> {
        let source_str = std::str::from_utf8(source).unwrap_or("");
        let mut endpoints = Vec::new();

        let decorator_methods = [
            ("@app.get(", HttpMethod::Get),
            ("@app.post(", HttpMethod::Post),
            ("@app.put(", HttpMethod::Put),
            ("@app.delete(", HttpMethod::Delete),
            ("@app.patch(", HttpMethod::Patch),
            ("@router.get(", HttpMethod::Get),
            ("@router.post(", HttpMethod::Post),
            ("@router.put(", HttpMethod::Put),
            ("@router.delete(", HttpMethod::Delete),
            ("@router.patch(", HttpMethod::Patch),
        ];

        let lines: Vec<&str> = source_str.lines().collect();
        for (line_no, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            for (pattern, method) in &decorator_methods {
                if trimmed.starts_with(pattern) {
                    let path = self.extract_path(trimmed, pattern);
                    let handler_name = self.find_next_function(&lines, line_no);

                    let handler = handler_name.as_ref().and_then(|name| {
                        symbols.iter().find(|s| {
                            s.qualified_name.ends_with(name)
                                && matches!(s.kind, SymbolKind::Function | SymbolKind::Method)
                        })
                    });

                    let auth = self.detect_auth_signals(trimmed, source_str);

                    endpoints.push(Endpoint {
                        method: *method,
                        path: normalize_path(&path),
                        handler: handler.map(|h| h.id.clone()),
                        source: EndpointSource::CodeExtraction("fastapi".to_string()),
                        file: file_path.to_path_buf(),
                        span: Span::new(0, 0, line_no + 1, line_no + 1),
                        request_type: None,
                        response_type: None,
                        auth,
                        tags: Vec::new(),
                        doc: handler.and_then(|h| h.doc.clone()),
                    });
                }
            }
        }

        Ok(endpoints)
    }
}

impl FastApiExtractor {
    fn extract_path(&self, line: &str, pattern: &str) -> String {
        let after = &line[pattern.len()..];
        if after.starts_with('"') || after.starts_with('\'') {
            let quote = &after[..1];
            let rest = &after[1..];
            if let Some(end) = rest.find(quote) {
                return rest[..end].to_string();
            }
        }
        "/".to_string()
    }

    fn find_next_function(&self, lines: &[&str], start: usize) -> Option<String> {
        for i in (start + 1)..lines.len().min(start + 5) {
            let trimmed = lines[i].trim();
            if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                let name_part = trimmed
                    .trim_start_matches("async ")
                    .trim_start_matches("def ");
                if let Some(paren) = name_part.find('(') {
                    return Some(name_part[..paren].to_string());
                }
            }
        }
        None
    }

    fn detect_auth_signals(&self, decorator_line: &str, source: &str) -> AuthInfo {
        let mut signals = Vec::new();

        if decorator_line.contains("Depends") {
            signals.push("Depends".to_string());
        }

        let auth_patterns = [
            "get_current_user",
            "oauth2_scheme",
            "HTTPBearer",
            "Security(",
            "api_key",
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

fn normalize_path(path: &str) -> String {
    path.replace("{", "{").replace("}", "}")
}
