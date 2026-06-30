use std::sync::atomic::{AtomicUsize, Ordering};

static GLOBAL_ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_DEALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

pub fn track_alloc(size: usize) {
    GLOBAL_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    GLOBAL_ALLOC_BYTES.fetch_add(size, Ordering::Relaxed);
}

pub fn track_dealloc(size: usize) {
    GLOBAL_DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    GLOBAL_DEALLOC_BYTES.fetch_add(size, Ordering::Relaxed);
}

pub fn global_stats() -> MemoryStats {
    MemoryStats {
        total_allocations: GLOBAL_ALLOC_COUNT.load(Ordering::Relaxed),
        total_bytes_allocated: GLOBAL_ALLOC_BYTES.load(Ordering::Relaxed),
        total_deallocations: GLOBAL_DEALLOC_COUNT.load(Ordering::Relaxed),
        total_bytes_deallocated: GLOBAL_DEALLOC_BYTES.load(Ordering::Relaxed),
        current_bytes: GLOBAL_ALLOC_BYTES
            .load(Ordering::Relaxed)
            .saturating_sub(GLOBAL_DEALLOC_BYTES.load(Ordering::Relaxed)),
    }
}

pub fn reset_global_stats() {
    GLOBAL_ALLOC_COUNT.store(0, Ordering::Relaxed);
    GLOBAL_ALLOC_BYTES.store(0, Ordering::Relaxed);
    GLOBAL_DEALLOC_COUNT.store(0, Ordering::Relaxed);
    GLOBAL_DEALLOC_BYTES.store(0, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_allocations: usize,
    pub total_bytes_allocated: usize,
    pub total_deallocations: usize,
    pub total_bytes_deallocated: usize,
    pub current_bytes: usize,
}

impl MemoryStats {
    pub fn leak_bytes(&self) -> usize {
        self.total_bytes_allocated.saturating_sub(self.total_bytes_deallocated)
    }
}

pub struct TrackedArena<T> {
    data: Vec<T>,
    label: String,
    alloc_count: usize,
    total_item_bytes: usize,
}

impl<T> TrackedArena<T> {
    pub fn new(label: &str) -> Self {
        TrackedArena {
            data: Vec::new(),
            label: label.to_string(),
            alloc_count: 0,
            total_item_bytes: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        self.alloc_count += 1;
        self.total_item_bytes += std::mem::size_of::<T>();
        self.data.push(item);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn stats(&self) -> CategoryStats {
        CategoryStats {
            label: self.label.clone(),
            item_count: self.data.len(),
            alloc_count: self.alloc_count,
            total_bytes: self.total_item_bytes,
            type_name: std::any::type_name::<T>().to_string(),
        }
    }
}

impl<T> std::ops::Deref for TrackedArena<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> {
        &self.data
    }
}

impl<T> std::ops::DerefMut for TrackedArena<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.data
    }
}

#[derive(Debug, Clone)]
pub struct CategoryStats {
    pub label: String,
    pub item_count: usize,
    pub alloc_count: usize,
    pub total_bytes: usize,
    pub type_name: String,
}

pub struct MemoryBudget {
    limit: usize,
    current: AtomicUsize,
}

impl MemoryBudget {
    pub fn new(limit: usize) -> Self {
        MemoryBudget { limit, current: AtomicUsize::new(0) }
    }

    pub fn try_reserve(&self, size: usize) -> bool {
        let current = self.current.load(Ordering::Relaxed);
        if current + size > self.limit {
            return false;
        }
        self.current.fetch_add(size, Ordering::Relaxed);
        true
    }

    pub fn release(&self, size: usize) {
        self.current.fetch_sub(size, Ordering::Relaxed);
    }

    pub fn used(&self) -> usize {
        self.current.load(Ordering::Relaxed)
    }

    pub fn remaining(&self) -> usize {
        self.limit.saturating_sub(self.used())
    }

    pub fn usage_ratio(&self) -> f32 {
        self.used() as f32 / self.limit.max(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_stats() {
        reset_global_stats();
        track_alloc(1024);
        track_alloc(512);
        track_dealloc(256);
        let stats = global_stats();
        assert_eq!(stats.total_allocations, 2);
        assert_eq!(stats.total_bytes_allocated, 1536);
        assert_eq!(stats.total_deallocations, 1);
        assert_eq!(stats.total_bytes_deallocated, 256);
        assert_eq!(stats.current_bytes, 1280);
    }

    #[test]
    fn test_tracked_arena() {
        let mut arena: TrackedArena<u64> = TrackedArena::new("test_entities");
        arena.push(1);
        arena.push(2);
        arena.push(3);
        let stats = arena.stats();
        assert_eq!(stats.item_count, 3);
        assert_eq!(stats.alloc_count, 3);
        assert_eq!(stats.label, "test_entities");
    }

    #[test]
    fn test_memory_budget() {
        let budget = MemoryBudget::new(1024);
        assert!(budget.try_reserve(512));
        assert!(budget.try_reserve(256));
        assert_eq!(budget.used(), 768);
        assert!(!budget.try_reserve(512));
        budget.release(256);
        assert_eq!(budget.used(), 512);
        assert!(budget.try_reserve(512));
    }
}
