/// Hierarchical similarity filter
pub struct SimilarityFilter;

impl SimilarityFilter {
    /// Fast pre-filter using first N dimensions
    pub fn quick_filter(
        query: &[f32],
        candidates: &[(Vec<f32>, usize)],
        threshold: f32,
        prefix_dim: usize,
    ) -> Vec<usize> {
        candidates
            .iter()
            .filter(|(embedding, _)| {
                let score = Self::dot_product(&query[..prefix_dim], &embedding[..prefix_dim]);
                score > threshold * 0.7 // Lower threshold for pre-filter
            })
            .map(|(_, idx)| *idx)
            .collect()
    }
    
    /// Full similarity calculation
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot = Self::dot_product(a, b);
        let norm_a = Self::norm(a);
        let norm_b = Self::norm(b);
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot / (norm_a * norm_b)
    }
    
    fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }
    
    fn norm(v: &[f32]) -> f32 {
        v.iter().map(|x| x * x).sum::<f32>().sqrt()
    }
    
    /// Two-stage filtering
    pub fn filter_similar(
        query: &[f32],
        candidates: Vec<(Vec<f32>, usize)>,
        threshold: f32,
    ) -> Vec<(usize, f32)> {
        // Stage 1: Quick filter
        let prefix_dim = query.len().min(64);
        let filtered_indices = Self::quick_filter(query, &candidates, threshold, prefix_dim);
        
        // Stage 2: Full calculation
        filtered_indices
            .into_iter()
            .filter_map(|idx| {
                let embedding = &candidates[idx].0;
                let score = Self::cosine_similarity(query, embedding);
                if score > threshold {
                    Some((idx, score))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((SimilarityFilter::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        
        let c = vec![0.0, 1.0, 0.0];
        assert!(SimilarityFilter::cosine_similarity(&a, &c).abs() < 0.001);
    }
    
    #[test]
    fn test_filter_similar() {
        let query = vec![1.0; 128];
        let candidates = vec![
            (vec![1.0; 128], 0),
            (vec![0.5; 128], 1),
            (vec![0.0; 128], 2),
        ];
        
        let results = SimilarityFilter::filter_similar(&query, candidates, 0.7);
        assert!(results.len() >= 1);
    }
}
