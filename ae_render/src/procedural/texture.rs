//! 程序化 PBR 贴图生成
//!
//! 零外部依赖的 PBR 贴图生成系统：
//! - `NoiseType`：Value/Worley/Fractal/Perlin-style noise
//! - `TextureGenerator`：生成 albedo/normal/roughness/metallic/AO/height 全套 PBR 贴图
//! - 物理参数映射（材质类型 → PBR 参数范围）
//!
//! 输出 RGBA8 格式，可直接上传到 GPU 纹理

use crate::texture::TextureFormat;

/// 噪声参数
#[derive(Debug, Clone)]
pub struct NoiseParams {
    pub noise_type: NoiseType,
    pub scale: f32,
    pub octaves: u32,
    pub persistence: f32,
    pub lacunarity: f32,
    pub seed: u64,
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            noise_type: NoiseType::Fractal,
            scale: 4.0,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            seed: 12345,
        }
    }
}

/// 噪声类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseType {
    /// 值噪声（最简单，方块感）
    Value,
    /// Worley/Cellular 噪声（细胞状，用于石头/皮革）
    Worley,
    /// 分形布朗运动（多层叠加，自然纹理）
    Fractal,
    /// 湍流（abs(sin) 叠加，云雾感）
    Turbulence,
}

/// PBR 贴图集合
#[derive(Debug, Clone)]
pub struct PbrTextureSet {
    pub width: u32,
    pub height: u32,
    /// 漫反射颜色（sRGB, RGBA8）
    pub albedo: Vec<u8>,
    /// 法线贴图（tangent space, RGBA8, xyz = normal, w = 1）
    pub normal: Vec<u8>,
    /// 粗糙度（R8, 单通道）
    pub roughness: Vec<u8>,
    /// 金属度（R8, 单通道）
    pub metallic: Vec<u8>,
    /// 环境遮蔽（R8, 单通道）
    pub ao: Vec<u8>,
    /// 高度贴图（R8, 单通道，用于视差）
    pub height_map: Vec<u8>,
}

/// 材质类型（驱动 PBR 参数）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialType {
    Wood,
    Stone,
    Concrete,
    Metal,
    Fabric,
    Skin,
    Brick,
    Plaster,
    Rust,
    Glass,
}

/// PBR 参数范围
#[derive(Debug, Clone, Copy)]
pub struct PbrRange {
    pub roughness_min: f32,
    pub roughness_max: f32,
    pub metallic_min: f32,
    pub metallic_max: f32,
    pub albedo_color: [f32; 3], // 基础颜色
    pub albedo_variation: f32,  // 颜色变化幅度
}

impl MaterialType {
    pub fn pbr_range(self) -> PbrRange {
        match self {
            MaterialType::Wood => PbrRange {
                roughness_min: 0.6,
                roughness_max: 0.9,
                metallic_min: 0.0,
                metallic_max: 0.0,
                albedo_color: [0.45, 0.30, 0.18],
                albedo_variation: 0.15,
            },
            MaterialType::Stone => PbrRange {
                roughness_min: 0.7,
                roughness_max: 0.95,
                metallic_min: 0.0,
                metallic_max: 0.0,
                albedo_color: [0.55, 0.52, 0.48],
                albedo_variation: 0.12,
            },
            MaterialType::Concrete => PbrRange {
                roughness_min: 0.75,
                roughness_max: 0.92,
                metallic_min: 0.0,
                metallic_max: 0.0,
                albedo_color: [0.65, 0.63, 0.60],
                albedo_variation: 0.08,
            },
            MaterialType::Metal => PbrRange {
                roughness_min: 0.2,
                roughness_max: 0.5,
                metallic_min: 0.85,
                metallic_max: 1.0,
                albedo_color: [0.75, 0.75, 0.78],
                albedo_variation: 0.05,
            },
            MaterialType::Fabric => PbrRange {
                roughness_min: 0.85,
                roughness_max: 0.98,
                metallic_min: 0.0,
                metallic_max: 0.0,
                albedo_color: [0.4, 0.4, 0.45],
                albedo_variation: 0.25,
            },
            MaterialType::Skin => PbrRange {
                roughness_min: 0.5,
                roughness_max: 0.7,
                metallic_min: 0.0,
                metallic_max: 0.0,
                albedo_color: [0.85, 0.70, 0.60],
                albedo_variation: 0.10,
            },
            MaterialType::Brick => PbrRange {
                roughness_min: 0.8,
                roughness_max: 0.95,
                metallic_min: 0.0,
                metallic_max: 0.0,
                albedo_color: [0.55, 0.30, 0.22],
                albedo_variation: 0.10,
            },
            MaterialType::Plaster => PbrRange {
                roughness_min: 0.7,
                roughness_max: 0.88,
                metallic_min: 0.0,
                metallic_max: 0.0,
                albedo_color: [0.80, 0.78, 0.72],
                albedo_variation: 0.06,
            },
            MaterialType::Rust => PbrRange {
                roughness_min: 0.85,
                roughness_max: 1.0,
                metallic_min: 0.0,
                metallic_max: 0.2,
                albedo_color: [0.50, 0.25, 0.12],
                albedo_variation: 0.20,
            },
            MaterialType::Glass => PbrRange {
                roughness_min: 0.02,
                roughness_max: 0.08,
                metallic_min: 0.0,
                metallic_max: 0.0,
                albedo_color: [0.85, 0.90, 0.95],
                albedo_variation: 0.02,
            },
        }
    }
}

/// 贴图生成器
pub struct TextureGenerator {
    width: u32,
    height: u32,
}

impl TextureGenerator {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// 生成完整 PBR 贴图集
    pub fn generate_pbr_set(
        &self,
        material: MaterialType,
        noise_params: &NoiseParams,
    ) -> PbrTextureSet {
        let pbr = material.pbr_range();
        let size = (self.width * self.height) as usize;

        let mut albedo = vec![0u8; size * 4];
        let mut normal = vec![0u8; size * 4];
        let mut roughness = vec![0u8; size];
        let mut metallic = vec![0u8; size];
        let mut ao = vec![0u8; size];
        let mut height = vec![0u8; size];

        // 预生成高度场（用于派生法线 + AO + 高度贴图）
        let height_field = self.generate_height_field(noise_params);

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let h = height_field[idx];

                // Albedo: 基础色 + 噪声变化
                let variation = self.value_noise_2d(
                    x as f32 * 0.3,
                    y as f32 * 0.3,
                    noise_params.seed.wrapping_add(1),
                ) * 2.0 - 1.0; // -1..1
                let v = variation * pbr.albedo_variation;
                albedo[idx * 4] = ((pbr.albedo_color[0] + v).clamp(0.0, 1.0) * 255.0) as u8;
                albedo[idx * 4 + 1] = ((pbr.albedo_color[1] + v).clamp(0.0, 1.0) * 255.0) as u8;
                albedo[idx * 4 + 2] = ((pbr.albedo_color[2] + v).clamp(0.0, 1.0) * 255.0) as u8;
                albedo[idx * 4 + 3] = 255;

                // Roughness: 范围映射 + 噪声调制
                let r_noise = self.value_noise_2d(
                    x as f32 * 0.5,
                    y as f32 * 0.5,
                    noise_params.seed.wrapping_add(2),
                );
                let r = pbr.roughness_min + (pbr.roughness_max - pbr.roughness_min) * r_noise;
                roughness[idx] = (r.clamp(0.0, 1.0) * 255.0) as u8;

                // Metallic: 范围映射 + 噪声调制
                let m_noise = self.value_noise_2d(
                    x as f32 * 0.8,
                    y as f32 * 0.8,
                    noise_params.seed.wrapping_add(3),
                );
                let m = pbr.metallic_min + (pbr.metallic_max - pbr.metallic_min) * m_noise;
                metallic[idx] = (m.clamp(0.0, 1.0) * 255.0) as u8;

                // Height: 直接从高度场
                height[idx] = (h.clamp(0.0, 1.0) * 255.0) as u8;

                // AO: 基于高度场局部对比度（简化版：高度低的地方更暗）
                let ao_val = (h * 0.5 + 0.5).clamp(0.3, 1.0);
                ao[idx] = (ao_val * 255.0) as u8;
            }
        }

        // 生成法线贴图（从高度场 Sobel 滤波）
        self.compute_normals_from_height(&height_field, &mut normal);

        PbrTextureSet {
            width: self.width,
            height: self.height,
            albedo,
            normal,
            roughness,
            metallic,
            ao,
            height_map: height,
        }
    }

    /// 生成高度场
    fn generate_height_field(&self, params: &NoiseParams) -> Vec<f32> {
        let size = (self.width * self.height) as usize;
        let mut field = vec![0.0f32; size];
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let nx = x as f32 / self.width as f32 * params.scale;
                let ny = y as f32 / self.height as f32 * params.scale;
                let h = match params.noise_type {
                    NoiseType::Value => self.value_noise_2d(nx, ny, params.seed),
                    NoiseType::Worley => self.worley_noise_2d(nx, ny, params.seed),
                    NoiseType::Fractal => self.fractal_noise(nx, ny, params),
                    NoiseType::Turbulence => self.turbulence_noise(nx, ny, params),
                };
                field[idx] = h;
            }
        }
        // 归一化到 0..1
        let min = field.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = field.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = (max - min).max(1e-6);
        for h in &mut field {
            *h = (*h - min) / range;
        }
        field
    }

    /// 值噪声（2D）
    fn value_noise_2d(&self, x: f32, y: f32, seed: u64) -> f32 {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;
        // 平滑插值（Hermite 曲线）
        let sx = fx * fx * (3.0 - 2.0 * fx);
        let sy = fy * fy * (3.0 - 2.0 * fy);
        let v00 = self.hash_2d(x0, y0, seed);
        let v10 = self.hash_2d(x1, y0, seed);
        let v01 = self.hash_2d(x0, y1, seed);
        let v11 = self.hash_2d(x1, y1, seed);
        let a = v00 * (1.0 - sx) + v10 * sx;
        let b = v01 * (1.0 - sx) + v11 * sx;
        a * (1.0 - sy) + b * sy
    }

    /// 哈希函数（返回 0..1）
    fn hash_2d(&self, x: i32, y: i32, seed: u64) -> f32 {
        let mut h = (x as u64).wrapping_mul(374761393)
            .wrapping_add((y as u64).wrapping_mul(668265263))
            .wrapping_add(seed);
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        h = h ^ (h >> 16);
        (h & 0x00FFFFFF) as f32 / 0x01000000 as f32
    }

    /// Worley/Cellular 噪声（2D）
    fn worley_noise_2d(&self, x: f32, y: f32, seed: u64) -> f32 {
        let cell_size = 1.0;
        let ix = (x / cell_size).floor() as i32;
        let iy = (y / cell_size).floor() as i32;
        let mut min_dist = f32::INFINITY;
        // 检查 3×3 邻域
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cx = ix + dx;
                let cy = iy + dy;
                // 每个 cell 内的随机点位置
                let px = cx as f32 + self.hash_2d(cx, cy, seed) * cell_size;
                let py = cy as f32 + self.hash_2d(cx, cy, seed.wrapping_add(1)) * cell_size;
                let dist = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
                if dist < min_dist {
                    min_dist = dist;
                }
            }
        }
        // 转换为 0..1（距离越近值越大）
        (1.0 - min_dist / (cell_size * 1.5)).max(0.0)
    }

    /// 分形布朗运动
    fn fractal_noise(&self, x: f32, y: f32, params: &NoiseParams) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;
        for _ in 0..params.octaves {
            value += self.value_noise_2d(x * frequency, y * frequency, params.seed) * amplitude;
            max_value += amplitude;
            amplitude *= params.persistence;
            frequency *= params.lacunarity;
        }
        value / max_value.max(1e-6)
    }

    /// 湍流噪声
    fn turbulence_noise(&self, x: f32, y: f32, params: &NoiseParams) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;
        for _ in 0..params.octaves {
            let n = self.value_noise_2d(x * frequency, y * frequency, params.seed);
            value += n.abs() * amplitude;
            max_value += amplitude;
            amplitude *= params.persistence;
            frequency *= params.lacunarity;
        }
        value / max_value.max(1e-6)
    }

    /// 从高度场计算法线（Sobel 滤波）
    fn compute_normals_from_height(&self, height: &[f32], output: &mut [u8]) {
        let w = self.width as i32;
        let h = self.height as i32;
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                // Sobel 卷积核
                let x_left = ((x - 1).max(0)) as u32;
                let x_right = ((x + 1).min(w - 1)) as u32;
                let y_up = ((y - 1).max(0)) as u32;
                let y_down = ((y + 1).min(h - 1)) as u32;
                let h00 = height[(y_up * w as u32 + x_left) as usize];
                let h10 = height[(y_up * w as u32 + x as u32) as usize];
                let h20 = height[(y_up * w as u32 + x_right) as usize];
                let h01 = height[(y as u32 * w as u32 + x_left) as usize];
                let h21 = height[(y as u32 * w as u32 + x_right) as usize];
                let h02 = height[(y_down * w as u32 + x_left) as usize];
                let h12 = height[(y_down * w as u32 + x as u32) as usize];
                let h22 = height[(y_down * w as u32 + x_right) as usize];
                // Sobel X
                let gx = (h20 + 2.0 * h21 + h22) - (h00 + 2.0 * h01 + h02);
                // Sobel Y
                let gy = (h00 + 2.0 * h10 + h20) - (h02 + 2.0 * h12 + h22);
                let strength = 2.0;
                let nx = -gx * strength;
                let ny = -gy * strength;
                let nz = 1.0;
                let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
                output[idx * 4] = ((nx / len * 0.5 + 0.5) * 255.0) as u8;
                output[idx * 4 + 1] = ((ny / len * 0.5 + 0.5) * 255.0) as u8;
                output[idx * 4 + 2] = ((nz / len * 0.5 + 0.5) * 255.0) as u8;
                output[idx * 4 + 3] = 255;
            }
        }
    }

    /// 生成砖墙图案 PBR 贴图
    pub fn generate_brick_texture(&self, brick_size: [f32; 2], mortar_width: f32) -> PbrTextureSet {
        let pbr = MaterialType::Brick.pbr_range();
        let size = (self.width * self.height) as usize;
        let mut albedo = vec![0u8; size * 4];
        let mut normal = vec![0u8; size * 4];
        let mut roughness = vec![0u8; size];
        let mut metallic = vec![0u8; size];
        let mut ao = vec![0u8; size];
        let mut height = vec![0u8; size];
        let mut height_field = vec![0.0f32; size];

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let u = x as f32 / self.width as f32;
                let v = y as f32 / self.height as f32;
                // 砖块行偏移（每行偏移半块）
                let row = (v / brick_size[1]).floor() as i32;
                let offset = if row % 2 == 0 { 0.0 } else { brick_size[0] * 0.5 };
                let bx = (u + offset) % brick_size[0];
                let by = v % brick_size[1];
                // 判断是否在砖块边缘（灰缝）
                let in_mortar_x = bx < mortar_width || bx > brick_size[0] - mortar_width;
                let in_mortar_y = by < mortar_width || by > brick_size[1] - mortar_width;
                let is_mortar = in_mortar_x || in_mortar_y;

                if is_mortar {
                    // 灰缝：灰色 + 低高度
                    albedo[idx * 4] = 120;
                    albedo[idx * 4 + 1] = 115;
                    albedo[idx * 4 + 2] = 110;
                    albedo[idx * 4 + 3] = 255;
                    roughness[idx] = 230;
                    height_field[idx] = 0.2;
                    ao[idx] = 100; // 灰缝处 AO 较暗
                } else {
                    // 砖块：红棕色 + 噪声变化
                    let variation = self.value_noise_2d(u * 20.0, v * 20.0, 999) * 0.2 - 0.1;
                    albedo[idx * 4] = ((pbr.albedo_color[0] + variation) * 255.0) as u8;
                    albedo[idx * 4 + 1] = ((pbr.albedo_color[1] + variation) * 255.0) as u8;
                    albedo[idx * 4 + 2] = ((pbr.albedo_color[2] + variation) * 255.0) as u8;
                    albedo[idx * 4 + 3] = 255;
                    roughness[idx] = ((pbr.roughness_min + pbr.roughness_max) * 0.5 * 255.0) as u8;
                    height_field[idx] = 0.8 + variation * 0.5;
                    ao[idx] = 220;
                }
                metallic[idx] = 0;
            }
        }
        self.compute_normals_from_height(&height_field, &mut normal);
        for (i, &h) in height_field.iter().enumerate() {
            height[i] = (h.clamp(0.0, 1.0) * 255.0) as u8;
        }

        PbrTextureSet {
            width: self.width,
            height: self.height,
            albedo,
            normal,
            roughness,
            metallic,
            ao,
            height_map: height,
        }
    }

    /// 获取支持的纹理格式（兼容性查询）
    pub fn supported_formats() -> &'static [TextureFormat] {
        &[
            TextureFormat::Rgba8Unorm,
            TextureFormat::R8Unorm,
            TextureFormat::Bc1RgbaUnormSrgb,
            TextureFormat::Bc3RgbaUnormSrgb,
            TextureFormat::Bc5RgUnorm,
            TextureFormat::Bc7RgbaUnorm,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_noise_range() {
        let gen = TextureGenerator::new(64, 64);
        for _ in 0..100 {
            let v = gen.value_noise_2d(0.5, 0.5, 12345);
            assert!(v >= 0.0 && v <= 1.0, "value noise out of range: {}", v);
        }
    }

    #[test]
    fn test_worley_noise_range() {
        let gen = TextureGenerator::new(64, 64);
        for _ in 0..100 {
            let v = gen.worley_noise_2d(0.5, 0.5, 12345);
            assert!(v >= 0.0 && v <= 1.0, "worley noise out of range: {}", v);
        }
    }

    #[test]
    fn test_fractal_noise_range() {
        let gen = TextureGenerator::new(64, 64);
        let params = NoiseParams::default();
        for _ in 0..100 {
            let v = gen.fractal_noise(0.5, 0.5, &params);
            assert!(v >= 0.0 && v <= 1.0, "fractal noise out of range: {}", v);
        }
    }

    #[test]
    fn test_pbr_set_generation() {
        let gen = TextureGenerator::new(64, 64);
        let params = NoiseParams::default();
        let set = gen.generate_pbr_set(MaterialType::Wood, &params);
        assert_eq!(set.albedo.len(), 64 * 64 * 4);
        assert_eq!(set.normal.len(), 64 * 64 * 4);
        assert_eq!(set.roughness.len(), 64 * 64);
        assert_eq!(set.metallic.len(), 64 * 64);
        assert_eq!(set.ao.len(), 64 * 64);
        assert_eq!(set.height_map.len(), 64 * 64);
    }

    #[test]
    fn test_metal_has_high_metallic() {
        let gen = TextureGenerator::new(32, 32);
        let params = NoiseParams::default();
        let set = gen.generate_pbr_set(MaterialType::Metal, &params);
        // 金属材质的 metallic 平均值应该 > 200
        let avg: f32 = set.metallic.iter().map(|&v| v as f32).sum::<f32>() / set.metallic.len() as f32;
        assert!(avg > 200.0, "metal metallic avg too low: {}", avg);
    }

    #[test]
    fn test_glass_has_low_roughness() {
        let gen = TextureGenerator::new(32, 32);
        let params = NoiseParams::default();
        let set = gen.generate_pbr_set(MaterialType::Glass, &params);
        // 玻璃粗糙度应该很低
        let avg: f32 = set.roughness.iter().map(|&v| v as f32).sum::<f32>() / set.roughness.len() as f32;
        assert!(avg < 30.0, "glass roughness avg too high: {}", avg);
    }

    #[test]
    fn test_brick_texture_generation() {
        let gen = TextureGenerator::new(64, 64);
        let set = gen.generate_brick_texture([0.25, 0.1], 0.01);
        assert_eq!(set.albedo.len(), 64 * 64 * 4);
        // 验证有灰缝（颜色偏灰）
        let mortar_count = set
            .albedo
            .chunks_exact(4)
            .filter(|c| c[0] == 120 && c[1] == 115 && c[2] == 110)
            .count();
        assert!(mortar_count > 0, "no mortar pixels found");
    }

    #[test]
    fn test_normal_map_valid() {
        let gen = TextureGenerator::new(32, 32);
        let params = NoiseParams::default();
        let set = gen.generate_pbr_set(MaterialType::Stone, &params);
        // 法线贴图 Z 分量应该接近 255（朝向 +Z）
        let z_avg: f32 = set
            .normal
            .chunks_exact(4)
            .map(|c| c[2] as f32)
            .sum::<f32>()
            / (set.normal.len() / 4) as f32;
        assert!(z_avg > 200.0, "normal Z avg too low: {}", z_avg);
    }

    #[test]
    fn test_pbr_range_lookup() {
        let wood = MaterialType::Wood.pbr_range();
        assert!(wood.roughness_min > 0.5);
        assert_eq!(wood.metallic_max, 0.0);

        let metal = MaterialType::Metal.pbr_range();
        assert!(metal.metallic_min > 0.5);
    }
}
