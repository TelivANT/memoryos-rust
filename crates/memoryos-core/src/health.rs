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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_state_serialization() {
        assert_eq!(
            serde_json::to_string(&DependencyState::Up).unwrap(),
            "\"up\""
        );
        assert_eq!(
            serde_json::to_string(&DependencyState::Down).unwrap(),
            "\"down\""
        );
        assert_eq!(
            serde_json::to_string(&DependencyState::Bypassed).unwrap(),
            "\"bypassed\""
        );
    }

    #[test]
    fn dependency_state_deserialization() {
        assert_eq!(
            serde_json::from_str::<DependencyState>("\"up\"").unwrap(),
            DependencyState::Up
        );
        assert_eq!(
            serde_json::from_str::<DependencyState>("\"down\"").unwrap(),
            DependencyState::Down
        );
        assert_eq!(
            serde_json::from_str::<DependencyState>("\"bypassed\"").unwrap(),
            DependencyState::Bypassed
        );
    }

    #[test]
    fn health_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&HealthMode::Ready).unwrap(),
            "\"ready\""
        );
        assert_eq!(
            serde_json::to_string(&HealthMode::DegradedReady).unwrap(),
            "\"degraded_ready\""
        );
        assert_eq!(
            serde_json::to_string(&HealthMode::NotReady).unwrap(),
            "\"not_ready\""
        );
    }

    #[test]
    fn health_status_full_serialization() {
        let status = HealthStatus {
            mode: HealthMode::Ready,
            redis: DependencyState::Up,
            qdrant: DependencyState::Up,
            upstream: DependencyState::Up,
            auth_cache: DependencyState::Bypassed,
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: HealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.mode, HealthMode::Ready);
        assert_eq!(deserialized.redis, DependencyState::Up);
        assert_eq!(deserialized.auth_cache, DependencyState::Bypassed);
    }

    #[test]
    fn health_status_degraded() {
        let status = HealthStatus {
            mode: HealthMode::DegradedReady,
            redis: DependencyState::Down,
            qdrant: DependencyState::Up,
            upstream: DependencyState::Up,
            auth_cache: DependencyState::Bypassed,
        };
        assert_eq!(status.mode, HealthMode::DegradedReady);
        assert_eq!(status.redis, DependencyState::Down);
    }
}
