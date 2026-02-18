use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use memoryos_adapters::{
    AzureOpenAiAdapter, ClaudeAdapter, DeepSeekAdapter, DefaultMemoryManager, GeminiAdapter,
    OllamaAdapter, OpenAiAdapter, OpenRouterAdapter, QdrantStorage, RedisStorage,
};
use memoryos_core::{AppConfig, AppError, Message};
use memoryos_ports::{LlmAdapter, MemoryManager};
use redis::streams::{StreamId, StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, Client};
use serde::Deserialize;
use tracing::{error, info, warn};

const ENV_STREAM_KEY: &str = "MEMORYOS_WORKER_STREAM";
const ENV_GROUP: &str = "MEMORYOS_WORKER_GROUP";
const ENV_CONSUMER: &str = "MEMORYOS_WORKER_CONSUMER";
const ENV_BLOCK_MS: &str = "MEMORYOS_WORKER_BLOCK_MS";
const ENV_BATCH_SIZE: &str = "MEMORYOS_WORKER_BATCH_SIZE";

const DEFAULT_STREAM_KEY: &str = "chat_log";
const DEFAULT_GROUP: &str = "memoryos-workers";
const DEFAULT_BLOCK_MS: usize = 5_000;
const DEFAULT_BATCH_SIZE: usize = 32;

#[derive(Debug, Clone)]
struct WorkerRuntimeConfig {
    stream_key: String,
    group: String,
    consumer: String,
    block_ms: usize,
    batch_size: usize,
    dlq_key: String,
}

#[derive(Debug, Deserialize)]
struct WorkerEvent {
    #[serde(default)]
    event_id: Option<String>,
    user_id: String,
    role: String,
    content: String,
    #[serde(default)]
    timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt()
        .json()
        .with_current_span(false)
        .init();

    let app_config = AppConfig::load()?;
    app_config.validate()?;
    let worker_cfg = load_worker_runtime_config();

    info!(
        "memoryos-worker starting: stream={}, group={}, consumer={}, batch_size={}, block_ms={}",
        worker_cfg.stream_key,
        worker_cfg.group,
        worker_cfg.consumer,
        worker_cfg.batch_size,
        worker_cfg.block_ms
    );

    let llm = build_llm_adapter(&app_config)?;
    let redis_storage = Arc::new(RedisStorage::new(
        &app_config.storage.redis.url,
        app_config.storage.redis.ttl_seconds,
        app_config.storage.redis.max_messages,
    )?);
    let qdrant_storage = Arc::new(QdrantStorage::new(&app_config.storage.vector.url).await?);
    let memory_manager: Arc<dyn MemoryManager> =
        Arc::new(DefaultMemoryManager::new_with_coordinator(
            redis_storage.clone(),
            qdrant_storage,
            llm,
            redis_storage,
        ));

    let stream_client = Client::open(app_config.storage.redis.url.as_str()).map_err(|e| {
        AppError::Config(format!("Failed to connect to Redis stream client: {}", e))
    })?;
    ensure_consumer_group(&stream_client, &worker_cfg).await?;

    loop {
        if let Err(err) = poll_once(&stream_client, &worker_cfg, memory_manager.clone()).await {
            error!("worker poll loop failed: {}", err);
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

fn load_worker_runtime_config() -> WorkerRuntimeConfig {
    let stream_key =
        std::env::var(ENV_STREAM_KEY).unwrap_or_else(|_| DEFAULT_STREAM_KEY.to_string());
    let group = std::env::var(ENV_GROUP).unwrap_or_else(|_| DEFAULT_GROUP.to_string());
    let consumer = std::env::var(ENV_CONSUMER).unwrap_or_else(|_| {
        format!(
            "{}-{}",
            std::env::var("HOSTNAME").unwrap_or_else(|_| "worker".to_string()),
            std::process::id()
        )
    });
    let block_ms = std::env::var(ENV_BLOCK_MS)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_BLOCK_MS);
    let batch_size = std::env::var(ENV_BATCH_SIZE)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_BATCH_SIZE);
    let dlq_key = format!("{}:dlq", stream_key);

    WorkerRuntimeConfig {
        stream_key,
        group,
        consumer,
        block_ms,
        batch_size,
        dlq_key,
    }
}

fn build_llm_adapter(config: &AppConfig) -> Result<Arc<dyn LlmAdapter>, AppError> {
    // Select the default provider
    let provider_name = &config.llm.default_provider;
    let provider_cfg = config.llm.providers.get(provider_name).ok_or_else(|| {
        AppError::Config(format!(
            "Default provider '{}' not configured",
            provider_name
        ))
    })?;

    let api_key = provider_cfg.resolve_api_key();
    let base_url = provider_cfg.base_url.clone();

    let adapter: Arc<dyn LlmAdapter> = match provider_cfg.provider_type.as_str() {
        "openai" => Arc::new(OpenAiAdapter::new(api_key, base_url)),
        "gemini" => Arc::new(GeminiAdapter::new(api_key, base_url)),
        "claude" => Arc::new(ClaudeAdapter::new(api_key, base_url)),
        "ollama" => Arc::new(OllamaAdapter::new(base_url)),
        "deepseek" => Arc::new(DeepSeekAdapter::new(api_key, base_url)),
        "openrouter" => Arc::new(OpenRouterAdapter::new(api_key, base_url)),
        "azure-openai" => Arc::new(AzureOpenAiAdapter::new(api_key, base_url)),
        p => {
            return Err(AppError::Config(format!(
                "Unsupported provider type '{}'",
                p
            )))
        }
    };
    Ok(adapter)
}

async fn ensure_consumer_group(client: &Client, cfg: &WorkerRuntimeConfig) -> Result<(), AppError> {
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;
    let create_result: redis::RedisResult<String> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(&cfg.stream_key)
        .arg(&cfg.group)
        .arg("$")
        .arg("MKSTREAM")
        .query_async(&mut conn)
        .await;

    match create_result {
        Ok(_) => info!(
            "Created consumer group '{}' on stream '{}'",
            cfg.group, cfg.stream_key
        ),
        Err(err) => {
            let msg = err.to_string();
            if !msg.contains("BUSYGROUP") {
                return Err(AppError::ExternalService(format!(
                    "Failed to create consumer group: {}",
                    msg
                )));
            }
            info!(
                "Consumer group '{}' already exists on '{}'",
                cfg.group, cfg.stream_key
            );
        }
    }
    Ok(())
}

async fn poll_once(
    client: &Client,
    cfg: &WorkerRuntimeConfig,
    memory_manager: Arc<dyn MemoryManager>,
) -> Result<(), AppError> {
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::ExternalService(format!("Redis connection failed: {}", e)))?;

    let options = StreamReadOptions::default()
        .group(&cfg.group, &cfg.consumer)
        .count(cfg.batch_size)
        .block(cfg.block_ms);

    let reply: StreamReadReply = conn
        .xread_options(&[&cfg.stream_key], &[">"], &options)
        .await
        .map_err(|e| AppError::ExternalService(format!("Redis XREADGROUP failed: {}", e)))?;

    for stream_key in reply.keys {
        for stream_id in stream_key.ids {
            handle_stream_entry(&mut conn, cfg, memory_manager.clone(), stream_id).await?;
        }
    }

    Ok(())
}

async fn handle_stream_entry<C: AsyncCommands + Send + Sync>(
    conn: &mut C,
    cfg: &WorkerRuntimeConfig,
    memory_manager: Arc<dyn MemoryManager>,
    stream_id: StreamId,
) -> Result<(), AppError> {
    let event = parse_worker_event(&stream_id).map_err(|e| {
        AppError::BadRequest(format!("invalid stream event {}: {}", stream_id.id, e))
    })?;
    let event_id = event
        .event_id
        .clone()
        .unwrap_or_else(|| stream_id.id.clone());

    let message = Message {
        role: event.role.clone(),
        content: event.content.clone(),
        timestamp: event.timestamp.unwrap_or_else(chrono::Utc::now),
    };

    match memory_manager
        .add_message_with_event(&event.user_id, message, Some(&event_id))
        .await
    {
        Ok(_) => {
            ack_message(conn, cfg, &stream_id.id).await?;
            info!(
                "processed memory event: stream_id={}, event_id={}, user_id={}",
                stream_id.id, event_id, event.user_id
            );
        }
        Err(err) => {
            warn!(
                "processing memory event failed: stream_id={}, event_id={}, user_id={}, error={}",
                stream_id.id, event_id, event.user_id, err
            );
            push_dlq(conn, cfg, &stream_id, &event_id, &event.user_id, &err).await?;
            ack_message(conn, cfg, &stream_id.id).await?;
        }
    }
    Ok(())
}

fn parse_worker_event(stream_id: &StreamId) -> Result<WorkerEvent, String> {
    if let Some(payload_raw) = field_as_string(&stream_id.map, "payload") {
        serde_json::from_str::<WorkerEvent>(&payload_raw)
            .map_err(|e| format!("payload json parse failed: {}", e))
    } else {
        let user_id = field_as_string(&stream_id.map, "user_id")
            .ok_or_else(|| "missing user_id".to_string())?;
        let role = field_as_string(&stream_id.map, "role").unwrap_or_else(|| "user".to_string());
        let content = field_as_string(&stream_id.map, "content")
            .ok_or_else(|| "missing content".to_string())?;
        let event_id = field_as_string(&stream_id.map, "event_id");
        let timestamp = field_as_string(&stream_id.map, "timestamp")
            .and_then(|v| chrono::DateTime::parse_from_rfc3339(&v).ok())
            .map(|v| v.with_timezone(&chrono::Utc));

        Ok(WorkerEvent {
            event_id,
            user_id,
            role,
            content,
            timestamp,
        })
    }
}

fn field_as_string(map: &HashMap<String, redis::Value>, key: &str) -> Option<String> {
    let value = map.get(key)?;
    redis::from_redis_value::<String>(value).ok()
}

async fn ack_message<C: AsyncCommands + Send + Sync>(
    conn: &mut C,
    cfg: &WorkerRuntimeConfig,
    stream_id: &str,
) -> Result<(), AppError> {
    conn.xack::<_, _, _, ()>(&cfg.stream_key, &cfg.group, &[stream_id])
        .await
        .map_err(|e| AppError::ExternalService(format!("Redis XACK failed: {}", e)))
}

async fn push_dlq<C: AsyncCommands + Send + Sync>(
    conn: &mut C,
    cfg: &WorkerRuntimeConfig,
    stream_id: &StreamId,
    event_id: &str,
    user_id: &str,
    err: &AppError,
) -> Result<(), AppError> {
    let payload = serde_json::to_string(&map_to_plain_json(&stream_id.map))
        .map_err(|e| AppError::Internal(format!("Failed to serialize DLQ payload: {}", e)))?;
    let fields = [
        ("source_stream", cfg.stream_key.as_str()),
        ("source_id", stream_id.id.as_str()),
        ("event_id", event_id),
        ("user_id", user_id),
        ("error", &err.to_string()),
        ("payload", payload.as_str()),
        ("failed_at", &chrono::Utc::now().to_rfc3339()),
    ];

    conn.xadd::<_, _, _, _, String>(&cfg.dlq_key, "*", &fields)
        .await
        .map(|_| ())
        .map_err(|e| AppError::ExternalService(format!("Redis DLQ XADD failed: {}", e)))
}

fn map_to_plain_json(map: &HashMap<String, redis::Value>) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for (key, value) in map {
        let v = redis::from_redis_value::<String>(value).unwrap_or_else(|_| format!("{:?}", value));
        out.insert(key.clone(), serde_json::Value::String(v));
    }
    serde_json::Value::Object(out)
}
