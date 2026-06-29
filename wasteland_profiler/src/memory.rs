use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

pub fn track_alloc(size: usize) {
    ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    ALLOC_BYTES.fetch_add(size, Ordering::Relaxed);
}

pub fn track_dealloc(size: usize) {
    DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    DEALLOC_BYTES.fetch_add(size, Ordering::Relaxed);
}

#[derive(Debug, Clone, Default)]
pub struct MemorySnapshot {
    pub alloc_count: usize,
    pub alloc_bytes: usize,
    pub dealloc_count: usize,
    pub dealloc_bytes: usize,
    pub current_bytes: usize,
    pub peak_bytes: usize,
}

pub struct MemoryTracker {
    snapshots: Vec<MemorySnapshot>,
    peak: usize,
}

impl MemoryTracker {
    pub fn new() -> Self {
        MemoryTracker { snapshots: Vec::new(), peak: 0 }
    }

    pub fn snapshot(&mut self) -> MemorySnapshot {
        let alloc_count = ALLOC_COUNT.load(Ordering::Relaxed);
        let alloc_bytes = ALLOC_BYTES.load(Ordering::Relaxed);
        let dealloc_count = DEALLOC_COUNT.load(Ordering::Relaxed);
        let dealloc_bytes = DEALLOC_BYTES.load(Ordering::Relaxed);
        let current = alloc_bytes.saturating_sub(dealloc_bytes);
        if current > self.peak {
            self.peak = current;
        }
        let snap = MemorySnapshot {
            alloc_count,
            alloc_bytes,
            dealloc_count,
            dealloc_bytes,
            current_bytes: current,
            peak_bytes: self.peak,
        };
        self.snapshots.push(snap.clone());
        snap
    }

    pub fn leak_check(&self) -> Option<usize> {
        if let Some(last) = self.snapshots.last() {
            let leaked = last.alloc_bytes.saturating_sub(last.dealloc_bytes);
            if leaked > 0 {
                return Some(leaked);
            }
        }
        None
    }

    pub fn reset_counters() {
        ALLOC_COUNT.store(0, Ordering::Relaxed);
        ALLOC_BYTES.store(0, Ordering::Relaxed);
        DEALLOC_COUNT.store(0, Ordering::Relaxed);
        DEALLOC_BYTES.store(0, Ordering::Relaxed);
    }

    pub fn history(&self) -> &[MemorySnapshot] {
        &self.snapshots
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct BudgetTracker {
    budgets: HashMap<String, usize>,
    usage: HashMap<String, usize>,
    limits: HashMap<String, usize>,
}

impl BudgetTracker {
    pub fn new() -> Self {
        BudgetTracker { budgets: HashMap::new(), usage: HashMap::new(), limits: HashMap::new() }
    }

    pub fn register_budget(&mut self, name: &str, limit_bytes: usize) {
        self.budgets.insert(name.to_string(), 0);
        self.limits.insert(name.to_string(), limit_bytes);
        self.usage.insert(name.to_string(), 0);
    }

    pub fn allocate(&mut self, name: &str, bytes: usize) -> bool {
        if let (Some(usage), Some(limit)) = (self.usage.get_mut(name), self.limits.get(name)) {
            if *usage + bytes > *limit {
                return false;
            }
            *usage += bytes;
            return true;
        }
        false
    }

    pub fn deallocate(&mut self, name: &str, bytes: usize) {
        if let Some(usage) = self.usage.get_mut(name) {
            *usage = usage.saturating_sub(bytes);
        }
    }

    pub fn usage_percent(&self, name: &str) -> Option<f32> {
        if let (Some(usage), Some(limit)) = (self.usage.get(name), self.limits.get(name)) {
            if *limit == 0 {
                return Some(0.0);
            }
            return Some(*usage as f32 / *limit as f32 * 100.0);
        }
        None
    }

    pub fn all_budgets(&self) -> Vec<(&str, usize, usize, f32)> {
        let mut result: Vec<_> = self
            .budgets
            .keys()
            .filter_map(|name| {
                let usage = *self.usage.get(name)?;
                let limit = *self.limits.get(name)?;
                let pct = if limit == 0 { 0.0 } else { usage as f32 / limit as f32 * 100.0 };
                Some((name.as_str(), usage, limit, pct))
            })
            .collect();
        result.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        result
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracker_snapshot() {
        MemoryTracker::reset_counters();
        let mut tracker = MemoryTracker::new();
        track_alloc(1024);
        track_alloc(2048);
        track_dealloc(1024);
        let snap = tracker.snapshot();
        assert_eq!(snap.alloc_bytes, 3072);
        assert_eq!(snap.dealloc_bytes, 1024);
        assert_eq!(snap.current_bytes, 2048);
    }

    #[test]
    fn test_leak_detection() {
        MemoryTracker::reset_counters();
        let mut tracker = MemoryTracker::new();
        track_alloc(100);
        tracker.snapshot();
        assert_eq!(tracker.leak_check(), Some(100));
    }

    #[test]
    fn test_budget_tracker() {
        let mut bt = BudgetTracker::new();
        bt.register_budget("physics", 1024);
        bt.register_budget("render", 2048);
        assert!(bt.allocate("physics", 500));
        assert!(bt.allocate("physics", 500));
        assert!(!bt.allocate("physics", 100));
        bt.deallocate("physics", 300);
        assert!(bt.allocate("physics", 100));
    }

    #[test]
    fn test_budget_usage_percent() {
        let mut bt = BudgetTracker::new();
        bt.register_budget("test", 1000);
        bt.allocate("test", 250);
        let pct = bt.usage_percent("test").unwrap();
        assert!((pct - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_budget_all_budgets() {
        let mut bt = BudgetTracker::new();
        bt.register_budget("a", 100);
        bt.register_budget("b", 200);
        bt.allocate("a", 80);
        bt.allocate("b", 20);
        let all = bt.all_budgets();
        assert_eq!(all.len(), 2);
        assert!(all[0].3 > all[1].3);
    }
}
