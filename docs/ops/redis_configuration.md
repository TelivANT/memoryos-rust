# Redis Production Configuration (P0)

> **Status**: Mandatory
> **Objective**: Zero Data Loss for Short-Term Memory (STM).

## 1. Persistence (Data Safety)

We strictly prohibit "Ephemeral Redis". MemoryOS treats Redis as a primary database for STM.

### `redis.conf` Requirements

```conf
# 1. AOF (Append Only File) - Primary
appendonly yes
# Sync every second (Balance between perf and safety)
# Max data loss: 1 second
appendfsync everysec 

# 2. RDB (Snapshot) - Backup
# Save if 1 key changed in 15 mins
save 900 1
# Save if 10 keys changed in 5 mins
save 300 10
# Save if 10000 keys changed in 60s
save 60 10000
```

## 2. Memory Management

```conf
# Max memory limit (e.g., 4GB)
maxmemory 4gb
# Eviction Policy: VOLATILE-LRU
# Only evict keys with TTL (e.g., cached translations). 
# NEVER evict STM queues (which should not have TTL).
maxmemory-policy volatile-lru
```

## 3. High Availability

### Topology
*   **Minimum**: 3 Nodes (Sentinel Mode).
*   **Recommended**: Redis Cluster (for > 100GB data).

### Client Config (Rust)
*   **Read Preference**: `PreferReplica` (Offload reads).
*   **Write Concern**: `1` (Master ack is enough). For critical data, use `WAIT 2 1000`.
