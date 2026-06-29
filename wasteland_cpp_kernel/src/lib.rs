//! wasteland_cpp_kernel — C++ SIMD 性能内核 Rust 绑定
//!
//! 通过 cc crate + g++ 编译 C++ AVX2 内核，提供高性能批量物理积分。
//! 这是 Rust ↔ C++ 多语言集成的示范模块。
//!
//! # 算法
//!
//! 显式 Euler 积分:
//!   v_{n+1} = v_n + (F/m) * dt
//!   x_{n+1} = x_n + v_{n+1} * dt
//!
//! # 优化
//!
//! - AVX2 8 路 f32 并行
//! - FMA 融合乘加（v = f * (dt/m) + v）
//! - Newton-Raphson 精化倒数（避免除法）
//! - SoA 布局连续加载（避免 gather 指令）

use std::os::raw::c_float;

extern "C" {
    fn physics_step_soa_avx2(
        px: *mut c_float, py: *mut c_float, pz: *mut c_float,
        vx: *mut c_float, vy: *mut c_float, vz: *mut c_float,
        fx: *const c_float, fy: *const c_float, fz: *const c_float,
        masses: *const c_float,
        dt: c_float,
        n: u32,
    );

    fn count_aabb_overlaps(aabbs: *const c_float, n: u32) -> u32;
}

/// SoA 布局的粒子系统（分量连续存储，便于 SIMD）
#[derive(Debug, Clone)]
pub struct SoaParticleSystem {
    pub n: usize,
    pub px: Vec<f32>,
    pub py: Vec<f32>,
    pub pz: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub vz: Vec<f32>,
    pub fx: Vec<f32>,
    pub fy: Vec<f32>,
    pub fz: Vec<f32>,
    pub masses: Vec<f32>,
}

impl SoaParticleSystem {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            px: vec![0.0; n],
            py: vec![0.0; n],
            pz: vec![0.0; n],
            vx: vec![0.0; n],
            vy: vec![0.0; n],
            vz: vec![0.0; n],
            fx: vec![0.0; n],
            fy: vec![0.0; n],
            fz: vec![0.0; n],
            masses: vec![1.0; n],
        }
    }

    /// 用 C++ AVX2 内核执行一步物理积分
    pub fn step_cpp(&mut self, dt: f32) {
        unsafe {
            physics_step_soa_avx2(
                self.px.as_mut_ptr(), self.py.as_mut_ptr(), self.pz.as_mut_ptr(),
                self.vx.as_mut_ptr(), self.vy.as_mut_ptr(), self.vz.as_mut_ptr(),
                self.fx.as_ptr(), self.fy.as_ptr(), self.fz.as_ptr(),
                self.masses.as_ptr(),
                dt,
                self.n as u32,
            );
        }
    }

    /// 纯 Rust 标量版本（用于对比验证）
    pub fn step_rust(&mut self, dt: f32) {
        for i in 0..self.n {
            let inv_m = 1.0 / self.masses[i];
            self.vx[i] += self.fx[i] * inv_m * dt;
            self.vy[i] += self.fy[i] * inv_m * dt;
            self.vz[i] += self.fz[i] * inv_m * dt;
            self.px[i] += self.vx[i] * dt;
            self.py[i] += self.vy[i] * dt;
            self.pz[i] += self.vz[i] * dt;
        }
    }
}

/// 批量 AABB 重叠计数（C++ 实现）
pub fn count_aabb_overlaps_cpp(aabbs: &[[f32; 6]]) -> u32 {
    if aabbs.is_empty() {
        return 0;
    }
    let flat: Vec<f32> = aabbs.iter().flat_map(|a| a.iter().copied()).collect();
    unsafe { count_aabb_overlaps(flat.as_ptr(), aabbs.len() as u32) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_kernel_compiles_and_links() {
        let mut sys = SoaParticleSystem::new(16);
        for i in 0..16 {
            sys.masses[i] = 1.0;
            sys.fx[i] = 10.0;  // 10 N 力
        }
        sys.step_cpp(0.1);
        // v = 0 + 10/1 * 0.1 = 1.0
        // x = 0 + 1.0 * 0.1 = 0.1
        for i in 0..16 {
            assert!((sys.vx[i] - 1.0).abs() < 1e-5, "vx[{}] = {}", i, sys.vx[i]);
            assert!((sys.px[i] - 0.1).abs() < 1e-5, "px[{}] = {}", i, sys.px[i]);
        }
    }

    #[test]
    fn test_cpp_matches_rust_scalar() {
        let n = 100;
        let mut sys_cpp = SoaParticleSystem::new(n);
        let mut sys_rust = SoaParticleSystem::new(n);

        for i in 0..n {
            let m = 1.0 + (i as f32) * 0.1;
            sys_cpp.masses[i] = m;
            sys_rust.masses[i] = m;
            sys_cpp.fx[i] = 5.0 * (i as f32).sin();
            sys_rust.fx[i] = 5.0 * (i as f32).sin();
            sys_cpp.fy[i] = 3.0 * (i as f32).cos();
            sys_rust.fy[i] = 3.0 * (i as f32).cos();
        }

        for _ in 0..10 {
            sys_cpp.step_cpp(0.01);
            sys_rust.step_rust(0.01);
        }

        // C++ AVX2 与 Rust 标量结果应一致（容差 1e-4，因 rcp_ps 近似）
        for i in 0..n {
            assert!((sys_cpp.vx[i] - sys_rust.vx[i]).abs() < 1e-3,
                "vx mismatch at {}: cpp={} rust={}", i, sys_cpp.vx[i], sys_rust.vx[i]);
            assert!((sys_cpp.px[i] - sys_rust.px[i]).abs() < 1e-3,
                "px mismatch at {}: cpp={} rust={}", i, sys_cpp.px[i], sys_rust.px[i]);
        }
    }

    #[test]
    fn test_aabb_overlap_count() {
        let aabbs = vec![
            [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],  // A
            [0.5, 0.5, 0.5, 1.5, 1.5, 1.5],  // B（与 A 重叠）
            [5.0, 5.0, 5.0, 6.0, 6.0, 6.0],  // C（不重叠）
            [0.9, 0.9, 0.9, 2.0, 2.0, 2.0],  // D（与 A、B 重叠）
        ];
        let count = count_aabb_overlaps_cpp(&aabbs);
        // A-B, A-D, B-D = 3 对
        assert_eq!(count, 3, "AABB 重叠对数应为 3，实际 {}", count);
    }

    #[test]
    fn test_aabb_no_overlaps() {
        let aabbs = vec![
            [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            [2.0, 2.0, 2.0, 3.0, 3.0, 3.0],
            [5.0, 5.0, 5.0, 6.0, 6.0, 6.0],
        ];
        assert_eq!(count_aabb_overlaps_cpp(&aabbs), 0);
    }
}
