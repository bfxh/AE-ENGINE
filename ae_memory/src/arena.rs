use std::alloc::{Layout, alloc, dealloc};
use std::cell::UnsafeCell;
use std::ptr::NonNull;

pub struct Arena {
    chunks: Vec<Chunk>,
    current: Option<ChunkCursor>,
    chunk_size: usize,
    total_allocated: usize,
    total_used: usize,
}

struct Chunk {
    ptr: NonNull<u8>,
    size: usize,
}

struct ChunkCursor {
    chunk_idx: usize,
    offset: usize,
}

impl Arena {
    pub fn new(chunk_size: usize) -> Self {
        Arena {
            chunks: Vec::new(),
            current: None,
            chunk_size: chunk_size.max(4096),
            total_allocated: 0,
            total_used: 0,
        }
    }

    pub fn alloc<T>(&mut self, value: T) -> &mut T {
        let layout = Layout::new::<T>();
        let size = layout.size();
        let align = layout.align();

        let ptr = self.alloc_raw(size, align);
        unsafe {
            ptr.as_ptr().cast::<T>().write(value);
            &mut *ptr.as_ptr().cast::<T>()
        }
    }

    pub fn alloc_slice<T: Copy>(&mut self, values: &[T]) -> &mut [T] {
        let layout = Layout::array::<T>(values.len()).unwrap();
        let size = layout.size();
        let align = layout.align();

        let ptr = self.alloc_raw(size, align);
        unsafe {
            ptr.as_ptr().copy_from_nonoverlapping(values.as_ptr() as *const u8, size);
            std::slice::from_raw_parts_mut(ptr.as_ptr() as *mut T, values.len())
        }
    }

    pub fn alloc_layout(&mut self, layout: Layout) -> NonNull<u8> {
        self.alloc_raw(layout.size(), layout.align())
    }

    fn alloc_raw(&mut self, mut size: usize, align: usize) -> NonNull<u8> {
        if size == 0 {
            size = 1;
        }

        let cursor = match &mut self.current {
            Some(c) => c,
            None => {
                self.allocate_chunk(size, align);
                self.current.as_mut().unwrap()
            },
        };

        let chunk = &self.chunks[cursor.chunk_idx];
        let chunk_ptr = chunk.ptr;
        let offset = (cursor.offset + align - 1) & !(align - 1);

        if offset + size > chunk.size {
            self.allocate_chunk(size, align);
            let new_cursor = self.current.as_mut().unwrap();
            let new_chunk = &self.chunks[new_cursor.chunk_idx];
            let new_offset = (new_cursor.offset + align - 1) & !(align - 1);
            new_cursor.offset = new_offset + size;
            self.total_used += size;
            unsafe { NonNull::new_unchecked(new_chunk.ptr.as_ptr().add(new_offset)) }
        } else {
            cursor.offset = offset + size;
            self.total_used += size;
            unsafe { NonNull::new_unchecked(chunk_ptr.as_ptr().add(offset)) }
        }
    }

    fn allocate_chunk(&mut self, min_size: usize, _align: usize) {
        let size = min_size.max(self.chunk_size).next_power_of_two();
        let layout = Layout::from_size_align(size, 16).unwrap();
        let ptr = unsafe { alloc(layout) };
        let ptr = NonNull::new(ptr).unwrap_or_else(|| std::alloc::handle_alloc_error(layout));

        self.total_allocated += size;
        let idx = self.chunks.len();
        self.chunks.push(Chunk { ptr, size });
        self.current = Some(ChunkCursor { chunk_idx: idx, offset: 0 });
    }

    pub fn reset(&mut self) {
        self.current = None;
        self.total_used = 0;
    }

    pub fn usage(&self) -> (usize, usize) {
        (self.total_used, self.total_allocated)
    }

    pub fn wasted(&self) -> usize {
        self.total_allocated - self.total_used
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        for chunk in &self.chunks {
            let layout = Layout::from_size_align(chunk.size, 16).unwrap();
            unsafe { dealloc(chunk.ptr.as_ptr(), layout) };
        }
    }
}

pub struct StackArena {
    buffer: UnsafeCell<Vec<u8>>,
    offset: UnsafeCell<usize>,
}

impl StackArena {
    pub fn new(capacity: usize) -> Self {
        StackArena { buffer: UnsafeCell::new(vec![0u8; capacity]), offset: UnsafeCell::new(0) }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        let layout = Layout::new::<T>();
        let offset = unsafe { &mut *self.offset.get() };
        let buffer = unsafe { &mut *self.buffer.get() };

        let aligned = (*offset + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();

        if end > buffer.len() {
            buffer.resize(end * 2, 0);
        }

        unsafe {
            let ptr = buffer.as_mut_ptr().add(aligned) as *mut T;
            ptr.write(value);
            *offset = end;
            &mut *ptr
        }
    }

    pub fn reset(&self) {
        unsafe {
            *self.offset.get() = 0;
        }
    }

    pub fn used(&self) -> usize {
        unsafe { *self.offset.get() }
    }
}

unsafe impl Send for StackArena {}
unsafe impl Sync for StackArena {}

pub struct FrameArena {
    inner: UnsafeCell<FrameArenaInner>,
}

struct FrameArenaInner {
    buffer: Vec<u8>,
    offset: usize,
    checkpoints: Vec<usize>,
}

impl FrameArena {
    pub fn new(capacity: usize) -> Self {
        FrameArena {
            inner: UnsafeCell::new(FrameArenaInner {
                buffer: vec![0u8; capacity],
                offset: 0,
                checkpoints: Vec::new(),
            }),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        let inner = unsafe { &mut *self.inner.get() };
        let layout = Layout::new::<T>();
        let aligned = (inner.offset + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();

        if end > inner.buffer.len() {
            inner.buffer.resize(end * 2, 0);
        }

        unsafe {
            let ptr = inner.buffer.as_mut_ptr().add(aligned) as *mut T;
            ptr.write(value);
            inner.offset = end;
            &mut *ptr
        }
    }

    pub fn push_checkpoint(&self) {
        let inner = unsafe { &mut *self.inner.get() };
        inner.checkpoints.push(inner.offset);
    }

    pub fn pop_checkpoint(&self) {
        let inner = unsafe { &mut *self.inner.get() };
        if let Some(offset) = inner.checkpoints.pop() {
            inner.offset = offset;
        }
    }

    pub fn reset(&self) {
        let inner = unsafe { &mut *self.inner.get() };
        inner.offset = 0;
        inner.checkpoints.clear();
    }
}

unsafe impl Send for FrameArena {}
unsafe impl Sync for FrameArena {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_alloc_and_reset() {
        let mut arena = Arena::new(1024);
        let a = arena.alloc(42i32);
        assert_eq!(*a, 42);
        *a = 100;
        assert_eq!(*a, 100);
        let (used, allocated) = arena.usage();
        assert!(used > 0);
        assert!(allocated >= 1024);
        arena.reset();
        let (used2, _) = arena.usage();
        assert_eq!(used2, 0);
    }

    #[test]
    fn test_arena_alloc_multiple_types() {
        let mut arena = Arena::new(4096);
        let a = arena.alloc(1u32);
        assert_eq!(*a, 1);
        let b = arena.alloc(std::f64::consts::PI);
        assert_eq!(*b, std::f64::consts::PI);
        let c = arena.alloc([1u8, 2, 3, 4]);
        assert_eq!(*c, [1, 2, 3, 4]);
    }

    #[test]
    fn test_arena_alloc_slice() {
        let mut arena = Arena::new(4096);
        let slice = arena.alloc_slice(&[1i32, 2, 3, 4, 5]);
        assert_eq!(slice, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_stack_arena() {
        let arena = StackArena::new(1024);
        let a = arena.alloc(42u64);
        let b = arena.alloc(std::f32::consts::PI);
        assert_eq!(*a, 42);
        assert_eq!(*b, std::f32::consts::PI);
        assert!(arena.used() > 0);
        arena.reset();
        assert_eq!(arena.used(), 0);
    }

    #[test]
    fn test_frame_arena_checkpoint() {
        let arena = FrameArena::new(1024);
        arena.push_checkpoint();
        let _a = arena.alloc(1u32);
        let _b = arena.alloc(2u32);
        let after_alloc = unsafe { &*arena.inner.get() }.offset;
        arena.pop_checkpoint();
        let after_pop = unsafe { &*arena.inner.get() }.offset;
        assert!(after_pop < after_alloc);
    }

    #[test]
    fn test_arena_alignment() {
        let mut arena = Arena::new(4096);
        let a = arena.alloc(1u64);
        let ptr = a as *mut u64 as usize;
        assert_eq!(ptr % 8, 0);
    }
}
