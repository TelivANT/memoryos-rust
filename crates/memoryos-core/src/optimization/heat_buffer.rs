use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Heat score buffer for batch updates
pub struct HeatBuffer {
    buffer: Arc<RwLock<HashMap<Uuid, HeatUpdate>>>,
}

pub struct HeatUpdate {
    pub access_count: u32,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
}

impl HeatBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_access(&self, segment_id: Uuid) {
        let mut buffer = self.buffer.write().await;

        buffer
            .entry(segment_id)
            .and_modify(|e| {
                e.access_count += 1;
                e.last_accessed = chrono::Utc::now();
            })
            .or_insert(HeatUpdate {
                access_count: 1,
                last_accessed: chrono::Utc::now(),
            });
    }

    pub async fn drain(&self) -> HashMap<Uuid, HeatUpdate> {
        let mut buffer = self.buffer.write().await;
        std::mem::take(&mut *buffer)
    }

    pub async fn size(&self) -> usize {
        self.buffer.read().await.len()
    }
}

impl Default for HeatBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_heat_buffer() {
        let buffer = HeatBuffer::new();
        let id = Uuid::new_v4();

        buffer.record_access(id).await;
        buffer.record_access(id).await;

        assert_eq!(buffer.size().await, 1);

        let updates = buffer.drain().await;
        assert_eq!(updates.get(&id).unwrap().access_count, 2);
        assert_eq!(buffer.size().await, 0);
    }
}
