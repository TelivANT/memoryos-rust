//! Prometheus metrics for MemoryOS

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_histogram_vec, register_int_gauge, CounterVec, HistogramVec,
    IntGauge,
};

lazy_static! {
    // Request metrics
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_http_requests_total",
        "Total HTTP requests",
        &["method", "path", "status"]
    )
    .unwrap();

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_http_request_duration_seconds",
        "HTTP request duration",
        &["method", "path"]
    )
    .unwrap();

    // Memory operation metrics
    pub static ref MEMORY_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_memory_operations_total",
        "Total memory operations",
        &["operation", "status"]
    )
    .unwrap();

    pub static ref MEMORY_OPERATION_DURATION: HistogramVec = register_histogram_vec!(
        "memoryos_memory_operation_duration_seconds",
        "Memory operation duration",
        &["operation"]
    )
    .unwrap();

    // Vector DB metrics
    pub static ref VECTOR_DB_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        "memoryos_vector_db_operations_total",
        "Total vector DB operations",
        &["db_type", "operation", "status"]
    )
    .unwrap();

    pub static ref VECTOR_DB_LATENCY: HistogramVec = register_histogram_vec!(
        "memoryos_vector_db_latency_seconds",
        "Vector DB operation latency",
        &["db_type", "operation"]
    )
    .unwrap();

    // System metrics
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

/// Get all metrics as Prometheus text format
pub fn gather_metrics() -> String {
    use prometheus::{Encoder, TextEncoder};
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
