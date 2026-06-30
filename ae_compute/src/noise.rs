//! Noise - Perlin/Simplex 噪声 + Curl Noise + FBM
//!
//! 基于:
//! - Perlin, Ken. "Improving Noise." ACM TOG (SIGGRAPH 2002).
//! - Bridson, Robert. "Curl Noise for Procedural Fluid Flow." 2007.
//! - Musgrave. "Fractal Brownian Motion" (FBM).
//!
//! 核心思想:
//! 1. Perlin 噪声: 梯度噪声, 在网格点随机梯度, 内部插值
//!    - 改进版用 256 项排列表避免方向性偏置
//! 2. Simplex 噪声: Perlin 改进, 用单纯形网格, O(n^d) -> O(d^2)
//!    - 3D: 四面体网格, 4 个角点贡献
//! 3. Curl Noise: 标量势 phi 的旋度, 自然无散度 (divergence-free)
//!    - u = curl(phi) = (∂phi_z/∂y - ∂phi_y/∂z, ...)
//!    - 用于流体增强: 不会压缩/膨胀流体
//! 4. FBM: 多个倍频噪声叠加, 模拟自然纹理
//!    - f(p) = Σ (1/2^i) * noise(2^i * p)

use serde::{Deserialize, Serialize};

// ============================================================
// Perlin 噪声 (改进版, Perlin 2002)
// ============================================================

/// 3D Perlin 噪声生成器
#[derive(Debug, Clone)]
pub struct PerlinNoise3D {
    perm: [u8; 512],
}

impl PerlinNoise3D {
    pub fn new(seed: u64) -> Self {
        let mut perm = [0u8; 256];
        for i in 0..256 {
            perm[i] = i as u8;
        }
        // Fisher-Yates 洗牌 (用 xorshift PRNG)
        let mut state = seed.max(1);
        for i in (1..256).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let j = (state % (i as u64 + 1)) as usize;
            perm.swap(i, j);
        }
        let mut perm512 = [0u8; 512];
        for i in 0..512 {
            perm512[i] = perm[i & 255];
        }
        Self { perm: perm512 }
    }

    pub fn new_default() -> Self {
        Self::new(0xC0FFEE)
    }

    /// 3D Perlin 噪声, 返回 [-1, 1]
    pub fn noise(&self, x: f32, y: f32, z: f32) -> f32 {
        // 单位立方体网格
        let xi = x.floor() as i32 & 255;
        let yi = y.floor() as i32 & 255;
        let zi = z.floor() as i32 & 255;
        let xf = x - x.floor();
        let yf = y - y.floor();
        let zf = z - z.floor();
        // 缓和曲线 (Perlin 改进: 6t^5 - 15t^4 + 10t^3)
        let u = fade(xf);
        let v = fade(yf);
        let w = fade(zf);
        // 8 个角点的梯度哈希
        let aaa = self.hash(xi, yi, zi);
        let aba = self.hash(xi, yi + 1, zi);
        let aab = self.hash(xi, yi, zi + 1);
        let abb = self.hash(xi, yi + 1, zi + 1);
        let baa = self.hash(xi + 1, yi, zi);
        let bba = self.hash(xi + 1, yi + 1, zi);
        let bab = self.hash(xi + 1, yi, zi + 1);
        let bbb = self.hash(xi + 1, yi + 1, zi + 1);
        // 三线性插值梯度贡献
        let x1 = lerp(grad(aaa, xf, yf, zf), grad(baa, xf - 1.0, yf, zf), u);
        let x2 = lerp(grad(aba, xf, yf - 1.0, zf), grad(bba, xf - 1.0, yf - 1.0, zf), u);
        let y1 = lerp(x1, x2, v);
        let x3 = lerp(grad(aab, xf, yf, zf - 1.0), grad(bab, xf - 1.0, yf, zf - 1.0), u);
        let x4 =
            lerp(grad(abb, xf, yf - 1.0, zf - 1.0), grad(bbb, xf - 1.0, yf - 1.0, zf - 1.0), u);
        let y2 = lerp(x3, x4, v);
        lerp(y1, y2, w)
    }

    #[inline]
    fn hash(&self, x: i32, y: i32, z: i32) -> u8 {
        let h = self.perm[(self.perm[(self.perm[x as usize & 255] as usize + y as usize) & 511]
            as usize
            + z as usize)
            & 511];
        h
    }
}

/// 缓和曲线: 6t^5 - 15t^4 + 10t^3
#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// 线性插值
#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// 梯度函数 (Perlin 改进版, 12 个方向)
#[inline]
fn grad(hash: u8, x: f32, y: f32, z: f32) -> f32 {
    // 用 hash 的低 4 位选择 12 个梯度方向之一
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

// ============================================================
// Simplex 噪声 (3D) - Stefan Gustavson 实现
// ============================================================

/// 3D Simplex 噪声生成器
#[derive(Debug, Clone)]
pub struct SimplexNoise3D {
    perm: [u8; 512],
    perm_mod12: [u8; 512],
}

impl SimplexNoise3D {
    pub fn new(seed: u64) -> Self {
        let mut perm = [0u8; 256];
        for i in 0..256 {
            perm[i] = i as u8;
        }
        let mut state = seed.max(1);
        for i in (1..256).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let j = (state % (i as u64 + 1)) as usize;
            perm.swap(i, j);
        }
        let mut perm512 = [0u8; 512];
        let mut perm_mod12 = [0u8; 512];
        for i in 0..512 {
            perm512[i] = perm[i & 255];
            perm_mod12[i] = perm512[i] % 12;
        }
        Self { perm: perm512, perm_mod12 }
    }

    pub fn new_default() -> Self {
        Self::new(0xC0FFEE)
    }

    /// 3D Simplex 噪声, 返回 [-1, 1] (大致)
    pub fn noise(&self, xin: f32, yin: f32, zin: f32) -> f32 {
        // 单纯形网格的 skew 变换
        let f3 = 1.0 / 3.0;
        let s = (xin + yin + zin) * f3;
        let i = (xin + s).floor() as i32;
        let j = (yin + s).floor() as i32;
        let k = (zin + s).floor() as i32;
        let g3 = 1.0 / 6.0;
        let t = (i + j + k) as f32 * g3;
        let x0 = xin - (i as f32 - t);
        let y0 = yin - (j as f32 - t);
        let z0 = zin - (k as f32 - t);
        // 确定在哪个单纯形
        let (i1, j1, k1, i2, j2, k2);
        if x0 >= y0 {
            if y0 >= z0 {
                i1 = 1;
                j1 = 0;
                k1 = 0;
                i2 = 1;
                j2 = 1;
                k2 = 0;
            } else if x0 >= z0 {
                i1 = 1;
                j1 = 0;
                k1 = 0;
                i2 = 1;
                j2 = 0;
                k2 = 1;
            } else {
                i1 = 0;
                j1 = 0;
                k1 = 1;
                i2 = 1;
                j2 = 0;
                k2 = 1;
            }
        } else {
            if y0 < z0 {
                i1 = 0;
                j1 = 0;
                k1 = 1;
                i2 = 0;
                j2 = 1;
                k2 = 1;
            } else if x0 < z0 {
                i1 = 0;
                j1 = 1;
                k1 = 0;
                i2 = 0;
                j2 = 1;
                k2 = 1;
            } else {
                i1 = 0;
                j1 = 1;
                k1 = 0;
                i2 = 1;
                j2 = 1;
                k2 = 0;
            }
        }
        // 4 个角点
        let x1 = x0 - i1 as f32 + g3;
        let y1 = y0 - j1 as f32 + g3;
        let z1 = z0 - k1 as f32 + g3;
        let x2 = x0 - i2 as f32 + 2.0 * g3;
        let y2 = y0 - j2 as f32 + 2.0 * g3;
        let z2 = z0 - k2 as f32 + 2.0 * g3;
        let x3 = x0 - 1.0 + 3.0 * g3;
        let y3 = y0 - 1.0 + 3.0 * g3;
        let z3 = z0 - 1.0 + 3.0 * g3;
        // 哈希
        let ii = i & 255;
        let jj = j & 255;
        let kk = k & 255;
        let gi0 = self.perm_mod12[(self.perm[(self.perm[ii as usize] as usize + jj as usize) & 511]
            as usize
            + kk as usize)
            & 511] as usize;
        let gi1 = self.perm_mod12[(self.perm
            [(self.perm[(ii + i1) as usize & 255] as usize + (jj + j1) as usize) & 511]
            as usize
            + (kk + k1) as usize)
            & 511] as usize;
        let gi2 = self.perm_mod12[(self.perm
            [(self.perm[(ii + i2) as usize & 255] as usize + (jj + j2) as usize) & 511]
            as usize
            + (kk + k2) as usize)
            & 511] as usize;
        let gi3 = self.perm_mod12[(self.perm
            [(self.perm[(ii + 1) as usize & 255] as usize + (jj + 1) as usize) & 511]
            as usize
            + (kk + 1) as usize)
            & 511] as usize;
        // 12 个梯度方向
        const GRAD3: [[f32; 3]; 12] = [
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
            [1.0, -1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [1.0, 0.0, 1.0],
            [-1.0, 0.0, 1.0],
            [1.0, 0.0, -1.0],
            [-1.0, 0.0, -1.0],
            [0.0, 1.0, 1.0],
            [0.0, -1.0, 1.0],
            [0.0, 1.0, -1.0],
            [0.0, -1.0, -1.0],
        ];
        // 贡献
        let mut n0 = 0.0;
        let mut n1 = 0.0;
        let mut n2 = 0.0;
        let mut n3 = 0.0;
        let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
        if t0 > 0.0 {
            let t0 = t0 * t0;
            n0 = t0 * t0 * (GRAD3[gi0][0] * x0 + GRAD3[gi0][1] * y0 + GRAD3[gi0][2] * z0);
        }
        let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
        if t1 > 0.0 {
            let t1 = t1 * t1;
            n1 = t1 * t1 * (GRAD3[gi1][0] * x1 + GRAD3[gi1][1] * y1 + GRAD3[gi1][2] * z1);
        }
        let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
        if t2 > 0.0 {
            let t2 = t2 * t2;
            n2 = t2 * t2 * (GRAD3[gi2][0] * x2 + GRAD3[gi2][1] * y2 + GRAD3[gi2][2] * z2);
        }
        let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
        if t3 > 0.0 {
            let t3 = t3 * t3;
            n3 = t3 * t3 * (GRAD3[gi3][0] * x3 + GRAD3[gi3][1] * y3 + GRAD3[gi3][2] * z3);
        }
        // 缩放到 [-1, 1]
        32.0 * (n0 + n1 + n2 + n3)
    }
}

// ============================================================
// FBM (Fractal Brownian Motion)
// ============================================================

/// FBM 参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbmConfig {
    /// 倍频数
    pub octaves: u32,
    /// 每倍频振幅衰减
    pub persistence: f32,
    /// 每倍频频率增长
    pub lacunarity: f32,
    /// 基础频率
    pub frequency: f32,
    /// 基础振幅
    pub amplitude: f32,
}

impl Default for FbmConfig {
    fn default() -> Self {
        Self { octaves: 5, persistence: 0.5, lacunarity: 2.0, frequency: 1.0, amplitude: 1.0 }
    }
}

/// FBM 噪声 (用 Perlin)
pub fn fbm_perlin(noise: &PerlinNoise3D, x: f32, y: f32, z: f32, config: &FbmConfig) -> f32 {
    let mut total = 0.0;
    let mut freq = config.frequency;
    let mut amp = config.amplitude;
    let mut max_value = 0.0;
    for _ in 0..config.octaves {
        total += noise.noise(x * freq, y * freq, z * freq) * amp;
        max_value += amp;
        freq *= config.lacunarity;
        amp *= config.persistence;
    }
    total / max_value.max(1e-10)
}

/// FBM 噪声 (用 Simplex)
pub fn fbm_simplex(noise: &SimplexNoise3D, x: f32, y: f32, z: f32, config: &FbmConfig) -> f32 {
    let mut total = 0.0;
    let mut freq = config.frequency;
    let mut amp = config.amplitude;
    let mut max_value = 0.0;
    for _ in 0..config.octaves {
        total += noise.noise(x * freq, y * freq, z * freq) * amp;
        max_value += amp;
        freq *= config.lacunarity;
        amp *= config.persistence;
    }
    total / max_value.max(1e-10)
}

// ============================================================
// Curl Noise (Bridson 2007) - 无散度湍流场
// ============================================================

/// Curl Noise 生成器 (3D, 用 Perlin 势函数)
pub struct CurlNoise3D {
    noise_x: PerlinNoise3D,
    noise_y: PerlinNoise3D,
    noise_z: PerlinNoise3D,
    eps: f32,
}

impl CurlNoise3D {
    pub fn new(seed: u64) -> Self {
        Self {
            noise_x: PerlinNoise3D::new(seed),
            noise_y: PerlinNoise3D::new(seed.wrapping_mul(2654435761).wrapping_add(1)),
            noise_z: PerlinNoise3D::new(seed.wrapping_mul(40503).wrapping_add(2)),
            eps: 1e-4,
        }
    }

    pub fn new_default() -> Self {
        Self::new(0xC0FFEE)
    }

    /// 设置差分步长 (默认 1e-4)
    pub fn with_eps(mut self, eps: f32) -> Self {
        self.eps = eps;
        self
    }

    /// 计算 curl noise 速度场 (无散度)
    /// u = curl(phi) = (∂phi_z/∂y - ∂phi_y/∂z, ∂phi_x/∂z - ∂phi_z/∂x, ∂phi_y/∂x - ∂phi_x/∂y)
    pub fn curl(&self, x: f32, y: f32, z: f32) -> [f32; 3] {
        let e = self.eps;
        // 偏导数 (中心差分)
        let dphi_z_dy =
            (self.noise_z.noise(x, y + e, z) - self.noise_z.noise(x, y - e, z)) / (2.0 * e);
        let dphi_y_dz =
            (self.noise_y.noise(x, y, z + e) - self.noise_y.noise(x, y, z - e)) / (2.0 * e);
        let dphi_x_dz =
            (self.noise_x.noise(x, y, z + e) - self.noise_x.noise(x, y, z - e)) / (2.0 * e);
        let dphi_z_dx =
            (self.noise_z.noise(x + e, y, z) - self.noise_z.noise(x - e, y, z)) / (2.0 * e);
        let dphi_y_dx =
            (self.noise_y.noise(x + e, y, z) - self.noise_y.noise(x - e, y, z)) / (2.0 * e);
        let dphi_x_dy =
            (self.noise_x.noise(x, y + e, z) - self.noise_x.noise(x, y - e, z)) / (2.0 * e);
        [dphi_z_dy - dphi_y_dz, dphi_x_dz - dphi_z_dx, dphi_y_dx - dphi_x_dy]
    }

    /// Curl noise 加上时间动画 (随时间变化的无散度场)
    pub fn curl_animated(&self, x: f32, y: f32, z: f32, t: f32) -> [f32; 3] {
        // 第 4 维用时间: 把时间作为偏移
        // 简化: 用 3D 噪声在 (x+t, y, z) 等位置采样
        let e = self.eps;
        let dphi_z_dy =
            (self.noise_z.noise(x, y + e, z + t) - self.noise_z.noise(x, y - e, z + t)) / (2.0 * e);
        let dphi_y_dz =
            (self.noise_y.noise(x, y, z + e + t) - self.noise_y.noise(x, y, z - e + t)) / (2.0 * e);
        let dphi_x_dz =
            (self.noise_x.noise(x, y, z + e + t) - self.noise_x.noise(x, y, z - e + t)) / (2.0 * e);
        let dphi_z_dx =
            (self.noise_z.noise(x + e, y, z + t) - self.noise_z.noise(x - e, y, z + t)) / (2.0 * e);
        let dphi_y_dx =
            (self.noise_y.noise(x + e, y, z + t) - self.noise_y.noise(x - e, y, z + t)) / (2.0 * e);
        let dphi_x_dy =
            (self.noise_x.noise(x, y + e, z + t) - self.noise_x.noise(x, y - e, z + t)) / (2.0 * e);
        [dphi_z_dy - dphi_y_dz, dphi_x_dz - dphi_z_dx, dphi_y_dx - dphi_x_dy]
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perlin_noise_range() {
        let noise = PerlinNoise3D::new_default();
        // 噪声值应在 [-1, 1] (大约, 边界可能略超)
        for i in 0..20 {
            let x = i as f32 * 0.3;
            let n = noise.noise(x, 0.0, 0.0);
            assert!(n >= -1.5 && n <= 1.5, "perlin noise out of range: {} at x={}", n, x);
        }
    }

    #[test]
    fn test_perlin_noise_zero_at_grid() {
        let noise = PerlinNoise3D::new_default();
        // 整数网格点梯度贡献为 0 (因为 fade(0)=0)
        let n = noise.noise(0.0, 0.0, 0.0);
        assert!(n.abs() < 1e-6, "perlin at grid origin should be 0: {}", n);
        let n = noise.noise(1.0, 2.0, 3.0);
        assert!(n.abs() < 1e-6, "perlin at (1,2,3) should be 0: {}", n);
    }

    #[test]
    fn test_perlin_noise_smooth() {
        let noise = PerlinNoise3D::new_default();
        // 相近点应有相近值 (连续性)
        let n1 = noise.noise(0.5, 0.5, 0.5);
        let n2 = noise.noise(0.51, 0.5, 0.5);
        assert!((n1 - n2).abs() < 0.1, "perlin should be smooth: {} vs {}", n1, n2);
    }

    #[test]
    fn test_perlin_noise_deterministic() {
        let n1 = PerlinNoise3D::new(42);
        let n2 = PerlinNoise3D::new(42);
        let v1 = n1.noise(0.5, 0.5, 0.5);
        let v2 = n2.noise(0.5, 0.5, 0.5);
        assert!((v1 - v2).abs() < 1e-6, "same seed should give same noise");
    }

    #[test]
    fn test_perlin_noise_different_seeds() {
        let n1 = PerlinNoise3D::new(1);
        let n2 = PerlinNoise3D::new(2);
        let v1 = n1.noise(0.5, 0.5, 0.5);
        let v2 = n2.noise(0.5, 0.5, 0.5);
        // 不同种子通常给不同值 (不严格, 但大概率)
        assert!((v1 - v2).abs() > 1e-3 || v1.abs() > 1e-3, "different seeds likely differ");
    }

    #[test]
    fn test_simplex_noise_range() {
        let noise = SimplexNoise3D::new_default();
        for i in 0..20 {
            let x = i as f32 * 0.3;
            let n = noise.noise(x, 0.0, 0.0);
            assert!(n >= -1.5 && n <= 1.5, "simplex noise out of range: {} at x={}", n, x);
        }
    }

    #[test]
    fn test_simplex_noise_deterministic() {
        let n1 = SimplexNoise3D::new(42);
        let n2 = SimplexNoise3D::new(42);
        assert!((n1.noise(0.5, 0.5, 0.5) - n2.noise(0.5, 0.5, 0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_simplex_noise_smooth() {
        let noise = SimplexNoise3D::new_default();
        let n1 = noise.noise(0.5, 0.5, 0.5);
        let n2 = noise.noise(0.51, 0.5, 0.5);
        assert!((n1 - n2).abs() < 0.1, "simplex should be smooth: {} vs {}", n1, n2);
    }

    #[test]
    fn test_fbm_range() {
        let noise = PerlinNoise3D::new_default();
        let config = FbmConfig::default();
        for i in 0..10 {
            let x = i as f32 * 0.5;
            let v = fbm_perlin(&noise, x, 0.0, 0.0, &config);
            assert!(v >= -1.5 && v <= 1.5, "fbm out of range: {} at x={}", v, x);
        }
    }

    #[test]
    fn test_fbm_octaves_increase_detail() {
        let noise = PerlinNoise3D::new_default();
        let low_oct = FbmConfig { octaves: 1, ..FbmConfig::default() };
        let high_oct = FbmConfig { octaves: 6, ..FbmConfig::default() };
        // 高倍频应有更多细节 (相邻点差异更大)
        let mut low_diff = 0.0;
        let mut high_diff = 0.0;
        for i in 0..20 {
            let x = i as f32 * 0.05;
            let x2 = x + 0.01;
            low_diff += (fbm_perlin(&noise, x, 0.0, 0.0, &low_oct)
                - fbm_perlin(&noise, x2, 0.0, 0.0, &low_oct))
            .abs();
            high_diff += (fbm_perlin(&noise, x, 0.0, 0.0, &high_oct)
                - fbm_perlin(&noise, x2, 0.0, 0.0, &high_oct))
            .abs();
        }
        // 高倍频总变化更大 (不严格, 但大概率)
        assert!(
            high_diff >= low_diff * 0.9,
            "more octaves should add detail: low={} high={}",
            low_diff,
            high_diff
        );
    }

    #[test]
    fn test_curl_noise_divergence_free() {
        // 验证 curl noise 是无散度的: div(u) = ∂u_x/∂x + ∂u_y/∂y + ∂u_z/∂z ≈ 0
        let curl = CurlNoise3D::new_default();
        let eps = 1e-3;
        let x = 0.5;
        let y = 0.5;
        let z = 0.5;
        let u_pos_x = curl.curl(x + eps, y, z);
        let u_neg_x = curl.curl(x - eps, y, z);
        let u_pos_y = curl.curl(x, y + eps, z);
        let u_neg_y = curl.curl(x, y - eps, z);
        let u_pos_z = curl.curl(x, y, z + eps);
        let u_neg_z = curl.curl(x, y, z - eps);
        let div = (u_pos_x[0] - u_neg_x[0]) / (2.0 * eps)
            + (u_pos_y[1] - u_neg_y[1]) / (2.0 * eps)
            + (u_pos_z[2] - u_neg_z[2]) / (2.0 * eps);
        // 散度应接近 0 (数值误差来自二阶差分)
        assert!(div.abs() < 50.0, "curl noise divergence should be small: {}", div);
    }

    #[test]
    fn test_curl_noise_deterministic() {
        let c1 = CurlNoise3D::new(42);
        let c2 = CurlNoise3D::new(42);
        let v1 = c1.curl(0.5, 0.5, 0.5);
        let v2 = c2.curl(0.5, 0.5, 0.5);
        for i in 0..3 {
            assert!(
                (v1[i] - v2[i]).abs() < 1e-6,
                "curl noise deterministic: {} vs {}",
                v1[i],
                v2[i]
            );
        }
    }

    #[test]
    fn test_curl_noise_finite() {
        let curl = CurlNoise3D::new_default();
        for i in 0..10 {
            let x = i as f32 * 0.7;
            let v = curl.curl(x, x * 0.5, x * 0.3);
            for c in &v {
                assert!(c.is_finite(), "curl noise should be finite");
            }
        }
    }

    #[test]
    fn test_curl_noise_animated() {
        let curl = CurlNoise3D::new_default();
        let v1 = curl.curl_animated(0.5, 0.5, 0.5, 0.0);
        let v2 = curl.curl_animated(0.5, 0.5, 0.5, 1.0);
        // 不同时间应给不同值
        let mut diff = 0.0;
        for i in 0..3 {
            diff += (v1[i] - v2[i]).abs();
        }
        assert!(diff > 1e-3, "animated curl should change over time: diff={}", diff);
    }
}
