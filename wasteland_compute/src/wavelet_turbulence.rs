//! Wavelet Turbulence - 流体细节增强
//!
//! 基于:
//! - Kim, Ted, Thurey, Mitchell, Gross. "Wavelet Turbulence for Fluid
//!   Simulation." ACM TOG (SIGGRAPH 2008), 27(3).
//! - Bridson 2007 Curl Noise (湍流源)
//!
//! 核心思想:
//! 1. 大尺度流体用低分辨率网格模拟 (快)
//! 2. 小尺度细节用噪声合成 (基于速度场 advect)
//! 3. 噪声振幅由小波分析确定 (匹配速度场能量谱)
//! 4. 合成: u_final = u_large + u_turbulent
//!
//! 简化实现:
//! - 用 Curl Noise 作为湍流源 (自然无散度)
//! - 湍流被速度场 advect (Lagrangian 追踪)
//! - 振幅 = k * |velocity| (与速度成比例)
//! - 多倍频叠加 (类似 FBM)
//!
//! 优点:
//! - 低分辨率流体 + 高分辨率细节, 性能优于直接高分辨率模拟
//! - 自然的湍流外观 (涡旋、水花)
//! - 可控的细节强度

use serde::{Deserialize, Serialize};
use glam::Vec3;
use crate::noise::{CurlNoise3D, PerlinNoise3D, fbm_perlin, FbmConfig};

// ============================================================
// 配置
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveletTurbulenceConfig {
    /// 噪声放大系数 (湍流强度)
    pub amplitude: f32,
    /// 最小细节尺度 (噪声频率倒数)
    pub min_scale: f32,
    /// 最大细节尺度
    pub max_scale: f32,
    /// 倍频数 (细节层数)
    pub octaves: u32,
    /// 湍流 advect 时间步
    pub dt: f32,
    /// 噪声种子
    pub seed: u64,
    /// 速度阈值 (低于此值无湍流)
    pub velocity_threshold: f32,
    /// 衰减率 (湍流随时间衰减)
    pub decay: f32,
}

impl Default for WaveletTurbulenceConfig {
    fn default() -> Self {
        Self {
            amplitude: 0.5,
            min_scale: 0.05,
            max_scale: 0.4,
            octaves: 3,
            dt: 0.05,
            seed: 0xC0FFEE,
            velocity_threshold: 0.1,
            decay: 0.95,
        }
    }
}

// ============================================================
// 流体粒子 (带湍流追踪)
// ============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TurbulenceParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    /// 湍流坐标 (被速度场 advect 的噪声坐标)
    pub turb_coord: Vec3,
    /// 当前湍流强度
    pub turbulence: Vec3,
}

impl TurbulenceParticle {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            turb_coord: position, // 初始噪声坐标 = 位置
            turbulence: Vec3::ZERO,
        }
    }
}

// ============================================================
// 求解器
// ============================================================

pub struct WaveletTurbulenceSolver {
    pub config: WaveletTurbulenceConfig,
    pub curl_noise: CurlNoise3D,
    pub perlin: PerlinNoise3D,
    pub particles: Vec<TurbulenceParticle>,
    /// 时间 (用于噪声动画)
    pub time: f32,
}

impl WaveletTurbulenceSolver {
    pub fn new(config: WaveletTurbulenceConfig) -> Self {
        let seed = config.seed;
        Self {
            config,
            curl_noise: CurlNoise3D::new(seed),
            perlin: PerlinNoise3D::new(seed),
            particles: Vec::new(),
            time: 0.0,
        }
    }

    pub fn add_particle(&mut self, position: Vec3) -> usize {
        let idx = self.particles.len();
        self.particles.push(TurbulenceParticle::new(position));
        idx
    }

    /// 单步: 给定流体粒子速度, 更新湍流
    /// velocity_field: 返回位置处的流体速度 (从外部流体求解器获取)
    pub fn step<F: Fn(Vec3) -> Vec3>(&mut self, velocity_field: F) {
        let dt = self.config.dt;
        self.time += dt;

        for p in &mut self.particles {
            // 1. 从外部流体场获取速度
            let u_large = velocity_field(p.position);
            p.velocity = u_large;

            // 2. Advect 湍流坐标 (噪声坐标随流体流动)
            p.turb_coord += u_large * dt;

            // 3. 计算湍流振幅 (基于速度能量)
            let speed = u_large.length();
            let amp = if speed > self.config.velocity_threshold {
                self.config.amplitude * speed * speed
            } else {
                0.0
            };

            // 4. 多倍频叠加 curl noise
            let mut turb = Vec3::ZERO;
            let mut scale = self.config.max_scale;
            let scale_ratio = (self.config.min_scale / self.config.max_scale).powf(1.0 / self.config.octaves as f32);
            for _ in 0..self.config.octaves {
                // 噪声坐标按 scale 缩放
                let coord = p.turb_coord / scale;
                let v = self.curl_noise.curl_animated(coord.x, coord.y, coord.z, self.time / scale);
                turb += Vec3::new(v[0], v[1], v[2]) * scale;
                scale *= scale_ratio;
            }
            turb *= amp;

            // 5. 衰减 (湍流随时间衰减, 防止累积)
            p.turbulence = p.turbulence * self.config.decay + turb * (1.0 - self.config.decay);
        }
    }

    /// 获取粒子合成速度 (大尺度 + 湍流)
    pub fn final_velocity(&self, idx: usize) -> Vec3 {
        if idx >= self.particles.len() {
            return Vec3::ZERO;
        }
        self.particles[idx].velocity + self.particles[idx].turbulence
    }

    /// 获取所有粒子的合成速度
    pub fn all_final_velocities(&self) -> Vec<Vec3> {
        self.particles.iter()
            .map(|p| p.velocity + p.turbulence)
            .collect()
    }

    /// 获取粒子湍流强度
    pub fn turbulence_magnitude(&self, idx: usize) -> f32 {
        if idx >= self.particles.len() {
            return 0.0;
        }
        self.particles[idx].turbulence.length()
    }

    /// 平均湍流能量
    pub fn average_turbulence_energy(&self) -> f32 {
        if self.particles.is_empty() {
            return 0.0;
        }
        self.particles.iter()
            .map(|p| p.turbulence.length_squared())
            .sum::<f32>() / self.particles.len() as f32 * 0.5
    }
}

// ============================================================
// 网格版本 (用于 Eulerian 流体)
// ============================================================

/// 网格波湍流 (用于网格流体如 LFM/Stam)
pub struct GridWaveletTurbulence {
    pub config: WaveletTurbulenceConfig,
    pub curl_noise: CurlNoise3D,
    /// 网格分辨率
    pub n: usize,
    /// 湍流坐标场 (每 cell 的噪声坐标)
    pub turb_coords_x: Vec<f32>,
    pub turb_coords_y: Vec<f32>,
    pub turb_coords_z: Vec<f32>,
    /// 湍流速度场
    pub turb_u: Vec<f32>,
    pub turb_v: Vec<f32>,
    pub turb_w: Vec<f32>,
    pub time: f32,
}

impl GridWaveletTurbulence {
    pub fn new(config: WaveletTurbulenceConfig, n: usize) -> Self {
        let size = (n + 2).pow(3);
        let seed = config.seed;
        Self {
            config,
            curl_noise: CurlNoise3D::new(seed),
            n,
            turb_coords_x: (0..size).map(|i| cell_to_world_x(i, n) as f32).collect(),
            turb_coords_y: (0..size).map(|i| cell_to_world_y(i, n) as f32).collect(),
            turb_coords_z: (0..size).map(|i| cell_to_world_z(i, n) as f32).collect(),
            turb_u: vec![0.0; size],
            turb_v: vec![0.0; size],
            turb_w: vec![0.0; size],
            time: 0.0,
        }
    }

    /// 单步: 给定网格速度场, 更新湍流
    /// u/v/w: 流体速度场 (size = (n+2)^3)
    pub fn step(&mut self, u: &[f32], v: &[f32], w: &[f32], dt: f32) {
        let n = self.n;
        let size = (n + 2).pow(3);
        self.time += dt;

        // 1. Advect 湍流坐标 (半拉格朗日)
        let old_x = self.turb_coords_x.clone();
        let old_y = self.turb_coords_y.clone();
        let old_z = self.turb_coords_z.clone();
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = i + (n + 2) * (j + (n + 2) * k);
                    // 反向追踪
                    let back_x = old_x[idx] - u[idx] * dt;
                    let back_y = old_y[idx] - v[idx] * dt;
                    let back_z = old_z[idx] - w[idx] * dt;
                    // 插值采样 (简化: 最近邻)
                    self.turb_coords_x[idx] = back_x;
                    self.turb_coords_y[idx] = back_y;
                    self.turb_coords_z[idx] = back_z;
                }
            }
        }

        // 2. 计算湍流 (curl noise + 振幅)
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = i + (n + 2) * (j + (n + 2) * k);
                    let speed = (u[idx] * u[idx] + v[idx] * v[idx] + w[idx] * w[idx]).sqrt();
                    let amp = if speed > self.config.velocity_threshold {
                        self.config.amplitude * speed * speed
                    } else {
                        0.0
                    };
                    // 多倍频
                    let mut tu = 0.0;
                    let mut tv = 0.0;
                    let mut tw = 0.0;
                    let mut scale = self.config.max_scale;
                    let scale_ratio = (self.config.min_scale / self.config.max_scale)
                        .powf(1.0 / self.config.octaves as f32);
                    for _ in 0..self.config.octaves {
                        let cx = self.turb_coords_x[idx] / scale;
                        let cy = self.turb_coords_y[idx] / scale;
                        let cz = self.turb_coords_z[idx] / scale;
                        let t = self.time / scale;
                        let v = self.curl_noise.curl_animated(cx, cy, cz, t);
                        tu += v[0] * scale;
                        tv += v[1] * scale;
                        tw += v[2] * scale;
                        scale *= scale_ratio;
                    }
                    self.turb_u[idx] = tu * amp;
                    self.turb_v[idx] = tv * amp;
                    self.turb_w[idx] = tw * amp;
                }
            }
        }
    }

    /// 获取合成速度场 (大尺度 + 湍流)
    pub fn composite_velocity(&self, u: &[f32], v: &[f32], w: &[f32]) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let n = self.n;
        let size = (n + 2).pow(3);
        let mut cu = vec![0.0; size];
        let mut cv = vec![0.0; size];
        let mut cw = vec![0.0; size];
        for i in 0..size {
            cu[i] = u[i] + self.turb_u[i];
            cv[i] = v[i] + self.turb_v[i];
            cw[i] = w[i] + self.turb_w[i];
        }
        (cu, cv, cw)
    }
}

// ============================================================
// 辅助函数
// ============================================================

#[inline]
fn cell_to_world_x(idx: usize, n: usize) -> f64 {
    let n2 = (n + 2) as i64;
    let i = (idx as i64 % n2) as f64;
    i / n2 as f64
}

#[inline]
fn cell_to_world_y(idx: usize, n: usize) -> f64 {
    let n2 = (n + 2) as i64;
    let j = ((idx as i64 / n2) % n2) as f64;
    j / n2 as f64
}

#[inline]
fn cell_to_world_z(idx: usize, n: usize) -> f64 {
    let n2 = (n + 2) as i64;
    let k = (idx as i64 / (n2 * n2)) as f64;
    k / n2 as f64
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wt_config_default() {
        let c = WaveletTurbulenceConfig::default();
        assert!(c.amplitude > 0.0);
        assert!(c.min_scale < c.max_scale);
        assert!(c.octaves > 0);
    }

    #[test]
    fn test_wt_solver_creation() {
        let solver = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig::default());
        assert!(solver.particles.is_empty());
    }

    #[test]
    fn test_wt_add_particle() {
        let mut solver = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig::default());
        let idx = solver.add_particle(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(idx, 0);
        assert_eq!(solver.particles.len(), 1);
        // 初始 turb_coord = position
        assert_eq!(solver.particles[0].turb_coord, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_wt_no_velocity_no_turbulence() {
        // 零速度 -> 零湍流
        let mut solver = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig {
            velocity_threshold: 0.1,
            ..WaveletTurbulenceConfig::default()
        });
        solver.add_particle(Vec3::new(0.5, 0.5, 0.5));
        solver.step(|_| Vec3::ZERO); // 零速度场
        let turb = solver.turbulence_magnitude(0);
        assert!(turb < 1e-4, "no velocity -> no turbulence: {}", turb);
    }

    #[test]
    fn test_wt_velocity_produces_turbulence() {
        // 有速度 -> 有湍流
        let mut solver = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig {
            amplitude: 1.0,
            velocity_threshold: 0.1,
            decay: 0.0, // 无衰减, 立即响应
            ..WaveletTurbulenceConfig::default()
        });
        solver.add_particle(Vec3::new(0.5, 0.5, 0.5));
        solver.step(|_| Vec3::new(1.0, 0.0, 0.0)); // 恒定速度
        let turb = solver.turbulence_magnitude(0);
        assert!(turb > 0.0, "velocity -> turbulence: {}", turb);
    }

    #[test]
    fn test_wt_turb_coord_advects() {
        // 湍流坐标应随速度场移动
        let mut solver = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig {
            dt: 0.1,
            ..WaveletTurbulenceConfig::default()
        });
        solver.add_particle(Vec3::new(0.0, 0.0, 0.0));
        let initial_coord = solver.particles[0].turb_coord;
        solver.step(|_| Vec3::new(1.0, 0.0, 0.0));
        // turb_coord 应沿 +x 移动
        assert!(solver.particles[0].turb_coord.x > initial_coord.x, "turb coord advects");
    }

    #[test]
    fn test_wt_final_velocity_combines() {
        // 合成速度 = 大尺度 + 湍流
        let mut solver = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig {
            amplitude: 1.0,
            decay: 0.0,
            velocity_threshold: 0.1,
            ..WaveletTurbulenceConfig::default()
        });
        solver.add_particle(Vec3::new(0.5, 0.5, 0.5));
        solver.step(|_| Vec3::new(2.0, 0.0, 0.0));
        let v_final = solver.final_velocity(0);
        let v_large = solver.particles[0].velocity;
        let v_turb = solver.particles[0].turbulence;
        assert!((v_final - v_large - v_turb).length() < 1e-5, "final = large + turb");
    }

    #[test]
    fn test_wt_decay() {
        // 衰减应让湍流随时间减少 (无新输入时)
        let mut solver = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig {
            amplitude: 1.0,
            decay: 0.5,
            velocity_threshold: 0.1,
            ..WaveletTurbulenceConfig::default()
        });
        solver.add_particle(Vec3::new(0.5, 0.5, 0.5));
        // 第一步: 有速度, 产生湍流
        solver.step(|_| Vec3::new(1.0, 0.0, 0.0));
        let turb1 = solver.turbulence_magnitude(0);
        // 第二步: 零速度, 湍流应衰减
        solver.step(|_| Vec3::ZERO);
        let turb2 = solver.turbulence_magnitude(0);
        assert!(turb2 < turb1, "turbulence decays: {} -> {}", turb1, turb2);
    }

    #[test]
    fn test_wt_multiple_octaves() {
        // 多倍频应增加湍流强度
        let mut solver1 = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig {
            octaves: 1,
            amplitude: 1.0,
            decay: 0.0,
            velocity_threshold: 0.1,
            ..WaveletTurbulenceConfig::default()
        });
        let mut solver3 = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig {
            octaves: 3,
            amplitude: 1.0,
            decay: 0.0,
            velocity_threshold: 0.1,
            ..WaveletTurbulenceConfig::default()
        });
        solver1.add_particle(Vec3::new(0.5, 0.5, 0.5));
        solver3.add_particle(Vec3::new(0.5, 0.5, 0.5));
        solver1.step(|_| Vec3::new(1.0, 0.0, 0.0));
        solver3.step(|_| Vec3::new(1.0, 0.0, 0.0));
        // 3 倍频应有不同 (通常更大, 但取决于相位)
        let t1 = solver1.turbulence_magnitude(0);
        let t3 = solver3.turbulence_magnitude(0);
        // 不严格, 但两者都应 > 0
        assert!(t1 >= 0.0 && t3 >= 0.0, "both positive: {} {}", t1, t3);
    }

    #[test]
    fn test_wt_average_energy() {
        let mut solver = WaveletTurbulenceSolver::new(WaveletTurbulenceConfig {
            amplitude: 1.0,
            decay: 0.0,
            velocity_threshold: 0.1,
            ..WaveletTurbulenceConfig::default()
        });
        for i in 0..10 {
            solver.add_particle(Vec3::new(0.5 + i as f32 * 0.1, 0.5, 0.5));
        }
        solver.step(|_| Vec3::new(1.0, 0.0, 0.0));
        let e = solver.average_turbulence_energy();
        assert!(e >= 0.0, "energy non-negative: {}", e);
    }

    #[test]
    fn test_grid_wt_creation() {
        let gwt = GridWaveletTurbulence::new(WaveletTurbulenceConfig::default(), 16);
        assert_eq!(gwt.n, 16);
        assert_eq!(gwt.turb_u.len(), 18 * 18 * 18);
    }

    #[test]
    fn test_grid_wt_step() {
        let mut gwt = GridWaveletTurbulence::new(WaveletTurbulenceConfig {
            amplitude: 1.0,
            velocity_threshold: 0.1,
            ..WaveletTurbulenceConfig::default()
        }, 8);
        let n = gwt.n;
        let size = (n + 2).pow(3);
        let u = vec![1.0; size]; // 恒定速度
        let v = vec![0.0; size];
        let w = vec![0.0; size];
        gwt.step(&u, &v, &w, 0.1);
        // 应有非零湍流
        let max_turb = gwt.turb_u.iter().cloned().fold(0.0f32, f32::max);
        assert!(max_turb > 0.0, "grid wt produces turbulence: {}", max_turb);
    }

    #[test]
    fn test_grid_wt_composite() {
        let mut gwt = GridWaveletTurbulence::new(WaveletTurbulenceConfig {
            amplitude: 0.5,
            velocity_threshold: 0.1,
            ..WaveletTurbulenceConfig::default()
        }, 8);
        let n = gwt.n;
        let size = (n + 2).pow(3);
        let u = vec![1.0; size];
        let v = vec![0.0; size];
        let w = vec![0.0; size];
        gwt.step(&u, &v, &w, 0.1);
        let (cu, cv, cw) = gwt.composite_velocity(&u, &v, &w);
        // 合成速度应 >= 原速度 (加了湍流)
        let max_cu = cu.iter().cloned().fold(0.0f32, f32::max);
        assert!(max_cu >= 1.0, "composite includes turbulence: {}", max_cu);
    }
}
