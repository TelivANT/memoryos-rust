#!/bin/bash
# MemoryOS 完整部署脚本（K3s + 中间件 + Gateway）

set -e

REMOTE_HOST="${REMOTE_HOST:-104.194.91.83}"
REMOTE_PORT="${REMOTE_PORT:-26974}"
REMOTE_USER="${REMOTE_USER:-root}"
NAMESPACE="memoryos"

echo "🚀 MemoryOS 完整部署"
echo "===================="
echo ""

# 步骤 1: 部署 K3s 和中间件
echo "📦 步骤 1/3: 部署 K3s 和中间件..."
./scripts/deploy-k3s.sh

# 步骤 2: 构建并推送镜像
echo ""
echo "🐳 步骤 2/3: 构建 Docker 镜像..."
echo "正在构建 memoryos-gateway:latest..."
docker build -t memoryos-gateway:latest .

echo "推送镜像到远程服务器..."
docker save memoryos-gateway:latest | ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} \
    "docker load"

echo "导入镜像到 k3s..."
ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
# k3s 使用 containerd，需要导入镜像
docker save memoryos-gateway:latest | sudo k3s ctr images import -
EOF

# 步骤 3: 部署 Gateway
echo ""
echo "🚀 步骤 3/3: 部署 MemoryOS Gateway..."
scp -P ${REMOTE_PORT} k8s/memoryos-gateway.yaml ${REMOTE_USER}@${REMOTE_HOST}:/tmp/

ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
export KUBECONFIG=~/.kube/config
kubectl apply -f /tmp/memoryos-gateway.yaml

echo "等待 Gateway 就绪..."
kubectl wait --for=condition=ready pod -l app=memoryos-gateway -n memoryos --timeout=120s

echo "✅ Gateway 部署完成"
EOF

# 显示最终状态
echo ""
echo "📊 部署状态"
echo "===================="
ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
export KUBECONFIG=~/.kube/config
kubectl get all -n memoryos
EOF

# 获取访问地址
echo ""
echo "🌐 访问信息"
echo "===================="
NODE_PORT=$(ssh -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST} \
    "kubectl get svc memoryos-gateway -n memoryos -o jsonpath='{.spec.ports[0].nodePort}'")

cat << EOFINFO

Gateway 访问地址:
  http://${REMOTE_HOST}:${NODE_PORT}

健康检查:
  curl http://${REMOTE_HOST}:${NODE_PORT}/health/status

聊天 API:
  curl -X POST http://${REMOTE_HOST}:${NODE_PORT}/v1/chat/completions \\
    -H "Authorization: Bearer memoryos-secret-key-12345" \\
    -H "Content-Type: application/json" \\
    -d '{"model": "gemini-3-pro-preview", "messages": [{"role": "user", "content": "Hello"}]}'

查看日志:
  kubectl logs -n memoryos -l app=memoryos-gateway -f

扩容:
  kubectl scale deployment memoryos-gateway -n memoryos --replicas=5

删除部署:
  kubectl delete namespace memoryos

EOFINFO

echo "✅ 部署完成！"
