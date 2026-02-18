# MemoryOS Deployment Guide

Quick deployment guide for MemoryOS-Rust.

## 🚀 Quick Start (Docker Compose)

### Prerequisites
- Docker 20.10+
- docker-compose 1.29+

### Steps

1. **Clone repository**:
```bash
git clone https://github.com/BAI-LAB/MemoryOS.git
cd MemoryOS/MemoryOS-Rust
```

2. **Configure environment**:
```bash
cp .env.example .env
# Edit .env with your API keys
```

3. **Deploy**:
```bash
./deploy.sh
```

4. **Verify**:
```bash
curl http://localhost:8080/health
```

## 🐳 Docker Deployment

### Build image
```bash
docker build -t memoryos:latest .
```

### Run container
```bash
docker run -d \
  --name memoryos \
  -p 8080:8080 \
  -e OPENAI_API_KEY=your_key \
  memoryos:latest
```

## ☸️ Kubernetes Deployment

### Prerequisites
- Kubernetes 1.24+
- kubectl configured

### Deploy

1. **Create namespace**:
```bash
kubectl apply -f k8s/deployment.yaml
```

2. **Update secrets**:
```bash
kubectl edit secret memoryos-secrets -n memoryos
```

3. **Verify**:
```bash
kubectl get pods -n memoryos
kubectl logs -f deployment/memoryos-gateway -n memoryos
```

4. **Access service**:
```bash
kubectl port-forward svc/memoryos-gateway 8080:80 -n memoryos
```

## 📊 Monitoring

### Prometheus

Add to `prometheus.yml`:
```yaml
scrape_configs:
  - job_name: 'memoryos'
    static_configs:
      - targets: ['memoryos-gateway:8080']
    metrics_path: '/metrics'
```

### Grafana

Import dashboard from `monitoring/grafana-dashboard.json`

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENAI_API_KEY` | OpenAI API key | - |
| `GEMINI_API_KEY` | Gemini API key | - |
| `CLAUDE_API_KEY` | Claude API key | - |
| `REDIS_URL` | Redis connection URL | `redis://localhost:6379` |
| `QDRANT_URL` | Qdrant connection URL | `http://localhost:6333` |

### Config File

Mount `config.toml` to `/app/config.toml` in container.

See `config.production.toml` for production settings.

## 🧪 Testing

### Health Check
```bash
curl http://localhost:8080/health
```

### Performance Test
```bash
./perf_test.sh
```

### Load Test
```bash
ab -n 1000 -c 10 http://localhost:8080/health
```

## 🛡️ Security

### Production Checklist

- [ ] Use HTTPS (TLS termination at load balancer)
- [ ] Set strong API keys
- [ ] Enable rate limiting
- [ ] Configure CORS if needed
- [ ] Use secrets management (Vault, K8s Secrets)
- [ ] Enable audit logging
- [ ] Regular security updates

## 📈 Scaling

### Horizontal Scaling

**Docker Compose**:
```bash
docker-compose up -d --scale memoryos=3
```

**Kubernetes**:
```bash
kubectl scale deployment memoryos-gateway --replicas=5 -n memoryos
```

### Auto-scaling

HPA is configured in `k8s/deployment.yaml`:
- Min replicas: 3
- Max replicas: 10
- Target CPU: 70%
- Target Memory: 80%

## 🔍 Troubleshooting

### Service not starting

**Check logs**:
```bash
docker-compose logs memoryos
# or
kubectl logs -f deployment/memoryos-gateway -n memoryos
```

### Redis connection failed

**Verify Redis**:
```bash
redis-cli -h localhost -p 6379 ping
```

### Qdrant connection failed

**Verify Qdrant**:
```bash
curl http://localhost:6333/health
```

## 📞 Support

- GitHub Issues: https://github.com/BAI-LAB/MemoryOS/issues
- Documentation: https://bai-lab.github.io/MemoryOS/docs

---

**Last Updated**: 2026-02-17
