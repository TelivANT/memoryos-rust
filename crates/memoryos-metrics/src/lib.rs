//! Prometheus metrics for MemoryOS
//!
//! Pre-registered counters, histograms, and gauges for HTTP requests,
//! memory operations, vector DB, FAQ, router, and LLM metrics.

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_histogram_vec, register_int_counter_vec, register_int_gauge,
    CounterVec, HistogramVec, IntCounterVec, IntGauge,
};

lazy_static! {
    // ── HTTP metrics ──────────────────────────────────────────────
    // NOTE: These .expect() calls are intentional. Prometheus metric registration
    // only fails on duplicate metric names (a programming bug). Using lazy_static
    // ensures each metric is registered exactly once. A panic here indicates a
    // metric name collision that must be fixed in code.
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_http_requests_total",
        "Total HTTP requests",
        &["method", "path", "status"]
    )
    .expect("Failed to register HTTP_REQUESTS_TOTAL metric");

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "path"],
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Failed to register HTTP_REQUEST_DURATION metric");

    // ── Memory operation metrics ──────────────────────────────────
    pub static ref MEMORY_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_memory_operations_total",
        "Total memory operations",
        &["operation", "tier", "status"]
    )
    .expect("Failed to register MEMORY_OPERATIONS_TOTAL metric");

    pub static ref MEMORY_OPERATION_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_memory_operation_duration_seconds",
        "Memory operation duration in seconds",
        &["operation", "tier"]
    )
    .expect("Failed to register MEMORY_OPERATION_DURATION metric");

    // ── Vector DB metrics ─────────────────────────────────────────
    pub static ref VECTOR_DB_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_vector_db_operations_total",
        "Total vector DB operations",
        &["db_type", "operation", "status"]
    )
    .expect("Failed to register VECTOR_DB_OPERATIONS_TOTAL metric");

    pub static ref VECTOR_DB_LATENCY: HistogramVec = register_histogram_vec!(
        "memoryos_vector_db_latency_seconds",
        "Vector DB operation latency in seconds",
        &["db_type", "operation"]
    )
    .expect("Failed to register VECTOR_DB_LATENCY metric");

    // ── Router metrics ────────────────────────────────────────────
    pub static ref ROUTER_DECISIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "memoryos_router_decisions_total",
        "Total router decisions by tier",
        &["tier"]
    )
    .expect("Failed to register ROUTER_DECISIONS_TOTAL metric");

    pub static ref ROUTER_DECISION_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_router_decision_duration_seconds",
        "Router decision latency in seconds",
        &["tier"]
    )
    .expect("Failed to register ROUTER_DECISION_DURATION metric");

    // ── FAQ metrics ───────────────────────────────────────────────
    pub static ref FAQ_DIRECT_HITS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "memoryos_faq_direct_hits_total",
        "Total FAQ direct hit (Tier 0) responses",
        &["matched"]
    )
    .expect("Failed to register FAQ_DIRECT_HITS_TOTAL metric");

    pub static ref FAQ_PROMOTIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "memoryos_faq_promotions_total",
        "Total FAQ promotions",
        &["from", "to"]
    )
    .expect("Failed to register FAQ_PROMOTIONS_TOTAL metric");

    pub static ref FAQ_CLASSIFICATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "memoryos_faq_classifications_total",
        "Total LLM FAQ classification calls",
        &["status"]
    )
    .expect("Failed to register FAQ_CLASSIFICATIONS_TOTAL metric");

    // ── LLM metrics ──────────────────────────────────────────────
    pub static ref LLM_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_llm_requests_total",
        "Total LLM adapter requests",
        &["provider", "status"]
    )
    .expect("Failed to register LLM_REQUESTS_TOTAL metric");

    pub static ref LLM_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_llm_request_duration_seconds",
        "LLM adapter request duration in seconds",
        &["provider"],
        vec![0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]
    )
    .expect("Failed to register LLM_REQUEST_DURATION metric");

    pub static ref LLM_TOKENS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_llm_tokens_total",
        "Total LLM tokens processed (estimated)",
        &["provider", "direction"]
    )
    .expect("Failed to register LLM_TOKENS_TOTAL metric");

    // ── System gauges ─────────────────────────────────────────────
    pub static ref ACTIVE_USERS: IntGauge = register_int_gauge!(
        "memoryos_active_users",
        "Number of active users"
    )
    .expect("Failed to register ACTIVE_USERS metric");

    pub static ref SHORT_TERM_MESSAGES: IntGauge = register_int_gauge!(
        "memoryos_short_term_messages_total",
        "Total short-term messages stored"
    )
    .expect("Failed to register SHORT_TERM_MESSAGES metric");

    pub static ref MID_TERM_SEGMENTS: IntGauge = register_int_gauge!(
        "memoryos_mid_term_segments_total",
        "Total mid-term segments stored"
    )
    .expect("Failed to register MID_TERM_SEGMENTS metric");

    pub static ref LONG_TERM_MEMORIES: IntGauge = register_int_gauge!(
        "memoryos_long_term_memories_total",
        "Total long-term memories stored"
    )
    .expect("Failed to register LONG_TERM_MEMORIES metric");
}

/// Encode all registered Prometheus metrics into the text exposition format.
///
/// Returns an error string if encoding fails (should be extremely rare).
pub fn gather_metrics() -> String {
    use prometheus::{Encoder, TextEncoder};
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return format!("# ERROR: Failed to encode metrics: {}\n", e);
    }

    String::from_utf8(buffer).unwrap_or_else(|e| {
        tracing::error!("Failed to convert metrics to UTF-8: {}", e);
        format!("# ERROR: Failed to convert metrics to UTF-8: {}\n", e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gather_metrics_returns_valid_text() {
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/health", "200"])
            .inc();
        let output = gather_metrics();
        assert!(output.contains("memoryos_http_requests_total"));
    }

    #[test]
    fn test_router_metrics() {
        ROUTER_DECISIONS_TOTAL
            .with_label_values(&["direct_hit"])
            .inc();
        FAQ_DIRECT_HITS_TOTAL.with_label_values(&["true"]).inc();
        let output = gather_metrics();
        assert!(output.contains("memoryos_router_decisions_total"));
        assert!(output.contains("memoryos_faq_direct_hits_total"));
    }

    #[test]
    fn test_llm_metrics() {
        LLM_REQUESTS_TOTAL
            .with_label_values(&["openai", "success"])
            .inc();
        let output = gather_metrics();
        assert!(output.contains("memoryos_llm_requests_total"));
    }

    #[test]
    fn test_faq_classification_metrics() {
        FAQ_CLASSIFICATIONS_TOTAL
            .with_label_values(&["success"])
            .inc();
        let output = gather_metrics();
        assert!(output.contains("memoryos_faq_classifications_total"));
    }

    #[test]
    fn test_system_gauges() {
        ACTIVE_USERS.set(42);
        SHORT_TERM_MESSAGES.set(100);
        MID_TERM_SEGMENTS.set(50);
        LONG_TERM_MEMORIES.set(10);
        let output = gather_metrics();
        assert!(output.contains("memoryos_active_users"));
        assert!(output.contains("memoryos_short_term_messages_total"));
        assert!(output.contains("memoryos_mid_term_segments_total"));
        assert!(output.contains("memoryos_long_term_memories_total"));
    }

    #[test]
    fn test_vector_db_metrics() {
        VECTOR_DB_OPERATIONS_TOTAL
            .with_label_values(&["qdrant", "search", "success"])
            .inc();
        VECTOR_DB_LATENCY
            .with_label_values(&["qdrant", "search"])
            .observe(0.05);
        let output = gather_metrics();
        assert!(output.contains("memoryos_vector_db_operations_total"));
        assert!(output.contains("memoryos_vector_db_latency_seconds"));
    }

    #[test]
    fn test_memory_operation_metrics() {
        MEMORY_OPERATIONS_TOTAL
            .with_label_values(&["store", "stm", "success"])
            .inc();
        MEMORY_OPERATION_DURATION
            .with_label_values(&["store", "stm"])
            .observe(0.01);
        let output = gather_metrics();
        assert!(output.contains("memoryos_memory_operations_total"));
        assert!(output.contains("memoryos_memory_operation_duration_seconds"));
    }

    #[test]
    fn test_http_request_duration_histogram() {
        HTTP_REQUEST_DURATION
            .with_label_values(&["POST", "/v1/memory"])
            .observe(0.123);
        let output = gather_metrics();
        assert!(output.contains("memoryos_http_request_duration_seconds"));
    }

    #[test]
    fn test_faq_promotions_metric() {
        FAQ_PROMOTIONS_TOTAL
            .with_label_values(&["qa", "faq_candidate"])
            .inc();
        let output = gather_metrics();
        assert!(output.contains("memoryos_faq_promotions_total"));
    }

    #[test]
    fn test_llm_tokens_metric() {
        LLM_TOKENS_TOTAL
            .with_label_values(&["openai", "input"])
            .inc_by(150.0);
        LLM_TOKENS_TOTAL
            .with_label_values(&["openai", "output"])
            .inc_by(50.0);
        let output = gather_metrics();
        assert!(output.contains("memoryos_llm_tokens_total"));
    }
}
