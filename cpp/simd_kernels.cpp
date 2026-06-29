// simd_kernels.cpp — C++ AVX2 批量物理积分内核
//
// 用途: 演示 Rust ↔ C++ 多语言集成，提供高性能批量物理积分
// 编译: 通过 wasteland_physics/build.rs 用 cc crate 调 g++ 编译
// 调用: Rust 通过 extern "C" FFI 调用
//
// 算法: 显式 Euler 积分（力 → 速度 → 位置）
//   v_{n+1} = v_n + (F/m) * dt
//   x_{n+1} = x_n + v_{n+1} * dt
//
// 优化: AVX2 8 路并行 + FMA 融合乘加

#include <cstdint>
#include <immintrin.h>

extern "C" {

// SoA 布局批量积分（AVX2 8 路 f32 并行）
// 参数为分量连续数组: px[N], py[N], pz[N], ...
void physics_step_soa_avx2(
    float* px, float* py, float* pz,
    float* vx, float* vy, float* vz,
    const float* fx, const float* fy, const float* fz,
    const float* masses,
    float dt,
    uint32_t n
) {
    const __m256 v_dt = _mm256_set1_ps(dt);
    uint32_t i = 0;

    for (; i + 8 <= n; i += 8) {
        __m256 m = _mm256_loadu_ps(&masses[i]);
        // Newton-Raphson 精化倒数
        __m256 inv_m = _mm256_rcp_ps(m);
        inv_m = _mm256_add_ps(
            inv_m,
            _mm256_mul_ps(inv_m, _mm256_fnmadd_ps(m, inv_m, _mm256_set1_ps(1.0f)))
        );
        __m256 dt_inv_m = _mm256_mul_ps(v_dt, inv_m);

        // X 分量
        __m256 f_x = _mm256_loadu_ps(&fx[i]);
        __m256 v_x = _mm256_loadu_ps(&vx[i]);
        v_x = _mm256_fmadd_ps(f_x, dt_inv_m, v_x);
        _mm256_storeu_ps(&vx[i], v_x);
        __m256 p_x = _mm256_loadu_ps(&px[i]);
        p_x = _mm256_fmadd_ps(v_x, v_dt, p_x);
        _mm256_storeu_ps(&px[i], p_x);

        // Y 分量
        __m256 f_y = _mm256_loadu_ps(&fy[i]);
        __m256 v_y = _mm256_loadu_ps(&vy[i]);
        v_y = _mm256_fmadd_ps(f_y, dt_inv_m, v_y);
        _mm256_storeu_ps(&vy[i], v_y);
        __m256 p_y = _mm256_loadu_ps(&py[i]);
        p_y = _mm256_fmadd_ps(v_y, v_dt, p_y);
        _mm256_storeu_ps(&py[i], p_y);

        // Z 分量
        __m256 f_z = _mm256_loadu_ps(&fz[i]);
        __m256 v_z = _mm256_loadu_ps(&vz[i]);
        v_z = _mm256_fmadd_ps(f_z, dt_inv_m, v_z);
        _mm256_storeu_ps(&vz[i], v_z);
        __m256 p_z = _mm256_loadu_ps(&pz[i]);
        p_z = _mm256_fmadd_ps(v_z, v_dt, p_z);
        _mm256_storeu_ps(&pz[i], p_z);
    }

    // 标量尾处理
    for (; i < n; ++i) {
        float inv_m = 1.0f / masses[i];
        vx[i] += fx[i] * inv_m * dt;
        vy[i] += fy[i] * inv_m * dt;
        vz[i] += fz[i] * inv_m * dt;
        px[i] += vx[i] * dt;
        py[i] += vy[i] * dt;
        pz[i] += vz[i] * dt;
    }
}

// 批量 AABB 重叠测试（broad phase 加速）
// aabb: N*6 数组 (min_x, min_y, min_z, max_x, max_y, max_z) 交错
// 返回重叠对数
uint32_t count_aabb_overlaps(const float* aabbs, uint32_t n) {
    uint32_t count = 0;
    for (uint32_t i = 0; i < n; ++i) {
        float iminx = aabbs[i * 6 + 0];
        float iminy = aabbs[i * 6 + 1];
        float iminz = aabbs[i * 6 + 2];
        float imaxx = aabbs[i * 6 + 3];
        float imaxy = aabbs[i * 6 + 4];
        float imaxz = aabbs[i * 6 + 5];
        for (uint32_t j = i + 1; j < n; ++j) {
            float jminx = aabbs[j * 6 + 0];
            float jminy = aabbs[j * 6 + 1];
            float jminz = aabbs[j * 6 + 2];
            float jmaxx = aabbs[j * 6 + 3];
            float jmaxy = aabbs[j * 6 + 4];
            float jmaxz = aabbs[j * 6 + 5];
            // AABB 重叠条件: 三轴都有重叠
            if (imaxx >= jminx && jmaxx >= iminx &&
                imaxy >= jminy && jmaxy >= iminy &&
                imaxz >= jminz && jmaxz >= iminz) {
                ++count;
            }
        }
    }
    return count;
}

}  // extern "C"
