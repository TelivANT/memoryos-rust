# K8s ConfigMap 热更新支持

## 问题背景

在 K8s 环境中，ConfigMap 更新后：
1. Kubelet 会创建新的 `..data_tmp` 目录
2. 原子性地将 `..data` 符号链接指向新目录
3. 但挂载的文件 `config.toml` 的 **inode 和 mtime 可能不变**
4. 导致基于 mtime 的热更新检测失效

## 解决方案

### 双模式检测

```rust
// 1. mtime 检测（快速路径）- 适用于直接文件编辑
if mtime_changed {
    reload();
}

// 2. content hash 检测（慢速路径）- 适用于 K8s ConfigMap
if env::var("MEMORYOS_CONFIG_HASH_CHECK") == "true" {
    if content_hash_changed {
        reload();
    }
}
```

---

## 使用方式

### 本地开发（默认）
```bash
# 默认使用 mtime 检测，性能最优
cargo run --package memoryos-gateway

# 修改 config.toml 后 5 秒内自动生效
vim config.toml
```

### K8s 部署
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: memoryos-gateway
spec:
  template:
    spec:
      containers:
      - name: gateway
        image: memoryos-gateway:latest
        env:
        # 启用 content hash 检测（K8s ConfigMap 模式）
        - name: MEMORYOS_CONFIG_HASH_CHECK
          value: "true"
        volumeMounts:
        - name: config
          mountPath: /app/config.toml
          subPath: config.toml
      volumes:
      - name: config
        configMap:
          name: memoryos-config
```

### 更新 ConfigMap
```bash
# 1. 修改 ConfigMap
kubectl edit configmap memoryos-config

# 2. 等待 5-10 秒（kubelet 同步 + 热更新检测）
# 3. 查看日志确认
kubectl logs -f deployment/memoryos-gateway | grep "config hot reload"
```

---

## 性能对比

| 模式 | 检测方式 | 性能 | 适用场景 |
|------|----------|------|----------|
| **mtime** | 文件修改时间 | 极快（系统调用） | 本地开发、直接文件编辑 |
| **content-hash** | 文件内容 SHA | 较慢（读取+哈希） | K8s ConfigMap、符号链接 |

---

## 日志示例

### mtime 模式（本地）
```json
{
  "level": "INFO",
  "config_path": "config.toml",
  "method": "mtime",
  "message": "config hot reload detected and applied"
}
```

### content-hash 模式（K8s）
```json
{
  "level": "INFO",
  "config_path": "/app/config.toml",
  "method": "content-hash",
  "message": "config hot reload detected and applied (K8s ConfigMap mode)"
}
```

---

## 故障排查

### ConfigMap 更新后未生效

1. **检查环境变量**
   ```bash
   kubectl exec deployment/memoryos-gateway -- env | grep HASH_CHECK
   # 应该输出: MEMORYOS_CONFIG_HASH_CHECK=true
   ```

2. **检查 kubelet 同步延迟**
   ```bash
   # ConfigMap 更新后，kubelet 最多需要 60 秒同步到 Pod
   kubectl get configmap memoryos-config -o yaml | grep resourceVersion
   ```

3. **查看热更新日志**
   ```bash
   kubectl logs -f deployment/memoryos-gateway | grep reload
   ```

4. **手动触发重载**（如果需要立即生效）
   ```bash
   # 重启 Pod（不推荐，会中断服务）
   kubectl rollout restart deployment/memoryos-gateway
   ```

---

## 最佳实践

### 生产环境推荐配置

```yaml
env:
# 1. 启用 K8s 模式
- name: MEMORYOS_CONFIG_HASH_CHECK
  value: "true"

# 2. 可选：调整检测间隔（默认 5 秒）
- name: MEMORYOS_CONFIG_RELOAD_INTERVAL
  value: "10"  # 10 秒检测一次

# 3. 配置文件路径
- name: MEMORYOS_CONFIG
  value: "/app/config.toml"
```

### 配置变更流程

```bash
# 1. 更新 ConfigMap（使用 kubectl apply）
kubectl apply -f configmap.yaml

# 2. 等待同步（kubelet 默认 60 秒）
sleep 60

# 3. 验证生效
kubectl exec deployment/memoryos-gateway -- \
  curl -s http://localhost:8080/health/status | jq .config_version
```

---

## 技术细节

### 实现原理

```rust
pub struct ConfigManager {
    config: ArcSwap<AppConfig>,
    config_path: PathBuf,
    last_modified: Option<SystemTime>,
    last_content_hash: Option<u64>,  // 新增：内容哈希
}

pub fn reload_if_changed(&mut self) -> Result<bool> {
    // 1. 快速路径：mtime 检测
    if mtime_changed() {
        return reload();
    }

    // 2. 慢速路径：content hash 检测（仅在 K8s 模式）
    if env::var("MEMORYOS_CONFIG_HASH_CHECK") == "true" {
        let content = fs::read_to_string(&self.config_path)?;
        let hash = hash(&content);
        if hash != self.last_content_hash {
            self.last_content_hash = Some(hash);
            return reload();
        }
    }

    Ok(false)
}
```

### 为什么不默认启用 content-hash？

1. **性能开销**: 每 5 秒读取整个配置文件并计算哈希
2. **不必要**: 本地开发和大多数部署场景 mtime 足够
3. **按需启用**: K8s 用户显式启用，避免影响其他用户

---

## 相关文档

- [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md) - 部署指南
- [README.md](./README.md#配置热更新) - 快速开始
- [config.example.toml](./config.example.toml) - 配置示例

---

**总结**: 通过双模式检测，既保证了本地开发的性能，又支持了 K8s ConfigMap 的热更新需求。
