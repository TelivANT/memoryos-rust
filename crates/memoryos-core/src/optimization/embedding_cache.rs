use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// LRU Cache for embeddings
pub struct EmbeddingCache {
    cache: Arc<RwLock<LruCache>>,
}

struct LruCache {
    map: HashMap<String, CacheEntry>,
    capacity: usize,
    access_order: Vec<String>,
}

struct CacheEntry {
    embedding: Vec<f32>,
    access_count: u32,
}

impl EmbeddingCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache {
                map: HashMap::new(),
                capacity,
                access_order: Vec::new(),
            })),
        }
    }

    pub async fn get(&self, query: &str) -> Option<Vec<f32>> {
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.map.get(query) {
            let embedding = entry.embedding.clone();

            // Update access count
            if let Some(e) = cache.map.get_mut(query) {
                e.access_count += 1;
            }

            // Update LRU order
            if let Some(pos) = cache.access_order.iter().position(|k| k == query) {
                cache.access_order.remove(pos);
            }
            cache.access_order.push(query.to_string());

            return Some(embedding);
        }

        None
    }

    pub async fn put(&self, query: String, embedding: Vec<f32>) {
        let mut cache = self.cache.write().await;

        // If already exists, update it
        if cache.map.contains_key(&query) {
            if let Some(entry) = cache.map.get_mut(&query) {
                entry.embedding = embedding;
                entry.access_count += 1;
            }
            // Move to end (most recently used)
            if let Some(pos) = cache.access_order.iter().position(|k| k == &query) {
                cache.access_order.remove(pos);
            }
            cache.access_order.push(query);
            return;
        }

        // Evict LRU if full
        if cache.map.len() >= cache.capacity && !cache.access_order.is_empty() {
            let lru_key = cache.access_order.remove(0);
            cache.map.remove(&lru_key);
        }

        cache.map.insert(
            query.clone(),
            CacheEntry {
                embedding,
                access_count: 1,
            },
        );
        cache.access_order.push(query);
    }

    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            size: cache.map.len(),
            capacity: cache.capacity,
            hit_rate: 0.0, // TODO: track hits/misses
        }
    }
}

pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_cache() {
        let cache = EmbeddingCache::new(2);

        cache.put("query1".to_string(), vec![1.0, 2.0]).await;
        cache.put("query2".to_string(), vec![3.0, 4.0]).await;

        assert!(cache.get("query1").await.is_some());

        // Evict oldest
        cache.put("query3".to_string(), vec![5.0, 6.0]).await;
        assert!(cache.get("query2").await.is_none());
    }
}
