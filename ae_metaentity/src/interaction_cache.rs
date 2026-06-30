use hashbrown::HashMap;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;

use crate::interaction::InteractionResult;
use crate::meta_entity::MetaEntity;

/// 三级哈希索引缓存键
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionKey {
    pub category_a: u64,
    pub category_b: u64,
    pub distance_band: u8,
    pub element_hash: u64,
    pub bond_hash: u64,
    pub reactivity_band: u8,
}

impl InteractionKey {
    pub fn new(a: &MetaEntity, b: &MetaEntity, distance: f32) -> Self {
        let category_a = Self::compute_category_hash(&a.physics, &a.chemistry);
        let category_b = Self::compute_category_hash(&b.physics, &b.chemistry);
        let distance_band = (distance * 10.0).min(255.0) as u8;
        let element_hash = Self::compute_element_hash(&a.chemistry, &b.chemistry);
        let bond_hash = Self::compute_bond_hash(&a.chemistry, &b.chemistry);
        let reactivity_band =
            ((a.chemistry.reactivity * b.chemistry.reactivity).sqrt() * 10.0).min(255.0) as u8;

        Self { category_a, category_b, distance_band, element_hash, bond_hash, reactivity_band }
    }

    fn compute_category_hash(
        physics: &crate::meta_entity::PhysicsAttributes,
        chemistry: &crate::meta_entity::ChemistryAttributes,
    ) -> u64 {
        let mut hash: u64 = 0;
        hash ^= (physics.density as u64).wrapping_mul(0x9E3779B97F4A7C15);
        hash ^= (physics.hardness as u64).wrapping_mul(0xC6A4A7935BD1E995);
        hash ^= (chemistry.ph as u64).wrapping_mul(0xBF58476D1CE4E5B9);
        hash ^= (chemistry.reactivity as u64).wrapping_mul(0x94D049BB133111EB);
        hash
    }

    fn compute_element_hash(
        chem_a: &crate::meta_entity::ChemistryAttributes,
        chem_b: &crate::meta_entity::ChemistryAttributes,
    ) -> u64 {
        let mut hash: u64 = 0;
        for e in &chem_a.elemental_composition {
            hash ^= (e.element as u64).wrapping_mul(0x517CC1B727220A95);
        }
        for e in &chem_b.elemental_composition {
            hash ^= (e.element as u64).wrapping_mul(0x517CC1B727220A95);
        }
        hash
    }

    fn compute_bond_hash(
        chem_a: &crate::meta_entity::ChemistryAttributes,
        chem_b: &crate::meta_entity::ChemistryAttributes,
    ) -> u64 {
        let mut hash: u64 = 0;
        for b in &chem_a.bond_types {
            hash ^= (*b as u64).wrapping_mul(0x27D4EB2F165667C5);
        }
        for b in &chem_b.bond_types {
            hash ^= (*b as u64).wrapping_mul(0x27D4EB2F165667C5);
        }
        hash
    }
}

/// 三级哈希索引缓存
///
/// 层级结构：
///   Level 1: 精确匹配 — 完全相同的 InteractionKey
///   Level 2: 模糊分类 — 相同 category_a, category_b, distance_band
///   Level 3: 条件匹配 — 相同 element_hash, bond_hash
#[derive(Debug)]
pub struct InteractionCache {
    /// Level 1: 精确匹配缓存
    level1: LruCache<InteractionKey, CachedResult>,
    /// Level 2: 模糊分类缓存
    level2: HashMap<(u64, u64, u8), Vec<CachedResult>>,
    /// Level 3: 条件匹配缓存
    level3: HashMap<(u64, u64), Vec<CachedResult>>,
    /// 布隆过滤器预筛
    bloom: BloomFilter,
    /// 统计
    hits: CacheHits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    pub key: InteractionKey,
    pub force_magnitude: f32,
    pub heat: f32,
    pub has_chemical_changes: bool,
    pub has_biological_changes: bool,
    pub has_generated_entities: bool,
    pub interaction_type: u8,
    pub hit_count: u64,
    pub last_access_tick: u64,
}

#[derive(Debug)]
struct BloomFilter {
    bits: Vec<u64>,
    size: usize,
    hash_count: usize,
}

impl BloomFilter {
    fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let size = (-(expected_items as f64) * false_positive_rate.ln() / (2.0f64.ln().powi(2)))
            .ceil() as usize;
        let hash_count = ((size as f64 / expected_items as f64) * 2.0f64.ln()).ceil() as usize;
        let bits = vec![0u64; size.div_ceil(64)];
        Self { bits, size, hash_count }
    }

    fn insert(&mut self, key: &InteractionKey) {
        let h = Self::hash(key);
        for i in 0..self.hash_count {
            let idx =
                (h.wrapping_add((i as u64).wrapping_mul(0x9E3779B97F4A7C15)) as usize) % self.size;
            self.bits[idx / 64] |= 1 << (idx % 64);
        }
    }

    fn contains(&self, key: &InteractionKey) -> bool {
        let h = Self::hash(key);
        for i in 0..self.hash_count {
            let idx =
                (h.wrapping_add((i as u64).wrapping_mul(0x9E3779B97F4A7C15)) as usize) % self.size;
            if self.bits[idx / 64] & (1 << (idx % 64)) == 0 {
                return false;
            }
        }
        true
    }

    fn hash(key: &InteractionKey) -> u64 {
        key.category_a
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add(key.category_b.wrapping_mul(0xC6A4A7935BD1E995))
            .wrapping_add((key.distance_band as u64).wrapping_mul(0xBF58476D1CE4E5B9))
    }
}

#[derive(Debug, Default)]
struct CacheHits {
    level1_hits: u64,
    level2_hits: u64,
    level3_hits: u64,
    misses: u64,
    bloom_rejects: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub level1_hits: u64,
    pub level2_hits: u64,
    pub level3_hits: u64,
    pub misses: u64,
    pub bloom_rejects: u64,
    pub level1_size: usize,
    pub level2_entries: usize,
    pub level3_entries: usize,
    pub total_hit_rate: f64,
}

impl InteractionCache {
    pub fn new(level1_capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(level1_capacity.max(1)).unwrap();
        Self {
            level1: LruCache::new(capacity),
            level2: HashMap::new(),
            level3: HashMap::new(),
            bloom: BloomFilter::new(10000, 0.01),
            hits: CacheHits::default(),
        }
    }

    /// 查询缓存：三级回退
    pub fn lookup(&mut self, key: &InteractionKey, tick: u64) -> Option<CachedResult> {
        if !self.bloom.contains(key) {
            self.hits.bloom_rejects += 1;
            return None;
        }

        // Level 1: 精确匹配
        if let Some(result) = self.level1.get(key) {
            self.hits.level1_hits += 1;
            let mut result = result.clone();
            result.hit_count += 1;
            result.last_access_tick = tick;
            return Some(result);
        }

        // Level 2: 模糊分类
        let level2_key = (key.category_a, key.category_b, key.distance_band);
        if let Some(results) = self.level2.get(&level2_key) {
            for result in results {
                if result.key.element_hash == key.element_hash {
                    self.hits.level2_hits += 1;
                    let mut result = result.clone();
                    result.hit_count += 1;
                    result.last_access_tick = tick;
                    self.level1.put(key.clone(), result.clone());
                    return Some(result);
                }
            }
        }

        // Level 3: 条件匹配
        let level3_key = (key.element_hash, key.bond_hash);
        if let Some(results) = self.level3.get(&level3_key) {
            for result in results {
                if result.key.reactivity_band == key.reactivity_band {
                    self.hits.level3_hits += 1;
                    let mut result = result.clone();
                    result.hit_count += 1;
                    result.last_access_tick = tick;
                    self.level1.put(key.clone(), result.clone());
                    return Some(result);
                }
            }
        }

        self.hits.misses += 1;
        None
    }

    /// 插入缓存结果
    pub fn insert(&mut self, key: InteractionKey, result: &InteractionResult, tick: u64) {
        let cached = CachedResult {
            key: key.clone(),
            force_magnitude: result.force_on_a.length(),
            heat: result.heat_released,
            has_chemical_changes: !result.attribute_changes_a.is_empty()
                || !result.attribute_changes_b.is_empty(),
            has_biological_changes: false,
            has_generated_entities: !result.generated_entities.is_empty(),
            interaction_type: result.interaction_type as u8,
            hit_count: 1,
            last_access_tick: tick,
        };

        self.bloom.insert(&key);

        self.level1.put(key.clone(), cached.clone());

        let level2_key = (key.category_a, key.category_b, key.distance_band);
        self.level2.entry(level2_key).or_default().push(cached.clone());

        let level3_key = (key.element_hash, key.bond_hash);
        self.level3.entry(level3_key).or_default().push(cached);

        // 限制 Level 2/3 大小
        if self.level2.len() > 5000 {
            self.prune_level2(tick);
        }
        if self.level3.len() > 5000 {
            self.prune_level3(tick);
        }
    }

    fn prune_level2(&mut self, current_tick: u64) {
        let threshold = current_tick.saturating_sub(10000);
        self.level2.retain(|_, results| {
            results.retain(|r| r.last_access_tick > threshold);
            !results.is_empty()
        });
    }

    fn prune_level3(&mut self, current_tick: u64) {
        let threshold = current_tick.saturating_sub(10000);
        self.level3.retain(|_, results| {
            results.retain(|r| r.last_access_tick > threshold);
            !results.is_empty()
        });
    }

    pub fn stats(&self) -> CacheStats {
        let total = self.hits.level1_hits
            + self.hits.level2_hits
            + self.hits.level3_hits
            + self.hits.misses;
        let total_hits = self.hits.level1_hits + self.hits.level2_hits + self.hits.level3_hits;
        CacheStats {
            level1_hits: self.hits.level1_hits,
            level2_hits: self.hits.level2_hits,
            level3_hits: self.hits.level3_hits,
            misses: self.hits.misses,
            bloom_rejects: self.hits.bloom_rejects,
            level1_size: self.level1.len(),
            level2_entries: self.level2.len(),
            level3_entries: self.level3.len(),
            total_hit_rate: if total > 0 { total_hits as f64 / total as f64 } else { 0.0 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta_entity::*;

    #[test]
    fn test_cache_hit() {
        let mut cache = InteractionCache::new(100);
        let iron = MetaEntity::iron(glam::Vec3::ZERO, 0);
        let water = MetaEntity::water(glam::Vec3::new(0.5, 0.0, 0.0), 0);
        let key = InteractionKey::new(&iron, &water, 0.5);

        assert!(cache.lookup(&key, 0).is_none());
    }

    #[test]
    fn test_bloom_filter() {
        let mut bloom = BloomFilter::new(1000, 0.01);
        let iron = MetaEntity::iron(glam::Vec3::ZERO, 0);
        let water = MetaEntity::water(glam::Vec3::new(0.5, 0.0, 0.0), 0);
        let key = InteractionKey::new(&iron, &water, 0.5);

        bloom.insert(&key);
        assert!(bloom.contains(&key));
    }
}
