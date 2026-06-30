use godot::prelude::*;

use ae_memory::arena::Arena;
use ae_memory::tracker;

#[derive(GodotClass)]
#[class(base=Node, rename=WastelandMemoryManager)]
pub(crate) struct WastelandMemoryManager {
    #[var]
    arena_chunk_size_kb: i64,
    #[var]
    pool_capacity: i64,
    #[var]
    tracking_enabled: bool,

    arena: Option<Arena>,
    total_allocated: i64,
    peak_allocated: i64,
    allocation_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandMemoryManager {
    fn init(base: Base<Node>) -> Self {
        let chunk_size = 1024 * 1024;
        Self {
            arena_chunk_size_kb: 1024,
            pool_capacity: 1000,
            tracking_enabled: true,
            arena: Some(Arena::new(chunk_size)),
            total_allocated: 0,
            peak_allocated: 0,
            allocation_count: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandMemoryManager {
    #[func]
    fn reset_arena(&mut self, chunk_size_kb: i64) {
        let chunk_size = (chunk_size_kb * 1024) as usize;
        self.arena = Some(Arena::new(chunk_size.max(4096)));
        self.arena_chunk_size_kb = chunk_size_kb;
        self.total_allocated = 0;
        self.allocation_count = 0;
    }

    #[func]
    fn get_arena_stats(&self) -> Dictionary<Variant, Variant> {
        if let Some(ref arena) = self.arena {
            let (used, allocated) = arena.usage();
            dict! {
                "total_allocated_kb" => (allocated / 1024) as i64,
                "total_used_kb" => (used / 1024) as i64,
                "waste_kb" => ((allocated - used) / 1024) as i64,
                "chunk_size_kb" => self.arena_chunk_size_kb,
            }
        } else {
            dict! {
                "total_allocated_kb" => 0,
                "total_used_kb" => 0,
                "waste_kb" => 0,
            }
        }
    }

    #[func]
    fn record_allocation(&mut self, size_bytes: i64) {
        if self.tracking_enabled {
            self.total_allocated += size_bytes;
            self.allocation_count += 1;
            if self.total_allocated > self.peak_allocated {
                self.peak_allocated = self.total_allocated;
            }
            tracker::track_alloc(size_bytes as usize);
        }
    }

    #[func]
    fn record_deallocation(&mut self, size_bytes: i64) {
        if self.tracking_enabled {
            self.total_allocated = (self.total_allocated - size_bytes).max(0);
            tracker::track_dealloc(size_bytes as usize);
        }
    }

    #[func]
    fn get_memory_stats(&self) -> Dictionary<Variant, Variant> {
        let stats = tracker::global_stats();
        dict! {
            "total_allocated_mb" => self.total_allocated as f32 / 1048576.0,
            "peak_allocated_mb" => self.peak_allocated as f32 / 1048576.0,
            "allocation_count" => self.allocation_count,
            "current_allocations" => stats.total_allocations as i64,
            "leak_suspects" => (stats.total_allocations.saturating_sub(stats.total_deallocations)) as i64,
        }
    }

    #[func]
    fn check_memory_budget(&self, budget_mb: f32) -> bool {
        let current_mb = self.total_allocated as f32 / 1048576.0;
        current_mb <= budget_mb
    }

    #[func]
    fn reset_tracking(&mut self) {
        self.total_allocated = 0;
        self.peak_allocated = 0;
        self.allocation_count = 0;
        tracker::reset_global_stats();
    }
}
