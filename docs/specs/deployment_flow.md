# Deployment Flow: Infrastructure Mapping

> **Objective**: Identify the physical location and health of each component within 30 seconds.
> **Key**: 🌐 Global | 🌩️ Cloud Region | 🏢 On-Premise

## 1. Global Traffic Ingress (URL Mapping)

| Domain | DNS Record | Target Service | Purpose |
| :--- | :--- | :--- | :--- |
| `api.memoryos.internal` | CNAME -> `ingress-controller` | **Gateway Service** | User Chat / Admin API |
| `metrics.memoryos.internal` | CNAME -> `prometheus-server` | **Prometheus** | Monitoring Dashboard |

---

## 2. Kubernetes Cluster Topology (K8s Namespace: `memoryos-prod`)

### 🟢 Service Layer (Stateless Computing)

| Component | Logical Name | K8s Resource | Node Type | Replicas | Scaling Trigger |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **API Gateway** | `gateway` | `Deployment: gateway-v1` | **General Purpose** (e.g., m5.large) | 20+ | CPU > 70% |
| **Async Worker** | `worker` | `Deployment: worker-v1` | **Compute Optimized** (e.g., c5.xlarge) | 80+ | Queue Depth > 1000 |

### 🟡 Data Layer (Stateful Storage)

| Component | Logical Name | K8s Resource | Node Type | Storage Class | Persistence |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Redis** | `redis-cluster` | `StatefulSet: redis` | **Memory Optimized** (r5.large) | SSD (gp3) | Yes (AOF) |
| **Qdrant** | `qdrant-cluster` | `StatefulSet: qdrant` | **Memory/Storage Optimized** (r5.2xlarge) | High IOPS SSD (io2) | Yes (Snapshots) |
| **Message Queue** | `nats-jetstream` | `StatefulSet: nats` | **General Purpose** | SSD (gp3) | Yes (File Store) |
| **Metadata DB** | `postgres-primary` | `StatefulSet: postgres` | **General Purpose** | SSD (gp3) | Yes (WAL Archiving) |

---

## 3. Network Flow & Firewalls (Security Groups)

| Source | Destination | Protocol | Port | Allow Rule Description |
| :--- | :--- | :--- | :--- | :--- |
| **Internet** | **Ingress LB** | HTTPS | 443 | Public API Access |
| **Ingress LB** | **Gateway Pods** | HTTP | 8080 | Internal Routing |
| **Gateway Pods** | **Redis Cluster** | TCP | 6379 | Session/Cache Access |
| **Gateway Pods** | **Qdrant Cluster** | gRPC/HTTP | 6333/6334 | Vector Search |
| **Gateway Pods** | **NATS/Kafka** | TCP | 4222/9092 | Event Publishing |
| **Worker Pods** | **NATS/Kafka** | TCP | 4222/9092 | Event Consumption |
| **Worker Pods** | **Upstream API** | HTTPS | 443 | **Egress: OpenAI/Google** |

### 🔍 Quick Troubleshooting (30s Drill)
1.  **Check Pod Status**: `kubectl get pods -n memoryos-prod` (Are any crashing?)
2.  **Check Logs**: `kubectl logs -l app=gateway --tail=100` (Any 5xx errors?)
3.  **Check Redis**: `kubectl exec -it redis-0 -- redis-cli ping` (PONG?)
4.  **Check Qdrant**: `curl http://qdrant-cluster:6333/collections` (Responding?)
