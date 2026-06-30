use std::alloc::{Layout, alloc};
use std::fmt;

pub struct ZeroCopyBuf {
    ptr: *mut u8,
    len: usize,
    cap: usize,
}

unsafe impl Send for ZeroCopyBuf {}
unsafe impl Sync for ZeroCopyBuf {}

impl ZeroCopyBuf {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::array::<u8>(capacity).unwrap();
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            panic!("allocation failed");
        }
        ZeroCopyBuf { ptr, len: 0, cap: capacity }
    }

    pub fn from_vec(mut v: Vec<u8>) -> Self {
        let buf = ZeroCopyBuf { ptr: v.as_mut_ptr(), len: v.len(), cap: v.capacity() };
        std::mem::forget(v);
        buf
    }

    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    pub fn read<T: Copy>(&self, offset: usize) -> Option<T> {
        let size = std::mem::size_of::<T>();
        if offset + size > self.len {
            return None;
        }
        unsafe {
            let ptr = self.ptr.add(offset);
            Some((ptr as *const T).read_unaligned())
        }
    }

    pub fn write<T: Copy>(&mut self, offset: usize, value: T) -> bool {
        let size = std::mem::size_of::<T>();
        if offset + size > self.cap {
            return false;
        }
        unsafe {
            let ptr = self.ptr.add(offset);
            (ptr as *mut T).write_unaligned(value);
        }
        if offset + size > self.len {
            self.len = offset + size;
        }
        true
    }

    pub fn slice<T: Copy>(&self, offset: usize, count: usize) -> Option<&[T]> {
        let size = std::mem::size_of::<T>();
        if offset + count * size > self.len {
            return None;
        }
        unsafe {
            let ptr = self.ptr.add(offset) as *const T;
            Some(std::slice::from_raw_parts(ptr, count))
        }
    }

    pub fn slice_mut<T: Copy>(&mut self, offset: usize, count: usize) -> Option<&mut [T]> {
        let size = std::mem::size_of::<T>();
        if offset + count * size > self.cap {
            return None;
        }
        unsafe {
            let ptr = self.ptr.add(offset) as *mut T;
            Some(std::slice::from_raw_parts_mut(ptr, count))
        }
    }

    pub fn view<T: Copy>(&self, offset: usize) -> Option<&T> {
        let size = std::mem::size_of::<T>();
        if offset + size > self.len {
            return None;
        }
        unsafe {
            let ptr = self.ptr.add(offset) as *const T;
            Some(&*ptr)
        }
    }

    pub fn view_mut<T: Copy>(&mut self, offset: usize) -> Option<&mut T> {
        let size = std::mem::size_of::<T>();
        if offset + size > self.cap {
            return None;
        }
        unsafe {
            let ptr = self.ptr.add(offset) as *mut T;
            Some(&mut *ptr)
        }
    }
}

impl Drop for ZeroCopyBuf {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.cap > 0 {
            let layout = Layout::array::<u8>(self.cap).unwrap();
            unsafe {
                std::alloc::dealloc(self.ptr, layout);
            }
        }
    }
}

impl fmt::Debug for ZeroCopyBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZeroCopyBuf").field("len", &self.len).field("cap", &self.cap).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct TestStruct {
        x: f32,
        y: f32,
        z: f32,
        id: u32,
    }

    #[test]
    fn test_read_write_primitives() {
        let mut buf = ZeroCopyBuf::new(32);
        assert!(buf.write(0, 42u32));
        assert_eq!(buf.read::<u32>(0), Some(42));
        assert!(buf.write(4, std::f32::consts::PI));
        let val = buf.read::<f32>(4).unwrap();
        assert!((val - std::f32::consts::PI).abs() < 0.001);
    }

    #[test]
    fn test_view_struct() {
        let mut buf = ZeroCopyBuf::new(64);
        let s = TestStruct { x: 1.0, y: 2.0, z: 3.0, id: 42 };
        assert!(buf.write(0, s));
        let view = buf.view::<TestStruct>(0).unwrap();
        assert_eq!(view.x, 1.0);
        assert_eq!(view.id, 42);
    }

    #[test]
    fn test_slice() {
        let mut buf = ZeroCopyBuf::new(64);
        for i in 0..5 {
            assert!(buf.write(i * 4, i as u32));
        }
        let slice = buf.slice::<u32>(0, 5).unwrap();
        assert_eq!(slice, &[0u32, 1, 2, 3, 4]);
    }

    #[test]
    fn test_boundary_check() {
        let buf = ZeroCopyBuf::new(8);
        assert_eq!(buf.read::<u64>(4), None);
        assert_eq!(buf.read::<u32>(6), None);
    }

    #[test]
    fn test_zero_copy_len_update() {
        let mut buf = ZeroCopyBuf::new(32);
        assert!(buf.write(0, 1u32));
        assert_eq!(buf.len(), 4);
        assert!(buf.write(4, 2u32));
        assert_eq!(buf.len(), 8);
    }
}
