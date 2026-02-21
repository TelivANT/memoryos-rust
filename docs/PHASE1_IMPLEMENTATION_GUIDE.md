# Phase 1 API Implementation Guide

**Status**: Basic structure complete, actual logic pending  
**Date**: 2026-02-21

---

## ✅ Completed

1. API endpoint structure (3 endpoints)
2. Data structures (8 types)
3. UUID v7 for connector IDs
4. Routes integration

---

## ⏳ Remaining Work

### 1. Test Connection Implementation

**File**: `crates/memoryos-gateway/src/routes/wiki_connector.rs`

**Add helper function**:
```rust
fn create_connector(
    connector_type: &str,
    config: &HashMap<String, serde_json::Value>,
) -> Result<Box<dyn StorageConnector>, String> {
    match connector_type {
        "local" => {
            let path = config.get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'path'")?;
            Ok(Box::new(LocalConnector::new(PathBuf::from(path))))
        }
        "git" => {
            let url = config.get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'url'")?;
            let mut conn = GitConnector::new(url.to_string());
            if let Some(token) = config.get("token").and_then(|v| v.as_str()) {
                conn = conn.with_token(token.to_string());
            }
            Ok(Box::new(conn))
        }
        _ => Err(format!("Unsupported: {}", connector_type))
    }
}
```

**Update test_connection**:
```rust
async fn test_connection(
    State(_state): State<super::wiki::WikiState>,
    Json(req): Json<TestConnectionRequest>,
) -> impl IntoResponse {
    let mut connector = match create_connector(&req.connector_type, &req.config) {
        Ok(c) => c,
        Err(e) => return Json(TestConnectionResponse {
            success: false,
            error: Some(e),
            error_code: Some("INVALID_CONFIG".to_string()),
            ..Default::default()
        }),
    };

    match connector.connect().await {
        Ok(_) => Json(TestConnectionResponse {
            success: true,
            message: Some("Connection successful".to_string()),
            connector_id: Some(Uuid::now_v7().to_string()),
            ..Default::default()
        }),
        Err(e) => Json(TestConnectionResponse {
            success: false,
            error: Some(format!("{}", e)),
            error_code: Some("CONNECTION_FAILED".to_string()),
            ..Default::default()
        }),
    }
}
```

---

### 2. Browse Directory Implementation

**Challenge**: Need session management to store connected connectors

**Option A - Stateless** (Recommended for now):
```rust
async fn browse_directory(
    State(_state): State<super::wiki::WikiState>,
    Json(req): Json<BrowseDirectoryRequest>,
) -> impl IntoResponse {
    // For now, return error - requires session management
    Json(BrowseDirectoryResponse {
        path: req.path,
        entries: vec![],
        total: 0,
    })
}
```

**Option B - With Session** (Phase 2):
- Add `ConnectorSessions` to WikiState
- Store connector after test_connection
- Retrieve from session in browse_directory
- Implement session expiry (1 hour)

---

### 3. Add More Connector Metadata

**Update list_connectors** to include:
- WebDAV
- OSS (Alibaba)
- COS (Tencent)
- OBS (Huawei)
- SFTP

---

## 🎯 Recommended Approach

**For immediate usability**:
1. Implement test_connection with actual logic (30 min)
2. Keep browse_directory as stub for now
3. Merge PR #32
4. Implement session management in Phase 2

**Estimated time**: 30-60 minutes

---

## 📝 Notes

- StorageConnector trait requires `&mut self` for connect()
- Need to handle ownership carefully (Box vs Arc)
- Session management requires shared state
- Consider using Redis for distributed sessions (Phase 2)

---

## 🔗 Related Files

- `crates/memoryos-wiki-gen/src/storage/mod.rs` - StorageConnector trait
- `crates/memoryos-gateway/src/routes/wiki.rs` - WikiState definition
- `docs/STORAGE_CONNECTOR_API.md` - Full API design
