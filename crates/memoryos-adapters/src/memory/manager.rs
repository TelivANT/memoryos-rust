//! Memory Manager - orchestrates all memory layers

use async_trait::async_trait;
use memoryos_core::{
    AppError, KnowledgeItem, LongTermMemory, MemoryContext, Message, MidTermSegment, UserProfile,
};
use memoryos_ports::{
    ConcurrencyControl, LlmAdapter, MemoryManager, ShortTermStorage, VectorStorage,
};
use reqwest::StatusCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Embedding 缓存
struct EmbeddingCache {
    cache: RwLock<HashMap<String, Vec<f32>>>,
    max_size: usize,
}

impl EmbeddingCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    async fn get(&self, text: &str) -> Option<Vec<f32>> {
        self.cache.read().await.get(text).cloned()
    }

    async fn set(&self, text: String, embedding: Vec<f32>) {
        let mut cache = self.cache.write().await;
        if cache.len() >= self.max_size {
            // 简单 LRU：清空一半
            cache.clear();
            info!("Embedding cache cleared (reached max size)");
        }
        cache.insert(text, embedding);
    }
}

pub struct DefaultMemoryManager {
    short_term: Arc<dyn ShortTermStorage>,
    vector_store: Arc<dyn VectorStorage>,
    write_coordinator: Option<Arc<dyn ConcurrencyControl>>,
    history_storage: Option<Arc<dyn memoryos_ports::HistoryStorage>>,
    _llm: Arc<dyn LlmAdapter>,
    short_term_limit: usize,
    mid_term_limit: usize,
    short_term_capacity: usize,
    lock_ttl_ms: u64,
    dedup_ttl_seconds: usize,
    extraction_policy: ExtractionPolicy,
    embedding_cache: Arc<EmbeddingCache>,
    embedding_api_key: String,
    embedding_base_url: String,
    embedding_model: String,
}

#[derive(Default)]
struct ProfileExtraction {
    traits: Vec<String>,
    preferences: Vec<String>,
    background: Option<String>,
    knowledge: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RuleTarget {
    Trait,
    Preference,
    Background,
}

#[derive(Debug, Clone, Deserialize)]
struct ExtractionRule {
    marker: String,
    target: RuleTarget,
    #[serde(default)]
    format: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ExtractionPolicy {
    #[serde(default = "default_extraction_rules")]
    rules: Vec<ExtractionRule>,
    #[serde(default = "default_min_knowledge_chars")]
    min_knowledge_chars: usize,
}

impl Default for ExtractionPolicy {
    fn default() -> Self {
        Self {
            rules: default_extraction_rules(),
            min_knowledge_chars: default_min_knowledge_chars(),
        }
    }
}

fn default_extraction_rules() -> Vec<ExtractionRule> {
    vec![
        ExtractionRule {
            marker: "i like ".to_string(),
            target: RuleTarget::Preference,
            format: None,
        },
        ExtractionRule {
            marker: "i prefer ".to_string(),
            target: RuleTarget::Preference,
            format: None,
        },
        ExtractionRule {
            marker: "i am ".to_string(),
            target: RuleTarget::Trait,
            format: None,
        },
        ExtractionRule {
            marker: "i work as ".to_string(),
            target: RuleTarget::Background,
            format: Some("works as {value}".to_string()),
        },
        ExtractionRule {
            marker: "my name is ".to_string(),
            target: RuleTarget::Background,
            format: Some("name is {value}".to_string()),
        },
    ]
}

fn default_min_knowledge_chars() -> usize {
    20
}

fn load_extraction_policy_from_env() -> ExtractionPolicy {
    const ENV_KEY: &str = "MEMORYOS_EXTRACTION_POLICY_JSON";
    match std::env::var(ENV_KEY) {
        Ok(raw) if !raw.trim().is_empty() => match serde_json::from_str::<ExtractionPolicy>(&raw) {
            Ok(policy) => policy,
            Err(err) => {
                warn!(
                    "Invalid {} config, fallback to default extraction policy: {}",
                    ENV_KEY, err
                );
                ExtractionPolicy::default()
            }
        },
        _ => ExtractionPolicy::default(),
    }
}

impl DefaultMemoryManager {
    pub fn new(
        short_term: Arc<dyn ShortTermStorage>,
        vector_store: Arc<dyn VectorStorage>,
        llm: Arc<dyn LlmAdapter>,
    ) -> Self {
        let embedding_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let embedding_base_url = std::env::var("EMBEDDING_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let embedding_model = std::env::var("EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        Self {
            short_term,
            vector_store,
            write_coordinator: None,
            history_storage: None,
            _llm: llm,
            short_term_limit: 10,
            mid_term_limit: 5,
            short_term_capacity: 20,
            lock_ttl_ms: 15_000,
            dedup_ttl_seconds: 7_200,
            extraction_policy: load_extraction_policy_from_env(),
            embedding_cache: Arc::new(EmbeddingCache::new(1000)),
            embedding_api_key,
            embedding_base_url,
            embedding_model,
        }
    }

    pub fn with_history(
        mut self,
        history_storage: Arc<dyn memoryos_ports::HistoryStorage>,
    ) -> Self {
        self.history_storage = Some(history_storage);
        self
    }

    pub fn new_with_coordinator(
        short_term: Arc<dyn ShortTermStorage>,
        vector_store: Arc<dyn VectorStorage>,
        llm: Arc<dyn LlmAdapter>,
        write_coordinator: Arc<dyn ConcurrencyControl>,
    ) -> Self {
        let embedding_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let embedding_base_url = std::env::var("EMBEDDING_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let embedding_model = std::env::var("EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        Self {
            short_term,
            vector_store,
            write_coordinator: Some(write_coordinator),
            history_storage: None,
            _llm: llm,
            short_term_limit: 10,
            mid_term_limit: 5,
            short_term_capacity: 20,
            lock_ttl_ms: 15_000,
            dedup_ttl_seconds: 7_200,
            extraction_policy: load_extraction_policy_from_env(),
            embedding_cache: Arc::new(EmbeddingCache::new(1000)),
            embedding_api_key,
            embedding_base_url,
            embedding_model,
        }
    }

    pub fn new_with_coordinator_tuning(
        short_term: Arc<dyn ShortTermStorage>,
        vector_store: Arc<dyn VectorStorage>,
        llm: Arc<dyn LlmAdapter>,
        write_coordinator: Arc<dyn ConcurrencyControl>,
        lock_ttl_ms: u64,
        dedup_ttl_seconds: usize,
    ) -> Self {
        let embedding_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let embedding_base_url = std::env::var("EMBEDDING_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let embedding_model = std::env::var("EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        Self {
            short_term,
            vector_store,
            write_coordinator: Some(write_coordinator),
            history_storage: None,
            _llm: llm,
            short_term_limit: 10,
            mid_term_limit: 5,
            short_term_capacity: 20,
            lock_ttl_ms,
            dedup_ttl_seconds,
            extraction_policy: load_extraction_policy_from_env(),
            embedding_cache: Arc::new(EmbeddingCache::new(1000)),
            embedding_api_key,
            embedding_base_url,
            embedding_model,
        }
    }

    /// Generate embedding with caching
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AppError> {
        // 1. 检查缓存
        if let Some(cached) = self.embedding_cache.get(text).await {
            return Ok(cached);
        }

        // 2. 生成 embedding
        let embedding = self.generate_embedding_impl(text).await?;

        // 3. 缓存结果
        self.embedding_cache
            .set(text.to_string(), embedding.clone())
            .await;

        Ok(embedding)
    }

    /// Generate embedding using OpenAI API (implementation)
    async fn generate_embedding_impl(&self, text: &str) -> Result<Vec<f32>, AppError> {
        if self.embedding_api_key.is_empty() {
            return Ok(generate_simple_embedding(text));
        }

        let request = serde_json::json!({
            "input": text,
            "model": self.embedding_model
        });

        let url = format!("{}/embeddings", self.embedding_base_url);
        let response = match reqwest::Client::new()
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.embedding_api_key),
            )
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(r) => r,
            Err(err) => {
                warn!("Embeddings API request failed, using fallback: {}", err);
                return Ok(generate_simple_embedding(text));
            }
        };

        if response.status() != StatusCode::OK {
            warn!(
                "Embeddings API returned status {}, using fallback",
                response.status()
            );
            return Ok(generate_simple_embedding(text));
        }

        let json: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(err) => {
                warn!(
                    "Failed to parse embeddings response, using fallback: {}",
                    err
                );
                return Ok(generate_simple_embedding(text));
            }
        };

        let embedding = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| AppError::ExternalService("Invalid embeddings response format".into()))?
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect::<Vec<f32>>();

        if embedding.is_empty() {
            warn!("Embeddings API returned empty vector, using fallback");
            return Ok(generate_simple_embedding(text));
        }

        Ok(embedding)
    }

    async fn consolidate_memory(
        &self,
        user_id: &str,
        message: &Message,
        fencing_token: Option<u64>,
    ) -> Result<(), AppError> {
        let recent = self
            .short_term
            .get_recent(user_id, self.short_term_limit)
            .await
            .unwrap_or_default();

        if recent.len() >= self.short_term_limit {
            let summary = recent
                .iter()
                .take(3)
                .map(|m| format!("{}: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n");
            let embedding = self
                .generate_embedding(&summary)
                .await
                .unwrap_or_else(|_| vec![0.0; 1536]);
            let segment = MidTermSegment {
                id: uuid::Uuid::now_v7(),
                user_id: user_id.to_string(),
                summary,
                embedding,
                heat: 1.0,
                created_at: chrono::Utc::now(),
                access_count: 0,
                heat_score: 0.0,
                last_accessed: None,
                memory_type: memoryos_core::MemoryType::QA,
            };
            if let Err(err) = self.vector_store.store_segment(segment).await {
                warn!(
                    "Mid-term consolidation failed for user {}: {}",
                    user_id, err
                );
            }
        }

        if message.role == "user" && !message.content.trim().is_empty() {
            let extracted =
                extract_profile_and_knowledge(&message.content, &self.extraction_policy);
            let existing = self
                .vector_store
                .get_long_term(user_id)
                .await
                .unwrap_or(None);
            let mut long_term = existing.unwrap_or(LongTermMemory {
                user_id: user_id.to_string(),
                profile: UserProfile {
                    traits: vec![],
                    preferences: vec![],
                    background: String::new(),
                },
                knowledge: vec![],
                graph: None,
                updated_at: chrono::Utc::now(),
            });

            for t in extracted.traits {
                push_unique_limited(&mut long_term.profile.traits, t, 20);
            }
            for p in extracted.preferences {
                push_unique_limited(&mut long_term.profile.preferences, p, 20);
            }
            if let Some(bg) = extracted.background {
                if long_term.profile.background.is_empty() {
                    long_term.profile.background = bg;
                } else if !long_term.profile.background.contains(&bg) {
                    long_term.profile.background =
                        format!("{} | {}", long_term.profile.background, bg);
                }
            }

            for k in extracted.knowledge {
                long_term.knowledge.push(KnowledgeItem {
                    id: uuid::Uuid::now_v7(),
                    content: k,
                    embedding: vec![],
                    source: "chat_user_message".to_string(),
                    created_at: chrono::Utc::now(),
                });
            }
            if long_term.knowledge.len() > 50 {
                let drain = long_term.knowledge.len() - 50;
                long_term.knowledge.drain(0..drain);
            }
            long_term.updated_at = chrono::Utc::now();

            self.vector_store
                .store_long_term_with_fencing(long_term, fencing_token)
                .await?;
        }

        Ok(())
    }
}

fn generate_simple_embedding(text: &str) -> Vec<f32> {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let hash = hasher.finish();
    (0..1536)
        .map(|i| {
            let seed = hash.wrapping_add(i as u64);
            ((seed % 1000) as f32 / 1000.0) - 0.5
        })
        .collect()
}

fn push_unique_limited(target: &mut Vec<String>, value: String, max_len: usize) {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return;
    }
    if target.iter().any(|v| v.eq_ignore_ascii_case(&trimmed)) {
        return;
    }
    target.push(trimmed);
    if target.len() > max_len {
        let overflow = target.len() - max_len;
        target.drain(0..overflow);
    }
}

fn extract_profile_and_knowledge(text: &str, policy: &ExtractionPolicy) -> ProfileExtraction {
    let mut out = ProfileExtraction::default();
    let raw = text.trim();
    if raw.is_empty() {
        return out;
    }
    let lower = raw.to_ascii_lowercase();

    for rule in &policy.rules {
        let marker_lower = rule.marker.to_ascii_lowercase();
        if let Some(value) = extract_after(&lower, raw, &marker_lower) {
            match rule.target {
                RuleTarget::Trait => out.traits.push(value),
                RuleTarget::Preference => out.preferences.push(value),
                RuleTarget::Background => {
                    let formatted = match &rule.format {
                        Some(fmt) => fmt.replace("{value}", &value),
                        None => value,
                    };
                    out.background = Some(formatted);
                }
            }
        }
    }

    let looks_declarative = !raw.ends_with('?') && raw.len() >= policy.min_knowledge_chars;
    if looks_declarative {
        out.knowledge.push(raw.to_string());
    }
    out
}

fn extract_after(lower: &str, original: &str, marker: &str) -> Option<String> {
    let start = lower.find(marker)?;
    let from = start + marker.len();
    let slice = &original[from..];
    let end = slice
        .find(|c: char| ['.', ',', '!', '?', ';'].contains(&c))
        .unwrap_or(slice.len());
    let value = slice[..end].trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[async_trait]
impl MemoryManager for DefaultMemoryManager {
    async fn add_message(&self, user_id: &str, message: Message) -> Result<(), AppError> {
        self.add_message_with_event(user_id, message, None).await
    }

    async fn add_message_with_event(
        &self,
        user_id: &str,
        message: Message,
        event_id: Option<&str>,
    ) -> Result<(), AppError> {
        info!("Adding message for user: {}", user_id);

        let lock_key = format!("lock:profile:{}", user_id);
        let version_key = format!("version:profile:{}", user_id);
        let owner_id = uuid::Uuid::now_v7().to_string();

        if let (Some(coordinator), Some(event_id)) = (&self.write_coordinator, event_id) {
            if coordinator.is_event_processed(event_id).await? {
                info!("Skip duplicate event_id={} for user={}", event_id, user_id);
                return Ok(());
            }
        }

        let fencing_token = if let Some(coordinator) = &self.write_coordinator {
            match coordinator
                .acquire_fencing_lock(&lock_key, &owner_id, self.lock_ttl_ms)
                .await?
            {
                Some(token) => Some(token),
                None => {
                    return Err(AppError::RateLimited(format!(
                        "Concurrent update in progress for user {}",
                        user_id
                    )))
                }
            }
        } else {
            None
        };

        if let (Some(coordinator), Some(token)) = (&self.write_coordinator, fencing_token) {
            let accepted = coordinator
                .enforce_fencing_version(&version_key, token)
                .await?;
            if !accepted {
                if let Err(err) = coordinator
                    .release_fencing_lock(&lock_key, &owner_id, token)
                    .await
                {
                    warn!("Failed to release stale lock for user {}: {}", user_id, err);
                }
                return Err(AppError::RateLimited(format!(
                    "Stale fencing token for user {}",
                    user_id
                )));
            }
        }

        let renewal_task =
            if let (Some(coordinator), Some(token)) = (&self.write_coordinator, fencing_token) {
                let coordinator = Arc::clone(coordinator);
                let lock_key = lock_key.clone();
                let owner_id = owner_id.clone();
                let renew_interval = Duration::from_millis((self.lock_ttl_ms / 3).max(1_000));
                let ttl_ms = self.lock_ttl_ms;
                let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();

                let handle = tokio::spawn(async move {
                    let mut ticker = tokio::time::interval(renew_interval);
                    loop {
                        tokio::select! {
                            _ = &mut stop_rx => break,
                            _ = ticker.tick() => {
                                match coordinator
                                    .renew_fencing_lock(&lock_key, &owner_id, token, ttl_ms)
                                    .await
                                {
                                    Ok(true) => {}
                                    Ok(false) => {
                                        warn!("Lock renewal stopped: lock no longer owned");
                                        break;
                                    }
                                    Err(err) => {
                                        warn!("Lock renewal failed: {}", err);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });
                Some((stop_tx, handle))
            } else {
                None
            };

        let message_for_consolidation = message.clone();
        let message_content = message.content.clone();
        let message_id = format!("msg_{}", uuid::Uuid::now_v7());

        let operation_result = async {
            self.short_term.add_message(user_id, message).await?;

            // 记录历史
            if let Some(history) = &self.history_storage {
                let entry = memoryos_core::MemoryHistoryEntry {
                    id: uuid::Uuid::now_v7().to_string(),
                    memory_id: message_id,
                    old_content: None,
                    new_content: Some(message_content),
                    event_type: memoryos_core::HistoryEventType::Add,
                    created_at: chrono::Utc::now(),
                    actor_id: Some(user_id.to_string()),
                };
                if let Err(e) = history.add_entry(entry).await {
                    warn!("Failed to record history: {}", e);
                }
            }

            self.consolidate_memory(user_id, &message_for_consolidation, fencing_token)
                .await?;
            Ok::<(), AppError>(())
        }
        .await;

        if let Some((stop_tx, handle)) = renewal_task {
            let _ = stop_tx.send(());
            let _ = handle.await;
        }

        if let (Some(coordinator), Some(token)) = (&self.write_coordinator, fencing_token) {
            if let Err(err) = coordinator
                .release_fencing_lock(&lock_key, &owner_id, token)
                .await
            {
                warn!("Failed to release lock for user {}: {}", user_id, err);
            }
        }

        operation_result?;

        if let (Some(coordinator), Some(event_id)) = (&self.write_coordinator, event_id) {
            coordinator
                .mark_event_processed(event_id, self.dedup_ttl_seconds)
                .await?;
        }

        // 检查是否需要 consolidate 到 mid-term
        self.check_and_consolidate_internal(user_id).await?;

        Ok(())
    }

    async fn retrieve_context(
        &self,
        user_id: &str,
        query: &str,
    ) -> Result<MemoryContext, AppError> {
        info!("Retrieving context for user: {}", user_id);

        // 1. Get short-term memory
        let short_term = self
            .short_term
            .get_recent(user_id, self.short_term_limit)
            .await?;

        // 2. Search mid-term memory
        let query_embedding = self.generate_embedding(query).await?;
        let mid_term = self
            .vector_store
            .search_segments(user_id, query_embedding, self.mid_term_limit)
            .await?;

        // 3. Get long-term memory
        let long_term = self.vector_store.get_long_term(user_id).await?;

        Ok(MemoryContext {
            short_term,
            mid_term,
            long_term,
        })
    }
}

// 私有辅助方法
impl DefaultMemoryManager {
    /// 检查并执行 STM → MTM consolidation
    async fn check_and_consolidate_internal(&self, user_id: &str) -> Result<(), AppError> {
        // 获取 STM 中的消息数量
        let recent_messages = self
            .short_term
            .get_recent(user_id, self.short_term_capacity)
            .await?;

        // 如果 STM 达到容量，触发 consolidation
        if recent_messages.len() >= self.short_term_capacity {
            info!(
                "STM capacity reached for user {}, triggering consolidation",
                user_id
            );
            self.consolidate_to_mid_term_internal(user_id, &recent_messages)
                .await?;
        }

        Ok(())
    }

    /// 将 STM 合并到 MTM
    async fn consolidate_to_mid_term_internal(
        &self,
        user_id: &str,
        messages: &[Message],
    ) -> Result<(), AppError> {
        if messages.is_empty() {
            return Ok(());
        }

        info!(
            "Consolidating {} messages to MTM for user: {}",
            messages.len(),
            user_id
        );

        // 1. 生成对话摘要
        let summary = self.summarize_messages_internal(messages).await?;

        // 2. 生成 embedding
        let embedding = self.generate_embedding(&summary).await?;

        // 3. 构造 MidTermSegment
        let segment_id = uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_DNS,
            format!("{}:mtm:{}", user_id, chrono::Utc::now().timestamp()).as_bytes(),
        );

        let segment = MidTermSegment {
            id: segment_id,
            user_id: user_id.to_string(),
            summary: summary.clone(),
            embedding,
            heat: 1.0,
            created_at: chrono::Utc::now(),
            access_count: 0,
            heat_score: 0.0,
            last_accessed: None,
            memory_type: memoryos_core::MemoryType::QA,
        };

        // 4. 存储到向量数据库
        self.vector_store.store_segment(segment).await?;

        // 5. 清理 STM（保留最近 5 条消息）
        let keep_count = 5.min(messages.len());
        info!(
            "Consolidation completed, keeping {} recent messages in STM",
            keep_count
        );

        Ok(())
    }

    /// 使用 LLM 总结对话
    async fn summarize_messages_internal(&self, messages: &[Message]) -> Result<String, AppError> {
        // 构造对话文本
        let conversation = messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        // 简化版：直接返回拼接的对话（避免额外的 LLM 调用）
        // 在生产环境中，这里应该调用 LLM 生成摘要
        let summary = if conversation.len() > 500 {
            format!("{}...", &conversation[..500])
        } else {
            conversation
        };

        Ok(format!(
            "[Consolidated at {}] {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            summary
        ))
    }
}

/// Fallback memory manager used when phase-3 backends are unavailable.
pub struct NoopMemoryManager;

#[async_trait]
impl MemoryManager for NoopMemoryManager {
    async fn add_message(&self, _user_id: &str, _message: Message) -> Result<(), AppError> {
        Ok(())
    }

    async fn retrieve_context(
        &self,
        _user_id: &str,
        _query: &str,
    ) -> Result<MemoryContext, AppError> {
        Ok(MemoryContext {
            short_term: vec![],
            mid_term: vec![],
            long_term: None,
        })
    }
}

/// Partial-degraded manager that keeps available memory layers online.
pub struct DegradedMemoryManager {
    short_term: Option<Arc<dyn ShortTermStorage>>,
    vector_store: Option<Arc<dyn VectorStorage>>,
    _llm: Arc<dyn LlmAdapter>,
    short_term_limit: usize,
    mid_term_limit: usize,
}

impl DegradedMemoryManager {
    pub fn new(
        short_term: Option<Arc<dyn ShortTermStorage>>,
        vector_store: Option<Arc<dyn VectorStorage>>,
        llm: Arc<dyn LlmAdapter>,
    ) -> Self {
        Self {
            short_term,
            vector_store,
            _llm: llm,
            short_term_limit: 10,
            mid_term_limit: 5,
        }
    }

    async fn generate_embedding(&self, _text: &str) -> Result<Vec<f32>, AppError> {
        Ok(vec![0.0; 1536])
    }
}

#[async_trait]
impl MemoryManager for DegradedMemoryManager {
    async fn add_message(&self, user_id: &str, message: Message) -> Result<(), AppError> {
        self.add_message_with_event(user_id, message, None).await
    }

    async fn add_message_with_event(
        &self,
        user_id: &str,
        message: Message,
        _event_id: Option<&str>,
    ) -> Result<(), AppError> {
        if let Some(short_term) = &self.short_term {
            if let Err(err) = short_term.add_message(user_id, message).await {
                warn!("Degraded short-term add failed: {}", err);
            }
        }
        Ok(())
    }

    async fn retrieve_context(
        &self,
        user_id: &str,
        query: &str,
    ) -> Result<MemoryContext, AppError> {
        let short_term = if let Some(short_term_store) = &self.short_term {
            match short_term_store
                .get_recent(user_id, self.short_term_limit)
                .await
            {
                Ok(v) => v,
                Err(err) => {
                    warn!("Degraded short-term retrieve failed: {}", err);
                    vec![]
                }
            }
        } else {
            vec![]
        };

        let (mid_term, long_term) = if let Some(vector_store) = &self.vector_store {
            let query_embedding = self.generate_embedding(query).await?;
            let mid_term = match vector_store
                .search_segments(user_id, query_embedding, self.mid_term_limit)
                .await
            {
                Ok(v) => v,
                Err(err) => {
                    warn!("Degraded vector search failed: {}", err);
                    vec![]
                }
            };
            let long_term = match vector_store.get_long_term(user_id).await {
                Ok(v) => v,
                Err(err) => {
                    warn!("Degraded long-term read failed: {}", err);
                    None
                }
            };
            (mid_term, long_term)
        } else {
            (vec![], None)
        };

        Ok(MemoryContext {
            short_term,
            mid_term,
            long_term,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use memoryos_core::{LongTermMemory, MidTermSegment};
    use memoryos_ports::{ChatRequest, ChatResponse};
    use serde::Deserialize;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tokio::sync::Mutex;

    struct TestShortTermStorage {
        writes: Arc<AtomicUsize>,
        delay_ms: u64,
    }

    #[async_trait]
    impl ShortTermStorage for TestShortTermStorage {
        async fn add_message(&self, _user_id: &str, _message: Message) -> Result<(), AppError> {
            if self.delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            }
            self.writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn get_recent(
            &self,
            _user_id: &str,
            _limit: usize,
        ) -> Result<Vec<Message>, AppError> {
            Ok(vec![])
        }

        async fn clear(&self, _user_id: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    struct TestVectorStorage {
        long_term_writes: Arc<AtomicUsize>,
        last_fencing: Arc<Mutex<Option<u64>>>,
    }

    #[async_trait]
    impl VectorStorage for TestVectorStorage {
        async fn store_segment(&self, _segment: MidTermSegment) -> Result<(), AppError> {
            Ok(())
        }

        async fn search_segments(
            &self,
            _user_id: &str,
            _query_embedding: Vec<f32>,
            _limit: usize,
        ) -> Result<Vec<MidTermSegment>, AppError> {
            Ok(vec![])
        }

        async fn store_long_term(&self, _memory: LongTermMemory) -> Result<(), AppError> {
            self.long_term_writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn store_long_term_with_fencing(
            &self,
            _memory: LongTermMemory,
            fencing_token: Option<u64>,
        ) -> Result<(), AppError> {
            self.long_term_writes.fetch_add(1, Ordering::SeqCst);
            let mut guard = self.last_fencing.lock().await;
            *guard = fencing_token;
            Ok(())
        }

        async fn get_long_term(&self, _user_id: &str) -> Result<Option<LongTermMemory>, AppError> {
            Ok(None)
        }
    }

    #[derive(Default)]
    struct TestCoordinator {
        lock_available: AtomicBool,
        dedup_processed: AtomicBool,
        fencing_accept: AtomicBool,
        renew_calls: AtomicUsize,
        mark_calls: AtomicUsize,
    }

    #[async_trait]
    impl ConcurrencyControl for TestCoordinator {
        async fn acquire_fencing_lock(
            &self,
            _lock_key: &str,
            _owner_id: &str,
            _ttl_ms: u64,
        ) -> Result<Option<u64>, AppError> {
            if self.lock_available.load(Ordering::SeqCst) {
                Ok(Some(1))
            } else {
                Ok(None)
            }
        }

        async fn renew_fencing_lock(
            &self,
            _lock_key: &str,
            _owner_id: &str,
            _fencing_token: u64,
            _ttl_ms: u64,
        ) -> Result<bool, AppError> {
            self.renew_calls.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        }

        async fn release_fencing_lock(
            &self,
            _lock_key: &str,
            _owner_id: &str,
            _fencing_token: u64,
        ) -> Result<bool, AppError> {
            Ok(true)
        }

        async fn enforce_fencing_version(
            &self,
            _version_key: &str,
            _fencing_token: u64,
        ) -> Result<bool, AppError> {
            Ok(self.fencing_accept.load(Ordering::SeqCst))
        }

        async fn is_event_processed(&self, _event_id: &str) -> Result<bool, AppError> {
            Ok(self.dedup_processed.load(Ordering::SeqCst))
        }

        async fn mark_event_processed(
            &self,
            _event_id: &str,
            _ttl_seconds: usize,
        ) -> Result<(), AppError> {
            self.mark_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct TestLlmAdapter;

    #[async_trait]
    impl LlmAdapter for TestLlmAdapter {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, AppError> {
            Err(AppError::BadRequest("not used".to_string()))
        }

        fn name(&self) -> &str {
            "test"
        }
    }

    fn build_message() -> Message {
        Message {
            role: "user".to_string(),
            content: "hello".to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn duplicate_event_is_skipped() {
        let writes = Arc::new(AtomicUsize::new(0));
        let short_term: Arc<dyn ShortTermStorage> = Arc::new(TestShortTermStorage {
            writes: writes.clone(),
            delay_ms: 0,
        });
        let vector_store: Arc<dyn VectorStorage> = Arc::new(TestVectorStorage {
            long_term_writes: Arc::new(AtomicUsize::new(0)),
            last_fencing: Arc::new(Mutex::new(None)),
        });
        let llm: Arc<dyn LlmAdapter> = Arc::new(TestLlmAdapter);
        let coordinator = Arc::new(TestCoordinator {
            lock_available: AtomicBool::new(true),
            dedup_processed: AtomicBool::new(true),
            fencing_accept: AtomicBool::new(true),
            ..Default::default()
        });

        let manager = DefaultMemoryManager::new_with_coordinator_tuning(
            short_term,
            vector_store,
            llm,
            coordinator,
            1_000,
            60,
        );

        manager
            .add_message_with_event("u1", build_message(), Some("evt-1"))
            .await
            .unwrap();
        assert_eq!(writes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn lock_contention_returns_rate_limited() {
        let short_term: Arc<dyn ShortTermStorage> = Arc::new(TestShortTermStorage {
            writes: Arc::new(AtomicUsize::new(0)),
            delay_ms: 0,
        });
        let vector_store: Arc<dyn VectorStorage> = Arc::new(TestVectorStorage {
            long_term_writes: Arc::new(AtomicUsize::new(0)),
            last_fencing: Arc::new(Mutex::new(None)),
        });
        let llm: Arc<dyn LlmAdapter> = Arc::new(TestLlmAdapter);
        let coordinator = Arc::new(TestCoordinator {
            lock_available: AtomicBool::new(false),
            dedup_processed: AtomicBool::new(false),
            fencing_accept: AtomicBool::new(true),
            ..Default::default()
        });

        let manager = DefaultMemoryManager::new_with_coordinator_tuning(
            short_term,
            vector_store,
            llm,
            coordinator,
            1_000,
            60,
        );

        let err = manager
            .add_message_with_event("u1", build_message(), Some("evt-2"))
            .await
            .unwrap_err();
        match err {
            AppError::RateLimited(_) => {}
            _ => panic!("expected rate limited error"),
        }
    }

    #[tokio::test]
    async fn long_write_triggers_lock_renewal() {
        let writes = Arc::new(AtomicUsize::new(0));
        let short_term: Arc<dyn ShortTermStorage> = Arc::new(TestShortTermStorage {
            writes,
            delay_ms: 1_500,
        });
        let vector_store: Arc<dyn VectorStorage> = Arc::new(TestVectorStorage {
            long_term_writes: Arc::new(AtomicUsize::new(0)),
            last_fencing: Arc::new(Mutex::new(None)),
        });
        let llm: Arc<dyn LlmAdapter> = Arc::new(TestLlmAdapter);
        let coordinator = Arc::new(TestCoordinator {
            lock_available: AtomicBool::new(true),
            dedup_processed: AtomicBool::new(false),
            fencing_accept: AtomicBool::new(true),
            ..Default::default()
        });
        let coordinator_ref = coordinator.clone();

        let manager = DefaultMemoryManager::new_with_coordinator_tuning(
            short_term,
            vector_store,
            llm,
            coordinator,
            1_000,
            60,
        );

        manager
            .add_message_with_event("u1", build_message(), Some("evt-3"))
            .await
            .unwrap();

        assert!(coordinator_ref.renew_calls.load(Ordering::SeqCst) >= 1);
        assert!(coordinator_ref.mark_calls.load(Ordering::SeqCst) >= 1);
    }

    #[tokio::test]
    async fn stale_fencing_token_is_rejected() {
        let short_term: Arc<dyn ShortTermStorage> = Arc::new(TestShortTermStorage {
            writes: Arc::new(AtomicUsize::new(0)),
            delay_ms: 0,
        });
        let vector_store: Arc<dyn VectorStorage> = Arc::new(TestVectorStorage {
            long_term_writes: Arc::new(AtomicUsize::new(0)),
            last_fencing: Arc::new(Mutex::new(None)),
        });
        let llm: Arc<dyn LlmAdapter> = Arc::new(TestLlmAdapter);
        let coordinator = Arc::new(TestCoordinator {
            lock_available: AtomicBool::new(true),
            dedup_processed: AtomicBool::new(false),
            fencing_accept: AtomicBool::new(false),
            ..Default::default()
        });

        let manager = DefaultMemoryManager::new_with_coordinator_tuning(
            short_term,
            vector_store,
            llm,
            coordinator,
            1_000,
            60,
        );

        let err = manager
            .add_message_with_event("u1", build_message(), Some("evt-4"))
            .await
            .unwrap_err();
        match err {
            AppError::RateLimited(msg) => assert!(msg.contains("Stale fencing token")),
            _ => panic!("expected stale fencing token error"),
        }
    }

    #[tokio::test]
    async fn consolidation_passes_fencing_token_to_long_term_write() {
        let writes = Arc::new(AtomicUsize::new(0));
        let short_term: Arc<dyn ShortTermStorage> = Arc::new(TestShortTermStorage {
            writes,
            delay_ms: 0,
        });
        let lt_writes = Arc::new(AtomicUsize::new(0));
        let last_fencing = Arc::new(Mutex::new(None));
        let vector_store: Arc<dyn VectorStorage> = Arc::new(TestVectorStorage {
            long_term_writes: lt_writes.clone(),
            last_fencing: last_fencing.clone(),
        });
        let llm: Arc<dyn LlmAdapter> = Arc::new(TestLlmAdapter);
        let coordinator = Arc::new(TestCoordinator {
            lock_available: AtomicBool::new(true),
            dedup_processed: AtomicBool::new(false),
            fencing_accept: AtomicBool::new(true),
            ..Default::default()
        });

        let manager = DefaultMemoryManager::new_with_coordinator_tuning(
            short_term,
            vector_store,
            llm,
            coordinator,
            1_000,
            60,
        );

        manager
            .add_message_with_event(
                "u1",
                Message {
                    role: "user".to_string(),
                    content: "My name is Alice. I like hiking in mountains.".to_string(),
                    timestamp: chrono::Utc::now(),
                },
                Some("evt-5"),
            )
            .await
            .unwrap();

        assert!(lt_writes.load(Ordering::SeqCst) >= 1);
        let token = *last_fencing.lock().await;
        assert_eq!(token, Some(1));
    }

    #[test]
    fn extract_profile_and_knowledge_parses_signals() {
        let policy = ExtractionPolicy::default();
        let out = extract_profile_and_knowledge(
            "My name is Alice. I like hiking. I work as engineer.",
            &policy,
        );
        assert!(out
            .preferences
            .iter()
            .any(|v| v.to_ascii_lowercase().contains("hiking")));
        let bg = out.background.unwrap_or_default().to_ascii_lowercase();
        assert!(bg.contains("name is alice") || bg.contains("works as engineer"));
        assert!(!out.knowledge.is_empty());
    }

    #[test]
    fn extract_profile_and_knowledge_ignores_short_question() {
        let policy = ExtractionPolicy::default();
        let out = extract_profile_and_knowledge("Hi?", &policy);
        assert!(out.traits.is_empty());
        assert!(out.preferences.is_empty());
        assert!(out.knowledge.is_empty());
    }

    #[test]
    fn extract_policy_supports_custom_rules_and_threshold() {
        let policy = ExtractionPolicy {
            rules: vec![ExtractionRule {
                marker: "i live in ".to_string(),
                target: RuleTarget::Background,
                format: Some("lives in {value}".to_string()),
            }],
            min_knowledge_chars: 5,
        };

        let out = extract_profile_and_knowledge("I live in Shanghai.", &policy);
        assert_eq!(out.background.unwrap_or_default(), "lives in Shanghai");
        assert_eq!(out.knowledge.len(), 1);
    }

    #[derive(Debug, Deserialize)]
    struct EvalExpect {
        traits: Vec<String>,
        preferences: Vec<String>,
        background: Option<String>,
        knowledge_saved: bool,
    }

    #[derive(Debug, Deserialize)]
    struct EvalCase {
        id: String,
        input: String,
        expect: EvalExpect,
    }

    fn contains_ci(values: &[String], target: &str) -> bool {
        let target_lc = target.to_ascii_lowercase();
        values.iter().any(|v| v.to_ascii_lowercase() == target_lc)
    }

    #[test]
    fn extraction_eval_dataset_report() {
        let mut policy = ExtractionPolicy::default();
        policy.rules.push(ExtractionRule {
            marker: "i live in ".to_string(),
            target: RuleTarget::Background,
            format: Some("lives in {value}".to_string()),
        });

        let raw = include_str!("../../../../docs/references/extraction_eval_dataset.jsonl");
        let mut total = 0usize;
        let mut passed = 0usize;

        for line in raw.lines().filter(|line| !line.trim().is_empty()) {
            total += 1;
            let case: EvalCase = serde_json::from_str(line).unwrap_or_else(|err| {
                panic!("invalid eval case jsonl line: {} ({})", line, err);
            });
            let out = extract_profile_and_knowledge(&case.input, &policy);

            let traits_ok = case
                .expect
                .traits
                .iter()
                .all(|expected| contains_ci(&out.traits, expected));
            let prefs_ok = case
                .expect
                .preferences
                .iter()
                .all(|expected| contains_ci(&out.preferences, expected));
            let background_ok = match (&case.expect.background, &out.background) {
                (None, None) => true,
                (Some(expected), Some(actual)) => actual.eq_ignore_ascii_case(expected),
                _ => false,
            };
            let knowledge_ok = out.knowledge.is_empty() != case.expect.knowledge_saved;

            let case_ok = traits_ok && prefs_ok && background_ok && knowledge_ok;
            if case_ok {
                passed += 1;
            } else {
                eprintln!(
                    "[eval][FAIL] {} => out={{traits:{:?}, preferences:{:?}, background:{:?}, knowledge_saved:{}}}",
                    case.id,
                    out.traits,
                    out.preferences,
                    out.background,
                    !out.knowledge.is_empty()
                );
            }
        }

        let score = passed as f64 / total as f64;
        eprintln!(
            "[eval] extraction dataset score: {:.2}% ({}/{})",
            score * 100.0,
            passed,
            total
        );

        assert!(
            score >= 0.95,
            "extraction eval score below threshold: {}",
            score
        );
    }
}
