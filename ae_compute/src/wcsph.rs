//! WCSPH — Weakly Compressible Smoothed Particle Hydrodynamics
//!
//! 基于:
//! - Müller, Charypar, Gross. "Particle-Based Fluid Simulation for
//!   Interactive Applications." SCA 2003.
//! - Monaghan. "Simulating Free Surface Flows with SPH." JCP 1994.
//! - Batchelor. "An Introduction to Fluid Dynamics." 1967. (Tait equation)
//!
//! 核心思想:
//! 1. 流体用粒子离散化, 每个粒子携带质量、速度、密度、压力
//! 2. 任意场 A(r) 通过核函数插值得到: A(r) = Σ_j m_j/ρ_j · A_j · W(r - r_j, h)
//! 3. 压力由 Tait 状态方程计算: p = B·((ρ/ρ₀)^γ - 1)
//!    - B = ρ₀·c₀²/γ (体积模量)
//!    - c₀ = 速度参考值 (声速, 取 max_velocity × 10 保证可压缩度 < 1%)
//!    - γ = 7 (水的标准值)
//! 4. 加速度由 Navier-Stokes 动量方程给出:
//!    dv/dt = -(1/ρ)∇p + μ·∇²v + g
//!    - 压力项: -1/ρ·∇p (反对称形式: -(p_i+p_j)/(2ρ_i)·∇W_spiky)
//!    - 粘性项: μ·∇²v (Laplacian 形式: μ·Σ_j m_j·(v_j-v_i)/ρ_j·∇²W_visc)
//!    - 外力项: g (重力)
//! 5. Leapfrog 时间积分 (高效稳定):
//!    v(t+dt/2) = v(t-dt/2) + dt·a(t)
//!    x(t+dt)   = x(t) + dt·v(t+dt/2)
//! 6. CFL 稳定性条件: dt ≤ 0.25·h/c₀
//!
//! 优势:
//! - 真正的力-动量方法 (vs PBF 的位置约束)
//! - 易扩展 (表面张力、湍流、热传导、化学反应)
//! - 自然支持自由表面 (粒子可离开域)
//!
//! 局限:
//! - 时间步小 (CFL 限制, 声速大)
//! - 压力振荡 (用 PCISPH/IISPH/DFSPH 缓解)
//!
//! 应用: 通用流体仿真, 交互式应用, SPH 教学示例

use crate::pbf::{poly6, spiky_gradient, SpatialHash};
use glam::Vec3;

// ============================================================
// 配置
// ============================================================

/// WCSPH 配置参数
#[derive(Debug, Clone)]
pub struct WcsphConfig {
    /// 平滑长度 h (m)
    pub h: f32,
    /// 单个粒子质量 (kg)
    pub particle_mass: f32,
    /// 静止密度 ρ₀ (kg/m³)
    pub rest_density: f32,
    /// 动力粘度 μ (Pa·s) — 水 ≈ 1e-3
    pub viscosity: f32,
    /// 声速 c₀ (m/s) — 用于 Tait 方程, 通常取 max_velocity × 10
    pub speed_of_sound: f32,
    /// Tait 指数 γ (水 = 7)
    pub gamma: f32,
    /// 重力加速度 (m/s²)
    pub gravity: Vec3,
    /// 速度阻尼 (每步衰减系数)
    pub damping: f32,
    /// 边界恢复系数 (0=完全吸收, 1=完全弹性)
    pub restitution: f32,
}

impl Default for WcsphConfig {
    fn default() -> Self {
        Self {
            h: 0.1,
            particle_mass: 0.001,
            rest_density: 1000.0,
            viscosity: 0.01,
            speed_of_sound: 50.0,
            gamma: 7.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.0,
            restitution: 0.5,
        }
    }
}

// ============================================================
// 粒子
// ============================================================

/// WCSPH 粒子
#[derive(Debug, Clone)]
pub struct WcsphParticle {
    /// 位置
    pub position: Vec3,
    /// 速度 (Leapfrog: 在 t±dt/2 时刻)
    pub velocity: Vec3,
    /// 加速度 (在 t 时刻)
    pub acceleration: Vec3,
    /// 密度 ρ_i
    pub density: f32,
    /// 压力 p_i
    pub pressure: f32,
}

impl WcsphParticle {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            acceleration: Vec3::ZERO,
            density: 0.0,
            pressure: 0.0,
        }
    }
}

// ============================================================
// 核函数 (粘性核的 Laplacian)
// ============================================================

/// 粘性核的 Laplacian: ∇²W_visc(r, h) = 45/(π·h⁶)·(h - |r|)
///
/// 用于粘性力计算. 注意与 poly6 (用于密度)、spiky_gradient (用于压力梯度) 的区别.
#[inline]
pub fn viscosity_laplacian(r_len: f32, h: f32) -> f32 {
    if r_len >= h || r_len < 0.0 {
        return 0.0;
    }
    let h6 = h.powi(6);
    let coeff = 45.0 / (std::f32::consts::PI * h6);
    coeff * (h - r_len)
}

// ============================================================
// WCSPH 求解器
// ============================================================

/// WCSPH 求解器
pub struct WcsphSolver {
    /// 粒子集合
    pub particles: Vec<WcsphParticle>,
    /// 配置
    pub config: WcsphConfig,
    /// 空间哈希 (加速邻居查询)
    spatial_hash: SpatialHash,
    /// 邻居列表 (按粒子索引)
    neighbors: Vec<Vec<usize>>,
    /// 域边界 (粒子超出时反射)
    pub boundary_min: Vec3,
    pub boundary_max: Vec3,
    /// 模拟时间
    pub time: f32,
}

impl WcsphSolver {
    /// 创建求解器
    pub fn new(config: WcsphConfig) -> Self {
        let cell_size = config.h;
        Self {
            particles: Vec::new(),
            config,
            spatial_hash: SpatialHash::new(cell_size),
            neighbors: Vec::new(),
            boundary_min: Vec3::new(-1.0, -1.0, -1.0),
            boundary_max: Vec3::new(1.0, 1.0, 1.0),
            time: 0.0,
        }
    }

    /// 添加粒子
    pub fn add_particle(&mut self, p: WcsphParticle) {
        self.particles.push(p);
    }

    /// 粒子数
    pub fn num_particles(&self) -> usize {
        self.particles.len()
    }

    /// CFL 时间步上限: dt ≤ 0.25 · h / c₀
    pub fn cfl_dt(&self) -> f32 {
        0.25 * self.config.h / self.config.speed_of_sound
    }

    /// 体积模量 B = ρ₀·c₀²/γ
    pub fn bulk_modulus(&self) -> f32 {
        self.config.rest_density * self.config.speed_of_sound.powi(2) / self.config.gamma
    }

    // ========================================================
    // 邻居搜索
    // ========================================================

    /// 重建空间哈希和邻居列表
    fn rebuild_neighbors(&mut self) {
        let n = self.particles.len();
        self.spatial_hash.clear();
        for (i, p) in self.particles.iter().enumerate() {
            self.spatial_hash.insert(p.position, i);
        }
        self.neighbors = Vec::with_capacity(n);
        for p in &self.particles {
            self.neighbors.push(self.spatial_hash.query_neighbors(p.position));
        }
    }

    // ========================================================
    // 密度 & 压力
    // ========================================================

    /// 计算每个粒子的密度和压力
    ///
    /// ρ_i = Σ_j m_j · W_poly6(r_ij, h)
    /// p_i = B · ((ρ_i/ρ₀)^γ - 1)
    fn compute_densities_and_pressures(&mut self) {
        let h = self.config.h;
        let m = self.config.particle_mass;
        let rho_0 = self.config.rest_density;
        let b = self.bulk_modulus();
        let gamma = self.config.gamma;

        for i in 0..self.particles.len() {
            let pi = self.particles[i].position;
            let mut density = 0.0;
            for &j in &self.neighbors[i] {
                let r = self.particles[j].position - pi;
                let r_sq = r.length_squared();
                if r_sq <= h * h {
                    density += m * poly6(r_sq, h);
                }
            }
            // 静止密度下限 (防止粒子在边界处密度过低导致负压力过大)
            density = density.max(rho_0 * 0.1);

            let pressure = b * ((density / rho_0).powf(gamma) - 1.0);
            // 压力下限 0 (Tait 方程产生非负压力, 排除张力)
            let pressure = pressure.max(0.0);

            self.particles[i].density = density;
            self.particles[i].pressure = pressure;
        }
    }

    // ========================================================
    // 力 (加速度)
    // ========================================================

    /// 计算每个粒子的加速度
    ///
    /// dv/dt = -1/ρ·∇p + μ·∇²v + g
    fn compute_accelerations(&mut self) {
        let h = self.config.h;
        let m = self.config.particle_mass;
        let mu = self.config.viscosity;
        let g = self.config.gravity;

        let n = self.particles.len();
        let mut acc = vec![Vec3::ZERO; n];

        for i in 0..n {
            let pi = self.particles[i].position;
            let vi = self.particles[i].velocity;
            let rho_i = self.particles[i].density;
            let p_i = self.particles[i].pressure;

            if rho_i < 1e-6 {
                continue;
            }

            let mut f_pressure = Vec3::ZERO;
            let mut f_viscosity = Vec3::ZERO;

            for &j in &self.neighbors[i] {
                if i == j {
                    continue;
                }
                let r_vec = pi - self.particles[j].position;
                let r_len = r_vec.length();
                if r_len >= h || r_len < 1e-8 {
                    continue;
                }
                let rho_j = self.particles[j].density;
                let p_j = self.particles[j].pressure;
                let v_j = self.particles[j].velocity;

                if rho_j < 1e-6 {
                    continue;
                }

                // 压力力: -m_j·(p_i+p_j)/(2·ρ_i·ρ_j)·∇W_spiky
                // 注意: spiky_gradient 返回 -r_vec 上的梯度方向 (Müller 2003)
                let grad_w = spiky_gradient(r_vec, h);
                let p_avg = (p_i + p_j) * 0.5;
                f_pressure -= m * p_avg / (rho_i * rho_j) * grad_w;

                // 粘性力: μ·m_j·(v_j-v_i)/ρ_j·∇²W_visc
                let lap_w = viscosity_laplacian(r_len, h);
                f_viscosity += mu * m * (v_j - vi) / rho_j * lap_w;
            }

            // 注意: Müller 2003 的公式给出的是 "单位质量力" (加速度直接)
            // f_pressure_i (单位质量) = -1/ρ_i · Σ_j m_j·(p_i+p_j)/(2ρ_j)·∇W
            // 但这里我们已经除了 ρ_i, 所以 f_pressure 已经是加速度
            acc[i] = f_pressure + f_viscosity + g;
        }

        for i in 0..n {
            self.particles[i].acceleration = acc[i];
        }
    }

    // ========================================================
    // 边界处理
    // ========================================================

    /// 边界反射 (粒子超出边界时反弹)
    fn apply_boundary(&mut self) {
        let b_min = self.boundary_min;
        let b_max = self.boundary_max;
        let e = self.config.restitution;

        for p in &mut self.particles {
            for axis in 0..3 {
                if p.position[axis] < b_min[axis] {
                    p.position[axis] = b_min[axis];
                    if p.velocity[axis] < 0.0 {
                        p.velocity[axis] = -e * p.velocity[axis];
                    }
                } else if p.position[axis] > b_max[axis] {
                    p.position[axis] = b_max[axis];
                    if p.velocity[axis] > 0.0 {
                        p.velocity[axis] = -e * p.velocity[axis];
                    }
                }
            }
        }
    }

    // ========================================================
    // 时间步进 (Leapfrog)
    // ========================================================

    /// 一步时间步进 (Leapfrog 积分)
    ///
    /// v_{n+1/2} = v_{n-1/2} + dt·a_n
    /// x_{n+1} = x_n + dt·v_{n+1/2}
    pub fn step(&mut self, dt: f32) {
        // 1. 邻居
        self.rebuild_neighbors();

        // 2. 密度 + 压力
        self.compute_densities_and_pressures();

        // 3. 加速度
        self.compute_accelerations();

        // 4. Leapfrog: 速度 + 位置
        let damping = self.config.damping;
        for p in &mut self.particles {
            p.velocity += dt * p.acceleration;
            if damping > 0.0 {
                p.velocity *= 1.0 - damping * dt;
            }
            p.position += dt * p.velocity;
        }

        // 5. 边界
        self.apply_boundary();

        self.time += dt;
    }

    /// 安全时间步 (CFL 自动选择)
    pub fn step_safe(&mut self) {
        let dt = self.cfl_dt();
        self.step(dt);
    }

    // ========================================================
    // 诊断
    // ========================================================

    /// 最大速度 (用于 CFL 调试)
    pub fn max_velocity(&self) -> f32 {
        self.particles
            .iter()
            .map(|p| p.velocity.length())
            .fold(0.0_f32, f32::max)
    }

    /// 平均密度
    pub fn avg_density(&self) -> f32 {
        if self.particles.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.particles.iter().map(|p| p.density).sum();
        sum / self.particles.len() as f32
    }

    /// 总动能
    pub fn kinetic_energy(&self) -> f32 {
        let m = self.config.particle_mass;
        self.particles
            .iter()
            .map(|p| 0.5 * m * p.velocity.length_squared())
            .sum()
    }

    /// 总质量
    pub fn total_mass(&self) -> f32 {
        self.config.particle_mass * self.particles.len() as f32
    }
}
