# Phase 6 部署文档

**版本**: v2.0  
**创建时间**: 2026-02-17  
**适用版本**: v0.6.0+  
**状态**: 📝 准备中

---

## 📋 目录

- [环境要求](#环境要求)
- [快速开始](#快速开始)
- [开发环境部署](#开发环境部署)
- [生产环境部署](#生产环境部署)
- [配置说明](#配置说明)
- [监控和运维](#监控和运维)

---

## 环境要求

### 硬件要求

| 环境 | CPU | 内存 | 磁盘 | 网络 |
|------|-----|------|------|------|
| 开发 | 2核 | 4GB | 20GB | 10Mbps |
| 测试 | 4核 | 8GB | 50GB | 100Mbps |
| 生产 | 8核+ | 16GB+ | 200GB+ | 1Gbps+ |

### 软件要求

#### 必需组件
- **Rust**: 1.75+ (推荐 1.93+)
- **Redis**: 7.0+ (支持 Streams)
- **Qdrant**: 1.7+
- **Postgres**: 15+ (新增)
- **ONNX Runtime**: 1.16+ (新增)

#### 可选组件
- **Docker**: 24.0+
- **Kubernetes**: 1.28+
- **Nginx**: 1.24+ (负载均衡)
- **Prometheus**: 2.45+ (监控)
- **Grafana**: 10.0+ (可视化)

---

## 快速开始

### 1. 克隆代码

```bash
git clone https://github.com/BAI-LAB/MemoryOS.git
cd MemoryOS/MemoryOS-Rust
```

### 2. 安装依赖

#### macOS
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# ONNX Runtime
brew install onnxruntime

# Redis
brew install redis
brew services start redis

# Postgres
brew install postgresql@15
brew services start postgresql@15

# Qdrant (Docker)
docker run -d -p 6333:6333 -p 6334:6334 \
  -v $(pwd)/qdrant_storage:/qdrant/storage \
  qdrant/qdrant
```

#### Linux (Ubuntu/Debian)
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# ONNX Runtime
wget https://github.com/microsoft/onnxruntime/releases/download/v1.16.0/onnxruntime-linux-x64-1.16.0.tgz
tar -xzf onnxruntime-linux-x64-1.16.0.tgz
sudo cp onnxruntime-linux-x64-1.16.0/lib/* /usr/local/lib/
sudo ldconfig

# Redis
sudo apt-get update
sudo apt-get install redis-server
sudo systemctl start redis

# Postgres
sudo apt-get install postgresql-15
sudo systemctl start postgresql

# Qdrant (Docker)
docker run -d -p 6333:6333 -p 6334:6334 \
  -v $(pwd)/qdrant_storage:/qdrant/storage \
  qdrant/qdrant
```

### 3. 初始化数据库

```bash
# 创建 Postgres 数据库
createdb memoryos

# 运行迁移
sqlx database create
sqlx migrate run

# 或使用脚本
./scripts/setup_postgres.sh
```

### 4. 下载 Embedding 模型

```bash
# 下载 BGE-M3 模型
./scripts/download_models.sh

# 或手动下载
mkdir -p models/bge-m3
cd models/bge-m3
wget https://huggingface.co/BAAI/bge-m3/resolve/main/onnx/model.onnx
wget https://huggingface.co/BAAI/bge-m3/resolve/main/tokenizer.json
```

### 5. 配置环境变量

```bash
cp .env.example .env

# 编辑 .env
vim .env
```

```.env
# OpenAI API (用于 LLM 调用和 Fallback)
OPENAI_API_KEY=sk-xxx
OPENAI_BASE_URL=https://api.openai.com/v1

# Redis
REDIS_URL=redis://localhost:6379

# Qdrant
QDRANT_URL=http://localhost:6333

# Postgres
DATABASE_URL=postgres://postgres:password@localhost:5432/memoryos

# Embedding
EMBEDDING_PROVIDER=onnx  # onnx | openai
EMBEDDING_MODEL=BAAI/bge-m3
EMBEDDING_MODEL_PATH=./models/bge-m3

# 日志
RUST_LOG=info,memoryos=debug
```

### 6. 编译和运行

```bash
# 编译
cargo build --release

# 运行 Gateway
./target/release/memoryos-gateway

# 运行 Worker (另一个终端)
./target/release/memoryos-worker
```

### 7. 测试

```bash
# 健康检查
curl http://localhost:8080/health

# 聊天测试
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk_test_xxx" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

---

## 开发环境部署

### Docker Compose (推荐)

#### 1. 创建 docker-compose.yml

```yaml
version: '3.8'

services:
  # Gateway
  gateway:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info,memoryos=debug
      - REDIS_URL=redis://redis:6379
      - QDRANT_URL=http://qdrant:6333
      - DATABASE_URL=postgres://postgres:password@postgres:5432/memoryos
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - EMBEDDING_PROVIDER=onnx
      - EMBEDDING_MODEL_PATH=/app/models/bge-m3
    volumes:
      - ./models:/app/models
      - ./config.toml:/app/config.toml
    depends_on:
      - redis
      - qdrant
      - postgres
    command: ["./memoryos-gateway"]

  # Worker
  worker:
    build:
      context: .
      dockerfile: Dockerfile
    environment:
      - RUST_LOG=info,memoryos=debug
      - REDIS_URL=redis://redis:6379
      - QDRANT_URL=http://qdrant:6333
      - DATABASE_URL=postgres://postgres:password@postgres:5432/memoryos
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    depends_on:
      - redis
      - qdrant
      - postgres
    command: ["./memoryos-worker"]

  # Redis
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    command: redis-server --appendonly yes

  # Qdrant
  qdrant:
    image: qdrant/qdrant:latest
    ports:
      - "6333:6333"
      - "6334:6334"
    volumes:
      - qdrant_data:/qdrant/storage

  # Postgres
  postgres:
    image: postgres:15-alpine
    ports:
      - "5432:5432"
    environment:
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=memoryos
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  redis_data:
  qdrant_data:
  postgres_data:
```

#### 2. 启动服务

```bash
# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f gateway

# 停止服务
docker-compose down
```

---

## 生产环境部署

### Kubernetes 部署

#### 1. 创建 Namespace

```bash
kubectl create namespace memoryos
```

#### 2. 创建 ConfigMap

```yaml
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: memoryos-config
  namespace: memoryos
data:
  config.toml: |
    [server]
    host = "0.0.0.0"
    port = 8080
    
    [redis]
    url = "redis://redis-service:6379"
    pool_size = 100
    
    [qdrant]
    url = "http://qdrant-service:6333"
    pool_size = 50
    
    [postgres]
    url = "postgres://postgres:password@postgres-service:5432/memoryos"
    max_connections = 50
    
    [embedding]
    provider = "onnx"
    model = "BAAI/bge-m3"
    model_path = "/app/models/bge-m3"
```

```bash
kubectl apply -f k8s/configmap.yaml
```

#### 3. 创建 Secret

```bash
kubectl create secret generic memoryos-secrets \
  --from-literal=openai-api-key=sk-xxx \
  --namespace=memoryos
```

#### 4. 部署 Gateway

```yaml
# k8s/gateway-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: memoryos-gateway
  namespace: memoryos
spec:
  replicas: 3
  selector:
    matchLabels:
      app: memoryos-gateway
  template:
    metadata:
      labels:
        app: memoryos-gateway
    spec:
      containers:
      - name: gateway
        image: memoryos/gateway:v0.6.0
        ports:
        - containerPort: 8080
        env:
        - name: RUST_LOG
          value: "info,memoryos=debug"
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: memoryos-secrets
              key: openai-api-key
        volumeMounts:
        - name: config
          mountPath: /app/config.toml
          subPath: config.toml
        - name: models
          mountPath: /app/models
        resources:
          requests:
            memory: "2Gi"
            cpu: "1000m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: memoryos-config
      - name: models
        persistentVolumeClaim:
          claimName: models-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: memoryos-gateway
  namespace: memoryos
spec:
  selector:
    app: memoryos-gateway
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

```bash
kubectl apply -f k8s/gateway-deployment.yaml
```

#### 5. 部署 Worker

```yaml
# k8s/worker-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: memoryos-worker
  namespace: memoryos
spec:
  replicas: 2
  selector:
    matchLabels:
      app: memoryos-worker
  template:
    metadata:
      labels:
        app: memoryos-worker
    spec:
      containers:
      - name: worker
        image: memoryos/worker:v0.6.0
        env:
        - name: RUST_LOG
          value: "info,memoryos=debug"
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: memoryos-secrets
              key: openai-api-key
        volumeMounts:
        - name: config
          mountPath: /app/config.toml
          subPath: config.toml
        resources:
          requests:
            memory: "1Gi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
      volumes:
      - name: config
        configMap:
          name: memoryos-config
```

```bash
kubectl apply -f k8s/worker-deployment.yaml
```

#### 6. 部署中间件

```bash
# Redis
kubectl apply -f k8s/redis-statefulset.yaml

# Qdrant
kubectl apply -f k8s/qdrant-statefulset.yaml

# Postgres
kubectl apply -f k8s/postgres-statefulset.yaml
```

#### 7. 配置 Ingress

```yaml
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: memoryos-ingress
  namespace: memoryos
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/rate-limit: "100"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - api.memoryos.com
    secretName: memoryos-tls
  rules:
  - host: api.memoryos.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: memoryos-gateway
            port:
              number: 80
```

```bash
kubectl apply -f k8s/ingress.yaml
```

---

## 配置说明

### config.toml 完整配置

```toml
# Server 配置
[server]
host = "0.0.0.0"
port = 8080
workers = 4

# Redis 配置
[redis]
url = "redis://localhost:6379"
pool_size = 100
pool_timeout_ms = 5000
connection_timeout_ms = 3000

# Qdrant 配置
[qdrant]
url = "http://localhost:6333"
pool_size = 50
timeout_ms = 3000
collection_name = "memoryos"

# Postgres 配置
[postgres]
url = "postgres://postgres:password@localhost:5432/memoryos"
max_connections = 50
min_connections = 10
acquire_timeout_ms = 5000

# Embedding 配置
[embedding]
provider = "onnx"  # onnx | openai
model = "BAAI/bge-m3"
model_path = "./models/bge-m3"
cache_size = 1000
batch_size = 32
fallback_to_openai = true

# LLM 配置
[llm]
default_model = "gpt-4o-mini"
summarize_model = "gpt-4o-mini"
extract_model = "gpt-4o-mini"
temperature = 0.3
max_tokens = 1000

# Memory 配置
[memory]
short_term_capacity = 20
mid_term_capacity = 1000
mid_term_heat_threshold = 13.0
mid_term_similarity_threshold = 0.7

# Task Queue 配置
[tasks]
queue_name = "memoryos:tasks"
consumer_group = "memoryos-workers"
max_retries = 3
retry_delay_ms = 5000

# Auth 配置
[auth]
enabled = true
api_key_prefix = "sk_"

# Quota 配置
[quota]
enabled = true
default_daily_requests = 10000
default_daily_tokens = 1000000
default_concurrent_requests = 100

# Monitoring 配置
[monitoring]
prometheus_enabled = true
prometheus_port = 9090
log_level = "info"
log_format = "json"

# Rate Limiting 配置
[rate_limit]
enabled = true
max_requests_per_minute = 100
```

---

## 监控和运维

### 1. Prometheus 监控

#### prometheus.yml
```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'memoryos-gateway'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
```

#### 关键指标
```
# 请求指标
http_requests_total
http_requests_duration_seconds
http_requests_in_flight

# 认证指标
auth_success_total
auth_failure_total
quota_exceeded_total

# Embedding 指标
embedding_cache_hits_total
embedding_cache_misses_total
embedding_duration_seconds

# 任务指标
tasks_enqueued_total
tasks_processed_total
tasks_failed_total
task_duration_seconds
```

### 2. Grafana Dashboard

导入 Dashboard: `grafana/memoryos-dashboard.json`

### 3. 日志查询

```bash
# 查看 Gateway 日志
kubectl logs -f deployment/memoryos-gateway -n memoryos

# 查看 Worker 日志
kubectl logs -f deployment/memoryos-worker -n memoryos

# 查看错误日志
kubectl logs deployment/memoryos-gateway -n memoryos | grep ERROR

# 查看特定用户的日志
kubectl logs deployment/memoryos-gateway -n memoryos | grep "user_id=user_123"
```

### 4. 健康检查

```bash
# 存活检查
curl http://api.memoryos.com/health/live

# 就绪检查
curl http://api.memoryos.com/health/ready

# 详细状态
curl http://api.memoryos.com/health/status
```

### 5. 备份和恢复

#### 备份
```bash
# 备份 Postgres
pg_dump memoryos > backup_$(date +%Y%m%d).sql

# 备份 Redis
redis-cli --rdb /backup/dump.rdb

# 备份 Qdrant
curl -X POST http://localhost:6333/collections/memoryos/snapshots
```

#### 恢复
```bash
# 恢复 Postgres
psql memoryos < backup_20260217.sql

# 恢复 Redis
redis-cli --rdb /backup/dump.rdb

# 恢复 Qdrant
curl -X PUT http://localhost:6333/collections/memoryos/snapshots/upload \
  -F 'snapshot=@snapshot.tar'
```

---

## 故障排查

### 常见问题

#### 1. Gateway 无法启动
```bash
# 检查端口占用
lsof -i :8080

# 检查配置文件
cat config.toml

# 检查环境变量
env | grep MEMORYOS
```

#### 2. Embedding 失败
```bash
# 检查模型文件
ls -la models/bge-m3/

# 检查 ONNX Runtime
ldconfig -p | grep onnx

# 查看日志
tail -f logs/memoryos.log | grep embedding
```

#### 3. 任务队列堵塞
```bash
# 检查 Redis Stream
redis-cli XLEN memoryos:tasks

# 检查 Worker 状态
kubectl get pods -n memoryos | grep worker

# 清空队列（谨慎）
redis-cli DEL memoryos:tasks
```

---

**Phase 6 部署文档 - 完成！** 🚀
