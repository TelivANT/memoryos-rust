use crate::optimization::*;
use crate::error::Result;
use std::collections::HashMap;

/// Optimized FAQ matcher combining all optimization techniques
pub struct OptimizedFaqMatcher {
    bloom: BloomFilter,
    exact_cache: HashMap<u64, String>,
}

impl OptimizedFaqMatcher {
    pub fn new(capacity: usize) -> Self {
        Self {
            bloom: BloomFilter::new(capacity, 0.01),
            exact_cache: HashMap::new(),
        }
    }
    
    pub fn add_faq(&mut self, question: &str, answer: String) {
        self.bloom.insert(question);
        let hash = Self::hash_query(question);
        self.exact_cache.insert(hash, answer);
    }
    
    pub async fn match_faq(&self, query: &str) -> Option<String> {
        // Stage 1: Bloom filter check (< 1μs)
        if !self.bloom.contains(query) {
            return None;
        }
        
        // Stage 2: Exact match (< 1μs)
        let hash = Self::hash_query(query);
        if let Some(answer) = self.exact_cache.get(&hash) {
            return Some(answer.clone());
        }
        
        // Stage 3: Would do vector search here
        None
    }
    
    fn hash_query(query: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        hasher.finish()
    }
}

/// Optimized memory retriever
pub struct OptimizedRetriever {
    embedding_cache: EmbeddingCache,
}

impl OptimizedRetriever {
    pub fn new() -> Self {
        Self {
            embedding_cache: EmbeddingCache::new(1000),
        }
    }
    
    pub async fn retrieve_with_cache<F, Fut>(
        &self,
        query: &str,
        embed_fn: F,
    ) -> Result<Vec<f32>>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<f32>>>,
    {
        // Check cache first
        if let Some(embedding) = self.embedding_cache.get(query).await {
            return Ok(embedding);
        }
        
        // Cache miss, generate embedding
        let embedding = embed_fn(query.to_string()).await?;
        self.embedding_cache.put(query.to_string(), embedding.clone()).await;
        
        Ok(embedding)
    }
}

impl Default for OptimizedRetriever {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_optimized_faq_matcher() {
        let mut matcher = OptimizedFaqMatcher::new(100);
        
        matcher.add_faq("WiFi密码是多少？", "密码是12345678".to_string());
        
        let result = matcher.match_faq("WiFi密码是多少？").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "密码是12345678");
        
        let result = matcher.match_faq("不存在的问题").await;
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_optimized_retriever() {
        let retriever = OptimizedRetriever::new();
        
        let embedding1 = retriever.retrieve_with_cache("test", |_| async {
            Ok(vec![1.0, 2.0, 3.0])
        }).await.unwrap();
        
        // Second call should hit cache
        let embedding2 = retriever.retrieve_with_cache("test", |_| async {
            Ok(vec![4.0, 5.0, 6.0]) // Different, but should return cached
        }).await.unwrap();
        
        assert_eq!(embedding1, embedding2);
    }
}
