use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use redis::aio::MultiplexedConnection;
use redis::{Client, Value};
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{info, warn};

const DEFAULT_STREAM_KEY: &str = "chat_log";
const DEFAULT_GROUP: &str = "memoryos-workers";
const DEFAULT_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize)]
pub struct WorkerMonitorSnapshot {
    pub async_memory_enabled: bool,
    pub stream_key: String,
    pub group: String,
    pub worker_consumers: usize,
    pub last_check_at: Option<String>,
    pub last_error: Option<String>,
}

impl WorkerMonitorSnapshot {
    pub fn from_env(async_memory_enabled: bool) -> Self {
        Self {
            async_memory_enabled,
            stream_key: stream_key_from_env(),
            group: group_from_env(),
            worker_consumers: 0,
            last_check_at: None,
            last_error: None,
        }
    }
}

pub fn stream_key_from_env() -> String {
    std::env::var("MEMORYOS_WORKER_STREAM").unwrap_or_else(|_| DEFAULT_STREAM_KEY.to_string())
}

pub fn group_from_env() -> String {
    std::env::var("MEMORYOS_WORKER_GROUP").unwrap_or_else(|_| DEFAULT_GROUP.to_string())
}

pub fn spawn_worker_monitor(redis_url: String, status: Arc<RwLock<WorkerMonitorSnapshot>>) {
    let stream_key = stream_key_from_env();
    let group = group_from_env();
    let interval_secs = std::env::var("MEMORYOS_WORKER_MONITOR_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_INTERVAL_SECS);

    tokio::spawn(async move {
        let client = match Client::open(redis_url.clone()) {
            Ok(c) => c,
            Err(err) => {
                warn!(
                    redis_url = %redis_url,
                    error = %err,
                    "worker monitor disabled: failed to create redis client"
                );
                let mut snapshot = status.write().await;
                snapshot.last_check_at = Some(chrono::Utc::now().to_rfc3339());
                snapshot.last_error = Some(format!("redis client init failed: {}", err));
                return;
            }
        };

        info!(
            stream_key = %stream_key,
            group = %group,
            interval_secs = interval_secs,
            "worker monitor started"
        );

        let mut last_has_active_consumers: Option<bool> = None;
        loop {
            let check_result = check_group_consumers(&client, &stream_key, &group).await;
            match check_result {
                Ok(consumer_count) => {
                    let has_active = consumer_count > 0;
                    {
                        let mut snapshot = status.write().await;
                        snapshot.worker_consumers = consumer_count;
                        snapshot.last_check_at = Some(chrono::Utc::now().to_rfc3339());
                        snapshot.last_error = None;
                    }
                    if last_has_active_consumers != Some(has_active) {
                        if has_active {
                            info!(
                                stream_key = %stream_key,
                                group = %group,
                                active_consumers = consumer_count,
                                "worker consumers detected for async memory pipeline"
                            );
                        } else {
                            warn!(
                                stream_key = %stream_key,
                                group = %group,
                                "async memory pipeline enabled but no active worker consumers detected"
                            );
                        }
                    }
                    last_has_active_consumers = Some(has_active);
                }
                Err(err) => {
                    warn!(
                        stream_key = %stream_key,
                        group = %group,
                        error = %err,
                        "worker monitor check failed"
                    );
                    let mut snapshot = status.write().await;
                    snapshot.worker_consumers = 0;
                    snapshot.last_check_at = Some(chrono::Utc::now().to_rfc3339());
                    snapshot.last_error = Some(err.clone());
                    last_has_active_consumers = Some(false);
                }
            }

            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}

async fn check_group_consumers(
    client: &Client,
    stream_key: &str,
    group: &str,
) -> Result<usize, String> {
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connection failed: {}", e))?;

    let groups: Vec<HashMap<String, Value>> = match redis::cmd("XINFO")
        .arg("GROUPS")
        .arg(stream_key)
        .query_async::<Vec<HashMap<String, Value>>>(&mut conn)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("no such key") || msg.contains("NOGROUP") {
                return Ok(0);
            }
            return Err(format!("XINFO GROUPS failed: {}", msg));
        }
    };

    for group_entry in groups {
        if read_string_field(&group_entry, "name").as_deref() == Some(group) {
            let consumers = read_usize_field(&group_entry, "consumers").unwrap_or(0);
            if consumers == 0 {
                return Ok(0);
            }
            let active = count_active_consumers(&mut conn, stream_key, group).await?;
            return Ok(active);
        }
    }
    Ok(0)
}

async fn count_active_consumers(
    conn: &mut MultiplexedConnection,
    stream_key: &str,
    group: &str,
) -> Result<usize, String> {
    let consumers: Vec<HashMap<String, Value>> = redis::cmd("XINFO")
        .arg("CONSUMERS")
        .arg(stream_key)
        .arg(group)
        .query_async::<Vec<HashMap<String, Value>>>(conn)
        .await
        .map_err(|e| format!("XINFO CONSUMERS failed: {}", e))?;

    let active = consumers
        .iter()
        .filter(|entry| {
            read_usize_field(entry, "pending").unwrap_or(0) > 0
                || read_usize_field(entry, "idle").unwrap_or(usize::MAX) < 120_000
        })
        .count();

    Ok(active)
}

fn read_string_field(map: &HashMap<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(|v| redis::from_redis_value::<String>(v).ok())
}

fn read_usize_field(map: &HashMap<String, Value>, key: &str) -> Option<usize> {
    map.get(key)
        .and_then(|v| redis::from_redis_value::<i64>(v).ok())
        .and_then(|v| usize::try_from(v).ok())
}
