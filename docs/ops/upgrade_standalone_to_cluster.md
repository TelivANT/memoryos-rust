# Standalone to Cluster Upgrade Runbook

> Scope: MemoryOS-Rust gateway + worker + redis + qdrant  
> Goal: upgrade with minimal downtime and safe rollback

## 1. Target Topology

- Before: `1x gateway`, `1x redis`, `1x qdrant` (optional: `1x worker` when async pipeline is enabled)
- After: `Nx gateway`, `Mx worker`, shared `redis`, shared `qdrant`, LB/Ingress

## 2. Preconditions

1. Backup Redis (`SAVE`) and Qdrant snapshot.
2. Confirm config template is identical across nodes.
3. Confirm all workers use:
   - same `MEMORYOS_WORKER_GROUP`
   - unique `MEMORYOS_WORKER_CONSUMER`
4. Confirm health endpoint works on current standalone:
   - `/health/live`
   - `/health/ready`
   - `/health/status`

## 3. Upgrade Steps

1. Expand middleware capacity first (Redis/Qdrant).
2. Deploy a second worker and verify stream consumption.
3. Deploy a second gateway and add it to LB with 10% traffic.
4. Observe for 15-30 minutes:
   - queue backlog
   - DLQ growth
   - error rate / p95 latency
5. Gradually increase traffic and instance counts.

## 4. Verification Checklist

1. All gateways return `200` on `/health/ready`.
2. `chat_log` pending entries are stable or decreasing.
3. `chat_log:dlq` has no abnormal growth.
4. No duplicated side effects for same `event_id`.

## 5. Rollback

1. Remove new gateways from LB.
2. Stop newly added workers.
3. Restore previous config and restart standalone topology.
4. Review DLQ and replay only after root cause is fixed.
   - Dry run: `DRY_RUN=1 COUNT=20 ./scripts/replay_dlq.sh`
   - Execute: `DRY_RUN=0 COUNT=20 ./scripts/replay_dlq.sh`

## 6. Operational Notes

1. Keep `MEMORYOS_WORKER_GROUP` stable across rollout.
2. Never reuse `MEMORYOS_WORKER_CONSUMER` for two live workers.
3. Keep `gateway` and `worker` pinned to same event schema version during rollout.
