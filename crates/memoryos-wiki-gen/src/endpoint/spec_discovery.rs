use std::path::Path;

use tracing::debug;

use crate::error::{WikiGenError, WikiGenResult};
use crate::ir::*;
use crate::lang::ApiSpecType;

pub fn discover_api_spec(
    content: &[u8],
    file_path: &Path,
    spec_type: ApiSpecType,
) -> WikiGenResult<ApiSpec> {
    let text = std::str::from_utf8(content).map_err(|e| WikiGenError::Parse {
        file: file_path.display().to_string(),
        message: format!("Invalid UTF-8 in spec file: {}", e),
    })?;

    match spec_type {
        ApiSpecType::OpenApi | ApiSpecType::Swagger => parse_openapi(text, file_path, spec_type),
        ApiSpecType::Proto => parse_proto(text, file_path),
    }
}

fn parse_openapi(text: &str, file_path: &Path, spec_type: ApiSpecType) -> WikiGenResult<ApiSpec> {
    let json: serde_json::Value = if file_path.extension().and_then(|e| e.to_str()) == Some("json")
    {
        serde_json::from_str(text).map_err(|e| WikiGenError::Parse {
            file: file_path.display().to_string(),
            message: format!("Failed to parse OpenAPI JSON: {}", e),
        })?
    } else {
        return Ok(ApiSpec {
            kind: match spec_type {
                ApiSpecType::OpenApi => SpecKind::OpenApi,
                ApiSpecType::Swagger => SpecKind::Swagger,
                _ => SpecKind::OpenApi,
            },
            file: file_path.to_path_buf(),
            endpoints: Vec::new(),
        });
    };

    let mut endpoints = Vec::new();

    if let Some(paths) = json.get("paths").and_then(|p| p.as_object()) {
        for (path, methods) in paths {
            if let Some(methods_obj) = methods.as_object() {
                for (method_str, details) in methods_obj {
                    let method = match method_str.as_str() {
                        "get" => HttpMethod::Get,
                        "post" => HttpMethod::Post,
                        "put" => HttpMethod::Put,
                        "delete" => HttpMethod::Delete,
                        "patch" => HttpMethod::Patch,
                        _ => continue,
                    };

                    let summary = details
                        .get("summary")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());

                    let tags: Vec<String> = details
                        .get("tags")
                        .and_then(|t| t.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let has_security = details.get("security").is_some();

                    endpoints.push(Endpoint {
                        method,
                        path: path.clone(),
                        handler: None,
                        source: EndpointSource::Spec(match spec_type {
                            ApiSpecType::OpenApi => SpecKind::OpenApi,
                            ApiSpecType::Swagger => SpecKind::Swagger,
                            _ => SpecKind::OpenApi,
                        }),
                        file: file_path.to_path_buf(),
                        span: Span::new(0, 0, 0, 0),
                        request_type: None,
                        response_type: None,
                        auth: if has_security {
                            AuthInfo {
                                signals: vec!["security".to_string()],
                                classification: AuthClassification::Required,
                            }
                        } else {
                            AuthInfo::default()
                        },
                        tags,
                        doc: summary.map(|s| Doc {
                            raw: s.clone(),
                            format: DocFormat::Markdown,
                            summary: Some(s),
                        }),
                    });
                }
            }
        }
    }

    debug!(
        "Discovered {} endpoints from OpenAPI spec: {}",
        endpoints.len(),
        file_path.display()
    );

    Ok(ApiSpec {
        kind: match spec_type {
            ApiSpecType::OpenApi => SpecKind::OpenApi,
            ApiSpecType::Swagger => SpecKind::Swagger,
            _ => SpecKind::OpenApi,
        },
        file: file_path.to_path_buf(),
        endpoints,
    })
}

fn parse_proto(text: &str, file_path: &Path) -> WikiGenResult<ApiSpec> {
    let mut endpoints = Vec::new();

    let mut current_service = String::new();
    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("service ") {
            current_service = trimmed
                .trim_start_matches("service ")
                .split('{')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
        } else if trimmed.starts_with("rpc ") && !current_service.is_empty() {
            let rpc_name = trimmed
                .trim_start_matches("rpc ")
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            if !rpc_name.is_empty() {
                endpoints.push(Endpoint {
                    method: HttpMethod::Post,
                    path: format!("/{}/{}", current_service, rpc_name),
                    handler: None,
                    source: EndpointSource::Spec(SpecKind::Proto),
                    file: file_path.to_path_buf(),
                    span: Span::new(0, 0, 0, 0),
                    request_type: None,
                    response_type: None,
                    auth: AuthInfo::default(),
                    tags: vec![current_service.clone()],
                    doc: None,
                });
            }
        }
    }

    Ok(ApiSpec {
        kind: SpecKind::Proto,
        file: file_path.to_path_buf(),
        endpoints,
    })
}
