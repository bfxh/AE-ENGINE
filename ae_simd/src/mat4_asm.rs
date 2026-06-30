//! 4x4 矩阵 AVX2+FMA 内联汇编内核
//!
//! 一次处理 8 个 f32 (ymm)，4x4 矩阵 = 16 个 f32 = 2 个 ymm。
//!
//! 实现的内核:
//! - `mat4_mul_asm`: C = A * B (4x4 矩阵乘法, 列主序)
//! - `mat4_vec_mul_asm`: y = A * x (4x4 矩阵 × 4D 向量)
//! - `mat4_transpose_asm`: 行列转置 (利用 vunpcklps/vunpckhps + vshufps)
//! - `mat4_diag_mul_asm`: y = diag(d) * A (对角矩阵左乘)
//!
//! 列主序约定 (与 glam::Mat4 一致):
//! - 矩阵存储为 [m00, m10, m20, m30, m01, m11, m21, m31, m02, m12, m22, m32, m03, m13, m23, m33]
//! - 列 0 在内存最低位 [0..4]，列 1 在 [4..8]，列 2 在 [8..12]，列 3 在 [12..16]
//!
//! 编译开关: `RUSTFLAGS="-C target-feature=+avx2,+fma"`

#![cfg(all(target_arch = "x86_64", target_feature = "avx2"))]

use core::arch::asm;

/// 4x4 矩阵乘法: C = A * B (列主序)
///
/// 算法:
/// - C 的列 j = A * (B 的列 j)
/// - C[:,j] = sum_k A[:,k] * B[k,j]
/// - 由于列主序，A[:,k] 在内存中是连续的 4 个 f32
/// - B[k,j] 是 B 的第 k 行第 j 列元素，需要对 B 做转置或用 vbroadcastss 广播
///
/// 用 AVX2 一次加载 2 列 (8 个 f32)，FMA 累加。
///
/// # Safety
/// - a/b/c 切片长度必须 >= 16
/// - 切片不能重叠（c 不能等于 a 或 b）
pub unsafe fn mat4_mul_asm(a: &[f32], b: &[f32], c: &mut [f32]) {
    assert!(a.len() >= 16 && b.len() >= 16 && c.len() >= 16);

    // 思路: 用 vbroadcastss 把 B 的单个元素广播到 ymm (含 2 列的 8 个 f32)
    // 然后与 A 的对应列做 FMA
    // 一次处理 C 的 2 列 (8 个 f32)
    //
    // C[:, 0..2] = A[:, 0] * B[0, 0..2] + A[:, 1] * B[1, 0..2] + A[:, 2] * B[2, 0..2] + A[:, 3] * B[3, 0..2]
    //
    // 注意: 列主序下 B[k, j] = b[k + j*4]
    // B[0,0]=b[0], B[0,1]=b[4], B[1,0]=b[1], B[1,1]=b[5], ...
    // 我们需要把 {B[0,0], B[0,1]} = {b[0], b[4]} 打包成一个 ymm 的低半和高半，然后广播
    // 简化: 用 vbroadcastss 单独广播 b[k+j*4]
    //
    // 但更高效: 先把 B 的列 0/1 加载到 ymm0，列 2/3 加载到 ymm1
    // ymm0 = [b[0..4] | b[4..8]]  = [B_col0 | B_col1]
    // ymm1 = [b[8..12] | b[12..16]] = [B_col2 | B_col3]
    // 然后对每个 k, 用 vpermilps 把 B[k, 0..2] 提取到 ymm，做 FMA
    //
    // 这里用最简单直接的方法: 4 次 vbroadcastss + FMA

    unsafe {
        asm!(
            // 加载 A 的 4 列 (8 个 f32 = 2 列 per ymm)
            "vmovups ymm0, [{pa}]",        // ymm0 = A[:, 0..2] (列 0 和列 1)
            "vmovups ymm1, [{pa} + 32]",   // ymm1 = A[:, 2..4] (列 2 和列 3)
            // 提取 A 的单列到 ymm (用 vextractf128 + vzeroupper 不够, 用 vbroadcastss 重建)
            // A[:, 0] 在 ymm0 的低 128 位; A[:, 1] 在 ymm0 的高 128 位
            // A[:, 2] 在 ymm1 的低 128 位; A[:, 3] 在 ymm1 的高 128 位

            // C[:, 0..2] = A[:, 0] * B[0, 0..2] + ... (累积到 ymm4)
            // B[0, 0..2] = {b[0], b[4]}，需要广播 b[0] 到 ymm0 低 4 个，b[4] 到 ymm0 高 4 个
            // 用 vunpcklps 把 {b[0], b[4]} → [b[0], b[0], b[0], b[0], b[4], b[4], b[4], b[4]]
            // 简化: 直接用 vbroadcastss 加载 b[0] 到 xmm 然后 vinsertf128 拼接 b[4]

            // 加载 B 的 8 个元素 (B[0,0..2], B[1,0..2]):
            // B[0,0]=b[0], B[0,1]=b[4]
            // B[1,0]=b[1], B[1,1]=b[5]
            // 用 vunpcklps 解交织: 从 [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]
            //                      得到 [b[0], b[0], b[0], b[0], b[4], b[4], b[4], b[4]] (广播 b[0] 和 b[4])
            "vbroadcastss ymm2, dword ptr [{pb}]",        // ymm2 = b[0] x 8 (但我们只要低 4 是 b[0], 高 4 是 b[4])
            "vbroadcastss ymm3, dword ptr [{pb} + 16]",   // ymm3 = b[4] x 8
            "vblendps ymm2, ymm2, ymm3, 0b11110000",      // ymm2 = [b[0]*4 | b[4]*4] = B[0, 0..2] 广播

            "vbroadcastss ymm3, dword ptr [{pb} + 4]",    // ymm3 = b[1] x 8
            "vbroadcastss ymm4, dword ptr [{pb} + 20]",   // ymm4 = b[5] x 8
            "vblendps ymm3, ymm3, ymm4, 0b11110000",      // ymm3 = [b[1]*4 | b[5]*4] = B[1, 0..2] 广播

            "vbroadcastss ymm4, dword ptr [{pb} + 8]",    // ymm4 = b[2] x 8
            "vbroadcastss ymm5, dword ptr [{pb} + 24]",   // ymm5 = b[6] x 8
            "vblendps ymm4, ymm4, ymm5, 0b11110000",      // ymm4 = [b[2]*4 | b[6]*4] = B[2, 0..2] 广播

            "vbroadcastss ymm5, dword ptr [{pb} + 12]",   // ymm5 = b[3] x 8
            "vbroadcastss ymm6, dword ptr [{pb} + 28]",   // ymm6 = b[7] x 8
            "vblendps ymm5, ymm5, ymm6, 0b11110000",      // ymm5 = [b[3]*4 | b[7]*4] = B[3, 0..2] 广播

            // 提取 A 的各列到 ymm (低 4 重复 / 高 4 重复)
            // A[:, 0] 是 ymm0 的低 128 位 (xmm0)，A[:, 1] 是 ymm0 的高 128 位
            // 用 vextractf128 提取
            "vextractf128 xmm6, ymm0, 0",    // xmm6 = A[:, 0] (低 128 位)
            "vinsertf128 ymm6, ymm6, xmm6, 1", // ymm6 = A[:, 0] 重复 2 次 (低和高都是 A[:, 0])
            "vextractf128 xmm7, ymm0, 1",    // xmm7 = A[:, 1] (高 128 位)
            "vinsertf128 ymm7, ymm7, xmm7, 1", // ymm7 = A[:, 1] 重复 2 次

            // 但是我们需要 A[:, 0] 只在低 4，A[:, 1] 只在高 4
            // 重新构造: ymm_a0 = [A[:,0] | 0]
            //          ymm_a1 = [0 | A[:,1]]
            // 用 vblendps 与 ymm0 组合:
            // ymm_a0 = blend(ymm0_repeated_low, ymm_zero, 0b11110000) = [A[:,0] | 0]
            // ymm_a1 = blend(ymm_zero, ymm0_repeated_high, 0b11110000) = [0 | A[:,1]]

            "vxorps ymm8, ymm8, ymm8",                    // ymm8 = 0
            "vextractf128 xmm9, ymm0, 0",                 // xmm9 = A[:, 0]
            "vinsertf128 ymm9, ymm9, xmm8, 1",            // ymm9 = [A[:, 0] | 0]
            "vextractf128 xmm10, ymm0, 1",                // xmm10 = A[:, 1]
            "vinsertf128 ymm10, ymm8, xmm10, 1",          // ymm10 = [0 | A[:, 1]]

            // 同样提取 A[:, 2] 和 A[:, 3]
            "vextractf128 xmm11, ymm1, 0",                // xmm11 = A[:, 2]
            "vinsertf128 ymm11, ymm11, xmm8, 1",          // ymm11 = [A[:, 2] | 0]
            "vextractf128 xmm12, ymm1, 1",                // xmm12 = A[:, 3]
            "vinsertf128 ymm12, ymm8, xmm12, 1",          // ymm12 = [0 | A[:, 3]]

            // C[:, 0..2] = A[:, 0] * B[0, 0..2] + A[:, 1] * B[1, 0..2] + A[:, 2] * B[2, 0..2] + A[:, 3] * B[3, 0..2]
            // 注意: ymm_a0 = [A[:,0] | 0], ymm_b0 = [b[0]*4 | b[4]*4]
            // 所以 ymm_a0 * ymm_b0 = [A[:,0]*b[0] | 0]  ✓ (低 4 是 C[:, 0] 的部分累加, 高 4 是 0)
            // ymm_a1 = [0 | A[:, 1]], ymm_a1 * ymm_b1 = [0 | A[:,1]*b[5]]
            // 加起来: [A[:,0]*b[0] | A[:,1]*b[5]]  ✓

            "vmulps ymm13, ymm9, ymm2",       // ymm13 = A[:, 0] * B[0, 0..2] (低 4 有效, 高 4 为 0)
            "vfmadd231ps ymm13, ymm10, ymm3", // ymm13 += A[:, 1] * B[1, 0..2] (高 4 累加)
            "vfmadd231ps ymm13, ymm11, ymm4", // ymm13 += A[:, 2] * B[2, 0..2] (低 4 累加)
            "vfmadd231ps ymm13, ymm12, ymm5", // ymm13 += A[:, 3] * B[3, 0..2] (高 4 累加)

            "vmovups [{pc}], ymm13",          // 写入 C[:, 0..2]

            // C[:, 2..4] = A[:, 0] * B[0, 2..4] + ... 类似，用 b[8..15]
            "vbroadcastss ymm2, dword ptr [{pb} + 32]",   // ymm2 = b[8] x 8
            "vbroadcastss ymm3, dword ptr [{pb} + 48]",   // ymm3 = b[12] x 8
            "vblendps ymm2, ymm2, ymm3, 0b11110000",      // ymm2 = [b[8]*4 | b[12]*4]

            "vbroadcastss ymm3, dword ptr [{pb} + 36]",   // ymm3 = b[9] x 8
            "vbroadcastss ymm4, dword ptr [{pb} + 52]",   // ymm4 = b[13] x 8
            "vblendps ymm3, ymm3, ymm4, 0b11110000",      // ymm3 = [b[9]*4 | b[13]*4]

            "vbroadcastss ymm4, dword ptr [{pb} + 40]",   // ymm4 = b[10]
            "vbroadcastss ymm5, dword ptr [{pb} + 56]",   // ymm5 = b[14]
            "vblendps ymm4, ymm4, ymm5, 0b11110000",      // ymm4 = [b[10]*4 | b[14]*4]

            "vbroadcastss ymm5, dword ptr [{pb} + 44]",   // ymm5 = b[11]
            "vbroadcastss ymm6, dword ptr [{pb} + 60]",   // ymm6 = b[15]
            "vblendps ymm5, ymm5, ymm6, 0b11110000",      // ymm5 = [b[11]*4 | b[15]*4]

            "vmulps ymm13, ymm9, ymm2",
            "vfmadd231ps ymm13, ymm10, ymm3",
            "vfmadd231ps ymm13, ymm11, ymm4",
            "vfmadd231ps ymm13, ymm12, ymm5",

            "vmovups [{pc} + 32], ymm13",     // 写入 C[:, 2..4]

            "vzeroupper",

            pa = in(reg) a.as_ptr(),
            pb = in(reg) b.as_ptr(),
            pc = in(reg) c.as_mut_ptr(),

            out("ymm0") _,
            out("ymm1") _,
            out("ymm2") _,
            out("ymm3") _,
            out("ymm4") _,
            out("ymm5") _,
            out("ymm6") _,
            out("ymm7") _,
            out("ymm8") _,
            out("ymm9") _,
            out("ymm10") _,
            out("ymm11") _,
            out("ymm12") _,
            out("ymm13") _,
            options(preserves_flags, nostack),
        );
    }
}

/// 4x4 矩阵 × 4D 向量: y = A * x (列主序)
///
/// y[i] = sum_k A[i, k] * x[k]
///
/// 列主序下: A[i, k] = a[k * 4 + i]
/// 即 A 的列 k 是 a[k*4..k*4+4]
///
/// 算法: y = x[0]*A[:,0] + x[1]*A[:,1] + x[2]*A[:,2] + x[3]*A[:,3]
///      = x[0]*a[0..4] + x[1]*a[4..8] + x[2]*a[8..12] + x[3]*a[12..16]
///
/// 用 vbroadcastss 广播 x[k]，FMA 累加 4 次。
///
/// # Safety
/// - a 长度 >= 16, x 长度 >= 4, y 长度 >= 4
pub unsafe fn mat4_vec_mul_asm(a: &[f32], x: &[f32], y: &mut [f32]) {
    assert!(a.len() >= 16 && x.len() >= 4 && y.len() >= 4);

    unsafe {
        asm!(
            // y = x[0] * A[:, 0]
            "vbroadcastss ymm0, dword ptr [{px}]",         // ymm0 = x[0] x 8 (只用低 4)
            "vmulps ymm1, ymm0, [{pa}]",                   // ymm1 = x[0] * A[:, 0] (低 4 有效)
            // y += x[1] * A[:, 1]
            "vbroadcastss ymm0, dword ptr [{px} + 4]",
            "vfmadd231ps ymm1, ymm0, [{pa} + 16]",
            // y += x[2] * A[:, 2]
            "vbroadcastss ymm0, dword ptr [{px} + 8]",
            "vfmadd231ps ymm1, ymm0, [{pa} + 32]",
            // y += x[3] * A[:, 3]
            "vbroadcastss ymm0, dword ptr [{px} + 12]",
            "vfmadd231ps ymm1, ymm0, [{pa} + 48]",
            // 写入低 4 个 f32 (用 xmm)
            "vextractf128 xmm0, ymm1, 0",
            "vmovups xmmword ptr [{py}], xmm0",
            "vzeroupper",

            pa = in(reg) a.as_ptr(),
            px = in(reg) x.as_ptr(),
            py = in(reg) y.as_mut_ptr(),
            out("ymm0") _,
            out("ymm1") _,
            options(preserves_flags, nostack),
        );
    }
}

/// 4x4 矩阵转置 (列主序 ↔ 行主序，等价于转置)
///
/// 利用 AVX2 的 vunpcklps / vunpckhps / vshufps 完成 4x4 转置。
///
/// 输入矩阵 (列主序):
/// [m00, m10, m20, m30,   <- 列 0
///  m01, m11, m21, m31,   <- 列 1
///  m02, m12, m22, m32,   <- 列 2
///  m03, m13, m23, m33]   <- 列 3
///
/// 转置后 (列主序):
/// [m00, m01, m02, m03,   <- 原 [m00, m01, m02, m03] 即原第一行
///  m10, m11, m12, m13,
///  m20, m21, m22, m23,
///  m30, m31, m32, m33]
///
/// # Safety
/// - src/dst 长度 >= 16
pub unsafe fn mat4_transpose_asm(src: &[f32], dst: &mut [f32]) {
    assert!(src.len() >= 16 && dst.len() >= 16);

    unsafe {
        asm!(
            // 加载 4 列到 xmm0..xmm3 (每列 4 个 f32)
            "vmovups xmm0, xmmword ptr [{ps}]",         // xmm0 = col0 = [m00, m10, m20, m30]
            "vmovups xmm1, xmmword ptr [{ps} + 16]",    // xmm1 = col1 = [m01, m11, m21, m31]
            "vmovups xmm2, xmmword ptr [{ps} + 32]",    // xmm2 = col2 = [m02, m12, m22, m32]
            "vmovups xmm3, xmmword ptr [{ps} + 48]",    // xmm3 = col3 = [m03, m13, m23, m33]

            // 第一步: unpcklps 把 col0/col1 的低 2 元素交错，unpckhps 把高 2 元素交错
            // xmm4 = [m00, m01, m10, m11] (col0/col1 低 2 元素交错)
            // xmm5 = [m20, m21, m30, m31] (col0/col1 高 2 元素交错)
            "vunpcklps xmm4, xmm0, xmm1",
            "vunpckhps xmm5, xmm0, xmm1",
            // xmm6 = [m02, m03, m12, m13]
            // xmm7 = [m22, m23, m32, m33]
            "vunpcklps xmm6, xmm2, xmm3",
            "vunpckhps xmm7, xmm2, xmm3",

            // 第二步: shufps 组合得到转置后的列
            // 行 0: [m00, m01, m02, m03] = shuf(xmm4, xmm6, 0b01_00_01_00) 取两者低 2 元素
            // 行 1: [m10, m11, m12, m13] = shuf(xmm4, xmm6, 0b11_10_11_10) 取两者高 2 元素
            // 行 2: [m20, m21, m22, m23] = shuf(xmm5, xmm7, 0b01_00_01_00)
            // 行 3: [m30, m31, m32, m33] = shuf(xmm5, xmm7, 0b11_10_11_10)
            "vshufps xmm0, xmm4, xmm6, 0b01_00_01_00",
            "vshufps xmm1, xmm4, xmm6, 0b11_10_11_10",
            "vshufps xmm2, xmm5, xmm7, 0b01_00_01_00",
            "vshufps xmm3, xmm5, xmm7, 0b11_10_11_10",

            // 写入转置后的矩阵 (列主序存储，所以写入 4 个 xmm 即可)
            "vmovups xmmword ptr [{pd}], xmm0",
            "vmovups xmmword ptr [{pd} + 16], xmm1",
            "vmovups xmmword ptr [{pd} + 32], xmm2",
            "vmovups xmmword ptr [{pd} + 48], xmm3",

            ps = in(reg) src.as_ptr(),
            pd = in(reg) dst.as_mut_ptr(),
            out("xmm0") _,
            out("xmm1") _,
            out("xmm2") _,
            out("xmm3") _,
            out("xmm4") _,
            out("xmm5") _,
            out("xmm6") _,
            out("xmm7") _,
            options(preserves_flags, nostack),
        );
    }
}

/// 对角矩阵左乘: y = diag(d) * A (列主序)
///
/// y[i, j] = d[i] * A[i, j]
/// 列主序下: y[k*4+i] = d[i] * a[k*4+i]
/// 即对 A 的每一列，按 d 做元素级乘法
///
/// 用 vbroadcastss 把 d[i] 广播到 4 个位置 (相当于 d 在一列内的 4 个位置)
/// 然后与 A 的对应列做乘法
///
/// 算法: 一次处理 2 列 (8 个 f32 = 1 个 ymm)
/// - ymm_d_low = [d[0], d[1], d[2], d[3], d[0], d[1], d[2], d[3]] (d 重复 2 次)
/// - ymm_a = [A[:,0] | A[:,1]]
/// - ymm_y = ymm_d_low * ymm_a
///
/// # Safety
/// - d 长度 >= 4, a 长度 >= 16, y 长度 >= 16
pub unsafe fn mat4_diag_mul_asm(d: &[f32], a: &[f32], y: &mut [f32]) {
    assert!(d.len() >= 4 && a.len() >= 16 && y.len() >= 16);

    unsafe {
        asm!(
            // 加载 d 到 xmm0
            "vmovups xmm0, xmmword ptr [{pd}]",
            // 把 d 重复到 ymm0 (低 128 = d, 高 128 = d)
            "vinsertf128 ymm0, ymm0, xmm0, 1",  // ymm0 = [d | d]

            // 一次处理 2 列
            "vmovups ymm1, [{pa}]",             // ymm1 = [A[:,0] | A[:,1]]
            "vmulps ymm2, ymm0, ymm1",
            "vmovups [{py}], ymm2",

            "vmovups ymm1, [{pa} + 32]",        // ymm1 = [A[:,2] | A[:,3]]
            "vmulps ymm2, ymm0, ymm1",
            "vmovups [{py} + 32], ymm2",

            "vzeroupper",

            pd = in(reg) d.as_ptr(),
            pa = in(reg) a.as_ptr(),
            py = in(reg) y.as_mut_ptr(),
            out("ymm0") _,
            out("ymm1") _,
            out("ymm2") _,
            options(preserves_flags, nostack),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mat4_mul_asm_identity() {
        let a: [f32; 16] = [
            1.0, 2.0, 3.0, 4.0,  // col 0
            5.0, 6.0, 7.0, 8.0,  // col 1
            9.0, 10.0, 11.0, 12.0,  // col 2
            13.0, 14.0, 15.0, 16.0,  // col 3
        ];
        let mut i: [f32; 16] = [0.0; 16];
        i[0] = 1.0;
        i[5] = 1.0;
        i[10] = 1.0;
        i[15] = 1.0;
        let mut c: [f32; 16] = [0.0; 16];
        unsafe { mat4_mul_asm(&i, &a, &mut c) };
        // c should equal a (identity * a = a)
        for k in 0..16 {
            assert!((c[k] - a[k]).abs() < 1e-5, "k={}: c={} a={}", k, c[k], a[k]);
        }
    }

    #[test]
    fn test_mat4_mul_asm_known() {
        // A = [[1, 0, 0, 0], [0, 2, 0, 0], [0, 0, 3, 0], [0, 0, 0, 4]] (行主序视角)
        // 列主序存储: col0=[1,0,0,0], col1=[0,2,0,0], col2=[0,0,3,0], col3=[0,0,0,4]
        let a: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 3.0, 0.0,
            0.0, 0.0, 0.0, 4.0,
        ];
        // B = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]] (行主序视角)
        // 列主序存储: col0=[1,5,9,13], col1=[2,6,10,14], col2=[3,7,11,15], col3=[4,8,12,16]
        let b: [f32; 16] = [
            1.0, 5.0, 9.0, 13.0,
            2.0, 6.0, 10.0, 14.0,
            3.0, 7.0, 11.0, 15.0,
            4.0, 8.0, 12.0, 16.0,
        ];
        let mut c: [f32; 16] = [0.0; 16];
        unsafe { mat4_mul_asm(&a, &b, &mut c) };
        // C = A * B (行主序视角)
        // C[0,:] = [1*1, 1*2, 1*3, 1*4] = [1, 2, 3, 4]
        // C[1,:] = [2*5, 2*6, 2*7, 2*8] = [10, 12, 14, 16]
        // C[2,:] = [3*9, 3*10, 3*11, 3*12] = [27, 30, 33, 36]
        // C[3,:] = [4*13, 4*14, 4*15, 4*16] = [52, 56, 60, 64]
        // 列主序存储: col0=[1,10,27,52], col1=[2,12,30,56], col2=[3,14,33,60], col3=[4,16,36,64]
        let expected: [f32; 16] = [
            1.0, 10.0, 27.0, 52.0,
            2.0, 12.0, 30.0, 56.0,
            3.0, 14.0, 33.0, 60.0,
            4.0, 16.0, 36.0, 64.0,
        ];
        for k in 0..16 {
            assert!((c[k] - expected[k]).abs() < 1e-4, "k={}: c={} expected={}", k, c[k], expected[k]);
        }
    }

    #[test]
    fn test_mat4_vec_mul_asm() {
        // A = identity
        let mut a: [f32; 16] = [0.0; 16];
        a[0] = 1.0;
        a[5] = 1.0;
        a[10] = 1.0;
        a[15] = 1.0;
        let x: [f32; 4] = [3.0, 5.0, 7.0, 11.0];
        let mut y: [f32; 4] = [0.0; 4];
        unsafe { mat4_vec_mul_asm(&a, &x, &mut y) };
        for k in 0..4 {
            assert!((y[k] - x[k]).abs() < 1e-5, "k={}: y={} x={}", k, y[k], x[k]);
        }
    }

    #[test]
    fn test_mat4_vec_mul_asm_known() {
        // A 行主序: [[1, 0, 0, 0], [0, 2, 0, 0], [0, 0, 3, 0], [0, 0, 0, 4]]
        // 列主序: col0=[1,0,0,0], col1=[0,2,0,0], col2=[0,0,3,0], col3=[0,0,0,4]
        let a: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 3.0, 0.0,
            0.0, 0.0, 0.0, 4.0,
        ];
        let x: [f32; 4] = [3.0, 5.0, 7.0, 11.0];
        let mut y: [f32; 4] = [0.0; 4];
        unsafe { mat4_vec_mul_asm(&a, &x, &mut y) };
        // y = A * x = [1*3, 2*5, 3*7, 4*11] = [3, 10, 21, 44]
        let expected: [f32; 4] = [3.0, 10.0, 21.0, 44.0];
        for k in 0..4 {
            assert!((y[k] - expected[k]).abs() < 1e-4, "k={}: y={} expected={}", k, y[k], expected[k]);
        }
    }

    #[test]
    fn test_mat4_transpose_asm_identity() {
        let src: [f32; 16] = [
            1.0, 2.0, 3.0, 4.0,   // col 0
            5.0, 6.0, 7.0, 8.0,   // col 1
            9.0, 10.0, 11.0, 12.0, // col 2
            13.0, 14.0, 15.0, 16.0, // col 3
        ];
        let mut dst: [f32; 16] = [0.0; 16];
        unsafe { mat4_transpose_asm(&src, &mut dst) };
        // 转置后 col0 = src 的行 0 = [1, 5, 9, 13]
        // col1 = src 的行 1 = [2, 6, 10, 14]
        // col2 = src 的行 2 = [3, 7, 11, 15]
        // col3 = src 的行 3 = [4, 8, 12, 16]
        let expected: [f32; 16] = [
            1.0, 5.0, 9.0, 13.0,
            2.0, 6.0, 10.0, 14.0,
            3.0, 7.0, 11.0, 15.0,
            4.0, 8.0, 12.0, 16.0,
        ];
        for k in 0..16 {
            assert!((dst[k] - expected[k]).abs() < 1e-5, "k={}: dst={} expected={}", k, dst[k], expected[k]);
        }
    }

    #[test]
    fn test_mat4_diag_mul_asm() {
        let d: [f32; 4] = [2.0, 3.0, 4.0, 5.0];
        let a: [f32; 16] = [
            1.0, 2.0, 3.0, 4.0,   // col 0
            5.0, 6.0, 7.0, 8.0,   // col 1
            9.0, 10.0, 11.0, 12.0, // col 2
            13.0, 14.0, 15.0, 16.0, // col 3
        ];
        let mut y: [f32; 16] = [0.0; 16];
        unsafe { mat4_diag_mul_asm(&d, &a, &mut y) };
        // y[i, j] = d[i] * a[i, j]
        // col0: [2*1, 3*2, 4*3, 5*4] = [2, 6, 12, 20]
        // col1: [2*5, 3*6, 4*7, 5*8] = [10, 18, 28, 40]
        // col2: [2*9, 3*10, 4*11, 5*12] = [18, 30, 44, 60]
        // col3: [2*13, 3*14, 4*15, 5*16] = [26, 42, 60, 80]
        let expected: [f32; 16] = [
            2.0, 6.0, 12.0, 20.0,
            10.0, 18.0, 28.0, 40.0,
            18.0, 30.0, 44.0, 60.0,
            26.0, 42.0, 60.0, 80.0,
        ];
        for k in 0..16 {
            assert!((y[k] - expected[k]).abs() < 1e-4, "k={}: y={} expected={}", k, y[k], expected[k]);
        }
    }

    #[test]
    fn test_mat4_transpose_double_is_identity() {
        let src: [f32; 16] = [
            1.0, 2.0, 3.0, 4.0,
            5.0, 6.0, 7.0, 8.0,
            9.0, 10.0, 11.0, 12.0,
            13.0, 14.0, 15.0, 16.0,
        ];
        let mut tmp: [f32; 16] = [0.0; 16];
        let mut dst: [f32; 16] = [0.0; 16];
        unsafe {
            mat4_transpose_asm(&src, &mut tmp);
            mat4_transpose_asm(&tmp, &mut dst);
        }
        for k in 0..16 {
            assert!((dst[k] - src[k]).abs() < 1e-5, "k={}: dst={} src={}", k, dst[k], src[k]);
        }
    }

    #[test]
    fn test_mat4_mul_asm_against_sequential() {
        // 与朴素实现对比
        let a: [f32; 16] = [
            1.0, 2.0, 3.0, 4.0,
            5.0, 6.0, 7.0, 8.0,
            9.0, 10.0, 11.0, 12.0,
            13.0, 14.0, 15.0, 16.0,
        ];
        let b: [f32; 16] = [
            17.0, 18.0, 19.0, 20.0,
            21.0, 22.0, 23.0, 24.0,
            25.0, 26.0, 27.0, 28.0,
            29.0, 30.0, 31.0, 32.0,
        ];
        let mut c_asm: [f32; 16] = [0.0; 16];
        let mut c_ref: [f32; 16] = [0.0; 16];
        unsafe { mat4_mul_asm(&a, &b, &mut c_asm) };
        // 朴素列主序矩阵乘法: C[i, j] = sum_k A[i, k] * B[k, j]
        // A[i, k] = a[k*4 + i], B[k, j] = b[j*4 + k], C[i, j] = c[j*4 + i]
        for j in 0..4 {
            for i in 0..4 {
                let mut sum = 0.0f32;
                for k in 0..4 {
                    let aik = a[k * 4 + i];
                    let bkj = b[j * 4 + k];
                    sum += aik * bkj;
                }
                c_ref[j * 4 + i] = sum;
            }
        }
        for k in 0..16 {
            assert!((c_asm[k] - c_ref[k]).abs() < 1e-3, "k={}: asm={} ref={}", k, c_asm[k], c_ref[k]);
        }
    }
}
