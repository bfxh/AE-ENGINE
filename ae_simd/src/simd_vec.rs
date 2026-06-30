#![allow(clippy::missing_safety_doc)]

use bytemuck::{Pod, Zeroable};
use std::arch::x86_64::*;

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Simd4f(__m128);

impl Simd4f {
    #[inline(always)]
    pub unsafe fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self(_mm_set_ps(w, z, y, x))
    }

    #[inline(always)]
    pub unsafe fn splat(v: f32) -> Self {
        Self(_mm_set1_ps(v))
    }

    #[inline(always)]
    pub unsafe fn zero() -> Self {
        Self(_mm_setzero_ps())
    }

    #[inline(always)]
    pub unsafe fn load(ptr: *const f32) -> Self {
        Self(_mm_loadu_ps(ptr))
    }

    #[inline(always)]
    pub unsafe fn store(self, ptr: *mut f32) {
        _mm_storeu_ps(ptr, self.0);
    }

    #[inline(always)]
    pub unsafe fn add(self, rhs: Self) -> Self {
        Self(_mm_add_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn sub(self, rhs: Self) -> Self {
        Self(_mm_sub_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn mul(self, rhs: Self) -> Self {
        Self(_mm_mul_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn div(self, rhs: Self) -> Self {
        Self(_mm_div_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn min(self, rhs: Self) -> Self {
        Self(_mm_min_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn max(self, rhs: Self) -> Self {
        Self(_mm_max_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn sqrt(self) -> Self {
        Self(_mm_sqrt_ps(self.0))
    }

    #[inline(always)]
    pub unsafe fn rsqrt(self) -> Self {
        Self(_mm_rsqrt_ps(self.0))
    }

    #[inline(always)]
    pub unsafe fn abs(self) -> Self {
        let mask = _mm_set1_ps(-0.0);
        Self(_mm_andnot_ps(mask, self.0))
    }

    #[inline(always)]
    pub unsafe fn dot3(self, rhs: Self) -> f32 {
        let mul = _mm_mul_ps(self.0, rhs.0);
        let shuffle1 = _mm_shuffle_ps(mul, mul, 0b00_00_00_01);
        let add1 = _mm_add_ss(mul, shuffle1);
        let shuffle2 = _mm_shuffle_ps(mul, mul, 0b00_00_00_10);
        let add2 = _mm_add_ss(add1, shuffle2);
        _mm_cvtss_f32(add2)
    }

    #[inline(always)]
    pub unsafe fn cross3(self, rhs: Self) -> Self {
        let a_yzx = _mm_shuffle_ps(self.0, self.0, 0b11_01_00_10);
        let b_yzx = _mm_shuffle_ps(rhs.0, rhs.0, 0b11_01_00_10);
        let a_zxy = _mm_shuffle_ps(self.0, self.0, 0b11_00_10_01);
        let b_zxy = _mm_shuffle_ps(rhs.0, rhs.0, 0b11_00_10_01);
        let left = _mm_mul_ps(a_yzx, b_zxy);
        let right = _mm_mul_ps(a_zxy, b_yzx);
        Self(_mm_sub_ps(left, right))
    }

    #[inline(always)]
    pub unsafe fn length3(self) -> f32 {
        let d = self.dot3(self);
        if d > 0.0 { d.sqrt() } else { 0.0 }
    }

    #[inline(always)]
    pub unsafe fn normalize3(self) -> Self {
        let len = self.length3();
        if len > 1e-10 { self.div(Self::splat(len)) } else { Self::zero() }
    }

    #[inline(always)]
    pub unsafe fn fmadd(self, a: Self, b: Self) -> Self {
        Self(_mm_fmadd_ps(a.0, b.0, self.0))
    }

    #[inline(always)]
    pub unsafe fn fmsub(self, a: Self, b: Self) -> Self {
        Self(_mm_fmsub_ps(a.0, b.0, self.0))
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Simd8f(__m256);

impl Simd8f {
    #[inline(always)]
    pub unsafe fn new(
        a0: f32,
        a1: f32,
        a2: f32,
        a3: f32,
        a4: f32,
        a5: f32,
        a6: f32,
        a7: f32,
    ) -> Self {
        Self(_mm256_set_ps(a7, a6, a5, a4, a3, a2, a1, a0))
    }

    #[inline(always)]
    pub unsafe fn splat(v: f32) -> Self {
        Self(_mm256_set1_ps(v))
    }

    #[inline(always)]
    pub unsafe fn zero() -> Self {
        Self(_mm256_setzero_ps())
    }

    #[inline(always)]
    pub unsafe fn load(ptr: *const f32) -> Self {
        Self(_mm256_loadu_ps(ptr))
    }

    #[inline(always)]
    pub unsafe fn store(self, ptr: *mut f32) {
        _mm256_storeu_ps(ptr, self.0);
    }

    #[inline(always)]
    pub unsafe fn add(self, rhs: Self) -> Self {
        Self(_mm256_add_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn sub(self, rhs: Self) -> Self {
        Self(_mm256_sub_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn mul(self, rhs: Self) -> Self {
        Self(_mm256_mul_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn div(self, rhs: Self) -> Self {
        Self(_mm256_div_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn min(self, rhs: Self) -> Self {
        Self(_mm256_min_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn max(self, rhs: Self) -> Self {
        Self(_mm256_max_ps(self.0, rhs.0))
    }

    #[inline(always)]
    pub unsafe fn sqrt(self) -> Self {
        Self(_mm256_sqrt_ps(self.0))
    }

    #[inline(always)]
    pub unsafe fn abs(self) -> Self {
        let mask = _mm256_set1_ps(-0.0);
        Self(_mm256_andnot_ps(mask, self.0))
    }

    #[inline(always)]
    pub unsafe fn hadd(self) -> f32 {
        let lo = _mm256_castps256_ps128(self.0);
        let hi = _mm256_extractf128_ps(self.0, 1);
        let sum128 = _mm_add_ps(lo, hi);
        let shuffle1 = _mm_shuffle_ps(sum128, sum128, 0b00_00_11_10);
        let sum2 = _mm_add_ps(sum128, shuffle1);
        let shuffle2 = _mm_shuffle_ps(sum2, sum2, 0b00_00_00_01);
        let sum3 = _mm_add_ss(sum2, shuffle2);
        _mm_cvtss_f32(sum3)
    }

    #[inline(always)]
    pub unsafe fn fmadd(self, a: Self, b: Self) -> Self {
        Self(_mm256_fmadd_ps(a.0, b.0, self.0))
    }
}

pub fn has_avx2() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        is_x86_feature_detected!("avx2")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

pub fn has_fma() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        is_x86_feature_detected!("fma")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

pub fn has_sse4_2() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        is_x86_feature_detected!("sse4.2")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

#[cfg(test)]
#[cfg(target_arch = "x86_64")]
mod tests {
    use super::*;

    #[test]
    fn test_simd4f_new_and_store() {
        unsafe {
            let v = Simd4f::new(1.0, 2.0, 3.0, 4.0);
            let mut buf = [0.0f32; 4];
            v.store(buf.as_mut_ptr());
            assert!((buf[0] - 1.0).abs() < 0.001);
            assert!((buf[1] - 2.0).abs() < 0.001);
            assert!((buf[2] - 3.0).abs() < 0.001);
            assert!((buf[3] - 4.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_splat() {
        unsafe {
            let v = Simd4f::splat(3.5);
            let mut buf = [0.0f32; 4];
            v.store(buf.as_mut_ptr());
            for val in &buf {
                assert!((*val - 3.5).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_simd4f_zero() {
        unsafe {
            let v = Simd4f::zero();
            let mut buf = [0.0f32; 4];
            v.store(buf.as_mut_ptr());
            for val in &buf {
                assert!((*val - 0.0).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_simd4f_add() {
        unsafe {
            let a = Simd4f::new(1.0, 2.0, 3.0, 4.0);
            let b = Simd4f::new(5.0, 6.0, 7.0, 8.0);
            let r = a.add(b);
            let mut buf = [0.0f32; 4];
            r.store(buf.as_mut_ptr());
            assert!((buf[0] - 6.0).abs() < 0.001);
            assert!((buf[1] - 8.0).abs() < 0.001);
            assert!((buf[2] - 10.0).abs() < 0.001);
            assert!((buf[3] - 12.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_sub() {
        unsafe {
            let a = Simd4f::new(10.0, 9.0, 8.0, 7.0);
            let b = Simd4f::new(1.0, 2.0, 3.0, 4.0);
            let r = a.sub(b);
            let mut buf = [0.0f32; 4];
            r.store(buf.as_mut_ptr());
            assert!((buf[0] - 9.0).abs() < 0.001);
            assert!((buf[1] - 7.0).abs() < 0.001);
            assert!((buf[2] - 5.0).abs() < 0.001);
            assert!((buf[3] - 3.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_mul() {
        unsafe {
            let a = Simd4f::new(2.0, 3.0, 4.0, 5.0);
            let b = Simd4f::new(3.0, 4.0, 5.0, 6.0);
            let r = a.mul(b);
            let mut buf = [0.0f32; 4];
            r.store(buf.as_mut_ptr());
            assert!((buf[0] - 6.0).abs() < 0.001);
            assert!((buf[1] - 12.0).abs() < 0.001);
            assert!((buf[2] - 20.0).abs() < 0.001);
            assert!((buf[3] - 30.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_div() {
        unsafe {
            let a = Simd4f::new(10.0, 20.0, 30.0, 40.0);
            let b = Simd4f::new(2.0, 4.0, 5.0, 8.0);
            let r = a.div(b);
            let mut buf = [0.0f32; 4];
            r.store(buf.as_mut_ptr());
            assert!((buf[0] - 5.0).abs() < 0.001);
            assert!((buf[1] - 5.0).abs() < 0.001);
            assert!((buf[2] - 6.0).abs() < 0.001);
            assert!((buf[3] - 5.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_dot3() {
        unsafe {
            let a = Simd4f::new(1.0, 2.0, 3.0, 0.0);
            let b = Simd4f::new(4.0, 5.0, 6.0, 0.0);
            let d = a.dot3(b);
            assert!((d - 32.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_dot3_orthogonal() {
        unsafe {
            let a = Simd4f::new(1.0, 0.0, 0.0, 0.0);
            let b = Simd4f::new(0.0, 1.0, 0.0, 0.0);
            let d = a.dot3(b);
            assert!(d.abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_cross3() {
        unsafe {
            let a = Simd4f::new(0.0, 1.0, 0.0, 0.0);
            let b = Simd4f::new(1.0, 0.0, 0.0, 0.0);
            let r = a.cross3(b);
            let mut buf = [0.0f32; 4];
            r.store(buf.as_mut_ptr());
            assert!((buf[0] - 0.0).abs() < 0.001);
            assert!((buf[1] - 0.0).abs() < 0.001);
            assert!((buf[2] - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_length3() {
        unsafe {
            let v = Simd4f::new(3.0, 4.0, 0.0, 0.0);
            let len = v.length3();
            assert!((len - 5.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_normalize3() {
        unsafe {
            let v = Simd4f::new(3.0, 0.0, 0.0, 0.0);
            let n = v.normalize3();
            let mut buf = [0.0f32; 4];
            n.store(buf.as_mut_ptr());
            assert!((buf[0] - 1.0).abs() < 0.001);
            assert!((buf[1] - 0.0).abs() < 0.001);
            assert!((buf[2] - 0.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_normalize3_zero() {
        unsafe {
            let v = Simd4f::zero();
            let n = v.normalize3();
            let mut buf = [0.0f32; 4];
            n.store(buf.as_mut_ptr());
            for val in &buf {
                assert!((*val - 0.0).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_simd4f_min_max() {
        unsafe {
            let a = Simd4f::new(1.0, 5.0, 3.0, 7.0);
            let b = Simd4f::new(4.0, 2.0, 6.0, 0.0);
            let mn = a.min(b);
            let mx = a.max(b);
            let mut buf_min = [0.0f32; 4];
            let mut buf_max = [0.0f32; 4];
            mn.store(buf_min.as_mut_ptr());
            mx.store(buf_max.as_mut_ptr());
            assert!((buf_min[0] - 1.0).abs() < 0.001);
            assert!((buf_min[1] - 2.0).abs() < 0.001);
            assert!((buf_min[2] - 3.0).abs() < 0.001);
            assert!((buf_min[3] - 0.0).abs() < 0.001);
            assert!((buf_max[0] - 4.0).abs() < 0.001);
            assert!((buf_max[1] - 5.0).abs() < 0.001);
            assert!((buf_max[2] - 6.0).abs() < 0.001);
            assert!((buf_max[3] - 7.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_simd4f_sqrt() {
        unsafe {
            let v = Simd4f::new(4.0, 9.0, 16.0, 25.0);
            let r = v.sqrt();
            let mut buf = [0.0f32; 4];
            r.store(buf.as_mut_ptr());
            assert!((buf[0] - 2.0).abs() < 0.001);
            assert!((buf[1] - 3.0).abs() < 0.001);
            assert!((buf[2] - 4.0).abs() < 0.001);
            assert!((buf[3] - 5.0).abs() < 0.001);
        }
    }
}
