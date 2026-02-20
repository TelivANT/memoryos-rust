use std::path::Path;

use crate::endpoint::EndpointExtractor;
use crate::error::WikiGenResult;
use crate::ir::*;

pub struct AxumExtractor;

impl EndpointExtractor for AxumExtractor {
    fn framework_name(&self) -> &str {
        "axum"
    }

    fn extract(
        &self,
        symbols: &[Symbol],
        source: &[u8],
        file_path: &Path,
    ) -> WikiGenResult<Vec<Endpoint>> {
        let source_str = std::str::from_utf8(source).unwrap_or("");
        let mut endpoints = Vec::new();

        for line in source_str.lines().enumerate() {
            let (line_no, text) = line;
            let trimmed = text.trim();

            let method_path = self.parse_route_line(trimmed);
            if let Some((method, path, handler_name)) = method_path {
                let handler = symbols.iter().find(|s| {
                    s.qualified_name.ends_with(&handler_name)
                        && matches!(s.kind, SymbolKind::Function | SymbolKind::Method)
                });

                let auth = self.detect_auth_signals(source_str, &path);

                endpoints.push(Endpoint {
                    method,
                    path: normalize_path(&path),
                    handler: handler.map(|h| h.id.clone()),
                    source: EndpointSource::CodeExtraction("axum".to_string()),
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

        Ok(endpoints)
    }
}

impl AxumExtractor {
    fn parse_route_line(&self, line: &str) -> Option<(HttpMethod, String, String)> {
        let methods = [
            ("get(", HttpMethod::Get),
            ("post(", HttpMethod::Post),
            ("put(", HttpMethod::Put),
            ("delete(", HttpMethod::Delete),
            ("patch(", HttpMethod::Patch),
        ];

        for (prefix, method) in &methods {
            if let Some(idx) = line.find(&format!(".{}", prefix)) {
                let after = &line[idx + prefix.len() + 1..];
                if let Some(handler) = after.split(')').next() {
                    let handler = handler.trim();
                    let path = self.find_route_path(line, idx);
                    return Some((*method, path, handler.to_string()));
                }
            }

            if let Some(idx) = line.find(&format!("method_router::{}", prefix)) {
                let after = &line[idx + prefix.len() + 17..];
                if let Some(handler) = after.split(')').next() {
                    return Some((*method, "/".to_string(), handler.trim().to_string()));
                }
            }
        }

        if line.contains(".route(") {
            if let Some(start) = line.find(".route(\"") {
                let after = &line[start + 8..];
                if let Some(end) = after.find('"') {
                    let path = after[..end].to_string();
                    if let Some(handler_start) = after[end..].find(',') {
                        let handler_part = &after[end + handler_start + 1..];
                        if let Some(handler_end) = handler_part.find(')') {
                            let handler = handler_part[..handler_end].trim();
                            return Some((HttpMethod::All, path, handler.to_string()));
                        }
                    }
                }
            }
        }

        None
    }

    fn find_route_path(&self, line: &str, method_idx: usize) -> String {
        let before = &line[..method_idx];
        if let Some(quote_end) = before.rfind('"') {
            let before_quote = &before[..quote_end];
            if let Some(quote_start) = before_quote.rfind('"') {
                return before[quote_start + 1..quote_end].to_string();
            }
        }
        "/".to_string()
    }

    fn detect_auth_signals(&self, source: &str, _path: &str) -> AuthInfo {
        let mut signals = Vec::new();

        let auth_patterns = [
            "auth_middleware",
            "require_auth",
            "Authorization",
            "Bearer",
            "jwt",
            "rbac",
            "Permission",
            "authenticate",
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
    path.replace(":id", "{id}")
        .replace(":user_id", "{user_id}")
        .replace("<id>", "{id}")
}
