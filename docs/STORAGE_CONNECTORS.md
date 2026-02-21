# Storage Connectors Design

**Version**: 1.0  
**Date**: 2026-02-21  
**Status**: Planning

---

## 🎯 Overview

Wiki-Gen Storage Connectors provide a unified interface to read code from various storage sources, enabling wiki generation from any location.

---

## 🏗️ Architecture

### Core Abstraction

```rust
#[async_trait]
pub trait StorageConnector: Send + Sync {
    /// Connect to storage source
    async fn connect(&mut self) -> Result<()>;
    
    /// List files in directory
    async fn list_files(&self, path: &str) -> Result<Vec<FileEntry>>;
    
    /// Read file content
    async fn read_file(&self, path: &str) -> Result<Vec<u8>>;
    
    /// Check if path exists
    async fn exists(&self, path: &str) -> Result<bool>;
    
    /// Get file metadata
    async fn metadata(&self, path: &str) -> Result<FileMetadata>;
    
    /// Clone entire repo to temp directory (optional)
    async fn clone_to_temp(&self) -> Result<PathBuf>;
    
    /// Connector name
    fn name(&self) -> &str;
}
```

---

## 📦 Supported Connectors

### P0 - Core (Week 1)

#### 1. LocalConnector
- **Purpose**: Local filesystem access
- **Use Case**: Development, on-premise deployments
- **Dependencies**: `std::fs`, `tokio::fs`
- **Auth**: None
- **Status**: ✅ Implemented

#### 2. GitConnector
- **Purpose**: Git repository cloning
- **Providers**: GitHub, GitLab, Gitee, Bitbucket, Azure DevOps
- **Dependencies**: `git2`
- **Auth**: Token, SSH Key
- **Status**: ✅ Implemented

#### 3. S3Connector
- **Purpose**: S3-compatible object storage
- **Compatible**: AWS S3, MinIO, Wasabi, Cloudflare R2, DigitalOcean Spaces
- **Dependencies**: `aws-sdk-s3`
- **Auth**: Access Key + Secret Key
- **Status**: ✅ Implemented

#### 4. WebDavConnector
- **Purpose**: WebDAV protocol
- **Compatible**: 坚果云, Nextcloud, ownCloud, Seafile
- **Dependencies**: `reqwest`
- **Auth**: Basic Auth
- **Status**: ✅ Implemented

---

### P1 - Cloud Providers (Week 2-3)

#### 5. OssConnector (阿里云)
- **Purpose**: Aliyun OSS
- **Dependencies**: `aws-sdk-s3` (S3-compatible)
- **Auth**: AccessKeyId + AccessKeySecret
- **Status**: ✅ Implemented

#### 6. CosConnector (腾讯云)
- **Purpose**: Tencent Cloud COS
- **Dependencies**: `aws-sdk-s3` (S3-compatible)
- **Auth**: SecretId + SecretKey
- **Status**: ✅ Implemented

#### 7. ObsConnector (华为云)
- **Purpose**: Huawei Cloud OBS
- **Dependencies**: `aws-sdk-s3` (S3-compatible)
- **Auth**: AccessKeyId + SecretAccessKey
- **Status**: ✅ Implemented

#### 10. SftpConnector
- **Purpose**: SSH File Transfer Protocol
- **Dependencies**: `ssh2`
- **Auth**: Password, SSH Key
- **Status**: ✅ Implemented

---

### P2 - Cloud Drives (Week 4-6)

#### 11. OneDriveConnector
- **Purpose**: Microsoft OneDrive
- **Dependencies**: `onedrive-api` or REST API
- **Auth**: OAuth2 Token
- **Status**: 📋 Planned

#### 12. GoogleDriveConnector
- **Purpose**: Google Drive
- **Dependencies**: `google-drive3`
- **Auth**: OAuth2 Token
- **Status**: 📋 Planned

#### 13. DropboxConnector
- **Purpose**: Dropbox
- **Dependencies**: `dropbox-sdk` or REST API
- **Auth**: OAuth2 Token
- **Status**: 📋 Planned

#### 14. GcsConnector
- **Purpose**: Google Cloud Storage
- **Dependencies**: `google-cloud-storage`
- **Auth**: Service Account JSON
- **Status**: 📋 Planned

#### 15. AzureBlobConnector
- **Purpose**: Azure Blob Storage
- **Dependencies**: `azure-storage-blobs`
- **Auth**: Connection String or SAS Token
- **Status**: 📋 Planned

#### 16. BaiduNetdiskConnector (百度网盘)
- **Purpose**: Baidu Netdisk
- **Dependencies**: REST API
- **Auth**: Access Token
- **Status**: 📋 Planned

#### 17. AliyunDriveConnector (阿里云盘)
- **Purpose**: Aliyun Drive
- **Dependencies**: REST API
- **Auth**: Access Token
- **Status**: 📋 Planned

---

## 🔧 Implementation Plan

### Phase 1: Core Infrastructure (Week 1)

**Tasks**:
1. Define `StorageConnector` trait
2. Implement `LocalConnector` (refactor existing)
3. Implement `GitConnector` with Token/SSH auth
4. Add connector factory pattern
5. Update `WikiGenerator` to use connectors
6. Add integration tests

**Deliverables**:
- `crates/memoryos-wiki-gen/src/storage/mod.rs`
- `crates/memoryos-wiki-gen/src/storage/local.rs`
- `crates/memoryos-wiki-gen/src/storage/git.rs`
- `crates/memoryos-wiki-gen/src/storage/factory.rs`
- Tests

---

### Phase 2: S3 + WebDAV (Week 1)

**Tasks**:
1. Implement `S3Connector` with AWS SDK
2. Test with MinIO, AWS S3
3. Implement `WebDavConnector`
4. Test with 坚果云, Nextcloud
5. Update Gateway API

**Deliverables**:
- `crates/memoryos-wiki-gen/src/storage/s3.rs`
- `crates/memoryos-wiki-gen/src/storage/webdav.rs`
- Gateway API updates

---

### Phase 3: Cloud Providers (Week 2-3)

**Tasks**:
1. Implement OSS/COS/OBS connectors
2. Implement SMB/NFS/SFTP connectors
3. Add comprehensive tests
4. Update documentation

**Deliverables**:
- 6 new connectors
- Integration tests
- API documentation

---

### Phase 4: Cloud Drives (Week 4-6)

**Tasks**:
1. Implement OneDrive/Google Drive/Dropbox
2. Implement GCS/Azure Blob
3. Implement Baidu/Aliyun Drive
4. OAuth2 flow support
5. Complete documentation

**Deliverables**:
- 7 new connectors
- OAuth2 helper
- Complete user guide

---

## 📊 Coverage Matrix

| Connector | List | Read | Metadata | Stream | Auth | Priority |
|-----------|------|------|----------|--------|------|----------|
| Local | ✅ | ✅ | ✅ | ✅ | ❌ | P0 |
| Git | ✅ | ✅ | ✅ | ❌ | ✅ | P0 |
| S3 | ✅ | ✅ | ✅ | ✅ | ✅ | P0 |
| WebDAV | ✅ | ✅ | ✅ | ✅ | ✅ | P0 |
| OSS | ✅ | ✅ | ✅ | ✅ | ✅ | P1 |
| COS | ✅ | ✅ | ✅ | ✅ | ✅ | P1 |
| OBS | ✅ | ✅ | ✅ | ✅ | ✅ | P1 |
| SMB | ✅ | ✅ | ✅ | ✅ | ✅ | P1 |
| NFS | ✅ | ✅ | ✅ | ✅ | ⚠️ | P1 |
| SFTP | ✅ | ✅ | ✅ | ✅ | ✅ | P1 |
| OneDrive | ✅ | ✅ | ✅ | ✅ | ✅ | P2 |
| Google Drive | ✅ | ✅ | ✅ | ✅ | ✅ | P2 |
| Dropbox | ✅ | ✅ | ✅ | ✅ | ✅ | P2 |
| GCS | ✅ | ✅ | ✅ | ✅ | ✅ | P2 |
| Azure Blob | ✅ | ✅ | ✅ | ✅ | ✅ | P2 |
| 百度网盘 | ✅ | ✅ | ✅ | ⚠️ | ✅ | P2 |
| 阿里云盘 | ✅ | ✅ | ✅ | ⚠️ | ✅ | P2 |

---

## 🚀 Usage Examples

### Local Filesystem
```json
{
  "source": {
    "type": "local",
    "path": "/path/to/repo"
  }
}
```

### GitHub
```json
{
  "source": {
    "type": "git",
    "url": "https://github.com/user/repo.git",
    "branch": "main",
    "token": "ghp_xxx"
  }
}
```

### AWS S3
```json
{
  "source": {
    "type": "s3",
    "region": "us-east-1",
    "bucket": "my-code",
    "prefix": "projects/myapp",
    "access_key_id": "AKIA...",
    "secret_access_key": "xxx"
  }
}
```

### 阿里云 OSS
```json
{
  "source": {
    "type": "oss",
    "endpoint": "oss-cn-hangzhou.aliyuncs.com",
    "bucket": "my-code",
    "prefix": "projects/myapp",
    "access_key_id": "LTAI...",
    "access_key_secret": "xxx"
  }
}
```

### Samba/CIFS
```json
{
  "source": {
    "type": "smb",
    "server": "192.168.1.100",
    "share": "code",
    "path": "/projects/myapp",
    "username": "user",
    "password": "pass"
  }
}
```

---

## 📦 Dependencies

```toml
[dependencies]
# Core
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4"] }

# P0
git2 = "0.18"
aws-sdk-s3 = "1.0"
reqwest = { version = "0.11", features = ["json", "stream"] }

# P1
aliyun-oss-client = { version = "0.9", optional = true }
pavao = { version = "0.1", optional = true }  # SMB
ssh2 = { version = "0.9", optional = true }

# P2
google-cloud-storage = { version = "0.15", optional = true }
azure-storage-blobs = { version = "0.16", optional = true }

[features]
default = ["local", "git", "s3", "webdav"]
full = [
    "local", "git", "s3", "webdav",
    "oss", "cos", "obs",
    "smb", "nfs", "sftp",
    "onedrive", "google-drive", "dropbox",
    "gcs", "azure",
]
```

---

## 🎯 Success Criteria

- ✅ All P0 connectors working (Week 1)
- ✅ All P1 connectors working (Week 3)
- ✅ All P2 connectors working (Week 6)
- ✅ 90%+ test coverage
- ✅ Complete API documentation
- ✅ User guide with examples

---

## 📝 Notes

- Container storage (Docker Volume, K8s PV) excluded - not typical for code repos
- Special protocols (IPFS, Torrent) excluded - not enterprise use cases
- Focus on practical enterprise scenarios
- All connectors use async/await for performance
- Unified error handling across all connectors
