use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyState {
    Up,
    Down,
    Bypassed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthMode {
    Ready,
    DegradedReady,
    NotReady,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub mode: HealthMode,
    pub redis: DependencyState,
    pub qdrant: DependencyState,
    pub upstream: DependencyState,
    pub auth_cache: DependencyState,
}
