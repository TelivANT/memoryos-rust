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
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_http_requests_total",
        "Total HTTP requests",
        &["method", "path", "status"]
    )
    .unwrap();

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "path"],
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .unwrap();

    // ── Memory operation metrics ──────────────────────────────────
    pub static ref MEMORY_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_memory_operations_total",
        "Total memory operations",
        &["operation", "tier", "status"]
    )
    .unwrap();

    pub static ref MEMORY_OPERATION_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_memory_operation_duration_seconds",
        "Memory operation duration in seconds",
        &["operation", "tier"]
    )
    .unwrap();

    // ── Vector DB metrics ─────────────────────────────────────────
    pub static ref VECTOR_DB_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_vector_db_operations_total",
        "Total vector DB operations",
        &["db_type", "operation", "status"]
    )
    .unwrap();

    pub static ref VECTOR_DB_LATENCY: HistogramVec = register_histogram_vec!(
        "memoryos_vector_db_latency_seconds",
        "Vector DB operation latency in seconds",
        &["db_type", "operation"]
    )
    .unwrap();

    // ── Router metrics ────────────────────────────────────────────
    pub static ref ROUTER_DECISIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "memoryos_router_decisions_total",
        "Total router decisions by tier",
        &["tier"]
    )
    .unwrap();

    pub static ref ROUTER_DECISION_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_router_decision_duration_seconds",
        "Router decision latency in seconds",
        &["tier"]
    )
    .unwrap();

    // ── FAQ metrics ───────────────────────────────────────────────
    pub static ref FAQ_DIRECT_HITS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "memoryos_faq_direct_hits_total",
        "Total FAQ direct hit (Tier 0) responses",
        &["matched"]
    )
    .unwrap();

    pub static ref FAQ_PROMOTIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "memoryos_faq_promotions_total",
        "Total FAQ promotions",
        &["from", "to"]
    )
    .unwrap();

    pub static ref FAQ_CLASSIFICATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "memoryos_faq_classifications_total",
        "Total LLM FAQ classification calls",
        &["status"]
    )
    .unwrap();

    // ── LLM metrics ──────────────────────────────────────────────
    pub static ref LLM_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_llm_requests_total",
        "Total LLM adapter requests",
        &["provider", "status"]
    )
    .unwrap();

    pub static ref LLM_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_llm_request_duration_seconds",
        "LLM adapter request duration in seconds",
        &["provider"],
        vec![0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]
    )
    .unwrap();

    pub static ref LLM_TOKENS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_llm_tokens_total",
        "Total LLM tokens processed (estimated)",
        &["provider", "direction"]
    )
    .unwrap();

    // ── System gauges ─────────────────────────────────────────────
    pub static ref ACTIVE_USERS: IntGauge = register_int_gauge!(
        "memoryos_active_users",
        "Number of active users"
    )
    .unwrap();

    pub static ref SHORT_TERM_MESSAGES: IntGauge = register_int_gauge!(
        "memoryos_short_term_messages_total",
        "Total short-term messages stored"
    )
    .unwrap();

    pub static ref MID_TERM_SEGMENTS: IntGauge = register_int_gauge!(
        "memoryos_mid_term_segments_total",
        "Total mid-term segments stored"
    )
    .unwrap();

    pub static ref LONG_TERM_MEMORIES: IntGauge = register_int_gauge!(
        "memoryos_long_term_memories_total",
        "Total long-term memories stored"
    )
    .unwrap();
}

/// Encode all registered Prometheus metrics into the text exposition format.
pub fn gather_metrics() -> String {
    use prometheus::{Encoder, TextEncoder};
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
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
}
