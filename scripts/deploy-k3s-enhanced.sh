#!/bin/bash
# MemoryOS K3s 自动部署脚本（增强版）

set -euo pipefail

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 配置
REMOTE_HOST="${REMOTE_HOST:-104.194.91.83}"
REMOTE_PORT="${REMOTE_PORT:-26974}"
REMOTE_USER="${REMOTE_USER:-root}"
NAMESPACE="memoryos"
TIMEOUT=300

# 日志函数
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 错误处理
trap 'log_error "部署失败，正在回滚..."; rollback; exit 1' ERR

rollback() {
    log_warn "执行回滚操作..."
    ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} \
        "kubectl delete namespace ${NAMESPACE} --ignore-not-found=true" || true
}

# 参数验证
validate_params() {
    log_info "验证参数..."
    
    if ! ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} "echo ok" &>/dev/null; then
        log_error "无法连接到远程服务器 ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PORT}"
        exit 1
    fi
    
    log_info "参数验证通过"
}

# 检查 K3s 是否已安装
check_k3s() {
    log_info "检查 K3s 状态..."
    
    if ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} "command -v k3s" &>/dev/null; then
        log_info "K3s 已安装"
        return 0
    else
        log_warn "K3s 未安装"
        return 1
    fi
}

# 安装 K3s
install_k3s() {
    log_info "安装 K3s..."
    
    ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
curl -sfL https://get.k3s.io | sh -s - \
    --write-kubeconfig-mode 644 \
    --disable traefik

# 等待 K3s 就绪
timeout 60 bash -c 'until kubectl get nodes | grep -q Ready; do sleep 2; done'
EOF
    
    log_info "K3s 安装完成"
}

# 部署中间件
deploy_middleware() {
    log_info "部署 Redis 和 Qdrant..."
    
    ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml

# 创建命名空间
kubectl create namespace memoryos --dry-run=client -o yaml | kubectl apply -f -

# 部署 Redis
cat <<YAML | kubectl apply -f -
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: redis-pvc
  namespace: memoryos
spec:
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: 5Gi
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: redis
  namespace: memoryos
spec:
  replicas: 1
  selector:
    matchLabels:
      app: redis
  template:
    metadata:
      labels:
        app: redis
    spec:
      containers:
      - name: redis
        image: redis:7-alpine
        ports:
        - containerPort: 6379
        volumeMounts:
        - name: data
          mountPath: /data
        resources:
          requests:
            cpu: 100m
            memory: 256Mi
          limits:
            cpu: 500m
            memory: 512Mi
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: redis-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: redis
  namespace: memoryos
spec:
  selector:
    app: redis
  ports:
  - port: 6379
    targetPort: 6379
YAML

# 部署 Qdrant
cat <<YAML | kubectl apply -f -
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: qdrant-pvc
  namespace: memoryos
spec:
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: 10Gi
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: qdrant
  namespace: memoryos
spec:
  replicas: 1
  selector:
    matchLabels:
      app: qdrant
  template:
    metadata:
      labels:
        app: qdrant
    spec:
      containers:
      - name: qdrant
        image: qdrant/qdrant:latest
        ports:
        - containerPort: 6333
        - containerPort: 6334
        volumeMounts:
        - name: data
          mountPath: /qdrant/storage
        resources:
          requests:
            cpu: 200m
            memory: 512Mi
          limits:
            cpu: 1000m
            memory: 2Gi
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: qdrant-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: qdrant
  namespace: memoryos
spec:
  selector:
    app: qdrant
  ports:
  - name: http
    port: 6333
    targetPort: 6333
  - name: grpc
    port: 6334
    targetPort: 6334
YAML

# 等待 Pod 就绪
echo "等待 Redis 就绪..."
kubectl wait --for=condition=ready pod -l app=redis -n memoryos --timeout=120s

echo "等待 Qdrant 就绪..."
kubectl wait --for=condition=ready pod -l app=qdrant -n memoryos --timeout=120s
EOF
    
    log_info "中间件部署完成"
}

# 健康检查
health_check() {
    log_info "执行健康检查..."
    
    local max_retries=10
    local retry=0
    
    while [ $retry -lt $max_retries ]; do
        if ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} \
            "kubectl get pods -n ${NAMESPACE} | grep -q Running"; then
            log_info "健康检查通过"
            return 0
        fi
        
        retry=$((retry + 1))
        log_warn "健康检查失败，重试 $retry/$max_retries..."
        sleep 5
    done
    
    log_error "健康检查失败"
    return 1
}

# 显示部署信息
show_info() {
    log_info "部署信息："
    
    ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
echo ""
echo "📊 资源状态："
kubectl get all -n memoryos
echo ""
echo "💾 存储状态："
kubectl get pvc -n memoryos
EOF
}

# 主流程
main() {
    log_info "🚀 开始部署 MemoryOS K3s 集群"
    log_info "目标服务器: ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PORT}"
    echo ""
    
    validate_params
    
    if ! check_k3s; then
        install_k3s
    fi
    
    deploy_middleware
    health_check
    show_info
    
    log_info "✅ 部署完成！"
}

main "$@"
