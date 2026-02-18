use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Bloom Filter for FAQ fast matching
pub struct BloomFilter {
    bits: Vec<bool>,
    hash_count: usize,
}

impl BloomFilter {
    pub fn new(capacity: usize, false_positive_rate: f64) -> Self {
        let bits_count = Self::optimal_bits(capacity, false_positive_rate);
        let hash_count = Self::optimal_hashes(capacity, bits_count);
        
        Self {
            bits: vec![false; bits_count],
            hash_count,
        }
    }
    
    fn optimal_bits(n: usize, p: f64) -> usize {
        (-(n as f64) * p.ln() / (2.0_f64.ln().powi(2))).ceil() as usize
    }
    
    fn optimal_hashes(n: usize, m: usize) -> usize {
        ((m as f64 / n as f64) * 2.0_f64.ln()).ceil() as usize
    }
    
    fn hash(&self, item: &str, seed: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        seed.hash(&mut hasher);
        (hasher.finish() as usize) % self.bits.len()
    }
    
    pub fn insert(&mut self, item: &str) {
        for i in 0..self.hash_count {
            let pos = self.hash(item, i);
            self.bits[pos] = true;
        }
    }
    
    pub fn contains(&self, item: &str) -> bool {
        (0..self.hash_count).all(|i| self.bits[self.hash(item, i)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bloom_filter() {
        let mut bloom = BloomFilter::new(1000, 0.01);
        
        bloom.insert("WiFi密码是多少？");
        bloom.insert("如何报销？");
        
        assert!(bloom.contains("WiFi密码是多少？"));
        assert!(bloom.contains("如何报销？"));
        assert!(!bloom.contains("不存在的问题"));
    }
}
