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

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    /// 构造一个 2x2x2 粒子网格 (8 个粒子, 间距 h/2)
    fn make_grid_solver() -> WcsphSolver {
        let mut config = WcsphConfig::default();
        config.h = 0.2;
        config.particle_mass = 0.001;
        config.rest_density = 1000.0;
        config.speed_of_sound = 30.0;
        config.viscosity = 0.01;
        let mut s = WcsphSolver::new(config);
        s.boundary_min = Vec3::new(-1.0, -1.0, -1.0);
        s.boundary_max = Vec3::new(1.0, 1.0, 1.0);
        // 2x2x2 网格, 间距 0.1 (= h/2)
        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    let p = WcsphParticle::new(Vec3::new(
                        i as f32 * 0.1,
                        j as f32 * 0.1,
                        k as f32 * 0.1,
                    ));
                    s.add_particle(p);
                }
            }
        }
        s
    }

    #[test]
    fn test_config_default() {
        let c = WcsphConfig::default();
        assert!(c.h > 0.0);
        assert!(c.particle_mass > 0.0);
        assert!(c.rest_density > 0.0);
        assert!(c.speed_of_sound > 0.0);
        assert!(c.gamma > 0.0);
    }

    #[test]
    fn test_viscosity_laplacian_zero_at_h() {
        let h = 0.2;
        // r = h 时核为 0
        assert!(approx(viscosity_laplacian(h, h), 0.0, 1e-6));
    }

    #[test]
    fn test_viscosity_laplacian_zero_outside() {
        let h = 0.2;
        // r > h 时核为 0
        assert!(approx(viscosity_laplacian(0.3, h), 0.0, 1e-6));
        assert!(approx(viscosity_laplacian(-0.1, h), 0.0, 1e-6));
    }

    #[test]
    fn test_viscosity_laplacian_positive_inside() {
        let h = 0.2;
        // 0 < r < h 时核为正
        let val = viscosity_laplacian(0.1, h);
        assert!(val > 0.0, "viscosity_laplacian should be > 0 inside, got {}", val);
    }

    #[test]
    fn test_viscosity_laplacian_max_at_zero() {
        // r=0 时取得最大值 45/(π·h⁵)
        let h: f32 = 0.2;
        let expected = 45.0 / (std::f32::consts::PI * h.powi(5));
        let actual = viscosity_laplacian(0.0, h);
        // 使用相对容差 (值很大 ~44762, 浮点误差累积)
        let rel_err = (actual - expected).abs() / expected.abs();
        assert!(rel_err < 1e-5, "viscosity_laplacian(0) = {}, expected = {}, rel_err = {}", actual, expected, rel_err);
    }

    #[test]
    fn test_bulk_modulus() {
        let s = WcsphSolver::new(WcsphConfig::default());
        // B = ρ₀·c₀²/γ = 1000·50²/7 ≈ 357142.86
        let expected = 1000.0 * 50.0 * 50.0 / 7.0;
        assert!(approx(s.bulk_modulus(), expected, 1.0));
    }

    #[test]
    fn test_cfl_dt() {
        let s = WcsphSolver::new(WcsphConfig::default());
        // dt = 0.25·h/c₀ = 0.25·0.1/50 = 5e-4
        let expected = 0.25 * 0.1 / 50.0;
        assert!(approx(s.cfl_dt(), expected, 1e-9));
    }

    #[test]
    fn test_add_particle() {
        let mut s = WcsphSolver::new(WcsphConfig::default());
        assert_eq!(s.num_particles(), 0);
        s.add_particle(WcsphParticle::new(Vec3::ZERO));
        assert_eq!(s.num_particles(), 1);
    }

    #[test]
    fn test_grid_solver_creation() {
        let s = make_grid_solver();
        assert_eq!(s.num_particles(), 8);
    }

    #[test]
    fn test_density_computation() {
        let mut s = make_grid_solver();
        s.rebuild_neighbors();
        s.compute_densities_and_pressures();
        // 8 粒子紧密堆积, 密度应接近静止密度量级
        for p in &s.particles {
            assert!(p.density > 0.0, "density should be positive, got {}", p.density);
        }
    }

    #[test]
    fn test_pressure_computation() {
        let mut s = make_grid_solver();
        s.rebuild_neighbors();
        s.compute_densities_and_pressures();
        // 压力应为非负 (Tait 方程 + clamp)
        for p in &s.particles {
            assert!(p.pressure >= 0.0, "pressure should be non-negative, got {}", p.pressure);
        }
    }

    #[test]
    fn test_pressure_zero_at_rest_density() {
        // 当 ρ = ρ₀ 时, Tait 方程给出 p = 0
        let config = WcsphConfig::default();
        let b = config.rest_density * config.speed_of_sound.powi(2) / config.gamma;
        let p = b * (1.0_f32.powf(config.gamma) - 1.0);
        assert!(approx(p, 0.0, 1e-6));
    }

    #[test]
    fn test_pressure_increases_with_density() {
        let config = WcsphConfig::default();
        let b = config.rest_density * config.speed_of_sound.powi(2) / config.gamma;
        let p1 = b * (1.1_f32.powf(config.gamma) - 1.0);
        let p2 = b * (1.5_f32.powf(config.gamma) - 1.0);
        assert!(p2 > p1, "higher density should give higher pressure");
    }

    #[test]
    fn test_acceleration_gravity() {
        // 单粒子 (无邻居): 加速度 = 重力
        let mut s = WcsphSolver::new(WcsphConfig::default());
        s.add_particle(WcsphParticle::new(Vec3::ZERO));
        s.rebuild_neighbors();
        s.compute_densities_and_pressures();
        s.compute_accelerations();
        let g = s.config.gravity;
        let a = s.particles[0].acceleration;
        assert!(
            approx_vec(a, g, 1e-3),
            "lone particle should only feel gravity, got a = {:?}, g = {:?}",
            a,
            g
        );
    }

    fn approx_vec(a: Vec3, b: Vec3, tol: f32) -> bool {
        (a.x - b.x).abs() < tol && (a.y - b.y).abs() < tol && (a.z - b.z).abs() < tol
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = make_grid_solver();
        let dt = 1e-4;
        s.step(dt);
        assert!(approx(s.time, dt, 1e-9));
        s.step(dt);
        assert!(approx(s.time, 2.0 * dt, 1e-9));
    }

    #[test]
    fn test_step_no_explosion() {
        // 步进多步后, 粒子位置应保持有限
        let mut s = make_grid_solver();
        for _ in 0..20 {
            s.step(1e-4);
        }
        for p in &s.particles {
            assert!(p.position.is_finite(), "position not finite");
            assert!(p.velocity.is_finite(), "velocity not finite");
            assert!(p.density.is_finite(), "density not finite");
            // 不应飞出边界太远
            assert!(p.position.length() < 100.0, "particle flew away");
        }
    }

    #[test]
    fn test_gravity_makes_particles_fall() {
        // 单粒子下落 (无压力干扰)
        let mut s = WcsphSolver::new(WcsphConfig::default());
        s.boundary_min = Vec3::new(-100.0, -100.0, -100.0);
        s.boundary_max = Vec3::new(100.0, 100.0, 100.0);
        s.add_particle(WcsphParticle::new(Vec3::new(0.0, 1.0, 0.0)));
        let y_initial = s.particles[0].position.y;
        for _ in 0..10 {
            s.step(1e-3);
        }
        let y_now = s.particles[0].position.y;
        assert!(
            y_now < y_initial,
            "particle should fall under gravity, y_initial={}, y_now={}",
            y_initial,
            y_now
        );
    }

    #[test]
    fn test_boundary_reflection() {
        // 把粒子放在边界外, 检查反弹
        let mut s = WcsphSolver::new(WcsphConfig::default());
        s.boundary_min = Vec3::new(-0.5, -0.5, -0.5);
        s.boundary_max = Vec3::new(0.5, 0.5, 0.5);
        s.config.restitution = 1.0;
        let mut p = WcsphParticle::new(Vec3::new(0.6, 0.0, 0.0)); // 超出 x=0.5
        p.velocity = Vec3::new(2.0, 0.0, 0.0);
        s.add_particle(p);
        s.apply_boundary();
        assert!(s.particles[0].position.x <= 0.5 + 1e-6);
        // 反弹: 速度反向
        assert!(s.particles[0].velocity.x < 0.0, "velocity should reverse");
    }

    #[test]
    fn test_boundary_clamp() {
        let mut s = WcsphSolver::new(WcsphConfig::default());
        s.boundary_min = Vec3::new(-0.5, -0.5, -0.5);
        s.boundary_max = Vec3::new(0.5, 0.5, 0.5);
        s.config.restitution = 0.0;
        let mut p = WcsphParticle::new(Vec3::new(2.0, 2.0, 2.0));
        p.velocity = Vec3::new(1.0, 1.0, 1.0);
        s.add_particle(p);
        s.apply_boundary();
        assert!(s.particles[0].position.x <= 0.5 + 1e-6);
        assert!(s.particles[0].position.y <= 0.5 + 1e-6);
        assert!(s.particles[0].position.z <= 0.5 + 1e-6);
    }

    #[test]
    fn test_max_velocity() {
        let mut s = WcsphSolver::new(WcsphConfig::default());
        let mut p1 = WcsphParticle::new(Vec3::ZERO);
        p1.velocity = Vec3::new(3.0, 4.0, 0.0); // |v| = 5
        s.add_particle(p1);
        let mut p2 = WcsphParticle::new(Vec3::ZERO);
        p2.velocity = Vec3::new(1.0, 0.0, 0.0);
        s.add_particle(p2);
        assert!(approx(s.max_velocity(), 5.0, 1e-6));
    }

    #[test]
    fn test_kinetic_energy() {
        let mut s = WcsphSolver::new(WcsphConfig::default());
        s.config.particle_mass = 2.0;
        let mut p = WcsphParticle::new(Vec3::ZERO);
        p.velocity = Vec3::new(3.0, 4.0, 0.0); // |v| = 5, KE = 0.5·2·25 = 25
        s.add_particle(p);
        assert!(approx(s.kinetic_energy(), 25.0, 1e-6));
    }

    #[test]
    fn test_total_mass() {
        let s = make_grid_solver();
        // 8 粒子 × 0.001 kg = 0.008 kg
        assert!(approx(s.total_mass(), 0.008, 1e-9));
    }

    #[test]
    fn test_restitution_partial() {
        // e = 0.5: 反弹后速度减半
        let mut s = WcsphSolver::new(WcsphConfig::default());
        s.boundary_min = Vec3::new(-0.5, -0.5, -0.5);
        s.boundary_max = Vec3::new(0.5, 0.5, 0.5);
        s.config.restitution = 0.5;
        let mut p = WcsphParticle::new(Vec3::new(0.6, 0.0, 0.0));
        p.velocity = Vec3::new(2.0, 0.0, 0.0);
        s.add_particle(p);
        s.apply_boundary();
        // 反弹后 |v| = 0.5 × 2 = 1
        assert!(approx(s.particles[0].velocity.x, -1.0, 1e-6));
    }

    #[test]
    fn test_static_block_stability() {
        // 8 粒子静止块在容器底部, 应稳定不爆炸
        let mut s = make_grid_solver();
        s.boundary_min = Vec3::new(-0.5, -0.5, -0.5);
        s.boundary_max = Vec3::new(0.5, 0.5, 0.5);
        // 把块放到底部
        for p in s.particles.iter_mut() {
            p.position.y -= 0.4;
        }
        let dt = s.cfl_dt() * 0.5;
        for _ in 0..30 {
            s.step(dt);
        }
        for p in &s.particles {
            assert!(p.position.is_finite());
            assert!(p.velocity.is_finite());
            assert!(
                p.position.y >= s.boundary_min.y - 1e-3,
                "particle should stay above floor, y = {}",
                p.position.y
            );
        }
    }

    #[test]
    fn test_two_particle_pressure_repulsion() {
        // 两个超近距离粒子应相互排斥
        let mut s = WcsphSolver::new(WcsphConfig::default());
        s.config.gravity = Vec3::ZERO;
        s.config.h = 0.2;
        s.config.speed_of_sound = 30.0;
        s.config.particle_mass = 0.001;
        s.config.rest_density = 1000.0;
        let mut p1 = WcsphParticle::new(Vec3::new(-0.01, 0.0, 0.0));
        let mut p2 = WcsphParticle::new(Vec3::new(0.01, 0.0, 0.0));
        p1.density = 1500.0;
        p2.density = 1500.0;
        p1.pressure = 1000.0;
        p2.pressure = 1000.0;
        s.add_particle(p1);
        s.add_particle(p2);
        s.rebuild_neighbors();
        s.compute_accelerations();
        // p0 在 -x, 应被推向 -x (远离 p1)
        assert!(
            s.particles[0].acceleration.x < 0.0,
            "p0 should be pushed in -x, got a.x = {}",
            s.particles[0].acceleration.x
        );
        assert!(
            s.particles[1].acceleration.x > 0.0,
            "p1 should be pushed in +x, got a.x = {}",
            s.particles[1].acceleration.x
        );
    }

    #[test]
    fn test_damping_reduces_velocity() {
        let mut s = WcsphSolver::new(WcsphConfig::default());
        s.config.damping = 1.0;
        s.config.gravity = Vec3::ZERO;
        let mut p = WcsphParticle::new(Vec3::new(0.0, 0.0, 0.0));
        p.velocity = Vec3::new(10.0, 0.0, 0.0);
        s.add_particle(p);
        s.boundary_min = Vec3::new(-100.0, -100.0, -100.0);
        s.boundary_max = Vec3::new(100.0, 100.0, 100.0);
        let v0 = s.particles[0].velocity.length();
        s.step(0.1);
        let v1 = s.particles[0].velocity.length();
        assert!(v1 < v0, "damping should reduce velocity: v0={}, v1={}", v0, v1);
    }

    #[test]
    fn test_safe_step() {
        // step_safe 应使用 CFL 时间步
        let mut s = make_grid_solver();
        let t0 = s.time;
        s.step_safe();
        assert!(s.time > t0);
    }
}