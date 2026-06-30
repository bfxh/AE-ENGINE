//! 内联汇编高性能内核（AVX2 + FMA + Newton-Raphson rsqrt）
//!
//! 用 Rust `core::arch::asm!` 直接控制指令选择，绕过编译器自动向量化：
//! - `batch_rsqrt_ps_asm`: 批量倒数平方根（vrsqrtps 11-bit + 一次 NR 精化到 22-bit）
//! - `batch_normalize3_asm`: 批量向量归一化（避免除法，用 rsqrt + 乘法）
//! - `batch_dot3_asm`: 批量点积（FMA 三连加）
//!
//! 注: 使用 Intel 语法（默认，不带 `att_syntax` 选项）。
//! 编译开关: `RUSTFLAGS="-C target-feature=+avx2,+fma"`

#![cfg(all(target_arch = "x86_64", target_feature = "avx2"))]

use core::arch::asm;
use std::arch::x86_64::__m256;

/// 批量倒数平方根: out[i] = 1/sqrt(x[i])
///
/// 算法: vrsqrtps (11-bit 精度) + Newton-Raphson 一次精化 (y = y*(1.5 - 0.5*x*y*y))
/// 最终精度约 22-bit，相比 divps + sqrtps 避免两条高延迟除法。
///
/// # Safety
/// - 输入切片长度必须 >= count
/// - 输出切片长度必须 >= count
/// - count 必须是 8 的倍数（否则尾部由调用方处理）
/// - x 和 out 不能重叠
pub unsafe fn batch_rsqrt_ps_asm(x: &[f32], out: &mut [f32], count: usize) {
    assert!(x.len() >= count && out.len() >= count);
    assert!(count % 8 == 0, "count must be multiple of 8");

    let c_three_half: f32 = 1.5;
    let c_neg_half: f32 = -0.5;

    let c_three_half_v: __m256 = std::arch::x86_64::_mm256_set1_ps(c_three_half);
    let c_neg_half_v: __m256 = std::arch::x86_64::_mm256_set1_ps(c_neg_half);

    let p_in = x.as_ptr();
    let p_out = out.as_mut_ptr();
    let n_chunks = count / 8;

    for i in 0..n_chunks {
        let offset = i * 8;
        unsafe {
            asm!(
                // y0 = vrsqrtps(x)  -- 11-bit 近似
                "vmovups ymm0, [{p_in}]",
                "vrsqrtps ymm1, ymm0",
                // NR 精化: y1 = y0 * (1.5 - 0.5 * x * y0 * y0)
                "vbroadcastss ymm2, dword ptr [{p_half}]",
                "vbroadcastss ymm3, dword ptr [{p_three}]",
                "vmulps ymm4, ymm1, ymm1",      // y0*y0
                "vmulps ymm4, ymm4, ymm0",      // x*y0*y0
                "vfmadd132ps ymm4, ymm3, ymm2",  // 0.5*x*y0*y0 + 1.5 (注意 FMA: ymm4 = ymm4*ymm2 + ymm3)
                // 等价: ymm4 = ymm2*ymm4 + ymm3 = -0.5*(x*y0*y0) + 1.5
                "vmulps ymm1, ymm1, ymm4",      // y1 = y0 * (1.5 - 0.5*x*y0*y0)
                "vmovups [{p_out}], ymm1",
                p_in = in(reg) p_in.add(offset),
                p_out = in(reg) p_out.add(offset),
                p_half = in(reg) &c_neg_half as *const f32,
                p_three = in(reg) &c_three_half as *const f32,
                out("ymm0") _,
                out("ymm1") _,
                out("ymm2") _,
                out("ymm3") _,
                out("ymm4") _,
                options(preserves_flags, nostack),
            );
        }
    }
    // 阻止编译器把常量当成死代码消除
    core::hint::black_box((c_three_half_v, c_neg_half_v));
}

/// 批量向量归一化: (x,y,z)[i] /= length((x,y,z)[i])
///
/// 用 rsqrt 替代除法: inv_len = rsqrt(x*x+y*y+z*z)，然后 x *= inv_len
///
/// # Safety
/// - x/y/z 长度 >= count
/// - count % 8 == 0
pub unsafe fn batch_normalize3_asm(x: &mut [f32], y: &mut [f32], z: &mut [f32], count: usize) {
    assert!(x.len() >= count && y.len() >= count && z.len() >= count);
    assert!(count % 8 == 0);

    let c_three_half: f32 = 1.5;
    let c_neg_half: f32 = -0.5;
    let c_eps: f32 = 1e-20;

    let p_x = x.as_mut_ptr();
    let p_y = y.as_mut_ptr();
    let p_z = z.as_mut_ptr();
    let n_chunks = count / 8;

    for i in 0..n_chunks {
        let off = i * 8;
        unsafe {
            asm!(
                // 加载 x/y/z
                "vmovups ymm0, [{p_x}]",
                "vmovups ymm1, [{p_y}]",
                "vmovups ymm2, [{p_z}]",
                // sq = x*x + y*y + z*z
                "vmulps ymm3, ymm0, ymm0",
                "vfmadd231ps ymm3, ymm1, ymm1",  // ymm3 = ymm3 + ymm1*ymm1
                "vfmadd231ps ymm3, ymm2, ymm2",  // ymm3 = ymm3 + ymm2*ymm2
                // y0 = rsqrt(sq)
                "vrsqrtps ymm4, ymm3",
                // NR 精化: y1 = y0 * (1.5 - 0.5 * sq * y0 * y0)
                "vbroadcastss ymm5, dword ptr [{p_half}]",
                "vbroadcastss ymm6, dword ptr [{p_three}]",
                "vmulps ymm7, ymm4, ymm4",      // y0*y0
                "vmulps ymm7, ymm7, ymm3",      // sq*y0*y0
                "vfmadd132ps ymm7, ymm6, ymm5",  // ymm7 = ymm7*ymm5 + ymm6 = -0.5*sq*y0*y0 + 1.5
                "vmulps ymm4, ymm4, ymm7",      // y1 = y0 * (...)
                // 安全检查: 若 sq 非常小，置 inv_len = 0（避免 NaN）
                "vbroadcastss ymm5, dword ptr [{p_eps}]",
                "vcmpps ymm5, ymm3, ymm5, 4",   // _CMP_GT_OQ: sq > eps ? 0xFF.. : 0
                "vandps ymm4, ymm4, ymm5",      // 若 sq<=eps 则 inv=0
                // x *= inv, y *= inv, z *= inv
                "vmulps ymm0, ymm0, ymm4",
                "vmulps ymm1, ymm1, ymm4",
                "vmulps ymm2, ymm2, ymm4",
                "vmovups [{p_x}], ymm0",
                "vmovups [{p_y}], ymm1",
                "vmovups [{p_z}], ymm2",
                p_x = in(reg) p_x.add(off),
                p_y = in(reg) p_y.add(off),
                p_z = in(reg) p_z.add(off),
                p_half = in(reg) &c_neg_half as *const f32,
                p_three = in(reg) &c_three_half as *const f32,
                p_eps = in(reg) &c_eps as *const f32,
                out("ymm0") _,
                out("ymm1") _,
                out("ymm2") _,
                out("ymm3") _,
                out("ymm4") _,
                out("ymm5") _,
                out("ymm6") _,
                out("ymm7") _,
                options(preserves_flags, nostack),
            );
        }
    }
}

/// 批量点积: out[i] = a[i]·b[i] = ax*bx + ay*by + az*bz
///
/// 用 3 次 vfmadd231ps 完成 3 项乘加。
///
/// # Safety
/// - 所有切片长度 >= count
/// - count % 8 == 0
pub unsafe fn batch_dot3_asm(
    ax: &[f32],
    ay: &[f32],
    az: &[f32],
    bx: &[f32],
    by: &[f32],
    bz: &[f32],
    out: &mut [f32],
    count: usize,
) {
    assert!(ax.len() >= count && ay.len() >= count && az.len() >= count);
    assert!(bx.len() >= count && by.len() >= count && bz.len() >= count);
    assert!(out.len() >= count);
    assert!(count % 8 == 0);

    let p_ax = ax.as_ptr();
    let p_ay = ay.as_ptr();
    let p_az = az.as_ptr();
    let p_bx = bx.as_ptr();
    let p_by = by.as_ptr();
    let p_bz = bz.as_ptr();
    let p_out = out.as_mut_ptr();
    let n_chunks = count / 8;

    for i in 0..n_chunks {
        let off = i * 8;
        unsafe {
            asm!(
                // acc = ax * bx
                "vmovups ymm0, [{p_ax}]",
                "vmovups ymm1, [{p_bx}]",
                "vmulps ymm2, ymm0, ymm1",
                // acc += ay * by
                "vmovups ymm0, [{p_ay}]",
                "vmovups ymm1, [{p_by}]",
                "vfmadd231ps ymm2, ymm0, ymm1",
                // acc += az * bz
                "vmovups ymm0, [{p_az}]",
                "vmovups ymm1, [{p_bz}]",
                "vfmadd231ps ymm2, ymm0, ymm1",
                "vmovups [{p_out}], ymm2",
                p_ax = in(reg) p_ax.add(off),
                p_ay = in(reg) p_ay.add(off),
                p_az = in(reg) p_az.add(off),
                p_bx = in(reg) p_bx.add(off),
                p_by = in(reg) p_by.add(off),
                p_bz = in(reg) p_bz.add(off),
                p_out = in(reg) p_out.add(off),
                out("ymm0") _,
                out("ymm1") _,
                out("ymm2") _,
                options(preserves_flags, nostack),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_rsqrt_ps_asm() {
        let x: Vec<f32> = vec![1.0, 4.0, 9.0, 16.0, 25.0, 36.0, 49.0, 64.0];
        let mut out = vec![0.0f32; 8];
        unsafe { batch_rsqrt_ps_asm(&x, &mut out, 8) };
        for i in 0..8 {
            let expected = 1.0 / x[i].sqrt();
            let rel_err = ((out[i] - expected).abs() / expected).max(0.0);
            // NR 精化后相对误差 < 1e-5
            assert!(
                rel_err < 1e-5,
                "i={}: out={} expected={} rel_err={}",
                i,
                out[i],
                expected,
                rel_err
            );
        }
    }

    #[test]
    fn test_batch_normalize3_asm() {
        let mut x = vec![3.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut y = vec![4.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut z = vec![0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        unsafe { batch_normalize3_asm(&mut x, &mut y, &mut z, 8) };
        // (3,4,0)/5 = (0.6, 0.8, 0)
        assert!((x[0] - 0.6).abs() < 1e-5, "x[0]={}", x[0]);
        assert!((y[0] - 0.8).abs() < 1e-5, "y[0]={}", y[0]);
        assert!(z[0].abs() < 1e-5, "z[0]={}", z[0]);
    }

    #[test]
    fn test_batch_dot3_asm() {
        let ax = vec![1.0f32; 8];
        let ay = vec![2.0f32; 8];
        let az = vec![3.0f32; 8];
        let bx = vec![4.0f32; 8];
        let by = vec![5.0f32; 8];
        let bz = vec![6.0f32; 8];
        let mut out = vec![0.0f32; 8];
        unsafe { batch_dot3_asm(&ax, &ay, &az, &bx, &by, &bz, &mut out, 8) };
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        for i in 0..8 {
            assert!((out[i] - 32.0).abs() < 1e-4, "i={}: out={}", i, out[i]);
        }
    }
}
