use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct CacheEntry {
    data: Vec<u8>,
    size: usize,
    last_access: Instant,
    access_seq: u64,
    access_count: u64,
    pinned: bool,
}

#[derive(Debug, Clone)]
pub struct AssetCache {
    entries: HashMap<u64, CacheEntry>,
    max_size: usize,
    current_size: usize,
    #[allow(dead_code)]
    default_ttl: Duration,
    eviction_policy: EvictionPolicy,
    access_counter: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    Lru,
    Lfu,
    Fifo,
    SizeBased,
}

impl AssetCache {
    pub fn new(max_size: usize) -> Self {
        AssetCache {
            entries: HashMap::new(),
            max_size,
            current_size: 0,
            default_ttl: Duration::from_secs(300),
            eviction_policy: EvictionPolicy::Lru,
            access_counter: 0,
        }
    }

    pub fn set_eviction_policy(&mut self, policy: EvictionPolicy) {
        self.eviction_policy = policy;
    }

    pub fn insert(&mut self, id: u64, data: Vec<u8>) -> bool {
        let size = data.len();
        if size > self.max_size {
            return false;
        }
        while self.current_size + size > self.max_size {
            if self.evict_one().is_none() {
                return false;
            }
        }
        self.current_size += size;
        self.access_counter += 1;
        self.entries.insert(
            id,
            CacheEntry {
                data,
                size,
                last_access: Instant::now(),
                access_seq: self.access_counter,
                access_count: 1,
                pinned: false,
            },
        );
        true
    }

    pub fn get(&mut self, id: u64) -> Option<&[u8]> {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.last_access = Instant::now();
            self.access_counter += 1;
            entry.access_seq = self.access_counter;
            entry.access_count += 1;
            Some(&entry.data)
        } else {
            None
        }
    }

    pub fn remove(&mut self, id: u64) -> Option<Vec<u8>> {
        if let Some(entry) = self.entries.remove(&id) {
            self.current_size -= entry.size;
            Some(entry.data)
        } else {
            None
        }
    }

    pub fn pin(&mut self, id: u64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.pinned = true;
        }
    }

    pub fn unpin(&mut self, id: u64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.pinned = false;
        }
    }

    pub fn contains(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    fn evict_one(&mut self) -> Option<u64> {
        let candidates: Vec<u64> =
            self.entries.iter().filter(|(_, e)| !e.pinned).map(|(k, _)| *k).collect();
        if candidates.is_empty() {
            return None;
        }
        let victim = match self.eviction_policy {
            EvictionPolicy::Lru => {
                candidates.into_iter().min_by_key(|k| self.entries[k].access_seq)
            },
            EvictionPolicy::Lfu => {
                candidates.into_iter().min_by_key(|k| self.entries[k].access_count)
            },
            EvictionPolicy::Fifo => candidates.into_iter().next(),
            EvictionPolicy::SizeBased => {
                candidates.into_iter().max_by_key(|k| self.entries[k].size)
            },
        };
        if let Some(id) = victim {
            if let Some(entry) = self.entries.remove(&id) {
                self.current_size -= entry.size;
            }
        }
        victim
    }

    pub fn evict_expired(&mut self, ttl: Duration) -> usize {
        let now = Instant::now();
        let expired: Vec<u64> = self
            .entries
            .iter()
            .filter(|(_, e)| !e.pinned && now.duration_since(e.last_access) > ttl)
            .map(|(k, _)| *k)
            .collect();
        let count = expired.len();
        for id in expired {
            self.remove(id);
        }
        count
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_size = 0;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn size(&self) -> usize {
        self.current_size
    }

    pub fn hit_rate(&self) -> (u64, u64) {
        let hits: u64 = self.entries.values().map(|e| e.access_count.saturating_sub(1)).sum();
        let total = hits + self.entries.len() as u64;
        (hits, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut cache = AssetCache::new(1024 * 1024);
        assert!(cache.insert(1, vec![1, 2, 3]));
        assert_eq!(cache.get(1).unwrap(), &[1, 2, 3]);
    }

    #[test]
    fn test_eviction_on_full() {
        let mut cache = AssetCache::new(100);
        cache.insert(1, vec![0u8; 60]);
        cache.insert(2, vec![0u8; 50]);
        assert!(cache.contains(1) || cache.contains(2));
        assert!(cache.size() <= 100);
    }

    #[test]
    fn test_pin_prevents_eviction() {
        let mut cache = AssetCache::new(100);
        cache.insert(1, vec![0u8; 60]);
        cache.pin(1);
        cache.insert(2, vec![0u8; 60]);
        assert!(cache.contains(1));
    }

    #[test]
    fn test_remove() {
        let mut cache = AssetCache::new(1024);
        cache.insert(1, vec![0u8; 100]);
        assert_eq!(cache.size(), 100);
        cache.remove(1);
        assert_eq!(cache.size(), 0);
        assert!(!cache.contains(1));
    }

    #[test]
    fn test_clear() {
        let mut cache = AssetCache::new(1024);
        cache.insert(1, vec![0u8; 100]);
        cache.insert(2, vec![0u8; 200]);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_eviction_policy_lru() {
        let mut cache = AssetCache::new(100);
        cache.set_eviction_policy(EvictionPolicy::Lru);
        cache.insert(1, vec![0u8; 40]);
        cache.insert(2, vec![0u8; 40]);
        cache.get(1);
        cache.insert(3, vec![0u8; 40]);
        assert!(cache.contains(1));
        assert!(!cache.contains(2) || !cache.contains(3));
    }
}
