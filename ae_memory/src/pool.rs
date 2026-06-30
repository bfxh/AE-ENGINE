use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};

pub struct ObjectPool<T> {
    free: VecDeque<T>,
    total_allocated: usize,
    total_acquired: usize,
}

impl<T: Default> ObjectPool<T> {
    pub fn new(initial_capacity: usize) -> Self {
        let mut free = VecDeque::with_capacity(initial_capacity);
        for _ in 0..initial_capacity {
            free.push_back(T::default());
        }
        ObjectPool { free, total_allocated: initial_capacity, total_acquired: 0 }
    }

    pub fn acquire(&mut self) -> PoolGuard<T> {
        let item = self.free.pop_front().unwrap_or_else(|| {
            self.total_allocated += 1;
            T::default()
        });
        self.total_acquired += 1;
        PoolGuard { item: Some(item), _pool: self as *mut ObjectPool<T> }
    }

    pub fn available(&self) -> usize {
        self.free.len()
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.total_acquired, self.total_allocated)
    }

    fn release(&mut self, item: T) {
        self.free.push_back(item);
    }
}

impl<T> ObjectPool<T> {
    pub fn with_initializer(initial_capacity: usize, init: impl Fn() -> T) -> Self {
        let mut free = VecDeque::with_capacity(initial_capacity);
        for _ in 0..initial_capacity {
            free.push_back(init());
        }
        ObjectPool { free, total_allocated: initial_capacity, total_acquired: 0 }
    }
}

pub struct PoolGuard<T: Default> {
    item: Option<T>,
    _pool: *mut ObjectPool<T>,
}

impl<T: Default> Deref for PoolGuard<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.item.as_ref().unwrap()
    }
}

impl<T: Default> DerefMut for PoolGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.item.as_mut().unwrap()
    }
}

impl<T: Default> Drop for PoolGuard<T> {
    fn drop(&mut self) {
        if let Some(item) = self.item.take() {
            unsafe { &mut *self._pool }.release(item);
        }
    }
}

pub struct TypedPool<T> {
    free: Vec<T>,
    alloc_count: usize,
}

impl<T: Default> TypedPool<T> {
    pub fn new(initial_capacity: usize) -> Self {
        let mut free = Vec::with_capacity(initial_capacity);
        for _ in 0..initial_capacity {
            free.push(T::default());
        }
        TypedPool { free, alloc_count: initial_capacity }
    }

    pub fn acquire(&mut self) -> TypedPoolGuard<T> {
        let item = self.free.pop().unwrap_or_else(|| {
            self.alloc_count += 1;
            T::default()
        });
        TypedPoolGuard { item: Some(item), _pool: self as *mut TypedPool<T> }
    }

    pub fn acquire_with<F: FnOnce(&mut T)>(&mut self, init: F) -> TypedPoolGuard<T> {
        let mut guard = self.acquire();
        init(&mut guard);
        guard
    }

    pub fn available(&self) -> usize {
        self.free.len()
    }

    pub fn total_allocated(&self) -> usize {
        self.alloc_count
    }

    fn release(&mut self, item: T) {
        self.free.push(item);
    }
}

pub struct TypedPoolGuard<T: Default> {
    item: Option<T>,
    _pool: *mut TypedPool<T>,
}

impl<T: Default> Deref for TypedPoolGuard<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.item.as_ref().unwrap()
    }
}

impl<T: Default> DerefMut for TypedPoolGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.item.as_mut().unwrap()
    }
}

impl<T: Default> Drop for TypedPoolGuard<T> {
    fn drop(&mut self) {
        if let Some(item) = self.item.take() {
            unsafe { &mut *self._pool }.release(item);
        }
    }
}

pub struct SlabPool<T> {
    slabs: Vec<Vec<T>>,
    slab_size: usize,
    free_indices: Vec<(usize, usize)>,
}

impl<T: Default + Clone> SlabPool<T> {
    pub fn new(slab_size: usize) -> Self {
        SlabPool { slabs: Vec::new(), slab_size: slab_size.max(64), free_indices: Vec::new() }
    }

    pub fn allocate(&mut self) -> SlabPtr {
        if let Some((slab, idx)) = self.free_indices.pop() {
            SlabPtr { slab, idx }
        } else {
            if self.slabs.is_empty() || self.slabs.last().unwrap().len() >= self.slab_size {
                self.slabs.push(Vec::with_capacity(self.slab_size));
            }
            let slab = self.slabs.len() - 1;
            let idx = self.slabs[slab].len();
            self.slabs[slab].push(T::default());
            SlabPtr { slab, idx }
        }
    }

    pub fn get(&self, ptr: SlabPtr) -> &T {
        &self.slabs[ptr.slab][ptr.idx]
    }

    pub fn get_mut(&mut self, ptr: SlabPtr) -> &mut T {
        &mut self.slabs[ptr.slab][ptr.idx]
    }

    pub fn free(&mut self, ptr: SlabPtr) {
        self.free_indices.push((ptr.slab, ptr.idx));
    }

    pub fn iter(&self) -> impl Iterator<Item = (SlabPtr, &T)> {
        self.slabs.iter().enumerate().flat_map(|(si, slab)| {
            slab.iter().enumerate().map(move |(idx, item)| (SlabPtr { slab: si, idx }, item))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabPtr {
    pub slab: usize,
    pub idx: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_pool_acquire_release() {
        let mut pool: ObjectPool<u32> = ObjectPool::new(4);
        assert_eq!(pool.available(), 4);
        let a = pool.acquire();
        assert_eq!(pool.available(), 3);
        drop(a);
        assert_eq!(pool.available(), 4);
    }

    #[test]
    fn test_object_pool_grow() {
        let mut pool: ObjectPool<u32> = ObjectPool::new(2);
        let _a = pool.acquire();
        let _b = pool.acquire();
        let _c = pool.acquire();
        let (_, total) = pool.stats();
        assert_eq!(total, 3);
    }

    #[test]
    fn test_typed_pool_with_init() {
        #[derive(Default, Clone)]
        struct TestData {
            value: i32,
        }
        let mut pool: TypedPool<TestData> = TypedPool::new(2);
        let mut guard = pool.acquire_with(|d| d.value = 42);
        assert_eq!(guard.value, 42);
        guard.value = 100;
        drop(guard);
        let guard2 = pool.acquire();
        assert_eq!(guard2.value, 100);
    }

    #[test]
    fn test_slab_pool() {
        let mut slab: SlabPool<i32> = SlabPool::new(64);
        let p1 = slab.allocate();
        *slab.get_mut(p1) = 42;
        let p2 = slab.allocate();
        *slab.get_mut(p2) = 100;
        assert_eq!(*slab.get(p1), 42);
        assert_eq!(*slab.get(p2), 100);
        slab.free(p1);
        let p3 = slab.allocate();
        assert_eq!(p3, p1);
    }
}
