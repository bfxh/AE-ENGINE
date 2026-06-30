use hashbrown::HashMap;
use lru::LruCache;
use std::num::NonZeroUsize;

use crate::inference::InferenceResult;

const L1_CAPACITY: usize = 256;
const L2_CAPACITY: usize = 4096;
const L3_CAPACITY: usize = 16384;

pub struct InferenceCache {
    l1: HashMap<u64, InferenceResult>,
    l2: LruCache<u64, InferenceResult>,
    l3: LruCache<u64, InferenceResult>,
    hits: u64,
    misses: u64,
}

impl InferenceCache {
    pub fn new() -> Self {
        Self {
            l1: HashMap::with_capacity(L1_CAPACITY),
            l2: LruCache::new(NonZeroUsize::new(L2_CAPACITY).unwrap()),
            l3: LruCache::new(NonZeroUsize::new(L3_CAPACITY).unwrap()),
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, key: &u64) -> Option<InferenceResult> {
        if let Some(result) = self.l1.get(key) {
            self.hits += 1;
            return Some(result.clone());
        }

        if let Some(result) = self.l2.get(key) {
            self.hits += 1;
            let result = result.clone();
            self.l1.insert(*key, result.clone());
            return Some(result);
        }

        if let Some(result) = self.l3.get(key) {
            self.hits += 1;
            let result = result.clone();
            self.l1.insert(*key, result.clone());
            self.l2.put(*key, result.clone());
            return Some(result);
        }

        self.misses += 1;
        None
    }

    pub fn put(&mut self, key: u64, result: InferenceResult) {
        if self.l1.len() >= L1_CAPACITY {
            self.evict_l1();
        }
        self.l1.insert(key, result);
    }

    fn evict_l1(&mut self) {
        let keys: Vec<u64> = self.l1.keys().copied().collect();
        let evict_count = keys.len() / 4;

        for &key in keys.iter().take(evict_count) {
            if let Some((_, result)) = self.l1.remove_entry(&key) {
                self.l2.put(key, result);
            }
        }

        if self.l2.len() >= L2_CAPACITY {
            let l2_keys: Vec<u64> = { self.l2.iter().map(|(&k, _)| k).collect() };
            let evict_count = l2_keys.len() / 4;
            for key in l2_keys.iter().take(evict_count) {
                if let Some(result) = self.l2.pop(key) {
                    self.l3.put(*key, result);
                }
            }
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }

    pub fn clear(&mut self) {
        self.l1.clear();
        self.l2.clear();
        self.l3.clear();
        self.hits = 0;
        self.misses = 0;
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            l1_size: self.l1.len(),
            l2_size: self.l2.len(),
            l3_size: self.l3.len(),
            hits: self.hits,
            misses: self.misses,
            hit_rate: self.hit_rate(),
        }
    }
}

impl Default for InferenceCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub l1_size: usize,
    pub l2_size: usize,
    pub l3_size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::{InferenceMethod, InferenceResult};
    use crate::property_space::PropertyVector;

    fn make_result() -> InferenceResult {
        InferenceResult {
            property_vector: PropertyVector::new(),
            method: InferenceMethod::ExactMatch,
            confidence: ae_physics::fixed_point::FixedPoint::ONE,
            nearest_samples: vec![],
        }
    }

    #[test]
    fn test_cache_miss_then_hit() {
        let mut cache = InferenceCache::new();
        let key = 42u64;
        assert!(cache.get(&key).is_none());

        cache.put(key, make_result());
        assert!(cache.get(&key).is_some());
        assert!(cache.hit_rate() > 0.0);
    }

    #[test]
    fn test_l1_eviction_to_l2() {
        let mut cache = InferenceCache::new();
        for i in 0..300u64 {
            cache.put(i, make_result());
        }
        let stats = cache.stats();
        assert!(stats.l2_size > 0, "L2 should have evicted entries");
    }

    #[test]
    fn test_tiered_promotion() {
        let mut cache = InferenceCache::new();
        cache.put(1, make_result());
        assert!(cache.get(&1).is_some());

        for i in 2..300u64 {
            cache.put(i, make_result());
        }

        assert!(cache.get(&1).is_some());
    }
}
