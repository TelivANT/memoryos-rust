#!/bin/bash
# MemoryOS 远程 K3s 自动部署脚本
# 用途：在远程服务器上自动安装 k3s 并部署所有中间件

set -e

# 配置
REMOTE_HOST="${REMOTE_HOST:-104.194.91.83}"
REMOTE_PORT="${REMOTE_PORT:-26974}"
REMOTE_USER="${REMOTE_USER:-root}"
NAMESPACE="${NAMESPACE:-memoryos}"

echo "🚀 MemoryOS K3s 自动部署"
echo "========================"
echo "远程主机: ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PORT}"
echo "命名空间: ${NAMESPACE}"
echo ""

# 1. 检查 SSH 连接
echo "📡 1/6 检查 SSH 连接..."
if ! ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} "echo 'SSH OK'" > /dev/null 2>&1; then
    echo "❌ SSH 连接失败！"
    exit 1
fi
echo "✅ SSH 连接正常"

# 2. 安装 k3s
echo ""
echo "🔧 2/6 安装 k3s..."
ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
if ! command -v k3s &> /dev/null; then
    echo "正在安装 k3s..."
    curl -sfL https://get.k3s.io | sh -
    # 等待 k3s 启动
    sleep 10
    echo "✅ k3s 安装完成"
else
    echo "✅ k3s 已安装"
fi

# 配置 kubectl
mkdir -p ~/.kube
sudo cp /etc/rancher/k3s/k3s.yaml ~/.kube/config
sudo chown $(id -u):$(id -g) ~/.kube/config
export KUBECONFIG=~/.kube/config

# 验证
kubectl get nodes
EOF

# 3. 创建命名空间
echo ""
echo "📦 3/6 创建命名空间..."
ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << EOF
export KUBECONFIG=~/.kube/config
kubectl create namespace ${NAMESPACE} --dry-run=client -o yaml | kubectl apply -f -
echo "✅ 命名空间 ${NAMESPACE} 已创建"
EOF

# 4. 部署 Redis
echo ""
echo "🔴 4/6 部署 Redis..."
ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << 'EOFREDIS'
export KUBECONFIG=~/.kube/config
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: redis-pvc
  namespace: memoryos
spec:
  accessModes:
    - ReadWriteOnce
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
        - name: redis-data
          mountPath: /data
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
      volumes:
      - name: redis-data
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
  type: ClusterIP
EOF
echo "✅ Redis 部署完成"
EOFREDIS

# 5. 部署 Qdrant
echo ""
echo "🟣 5/6 部署 Qdrant..."
ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << 'EOFQDRANT'
export KUBECONFIG=~/.kube/config
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: qdrant-pvc
  namespace: memoryos
spec:
  accessModes:
    - ReadWriteOnce
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
        - name: qdrant-data
          mountPath: /qdrant/storage
        resources:
          requests:
            memory: "512Mi"
            cpu: "200m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
      volumes:
      - name: qdrant-data
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
  - name: rest
    port: 6333
    targetPort: 6333
  - name: grpc
    port: 6334
    targetPort: 6334
  type: ClusterIP
EOF
echo "✅ Qdrant 部署完成"
EOFQDRANT

# 6. 等待 Pod 就绪
echo ""
echo "⏳ 6/6 等待 Pod 就绪..."
ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << EOF
export KUBECONFIG=~/.kube/config
echo "等待 Redis..."
kubectl wait --for=condition=ready pod -l app=redis -n ${NAMESPACE} --timeout=120s
echo "等待 Qdrant..."
kubectl wait --for=condition=ready pod -l app=qdrant -n ${NAMESPACE} --timeout=120s
echo "✅ 所有 Pod 已就绪"
EOF

# 7. 显示状态
echo ""
echo "📊 部署状态"
echo "========================"
ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << EOF
export KUBECONFIG=~/.kube/config
kubectl get pods -n ${NAMESPACE}
echo ""
kubectl get svc -n ${NAMESPACE}
EOF

# 8. 生成连接信息
echo ""
echo "🔗 连接信息"
echo "========================"
cat << EOF

Redis 连接:
  内部: redis://redis.${NAMESPACE}.svc.cluster.local:6379
  外部: redis://localhost:6379 (需要 port-forward)

Qdrant 连接:
  内部: http://qdrant.${NAMESPACE}.svc.cluster.local:6334
  外部: http://localhost:6334 (需要 port-forward)

Port Forward 命令:
  kubectl port-forward -n ${NAMESPACE} svc/redis 6379:6379
  kubectl port-forward -n ${NAMESPACE} svc/qdrant 6333:6333 6334:6334

查看日志:
  kubectl logs -n ${NAMESPACE} -l app=redis -f
  kubectl logs -n ${NAMESPACE} -l app=qdrant -f

删除部署:
  kubectl delete namespace ${NAMESPACE}

EOF

echo "✅ 部署完成！"
