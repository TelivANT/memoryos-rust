# Deployment Plan for 100,000 Concurrent Users

## 1. Scale Overview (Scale Targets)

*   **Total Users**: 100,000+
*   **Concurrent Users (CCU)**: 10,000 (Estimate)
*   **QPS (Request Rate)**: 1,000 - 5,000 QPS (Bursty)
*   **Data Volume**: ~10 TB (Text/Vector/Logs per year)

## 2. Kubernetes Topology (100 Pods Plan)

We will distribute the 100 Pods across two distinct services:

### 2.1 Service A: MemoryOS Gateway (Frontend)
*   **Role**: Handle direct user traffic. Low latency, high throughput.
*   **Quantity**: **20 Pods** (Assuming 2 vCPU / 4GB RAM per Pod)
    *   **Logic**: Rust/Axum can easily handle 500 QPS per core. 20 Pods = 10,000+ QPS capacity.
*   **Scaling Trigger**: CPU Usage > 70% or Latency > 200ms.

### 2.2 Service B: MemoryOS Worker (Backend)
*   **Role**: Process memory updates, summaries, embeddings. High compute, long-running tasks.
*   **Quantity**: **80 Pods** (Assuming 1 vCPU / 2GB RAM per Pod)
    *   **Logic**: Memory processing is slow (LLM calls). We need many workers to drain the queue quickly.
*   **Scaling Trigger**: Queue Depth (Lag) > 1000 messages.

### 2.3 Service C: Analytics Worker (Mining)
*   **Role**: Cross-user pattern mining, Global Memory promotion.
*   **Quantity**: **5 Pods** (CronJob style or Daemon)
    *   **Logic**: Scans Qdrant periodically. Low frequency, high throughput scan.

### 2.4 Stateful Components (Databases)

These components should be managed via Operators or external services (RDS/Elasticache/Qdrant Cloud).

*   **Qdrant Cluster**:
    *   **Nodes**: 5 Nodes (High Memory Instance: 64GB RAM each)
    *   **Replication**: Factor 2 (HA)
    *   **Sharding**: By User ID.
*   **Redis Cluster**:
    *   **Nodes**: 3 Master + 3 Slave (Standard Instance)
    *   **Function**: Session Store, STM Cache, Rate Limiting.
*   **Message Queue**:
    *   **NATS JetStream** (Recommended) or **Kafka**.
    *   **Persistence**: Disk-based for durability.

## 3. Resource Estimation (Per User)

*   **Memory Footprint**: ~100 KB (Active Session + Cache)
*   **Storage Footprint**: ~10 MB (Vectors + Logs per year)
*   **Compute Cost**: ~0.001 vCPU (Average idle)

## 4. Middleware Requirements (Summary)

To support this scale, the following middleware is mandatory:

1.  **Nginx Ingress Controller**: Load balancing traffic to Gateway Service.
    *   **Critical Config**:
        ```nginx
        proxy_buffering off;          # Critical for LLM Streaming
        proxy_read_timeout 300s;      # Prevent timeouts during long generation
        underscores_in_headers on;    # If using legacy headers with underscores
        ```
2.  **Cert-Manager**: TLS/SSL termination.
3.  **Prometheus + Grafana**: Monitoring QPS, Latency, Queue Depth.
4.  **Fluentd / Filebeat**: Log aggregation to Elasticsearch/Loki.

## 5. Deployment Pitfalls & Solutions (避坑指南)

### 5.1 Nginx & Streaming
*   **Issue**: Nginx buffers responses by default, breaking the "typing effect" of LLMs.
*   **Fix**: Explicitly disable buffering in Ingress Annotations or Nginx Config.

### 5.2 K8s Networking (Service Exposure)
*   **Issue**: Using `NodePort` often hides the real Client IP, breaking Rate Limiting.
*   **Fix**: Use `externalTrafficPolicy: Local` on the Service definition, or trust `X-Forwarded-For` from the Load Balancer.

### 5.3 Single Node (Docker Compose)
*   **Issue**: File permission errors on mapped volumes (Qdrant data directory).
*   **Fix**: The official `docker-compose.yml` must explicitly handle UID/GID mapping or use named volumes.

### 5.4 Runtime Tuning (CPU Isolation)
*   **Issue**: Embedding calculation starving the Web Server.
*   **Fix**: Use `GOMAXPROCS` equivalent logic. Bind Axum to specific cores, and ONNX Runtime to others if possible, or use thread pool separation in config.

## 6. Deployment Strategy (Blue/Green)

Since the Gateway is stateless, we can perform zero-downtime updates:
1.  Spin up new version Pods (Green).
2.  Wait for health checks (Readiness Probe).
3.  Switch Service Selector to Green.
4.  Drain old connections (Graceful Shutdown).

For Workers, stop consuming from Queue, wait for current tasks to finish, then update.

## 7. Redis Configuration Requirements (MANDATORY)

See `docs/ops/redis_configuration.md` for full details on AOF/RDB settings.

## 8. Time Synchronization (CRITICAL)

**Data Consistency Requirement**: Clock skew between Gateway and Worker Pods > 1s can cause memory reordering bugs.

### 8.1 NTP Configuration
*   **DaemonSet**: Deploy `chrony` on all K8s Nodes.
*   **Monitor**: Alert if `node_time_offset_seconds > 0.5`.

### 8.2 Timestamp Source
*   **Best Practice**: Use Database Timestamps (e.g., SQL `NOW()`, Redis `TIME`) as the source of truth.
*   **Avoid**: Generating timestamps in Application logic (`Utc::now()`).

## 9. Worker Memory Management

**Issue**: Rust/Tokio/ONNX runtime might hold memory (fragmentation) over days.

### 9.1 Resource Limits (K8s)
```yaml
resources:
  limits:
    memory: 2Gi
  requests:
    memory: 1Gi
```

### 9.2 Auto-Restart Policy
*   **Strategy**: "Rolling Restart" every 24h.
*   **Logic**:
    1.  Stop consuming Queue.
    2.  Finish current tasks.
    3.  Exit process (K8s restarts it).
    *   *Result*: Clears memory fragmentation daily.
