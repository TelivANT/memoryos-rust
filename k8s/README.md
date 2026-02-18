# K8s Deployment Modes

## Files
- `k8s/deployment.yaml`: **App-only (production recommended)**. Gateway on K8s, middleware (Redis/Qdrant) provided by external/hosted infra.
- `k8s/middleware-demo.yaml`: **All-in-one demo**. Gateway + Redis + Qdrant in one namespace for integration verification.

## 1) App-only (Production)

Apply:
```bash
kubectl apply -f k8s/deployment.yaml
```

Key points:
- Override these env vars to your real infra endpoints:
  - `MEMORYOS__REDIS__URL`
  - `MEMORYOS__QDRANT__URL` (use **6334**, gRPC)
- Put real API key into `memoryos-secrets`.

## 2) Middleware Demo (Integration)

Apply:
```bash
kubectl apply -f k8s/middleware-demo.yaml
```

Verify:
```bash
kubectl -n memoryos-demo get pods
kubectl -n memoryos-demo get svc
kubectl -n memoryos-demo port-forward svc/memoryos-gateway 8080:80
curl http://127.0.0.1:8080/health/status
```

## Notes
- Qdrant in this project is accessed via `qdrant-client` gRPC path; service port **6334** must be reachable.
- If you deploy gateway as multiple replicas, keep middleware endpoints stable (managed Redis/Qdrant or dedicated StatefulSets).
