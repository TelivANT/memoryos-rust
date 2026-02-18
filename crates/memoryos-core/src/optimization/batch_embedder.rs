use crate::error::Result;

/// Batch embedding generator
pub struct BatchEmbedder;

impl BatchEmbedder {
    pub async fn embed_batch<F, Fut>(
        texts: Vec<String>,
        embed_fn: F,
    ) -> Result<Vec<Vec<f32>>>
    where
        F: Fn(Vec<String>) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Vec<f32>>>>,
    {
        const BATCH_SIZE: usize = 32;
        
        let mut all_embeddings = Vec::new();
        
        for chunk in texts.chunks(BATCH_SIZE) {
            let embeddings = embed_fn(chunk.to_vec()).await?;
            all_embeddings.extend(embeddings);
        }
        
        Ok(all_embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_batch_embedder() {
        let texts = vec![
            "text1".to_string(),
            "text2".to_string(),
            "text3".to_string(),
        ];
        
        let result = BatchEmbedder::embed_batch(texts, |batch| async move {
            Ok(batch.iter().map(|_| vec![1.0, 2.0]).collect())
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }
}
