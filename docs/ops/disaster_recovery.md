# Disaster Recovery Plan (P1)

> **Status**: Approved
> **Objective**: Define RTO/RPO targets and cross-region failover strategy.

## 1. Service Level Objectives (SLO)

### 1.1 RTO/RPO by Tier

| Tier | RTO (Recovery Time) | RPO (Data Loss) | Backup Frequency | Cost Impact |
| :--- | :--- | :--- | :--- | :--- |
| **Free** | 24 hours | 24 hours | Daily snapshot | Low |
| **Pro** | 4 hours | 1 hour | Hourly incremental | Medium |
| **Enterprise** | 1 hour | 5 minutes | Real-time replication | High |

**Definitions**:
- **RTO**: Maximum acceptable downtime before service restoration.
- **RPO**: Maximum acceptable data loss (time window).

---

## 2. Backup Strategy

### 2.1 Component-Specific Backup

#### Redis (STM)
- **Method**: AOF (Append-Only File) + RDB Snapshot
- **Frequency**: 
  - AOF: Every second (fsync)
  - RDB: Every 15 minutes
- **Retention**: 7 days
- **Storage**: S3 `s3://memoryos-backup/redis/{date}/`

#### Qdrant (MTM/LTM Vectors)
- **Method**: Native Snapshot API
- **Frequency**: 
  - Pro: Every hour
  - Enterprise: Every 15 minutes
- **Retention**: 30 days
- **Storage**: S3 `s3://memoryos-backup/qdrant/{date}/`

#### PostgreSQL (Metadata)
- **Method**: WAL (Write-Ahead Logging) + pg_dump
- **Frequency**: 
  - WAL: Continuous streaming
  - Full dump: Daily
- **Retention**: 90 days
- **Storage**: S3 `s3://memoryos-backup/postgres/{date}/`

### 2.2 Cross-Region Replication

**Architecture**:
```
Primary Region (us-west-2)
    ↓ (Async Replication)
Secondary Region (us-east-1)
```

**Replication Lag**:
- Target: < 5 minutes
- Alert: If lag > 10 minutes

---

## 3. Disaster Scenarios & Response

### 3.1 Scenario A: Single Pod Failure
**Impact**: Minimal (K8s auto-restart)
**RTO**: 30 seconds
**RPO**: 0 (No data loss)
**Action**: None (Automatic)

### 3.2 Scenario B: Redis Cluster Failure
**Impact**: STM unavailable, Gateway degraded mode
**RTO**: 15 minutes
**RPO**: 1 second (AOF)
**Action**:
1. Restore from latest RDB snapshot
2. Replay AOF log
3. Verify data integrity
4. Resume service

### 3.3 Scenario C: Qdrant Cluster Failure
**Impact**: Memory retrieval unavailable
**RTO**: 1 hour
**RPO**: 15 minutes (Enterprise) / 1 hour (Pro)
**Action**:
1. Spin up new Qdrant cluster
2. Restore from latest snapshot
3. Re-index missing data (from message queue replay)
4. Verify vector count matches expected

### 3.4 Scenario D: Entire Region Failure (Catastrophic)
**Impact**: Complete service outage
**RTO**: 4 hours (Pro) / 1 hour (Enterprise)
**RPO**: 1 hour (Pro) / 5 minutes (Enterprise)
**Action**:
1. **Failover to Secondary Region**:
   - Update DNS: `api.memoryos.com` → Secondary LB
   - TTL: 60 seconds (fast propagation)
2. **Restore Services**:
   - Redis: Restore from S3 backup
   - Qdrant: Restore from S3 backup
   - PostgreSQL: Promote read replica to primary
3. **Verify Data Integrity**:
   - Run consistency checks
   - Compare record counts
4. **Resume Traffic**:
   - Enable health checks
   - Gradual traffic shift (10% → 50% → 100%)

---

## 4. Disaster Recovery Drill (Quarterly)

### 4.1 Drill Schedule
- **Frequency**: Every 3 months
- **Duration**: 2 hours
- **Participants**: DevOps, SRE, Engineering Lead

### 4.2 Drill Procedure
1. **T-0**: Announce drill start (no user impact)
2. **T+5min**: Simulate primary region failure (block traffic)
3. **T+10min**: Execute failover to secondary
4. **T+30min**: Verify all services operational
5. **T+60min**: Run integration tests
6. **T+90min**: Restore primary region
7. **T+120min**: Failback to primary

### 4.3 Success Criteria
- ✅ RTO met (< target time)
- ✅ RPO met (data loss within acceptable range)
- ✅ All integration tests pass
- ✅ No manual intervention required (automated runbook)

### 4.4 Post-Drill Report
- Document actual RTO/RPO achieved
- Identify bottlenecks
- Update runbook
- Schedule remediation tasks

---

## 5. Backup Verification (Monthly)

**Problem**: Backups exist but are corrupted/incomplete.

**Solution**: Monthly restore test
1. Restore backup to isolated environment
2. Run smoke tests:
   - Redis: `PING` + `GET test_key`
   - Qdrant: Search test vector
   - PostgreSQL: `SELECT COUNT(*) FROM users`
3. Compare counts with production
4. Alert if mismatch > 1%

---

## 6. Data Retention Policy

### 6.1 Backup Lifecycle
| Data Type | Hot Backup | Warm Archive | Cold Archive | Deletion |
| :--- | :--- | :--- | :--- | :--- |
| **Redis** | 7 days (S3 Standard) | 30 days (S3 IA) | 90 days (Glacier) | After 90 days |
| **Qdrant** | 30 days (S3 Standard) | 90 days (S3 IA) | 1 year (Glacier) | After 1 year |
| **PostgreSQL** | 90 days (S3 Standard) | 1 year (S3 IA) | 7 years (Glacier Deep) | After 7 years |

### 6.2 Legal Hold
- If user requests data deletion (GDPR) → Mark backup for exclusion
- Regenerate backup without deleted user data
- Old backups expire naturally (per retention policy)

---

## 7. Monitoring & Alerting

### 7.1 Backup Health Metrics
- `backup_success_total{component="redis"}` (Counter)
- `backup_duration_seconds{component="qdrant"}` (Histogram)
- `backup_size_bytes{component="postgres"}` (Gauge)

### 7.2 Critical Alerts
| Alert | Condition | Severity | Action |
| :--- | :--- | :--- | :--- |
| **BackupFailed** | Last backup > 2x frequency | 🔴 Critical | Page on-call engineer |
| **ReplicationLag** | Lag > 10 minutes | 🟡 Warning | Investigate network |
| **BackupSizeAnomaly** | Size change > 50% | 🟡 Warning | Verify data integrity |

---

## 8. Runbook: Region Failover

**Trigger**: Primary region unreachable for > 5 minutes

**Automated Steps** (via Terraform/Ansible):
```bash
# 1. Update DNS
aws route53 change-resource-record-sets \
  --hosted-zone-id Z123 \
  --change-batch file://failover-dns.json

# 2. Promote Secondary PostgreSQL
aws rds promote-read-replica --db-instance-identifier memoryos-secondary

# 3. Restore Redis from S3
redis-cli --rdb /backup/latest.rdb
systemctl start redis

# 4. Restore Qdrant from S3
qdrant-cli restore --snapshot s3://memoryos-backup/qdrant/latest.snapshot

# 5. Health Check
curl https://api.memoryos.com/health/ready
# Expected: 200 OK

# 6. Enable Traffic
kubectl scale deployment gateway --replicas=20
```

**Manual Verification**:
- [ ] Check Grafana dashboards (QPS, Latency, Error Rate)
- [ ] Run smoke test: Create user → Add memory → Retrieve memory
- [ ] Notify users via status page: "Service restored on backup region"

---

## 9. Cost Estimation

### 9.1 Backup Storage Cost (Monthly)
- Redis (7 days × 10GB): $0.23
- Qdrant (30 days × 500GB): $11.50
- PostgreSQL (90 days × 50GB): $1.15
- **Total**: ~$13/month (per 1000 users)

### 9.2 Cross-Region Replication Cost
- Data transfer: $0.02/GB
- Estimated: 100GB/day × 30 days × $0.02 = $60/month

### 9.3 Disaster Recovery Drill Cost
- Secondary region compute (2 hours): $5
- **Total**: $20/quarter

---

## 10. Compliance Requirements

### 10.1 SOC 2 Type II
- **Requirement**: Backup tested quarterly
- **Evidence**: Drill reports + restore test logs

### 10.2 ISO 27001
- **Requirement**: RTO/RPO documented and approved
- **Evidence**: This document + Board approval

### 10.3 GDPR
- **Requirement**: Backup data encrypted at rest
- **Implementation**: S3 SSE-KMS with customer-managed keys
