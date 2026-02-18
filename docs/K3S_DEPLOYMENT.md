# K3s 自动化部署指南

## 🎯 一键部署

### 方式 1: 完整部署（推荐）

```bash
# 部署 K3s + 中间件 + Gateway
cd /Users/delevan.tian/Code/MemoryOS/MemoryOS-Rust
./scripts/deploy-full.sh
```

**包含内容**:
- ✅ K3s 集群
- ✅ Redis (持久化存储)
- ✅ Qdrant (持久化存储)
- ✅ MemoryOS Gateway (2 副本)
- ✅ 自动健康检查
- ✅ 负载均衡

### 方式 2: 仅部署中间件

```bash
# 只部署 K3s + Redis + Qdrant
./scripts/deploy-k3s.sh
```

---

## 📦 部署架构

```
┌─────────────────────────────────────┐
│         K3s Cluster                 │
│                                     │
│  ┌──────────────────────────────┐  │
│  │  Namespace: memoryos         │  │
│  │                              │  │
│  │  ┌────────┐  ┌────────┐     │  │
│  │  │ Redis  │  │Qdrant  │     │  │
│  │  │  PVC   │  │  PVC   │     │  │
│  │  └────────┘  └────────┘     │  │
│  │                              │  │
│  │  ┌──────────────────────┐   │  │
│  │  │  Gateway (x2)        │   │  │
│  │  │  - Auto Scale        │   │  │
│  │  │  - Health Check      │   │  │
│  │  │  - Load Balancer     │   │  │
│  │  └──────────────────────┘   │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
```

---

## 🔧 配置说明

### 资源配置

| 组件 | CPU 请求 | CPU 限制 | 内存请求 | 内存限制 | 存储 |
|------|---------|---------|---------|---------|------|
| Redis | 100m | 500m | 256Mi | 512Mi | 5Gi |
| Qdrant | 200m | 1000m | 512Mi | 2Gi | 10Gi |
| Gateway | 200m | 1000m | 256Mi | 1Gi | - |

### 副本数

- Redis: 1 (单实例)
- Qdrant: 1 (单实例)
- Gateway: 2 (可扩展)

---

## 🌐 访问方式

### 内部访问（Pod 之间）

```toml
[storage.redis]
url = "redis://redis.memoryos.svc.cluster.local:6379"

[storage.vector]
url = "http://qdrant.memoryos.svc.cluster.local:6334"
```

### 外部访问

```bash
# Gateway (NodePort: 30080)
curl http://104.194.91.83:30080/health/status

# Port Forward (本地开发)
kubectl port-forward -n memoryos svc/redis 6379:6379
kubectl port-forward -n memoryos svc/qdrant 6333:6333 6334:6334
kubectl port-forward -n memoryos svc/memoryos-gateway 8080:8080
```

---

## 📊 运维命令

### 查看状态

```bash
# 所有资源
kubectl get all -n memoryos

# Pod 状态
kubectl get pods -n memoryos

# 服务状态
kubectl get svc -n memoryos

# 存储状态
kubectl get pvc -n memoryos
```

### 查看日志

```bash
# Gateway 日志
kubectl logs -n memoryos -l app=memoryos-gateway -f

# Redis 日志
kubectl logs -n memoryos -l app=redis -f

# Qdrant 日志
kubectl logs -n memoryos -l app=qdrant -f
```

### 扩缩容

```bash
# 扩容 Gateway 到 5 个副本
kubectl scale deployment memoryos-gateway -n memoryos --replicas=5

# 缩容到 1 个副本
kubectl scale deployment memoryos-gateway -n memoryos --replicas=1
```

### 更新配置

```bash
# 编辑 ConfigMap
kubectl edit configmap memoryos-config -n memoryos

# 重启 Gateway 使配置生效
kubectl rollout restart deployment memoryos-gateway -n memoryos
```

### 更新镜像

```bash
# 构建新镜像
docker build -t memoryos-gateway:v2 .

# 推送到远程
docker save memoryos-gateway:v2 | ssh -p 26974 root@104.194.91.83 "docker load && k3s ctr images import -"

# 更新部署
kubectl set image deployment/memoryos-gateway gateway=memoryos-gateway:v2 -n memoryos
```

---

## 🔒 安全配置

### API Key 管理

```bash
# 更新 API Key
kubectl edit configmap memoryos-config -n memoryos

# 重启生效
kubectl rollout restart deployment memoryos-gateway -n memoryos
```

### 网络策略（可选）

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: memoryos-network-policy
  namespace: memoryos
spec:
  podSelector:
    matchLabels:
      app: memoryos-gateway
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector: {}
    ports:
    - protocol: TCP
      port: 8080
```

---

## 🗑️ 清理

### 删除整个部署

```bash
kubectl delete namespace memoryos
```

### 仅删除 Gateway

```bash
kubectl delete deployment memoryos-gateway -n memoryos
kubectl delete svc memoryos-gateway memoryos-gateway-lb -n memoryos
```

### 卸载 K3s

```bash
ssh -p 26974 root@104.194.91.83 "/usr/local/bin/k3s-uninstall.sh"
```

---

## 🐛 故障排查

### Pod 无法启动

```bash
# 查看 Pod 详情
kubectl describe pod <pod-name> -n memoryos

# 查看事件
kubectl get events -n memoryos --sort-by='.lastTimestamp'
```

### 镜像拉取失败

```bash
# 检查镜像是否存在
ssh -p 26974 root@104.194.91.83 "k3s ctr images ls | grep memoryos"

# 重新导入镜像
docker save memoryos-gateway:latest | ssh -p 26974 root@104.194.91.83 "k3s ctr images import -"
```

### 服务无法访问

```bash
# 检查服务
kubectl get svc -n memoryos

# 检查端点
kubectl get endpoints -n memoryos

# 测试内部连接
kubectl run -it --rm debug --image=alpine --restart=Never -n memoryos -- sh
# 在 Pod 内: wget -O- http://memoryos-gateway:8080/health
```

---

## 📈 监控（可选）

### 部署 Prometheus + Grafana

```bash
# 添加 Helm repo
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

# 安装 Prometheus
helm install prometheus prometheus-community/kube-prometheus-stack -n monitoring --create-namespace

# 访问 Grafana
kubectl port-forward -n monitoring svc/prometheus-grafana 3000:80
# 默认用户名: admin, 密码: prom-operator
```

---

## 🎯 优势

| 特性 | Docker Compose | K3s |
|------|---------------|-----|
| 自动重启 | ✅ | ✅ |
| 负载均衡 | ❌ | ✅ |
| 自动扩缩容 | ❌ | ✅ |
| 健康检查 | ⚠️ 基础 | ✅ 完整 |
| 滚动更新 | ❌ | ✅ |
| 资源限制 | ⚠️ 基础 | ✅ 完整 |
| 持久化存储 | ✅ | ✅ |
| 生产就绪 | ⚠️ | ✅ |

---

## 📚 相关文档

- [K3s 官方文档](https://docs.k3s.io/)
- [Kubernetes 官方文档](https://kubernetes.io/docs/)
- [MemoryOS 部署指南](./DEPLOYMENT.md)
