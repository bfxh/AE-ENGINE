use std::alloc::{Layout, alloc, dealloc};
use std::ptr::NonNull;

const DEFAULT_BLOCK_SIZE: usize = 64 * 1024 * 1024;
const MAX_BLOCK_COUNT: usize = 16;

pub struct AiMemoryPool {
    blocks: Vec<MemoryBlock>,
    block_size: usize,
    total_allocated: usize,
    total_used: usize,
    peak_used: usize,
}

struct MemoryBlock {
    ptr: NonNull<u8>,
    size: usize,
    offset: usize,
}

impl MemoryBlock {
    fn new(size: usize) -> Option<Self> {
        let layout = Layout::from_size_align(size, 64).ok()?;
        let ptr = unsafe { alloc(layout) };
        NonNull::new(ptr).map(|ptr| MemoryBlock { ptr, size, offset: 0 })
    }

    fn remaining(&self) -> usize {
        self.size.saturating_sub(self.offset)
    }
}

impl Drop for MemoryBlock {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.size, 64).unwrap();
        unsafe {
            dealloc(self.ptr.as_ptr(), layout);
        }
    }
}

unsafe impl Send for MemoryBlock {}
unsafe impl Sync for MemoryBlock {}

impl AiMemoryPool {
    pub fn new() -> Self {
        AiMemoryPool {
            blocks: Vec::with_capacity(4),
            block_size: DEFAULT_BLOCK_SIZE,
            total_allocated: 0,
            total_used: 0,
            peak_used: 0,
        }
    }

    pub fn with_block_size(block_size: usize) -> Self {
        AiMemoryPool {
            blocks: Vec::with_capacity(4),
            block_size,
            total_allocated: 0,
            total_used: 0,
            peak_used: 0,
        }
    }

    pub fn allocate(&mut self, size: usize) -> Option<NonNull<u8>> {
        let aligned_size = (size + 63) & !63;
        for block in &mut self.blocks {
            if block.remaining() >= aligned_size {
                let ptr = unsafe { block.ptr.as_ptr().add(block.offset) };
                block.offset += aligned_size;
                self.total_used += aligned_size;
                self.peak_used = self.peak_used.max(self.total_used);
                return NonNull::new(ptr);
            }
        }
        if self.blocks.len() >= MAX_BLOCK_COUNT {
            return None;
        }
        let block_size = self.block_size.max(aligned_size);
        let mut block = MemoryBlock::new(block_size)?;
        let ptr = unsafe { block.ptr.as_ptr().add(block.offset) };
        block.offset += aligned_size;
        self.total_allocated += block_size;
        self.total_used += aligned_size;
        self.peak_used = self.peak_used.max(self.total_used);
        self.blocks.push(block);
        NonNull::new(ptr)
    }

    pub fn allocate_array<T>(&mut self, count: usize) -> Option<NonNull<T>> {
        let size = count * std::mem::size_of::<T>();
        self.allocate(size).map(|ptr| ptr.cast())
    }

    pub fn allocate_slice<T: Copy>(&mut self, data: &[T]) -> Option<&mut [T]> {
        let ptr: NonNull<T> = self.allocate_array(data.len())?;
        unsafe {
            let slice = std::slice::from_raw_parts_mut(ptr.as_ptr(), data.len());
            slice.copy_from_slice(data);
            Some(slice)
        }
    }

    pub fn reset(&mut self) {
        for block in &mut self.blocks {
            block.offset = 0;
        }
        self.total_used = 0;
    }

    pub fn compact(&mut self) {
        self.blocks.retain(|block| block.offset > 0);
    }

    pub fn shrink_to_fit(&mut self) {
        self.blocks.clear();
        self.total_allocated = 0;
        self.total_used = 0;
    }

    pub fn stats(&self) -> PoolStats {
        PoolStats {
            block_count: self.blocks.len(),
            total_allocated: self.total_allocated,
            total_used: self.total_used,
            peak_used: self.peak_used,
            waste: self.total_allocated.saturating_sub(self.total_used),
        }
    }

    pub fn usage_percent(&self) -> f64 {
        if self.total_allocated == 0 {
            0.0
        } else {
            self.total_used as f64 / self.total_allocated as f64 * 100.0
        }
    }
}

impl Default for AiMemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub block_count: usize,
    pub total_allocated: usize,
    pub total_used: usize,
    pub peak_used: usize,
    pub waste: usize,
}

pub struct ScratchBuffer {
    pub data: Vec<f32>,
    capacity: usize,
}

impl ScratchBuffer {
    pub fn new(capacity: usize) -> Self {
        ScratchBuffer { data: vec![0.0f32; capacity], capacity }
    }

    pub fn ensure(&mut self, size: usize) -> &mut [f32] {
        if size > self.data.len() {
            self.data.resize(size, 0.0);
            self.capacity = size;
        }
        &mut self.data[..size]
    }

    pub fn fill(&mut self, value: f32) {
        self.data.fill(value);
    }

    pub fn zero(&mut self) {
        self.data.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let mut pool = AiMemoryPool::with_block_size(1024);
        let ptr = pool.allocate(256);
        assert!(ptr.is_some());
        let stats = pool.stats();
        assert_eq!(stats.total_used, 256);
    }

    #[test]
    fn test_multiple_allocations_same_block() {
        let mut pool = AiMemoryPool::with_block_size(4096);
        let _a = pool.allocate(1024);
        let _b = pool.allocate(1024);
        let _c = pool.allocate(1024);
        let stats = pool.stats();
        assert_eq!(stats.block_count, 1);
        assert_eq!(stats.total_used, 3072);
    }

    #[test]
    fn test_new_block_when_full() {
        let mut pool = AiMemoryPool::with_block_size(1024);
        let _a = pool.allocate(1000);
        let _b = pool.allocate(1000);
        let stats = pool.stats();
        assert!(stats.block_count >= 2);
    }

    #[test]
    fn test_alignment() {
        let mut pool = AiMemoryPool::new();
        let ptr = pool.allocate(100).unwrap();
        assert_eq!(ptr.as_ptr() as usize % 64, 0);
    }

    #[test]
    fn test_reset() {
        let mut pool = AiMemoryPool::with_block_size(1024);
        let _a = pool.allocate(500);
        pool.reset();
        assert_eq!(pool.stats().total_used, 0);
        let _b = pool.allocate(500);
        assert!(pool.stats().total_used > 0);
    }

    #[test]
    fn test_scratch_buffer() {
        let mut buf = ScratchBuffer::new(256);
        let slice = buf.ensure(512);
        assert_eq!(slice.len(), 512);
        buf.zero();
        for &val in buf.data.iter().take(512) {
            assert_eq!(val, 0.0);
        }
    }

    #[test]
    fn test_usage_percent() {
        let mut pool = AiMemoryPool::with_block_size(2048);
        pool.allocate(1024);
        let pct = pool.usage_percent();
        assert!(pct > 40.0);
    }

    #[test]
    fn test_max_blocks() {
        let mut pool = AiMemoryPool::with_block_size(256);
        for i in 0..MAX_BLOCK_COUNT {
            let ptr = pool.allocate(200);
            assert!(ptr.is_some(), "allocation {} failed", i);
        }
        let ptr = pool.allocate(200);
        assert!(ptr.is_none());
    }
}
