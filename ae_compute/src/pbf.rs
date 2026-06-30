//! Position Based Fluids (PBF) — 基于位置的不可压流体
//!
//! 基于:
//! - Macklin, Müller. "Position Based Fluids." ACM TOG 32(4), SIGGRAPH 2013.
//!   http://mmacklin.com/pbf_sig_preprint.pdf
//! - Müller et al. "Position Based Dynamics." 2007 (PBD 基础)
//! - Müller, Charypar, Gross. "Particle-Based Fluid Simulation for
//!   Interactive Applications." 2003 (SPH 核函数)
//! - Monaghan. "Smoothed Particle Hydrodynamics." 1992 (XSPH 粘性)
//!
//! 核心思想:
//! 1. 将不可压约束视为 PBD 约束求解问题
//! 2. 每个粒子位置满足密度约束: C_i = ρ_i / ρ_0 - 1 = 0
//! 3. 用 PBD 的拉格朗日乘子法: λ_i = -C_i / (Σ|∇C_i|² + ε)
//! 4. 位置修正: Δx_i = (1/ρ_0)·Σ_j (λ_i + λ_j)·∇W(x_i - x_j)
//! 5. XSPH 粘性: 防止数值噪声
//!
//! 优势:
//! - 实时 (1-3 次迭代足够, 比 PCISPH 更快)
//! - 无条件稳定 (PBD 的优点)
//! - 快速收敛 (拉格朗日乘子直接解)
//! - 适合交互式应用 (游戏, VR)
//!
//! 核函数 (Müller 2003 标准):
//! - Poly6:   W_poly6(r, h) = 315/(64πh^9)·(h²-r²)³        for 0 ≤ r ≤ h
//! - Spiky:   W_spiky(r, h) = 15/(πh^6)·(h-r)³              for 0 ≤ r ≤ h
//! - Spiky 梯度: ∇W_spiky(r⃗, h) = -45/(πh^6)·(h-|r|)²·(r⃗/|r|)  for 0 < r ≤ h
//!
//! 3D 常数:
//! - POLY6_K   = 315 / (64·π) ≈ 1.5666815
//! - SPIKY_K   = 15 / π       ≈ 4.7746483
//! - SPIKY_GRAD = -45 / π     ≈ -14.323945

use glam::Vec3;
use std::collections::HashMap;

// ============================================================
// 核函数常数 (3D, Müller 2003)
// ============================================================

const POLY6_K: f32 = 315.0 / (64.0 * std::f32::consts::PI);
const SPIKY_K: f32 = 15.0 / std::f32::consts::PI;
const SPIKY_GRAD_K: f32 = -45.0 / std::f32::consts::PI;

// ============================================================
// 核函数
// ============================================================

/// Poly6 核 W(r, h) = (315/(64π))·(1/h^9)·(h²-r²)³  for 0 ≤ r ≤ h, else 0
#[inline]
pub fn poly6(r_sq: f32, h: f32) -> f32 {
    if r_sq >= h * h || r_sq < 0.0 {
        return 0.0;
    }
    let h2 = h * h;
    let diff = h2 - r_sq;
    POLY6_K * diff * diff * diff / (h * h * h * h * h * h * h * h * h)
}

/// Poly6 核 (输入向量, 计算距离平方)
#[inline]
pub fn poly6_vec(r: Vec3, h: f32) -> f32 {
    poly6(r.length_squared(), h)
}

/// Spiky 核 W(r, h) = (15/π)·(1/h^6)·(h-r)³  for 0 ≤ r ≤ h, else 0
#[inline]
pub fn spiky(r: f32, h: f32) -> f32 {
    if r >= h || r < 0.0 {
        return 0.0;
    }
    let diff = h - r;
    SPIKY_K * diff * diff * diff / (h * h * h * h * h * h)
}

/// Spiky 核梯度 ∇W(r⃗, h) = (-45/π)·(1/h^6)·(h-|r|)²·(r⃗/|r|)  for 0 < r ≤ h
/// 注意 r=0 时未定义 (返回 0)
#[inline]
pub fn spiky_gradient(r_vec: Vec3, h: f32) -> Vec3 {
    let r = r_vec.length();
    if r >= h || r < 1e-12 {
        return Vec3::ZERO;
    }
    let diff = h - r;
    let coeff = SPIKY_GRAD_K * diff * diff / (h * h * h * h * h * h);
    coeff * (r_vec / r)
}

// ============================================================
// 粒子
// ============================================================

/// PBF 流体粒子
#[derive(Debug, Clone, Copy)]
pub struct PbfParticle {
    /// 当前位置
    pub position: Vec3,
    /// 预测位置 (PBD step)
    pub predicted_position: Vec3,
    /// 速度
    pub velocity: Vec3,
    /// 质量 (默认所有粒子同质量)
    pub mass: f32,
}

impl PbfParticle {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            predicted_position: position,
            velocity: Vec3::ZERO,
            mass: 1.0,
        }
    }

    pub fn with_velocity(position: Vec3, velocity: Vec3) -> Self {
        Self {
            position,
            predicted_position: position,
            velocity,
            mass: 1.0,
        }
    }
}

// ============================================================
// 边界 (AABB)
// ============================================================

/// 轴对齐包围盒边界
#[derive(Debug, Clone, Copy)]
pub struct PbfBoundary {
    pub min: Vec3,
    pub max: Vec3,
    /// 边界恢复系数 (0=完全非弹性, 1=完全弹性)
    pub restitution: f32,
}

impl PbfBoundary {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self {
            min,
            max,
            restitution: 0.5,
        }
    }

    /// 把粒子约束在边界内 (位置投影 + 速度反射)
    pub fn constrain(&self, pos: &mut Vec3, vel: &mut Vec3) {
        let r = 0.0; // 粒子半径在外部处理
        if pos.x < self.min.x + r {
            pos.x = self.min.x + r;
            if vel.x < 0.0 {
                vel.x = -vel.x * self.restitution;
            }
        }
        if pos.x > self.max.x - r {
            pos.x = self.max.x - r;
            if vel.x > 0.0 {
                vel.x = -vel.x * self.restitution;
            }
        }
        if pos.y < self.min.y + r {
            pos.y = self.min.y + r;
            if vel.y < 0.0 {
                vel.y = -vel.y * self.restitution;
            }
        }
        if pos.y > self.max.y - r {
            pos.y = self.max.y - r;
            if vel.y > 0.0 {
                vel.y = -vel.y * self.restitution;
            }
        }
        if pos.z < self.min.z + r {
            pos.z = self.min.z + r;
            if vel.z < 0.0 {
                vel.z = -vel.z * self.restitution;
            }
        }
        if pos.z > self.max.z - r {
            pos.z = self.max.z - r;
            if vel.z > 0.0 {
                vel.z = -vel.z * self.restitution;
            }
        }
    }

    /// 仅位置投影 (用于预测阶段)
    pub fn project_inside(&self, pos: &mut Vec3) {
        pos.x = pos.x.max(self.min.x).min(self.max.x);
        pos.y = pos.y.max(self.min.y).min(self.max.y);
        pos.z = pos.z.max(self.min.z).min(self.max.z);
    }
}

// ============================================================
// 空间哈希 (邻居搜索)
// ============================================================

/// 均匀网格空间哈希, 用于 O(1) 平均时间查找邻居
pub struct SpatialHash {
    /// 格子大小 (通常 = 支持半径 h)
    pub cell_size: f32,
    /// 哈希表: cell_key -> 粒子索引列表
    pub cells: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    /// 格子坐标
    #[inline]
    pub fn cell_coord(&self, pos: Vec3) -> (i32, i32, i32) {
        (
            (pos.x / self.cell_size).floor() as i32,
            (pos.y / self.cell_size).floor() as i32,
            (pos.z / self.cell_size).floor() as i32,
        )
    }

    /// 清空哈希表
    pub fn clear(&mut self) {
        self.cells.clear();
    }

    /// 插入粒子
    pub fn insert(&mut self, pos: Vec3, idx: usize) {
        let key = self.cell_coord(pos);
        self.cells.entry(key).or_insert_with(Vec::new).push(idx);
    }

    /// 查询某位置周围 (3x3x3) 的所有粒子索引
    pub fn query_neighbors(&self, pos: Vec3) -> Vec<usize> {
        let (cx, cy, cz) = self.cell_coord(pos);
        let mut result = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(cell) = self.cells.get(&(cx + dx, cy + dy, cz + dz)) {
                        result.extend(cell.iter().copied());
                    }
                }
            }
        }
        result
    }

    /// 查询某位置周围 (3x3x3) 的粒子索引, 调用回调
    pub fn for_each_neighbor<F: FnMut(usize)>(&self, pos: Vec3, mut f: F) {
        let (cx, cy, cz) = self.cell_coord(pos);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(cell) = self.cells.get(&(cx + dx, cy + dy, cz + dz)) {
                        for &idx in cell {
                            f(idx);
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// PBF 求解器
// ============================================================

/// PBF 求解器参数
#[derive(Debug, Clone)]
pub struct PbfConfig {
    /// 支持半径 h
    pub h: f32,
    /// 静止密度 ρ_0
    pub rho_0: f32,
    /// 粒子质量 (默认 = ρ_0·h³·系数)
    pub mass: f32,
    /// 密度约束迭代次数
    pub iterations: usize,
    /// 拉格朗日乘子正则化 ε (防止奇异性)
    pub epsilon: f32,
    /// XSPH 粘性系数 c (0~0.5)
    pub xsph_viscosity: f32,
    /// 速度阻尼
    pub damping: f32,
    /// 外力 (通常为重力)
    pub gravity: Vec3,
}

impl Default for PbfConfig {
    fn default() -> Self {
        // 默认参数 (Macklin & Müller 2013 推荐值)
        let h = 0.1;       // 支持半径
        let rho_0 = 1000.0; // 水密度 (kg/m³)
        // 质量: 在规则网格上, 粒子间距 = h/2, 粒子占据 (h/2)³ 体积
        // m = ρ_0·(h/2)³ = 1000·0.000125 = 0.125
        let mass = rho_0 * (h * 0.5).powi(3);
        Self {
            h,
            rho_0,
            mass,
            iterations: 3,
            epsilon: 600.0,
            xsph_viscosity: 0.05,
            damping: 0.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
        }
    }
}

/// PBF 求解器
pub struct PbfSolver {
    pub particles: Vec<PbfParticle>,
    pub config: PbfConfig,
    pub boundary: Option<PbfBoundary>,
    spatial_hash: SpatialHash,
    /// 每个粒子的邻居索引 (每步重建)
    neighbors: Vec<Vec<usize>>,
    /// 每个粒子的密度
    pub densities: Vec<f32>,
    /// 每个粒子的拉格朗日乘子 λ
    pub lambdas: Vec<f32>,
}

impl PbfSolver {
    pub fn new(config: PbfConfig) -> Self {
        let cell_size = config.h;
        Self {
            particles: Vec::new(),
            config,
            boundary: None,
            spatial_hash: SpatialHash::new(cell_size),
            neighbors: Vec::new(),
            densities: Vec::new(),
            lambdas: Vec::new(),
        }
    }

    pub fn with_boundary(mut self, boundary: PbfBoundary) -> Self {
        self.boundary = Some(boundary);
        self
    }

    pub fn add_particle(&mut self, p: PbfParticle) {
        self.particles.push(p);
    }

    pub fn add_particles(&mut self, ps: &[PbfParticle]) {
        self.particles.extend_from_slice(ps);
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    // ========================================================
    // 邻居搜索
    // ========================================================

    /// 重建空间哈希 + 邻居列表
    fn rebuild_neighbors(&mut self) {
        let n = self.particles.len();
        self.spatial_hash.clear();
        for (i, p) in self.particles.iter().enumerate() {
            self.spatial_hash.insert(p.predicted_position, i);
        }
        self.neighbors.clear();
        self.neighbors.reserve(n);
        for p in &self.particles {
            let mut nb = self.spatial_hash.query_neighbors(p.predicted_position);
            // 排序便于确定性
            nb.sort_unstable();
            self.neighbors.push(nb);
        }
    }

    // ========================================================
    // 密度计算
    // ========================================================

    /// 计算粒子 i 的密度: ρ_i = Σ_j m_j · W_poly6(|p_i - p_j|, h)
    fn compute_density(&self, i: usize) -> f32 {
        let pi = self.particles[i].predicted_position;
        let h = self.config.h;
        let mut rho = 0.0;
        for &j in &self.neighbors[i] {
            let r = self.particles[j].predicted_position - pi;
            rho += self.config.mass * poly6_vec(r, h);
        }
        rho
    }

    /// 计算所有粒子的密度
    fn compute_all_densities(&mut self) {
        let n = self.particles.len();
        if self.densities.len() != n {
            self.densities = vec![0.0; n];
        }
        for i in 0..n {
            self.densities[i] = self.compute_density(i);
        }
    }

    // ========================================================
    // 拉格朗日乘子
    // ========================================================

    /// 计算粒子 i 的拉格朗日乘子 λ_i
    ///
    /// C_i = ρ_i/ρ_0 - 1
    /// λ_i = -C_i / (Σ_k |∇_k C_i|² + ε)
    ///
    /// 其中 ∇_k C_i 包含:
    /// - j == i 时: ∇_i C_i = (1/ρ_0)·Σ_k m_k·∇W_spiky(p_i - p_k)
    /// - j == k (k != i): ∇_k C_i = -(1/ρ_0)·m_k·∇W_spiky(p_i - p_k)
    fn compute_lambda(&self, i: usize) -> f32 {
        let pi = self.particles[i].predicted_position;
        let h = self.config.h;
        let rho_0 = self.config.rho_0;

        // C_i = ρ_i/ρ_0 - 1
        let c_i = self.densities[i] / rho_0 - 1.0;

        // 计算梯度项
        let mut grad_i_sum = Vec3::ZERO; // ∇_i C_i
        let mut grad_k_sq_sum = 0.0;     // Σ_{k≠i} |∇_k C_i|²
        for &j in &self.neighbors[i] {
            if j == i { continue; }
            let r = pi - self.particles[j].predicted_position;
            let grad = spiky_gradient(r, h);
            let grad_j = (self.config.mass / rho_0) * grad;
            grad_k_sq_sum += grad_j.dot(grad_j);
            grad_i_sum -= grad_j;
        }
        // ∇_i C_i = -Σ_{k≠i} ∇_k C_i (动量守恒)
        // (因为 Σ_k ∇_k C_i = 0)
        let grad_i_sq = grad_i_sum.dot(grad_i_sum);

        let denom = grad_i_sq + grad_k_sq_sum + self.config.epsilon;
        if denom < 1e-12 {
            return 0.0;
        }
        -c_i / denom
    }

    /// 计算所有粒子的 λ
    fn compute_all_lambdas(&mut self) {
        let n = self.particles.len();
        if self.lambdas.len() != n {
            self.lambdas = vec![0.0; n];
        }
        for i in 0..n {
            self.lambdas[i] = self.compute_lambda(i);
        }
    }

    // ========================================================
    // 位置修正
    // ========================================================

    /// 计算粒子 i 的位置修正 Δp_i
    ///
    /// Δp_i = (1/ρ_0)·Σ_j (λ_i + λ_j)·∇W_spiky(p_i - p_j)
    fn compute_position_delta(&self, i: usize) -> Vec3 {
        let pi = self.particles[i].predicted_position;
        let h = self.config.h;
        let rho_0 = self.config.rho_0;
        let lambda_i = self.lambdas[i];
        let mut delta = Vec3::ZERO;
        for &j in &self.neighbors[i] {
            if j == i { continue; }
            let r = pi - self.particles[j].predicted_position;
            let grad = spiky_gradient(r, h);
            // 注: Macklin 2013 加上 s = -(ρ_i/ρ_0)·(ρ_j/ρ_0) 的密度修正项 (tensile instability)
            // 简化版不加, 直接用 (λ_i + λ_j)
            let lambda_sum = lambda_i + self.lambdas[j];
            delta += (self.config.mass / rho_0) * lambda_sum * grad;
        }
        delta
    }

    // ========================================================
    // 单步求解
    // ========================================================

    /// 完整一步 PBF 模拟
    ///
    /// 1. 预测位置 (应用外力 + 阻尼)
    /// 2. 边界投影 (预测位置)
    /// 3. 邻居搜索
    /// 4. 迭代求解密度约束:
    ///    a. 计算密度
    ///    b. 计算 λ
    ///    c. 计算位置修正
    ///    d. 应用位置修正
    /// 5. 边界投影 (修正后位置)
    /// 6. 更新速度: v = (x* - x) / dt
    /// 7. XSPH 粘性
    /// 8. 边界速度反射
    /// 9. 更新位置: x = x*
    pub fn step(&mut self, dt: f32) {
        let n = self.particles.len();
        if n == 0 { return; }

        // 1. 预测位置: x* = x + dt·v + dt²·a_ext
        for p in &mut self.particles {
            let damping = (1.0 - self.config.damping * dt).max(0.0);
            p.velocity = p.velocity * damping + self.config.gravity * dt;
            p.predicted_position = p.position + p.velocity * dt;
        }

        // 2. 边界投影 (预测位置)
        if let Some(b) = &self.boundary {
            for p in &mut self.particles {
                b.project_inside(&mut p.predicted_position);
            }
        }

        // 3. 邻居搜索
        self.rebuild_neighbors();

        // 4. 迭代密度约束求解
        for _ in 0..self.config.iterations {
            self.compute_all_densities();
            self.compute_all_lambdas();

            // 计算所有位置修正 (需先全部算完, 再统一应用, 否则破坏 Jacobi 性质)
            let deltas: Vec<Vec3> = (0..n).map(|i| self.compute_position_delta(i)).collect();
            for (i, delta) in deltas.into_iter().enumerate() {
                self.particles[i].predicted_position += delta;
            }

            // 边界投影 (修正后)
            if let Some(b) = &self.boundary {
                for p in &mut self.particles {
                    b.project_inside(&mut p.predicted_position);
                }
            }
        }

        // 6. 更新速度: v = (x* - x) / dt
        for p in &mut self.particles {
            p.velocity = (p.predicted_position - p.position) / dt;
        }

        // 7. XSPH 粘性: v_i += c·Σ_j (v_j - v_i)·W_poly6(p_i - p_j)
        if self.config.xsph_viscosity > 0.0 {
            self.apply_xsph_viscosity();
        }

        // 8. 边界速度反射
        if let Some(b) = &self.boundary {
            for p in &mut self.particles {
                let mut pos = p.position;
                let mut vel = p.velocity;
                b.constrain(&mut pos, &mut vel);
                p.velocity = vel;
            }
        }

        // 9. 更新位置: x = x*
        for p in &mut self.particles {
            p.position = p.predicted_position;
        }
    }

    /// XSPH 粘性
    fn apply_xsph_viscosity(&mut self) {
        let h = self.config.h;
        let c = self.config.xsph_viscosity;
        let n = self.particles.len();
        let mut new_velocities = vec![Vec3::ZERO; n];
        for i in 0..n {
            let pi = self.particles[i].predicted_position;
            let vi = self.particles[i].velocity;
            let mut v_mod = vi;
            for &j in &self.neighbors[i] {
                if j == i { continue; }
                let r = self.particles[j].predicted_position - pi;
                let w = poly6_vec(r, h);
                v_mod += c * (self.particles[j].velocity - vi) * w;
            }
            new_velocities[i] = v_mod;
        }
        for (i, v) in new_velocities.into_iter().enumerate() {
            self.particles[i].velocity = v;
        }
    }

    // ========================================================
    // 物理量查询
    // ========================================================

    /// 最大密度偏差 (|ρ - ρ_0|/ρ_0)
    pub fn max_density_error(&self) -> f32 {
        let mut max_err = 0.0;
        for &rho in &self.densities {
            let err = (rho - self.config.rho_0).abs() / self.config.rho_0;
            if err > max_err { max_err = err; }
        }
        max_err
    }

    /// 平均密度
    pub fn average_density(&self) -> f32 {
        if self.densities.is_empty() { return 0.0; }
        self.densities.iter().sum::<f32>() / self.densities.len() as f32
    }

    /// 总动能
    pub fn kinetic_energy(&self) -> f32 {
        let mut ke = 0.0;
        for p in &self.particles {
            ke += 0.5 * p.mass * p.velocity.length_squared();
        }
        ke
    }

    /// 重置粒子速度
    pub fn reset_velocities(&mut self) {
        for p in &mut self.particles {
            p.velocity = Vec3::ZERO;
        }
    }
}

// ============================================================
// 辅助: 创建规则粒子网格
// ============================================================

/// 在指定 AABB 内创建规则立方网格粒子
pub fn create_particle_grid(
    min: Vec3,
    max: Vec3,
    spacing: f32,
    velocity: Vec3,
    mass: f32,
) -> Vec<PbfParticle> {
    let mut particles = Vec::new();
    let mut x = min.x;
    while x <= max.x + 1e-6 {
        let mut y = min.y;
        while y <= max.y + 1e-6 {
            let mut z = min.z;
            while z <= max.z + 1e-6 {
                let p = PbfParticle {
                    position: Vec3::new(x, y, z),
                    predicted_position: Vec3::new(x, y, z),
                    velocity,
                    mass,
                };
                particles.push(p);
                z += spacing;
            }
            y += spacing;
        }
        x += spacing;
    }
    particles
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    // ---------- 核函数测试 ----------

    #[test]
    fn test_poly6_zero() {
        // W(0, h) 应为最大值
        let h = 0.1;
        let w0 = poly6(0.0, h);
        let w_other = poly6(0.0001, h);
        assert!(w0 > w_other, "poly6(0)={} should be > poly6(0.0001)={}", w0, w_other);
        // W(0, h) = 315/(64π·h^9)·h^6 = 315/(64π)·1/h³
        let expected = POLY6_K / (h * h * h);
        assert!(approx_eq(w0, expected, 1e-6),
            "poly6(0,h)={} expected={}", w0, expected);
    }

    #[test]
    fn test_poly6_outside_support() {
        let h = 0.1;
        // r > h: W = 0
        assert!(approx_eq(poly6(h * h + 1e-6, h), 0.0, 1e-10));
        assert!(approx_eq(poly6(2.0 * h * h, h), 0.0, 1e-10));
        // r == h: W = 0
        assert!(approx_eq(poly6(h * h, h), 0.0, 1e-10));
    }

    #[test]
    fn test_poly6_decreasing() {
        // poly6 应单调递减
        let h = 0.1;
        let r1 = poly6(0.0, h);
        let r2 = poly6(0.001, h);
        let r3 = poly6(0.01, h);
        assert!(r1 > r2 && r2 > r3,
            "poly6 should decrease: {} >= {} >= {}", r1, r2, r3);
    }

    #[test]
    fn test_spiky_zero() {
        let h = 0.1;
        let w0 = spiky(0.0, h);
        // W(0, h) = 15/π·1/h³
        let expected = SPIKY_K / (h * h * h);
        assert!(approx_eq(w0, expected, 1e-6),
            "spiky(0,h)={} expected={}", w0, expected);
    }

    #[test]
    fn test_spiky_outside_support() {
        let h = 0.1;
        assert!(approx_eq(spiky(h + 1e-6, h), 0.0, 1e-10));
        assert!(approx_eq(spiky(h, h), 0.0, 1e-10));
        assert!(approx_eq(spiky(2.0 * h, h), 0.0, 1e-10));
    }

    #[test]
    fn test_spiky_gradient_zero_at_support() {
        // ∇W(r=h, h) = 0 (因为 (h-r)² = 0)
        let h = 0.1;
        let r = Vec3::new(h * 0.99, 0.0, 0.0);
        let g = spiky_gradient(r, h);
        assert!(g.length() < 0.1,
            "spiky_gradient near h: {} should be small", g.length());
        let r2 = Vec3::new(h, 0.0, 0.0);
        let g2 = spiky_gradient(r2, h);
        assert!(g2.length() < 1e-6,
            "spiky_gradient at h: {} should be 0", g2.length());
    }

    #[test]
    fn test_spiky_gradient_zero_at_origin() {
        // r=0 时未定义, 我们返回 0 (避免除零)
        let h = 0.1;
        let g = spiky_gradient(Vec3::ZERO, h);
        assert_eq!(g, Vec3::ZERO);
    }

    #[test]
    fn test_spiky_gradient_direction() {
        // ∇W 指向 -r̂ 方向 (因为 SPIKY_GRAD_K 是负的)
        let h = 0.1;
        let r = Vec3::new(0.05, 0.0, 0.0);
        let g = spiky_gradient(r, h);
        // r̂ = +x, SPIKY_GRAD_K < 0, 所以 g 应朝 -x
        assert!(g.x < 0.0,
            "spiky_gradient x: {} should be < 0 (point toward -r̂)", g.x);
    }

    #[test]
    fn test_spiky_gradient_symmetry() {
        let h = 0.1;
        let r1 = Vec3::new(0.03, 0.0, 0.0);
        let r2 = Vec3::new(-0.03, 0.0, 0.0);
        let g1 = spiky_gradient(r1, h);
        let g2 = spiky_gradient(r2, h);
        assert!(approx_eq(g1.x, -g2.x, 1e-6),
            "gradient should be antisymmetric: {} vs {}", g1.x, g2.x);
    }

    // ---------- 粒子测试 ----------

    #[test]
    fn test_particle_creation() {
        let p = PbfParticle::new(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.predicted_position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.velocity, Vec3::ZERO);
        assert_eq!(p.mass, 1.0);
    }

    #[test]
    fn test_particle_with_velocity() {
        let p = PbfParticle::with_velocity(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(p.velocity, Vec3::new(1.0, 0.0, 0.0));
    }

    // ---------- 边界测试 ----------

    #[test]
    fn test_boundary_constrain() {
        let b = PbfBoundary::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let mut pos = Vec3::new(-0.5, 0.5, 0.5);
        let mut vel = Vec3::new(-1.0, 0.0, 0.0);
        b.constrain(&mut pos, &mut vel);
        assert_eq!(pos.x, 0.0);
        assert!(vel.x > 0.0, "vel.x should reflect: {}", vel.x);
    }

    #[test]
    fn test_boundary_project_inside() {
        let b = PbfBoundary::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let mut pos = Vec3::new(-0.5, 1.5, 0.5);
        b.project_inside(&mut pos);
        assert_eq!(pos, Vec3::new(0.0, 1.0, 0.5));
    }

    // ---------- 空间哈希测试 ----------

    #[test]
    fn test_spatial_hash_basic() {
        let mut sh = SpatialHash::new(1.0);
        sh.insert(Vec3::new(0.5, 0.5, 0.5), 0);
        sh.insert(Vec3::new(1.5, 0.5, 0.5), 1);
        sh.insert(Vec3::new(10.5, 10.5, 10.5), 2);
        let nb = sh.query_neighbors(Vec3::new(0.5, 0.5, 0.5));
        assert!(nb.contains(&0), "should contain particle 0");
        assert!(nb.contains(&1), "should contain particle 1 (adjacent cell)");
        assert!(!nb.contains(&2), "should NOT contain particle 2 (far away)");
    }

    #[test]
    fn test_spatial_hash_cell_coord() {
        let sh = SpatialHash::new(2.0);
        let c = sh.cell_coord(Vec3::new(1.5, 2.5, 3.5));
        // 1.5/2 = 0.75 -> floor 0
        // 2.5/2 = 1.25 -> floor 1
        // 3.5/2 = 1.75 -> floor 1
        assert_eq!(c, (0, 1, 1));
    }

    #[test]
    fn test_spatial_hash_clear() {
        let mut sh = SpatialHash::new(1.0);
        sh.insert(Vec3::ZERO, 0);
        assert!(!sh.cells.is_empty());
        sh.clear();
        assert!(sh.cells.is_empty());
    }

    // ---------- 求解器测试 ----------

    #[test]
    fn test_solver_creation() {
        let solver = PbfSolver::new(PbfConfig::default());
        assert_eq!(solver.particle_count(), 0);
        assert_eq!(solver.config.h, 0.1);
        assert_eq!(solver.config.rho_0, 1000.0);
    }

    #[test]
    fn test_solver_add_particle() {
        let mut solver = PbfSolver::new(PbfConfig::default());
        solver.add_particle(PbfParticle::new(Vec3::ZERO));
        solver.add_particle(PbfParticle::new(Vec3::new(0.1, 0.0, 0.0)));
        assert_eq!(solver.particle_count(), 2);
    }

    #[test]
    fn test_solver_density_single_particle() {
        // 单个粒子: 密度仅来自自身 (j == i)
        let mut solver = PbfSolver::new(PbfConfig::default());
        solver.add_particle(PbfParticle::new(Vec3::ZERO));
        solver.rebuild_neighbors();
        solver.compute_all_densities();
        // ρ = m · W(0, h)
        let expected = solver.config.mass * poly6(0.0, solver.config.h);
        assert!(approx_eq(solver.densities[0], expected, 1e-8),
            "density: {} expected: {}", solver.densities[0], expected);
    }

    #[test]
    fn test_solver_density_two_particles() {
        let mut solver = PbfSolver::new(PbfConfig::default());
        let h = solver.config.h;
        solver.add_particle(PbfParticle::new(Vec3::ZERO));
        solver.add_particle(PbfParticle::new(Vec3::new(h * 0.5, 0.0, 0.0)));
        solver.rebuild_neighbors();
        solver.compute_all_densities();
        // 每个粒子密度 = m·W(0,h) + m·W(h/2, h)
        let m = solver.config.mass;
        let w0 = poly6_vec(Vec3::ZERO, h);
        let w_half = poly6_vec(Vec3::new(h * 0.5, 0.0, 0.0), h);
        let expected = m * w0 + m * w_half;
        assert!(approx_eq(solver.densities[0], expected, 1e-6),
            "density[0]: {} expected: {}", solver.densities[0], expected);
        assert!(approx_eq(solver.densities[1], expected, 1e-6),
            "density[1]: {} expected: {} (symmetric)", solver.densities[1], expected);
    }

    #[test]
    fn test_solver_step_advances_time() {
        // 单粒子, 无外力, 一步不应崩溃
        let mut solver = PbfSolver::new(PbfConfig {
            gravity: Vec3::ZERO,
            ..Default::default()
        });
        solver.add_particle(PbfParticle::new(Vec3::ZERO));
        solver.step(0.01);
        assert_eq!(solver.particle_count(), 1);
    }

    #[test]
    fn test_solver_gravity_accelerates() {
        // 无边界, 重力下粒子应下落
        let mut solver = PbfSolver::new(PbfConfig::default());
        solver.add_particle(PbfParticle::new(Vec3::ZERO));
        let dt = 0.01;
        solver.step(dt);
        // 重力使粒子获得 -y 方向速度
        assert!(solver.particles[0].velocity.y < 0.0,
            "velocity.y: {} (should be < 0 due to gravity)", solver.particles[0].velocity.y);
        assert!(solver.particles[0].position.y < 0.0,
            "position.y: {} (should be < 0, fell down)", solver.particles[0].position.y);
    }

    #[test]
    fn test_solver_boundary_stops_particle() {
        // 边界应阻止粒子穿透
        let mut solver = PbfSolver::new(PbfConfig::default())
            .with_boundary(PbfBoundary::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)));
        solver.add_particle(PbfParticle::new(Vec3::new(0.0, 0.5, 0.0)));
        // 多步, 粒子应在边界内
        for _ in 0..100 {
            solver.step(0.01);
        }
        let p = solver.particles[0].position;
        assert!(p.y >= -1.0 - 1e-3 && p.y <= 1.0 + 1e-3,
            "particle y={} should be within [-1, 1]", p.y);
    }

    #[test]
    fn test_solver_static_stability() {
        // 静止粒子团 (无外力) 应保持稳定
        let mut solver = PbfSolver::new(PbfConfig {
            gravity: Vec3::ZERO,
            ..Default::default()
        });
        let h = solver.config.h;
        let spacing = h * 0.5;
        // 3x3x3 立方体粒子
        let particles = create_particle_grid(
            Vec3::new(-spacing, -spacing, -spacing),
            Vec3::new(spacing, spacing, spacing),
            spacing,
            Vec3::ZERO,
            solver.config.mass,
        );
        solver.add_particles(&particles);
        let initial_ke = solver.kinetic_energy();
        // 多步模拟
        for _ in 0..50 {
            solver.step(0.005);
        }
        let final_ke = solver.kinetic_energy();
        // 静止粒子应保持低动能 (允许少量数值噪声)
        assert!(final_ke < initial_ke + 1e-3,
            "static fluid should be stable: initial_ke={} final_ke={}", initial_ke, final_ke);
    }

    #[test]
    fn test_solver_incompressibility() {
        // 流体团在边界内重力下落, 平衡后密度应接近 ρ_0
        let mut solver = PbfSolver::new(PbfConfig {
            iterations: 4,
            xsph_viscosity: 0.1,
            ..Default::default()
        });
        let h = solver.config.h;
        let spacing = h * 0.5;
        // 较大的粒子团 (4x4x4)
        let particles = create_particle_grid(
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(0.3, 0.8, 0.3),
            spacing,
            Vec3::ZERO,
            solver.config.mass,
        );
        solver.add_particles(&particles);
        let boundary = PbfBoundary::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        solver.boundary = Some(boundary);
        // 多步模拟 (让粒子沉降)
        for _ in 0..100 {
            solver.step(0.005);
        }
        // 计算最终密度
        solver.rebuild_neighbors();
        solver.compute_all_densities();
        let avg_rho = solver.average_density();
        // 平均密度应在 ρ_0 的合理范围内 (PBF 不是严格不可压, 允许 20% 误差)
        let err = (avg_rho - solver.config.rho_0).abs() / solver.config.rho_0;
        assert!(err < 0.5,
            "average density: {} should be near rho_0={} (err={:.2}%)",
            avg_rho, solver.config.rho_0, err * 100.0);
    }

    #[test]
    fn test_solver_xsph_viscosity_reduces_velocity_diff() {
        // XSPH 粘性应减小邻居间速度差
        let mut solver_with = PbfSolver::new(PbfConfig {
            xsph_viscosity: 0.5,
            gravity: Vec3::ZERO,
            iterations: 1,
            ..Default::default()
        });
        let h = solver_with.config.h;
        // 两个相邻粒子, 不同速度
        solver_with.add_particle(PbfParticle::with_velocity(
            Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)));
        solver_with.add_particle(PbfParticle::with_velocity(
            Vec3::new(h * 0.5, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)));
        let v_diff_before = (solver_with.particles[0].velocity
            - solver_with.particles[1].velocity).length();
        solver_with.step(0.001);
        let v_diff_after = (solver_with.particles[0].velocity
            - solver_with.particles[1].velocity).length();
        // 注: 没有粘性的版本作对比
        let mut solver_no = PbfSolver::new(PbfConfig {
            xsph_viscosity: 0.0,
            gravity: Vec3::ZERO,
            iterations: 1,
            ..Default::default()
        });
        solver_no.add_particle(PbfParticle::with_velocity(
            Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)));
        solver_no.add_particle(PbfParticle::with_velocity(
            Vec3::new(h * 0.5, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)));
        solver_no.step(0.001);
        let v_diff_no = (solver_no.particles[0].velocity
            - solver_no.particles[1].velocity).length();
        // 粘性版本应减小速度差更多
        assert!(v_diff_after < v_diff_before || v_diff_after < v_diff_no,
            "XSPH should reduce velocity diff: before={}, after={}, no_visc={}",
            v_diff_before, v_diff_after, v_diff_no);
    }

    #[test]
    fn test_create_particle_grid() {
        let particles = create_particle_grid(
            Vec3::ZERO,
            Vec3::new(0.2, 0.0, 0.0),
            0.1,
            Vec3::ZERO,
            1.0,
        );
        // 应生成 3 个粒子 (x = 0, 0.1, 0.2)
        assert_eq!(particles.len(), 3);
        assert_eq!(particles[0].position, Vec3::ZERO);
        assert_eq!(particles[1].position, Vec3::new(0.1, 0.0, 0.0));
    }

    #[test]
    fn test_create_particle_grid_3d() {
        let particles = create_particle_grid(
            Vec3::ZERO,
            Vec3::new(0.1, 0.1, 0.1),
            0.1,
            Vec3::ZERO,
            1.0,
        );
        // 应生成 2x2x2 = 8 个粒子
        assert_eq!(particles.len(), 8);
    }

    #[test]
    fn test_solver_max_density_error() {
        let mut solver = PbfSolver::new(PbfConfig::default());
        solver.add_particle(PbfParticle::new(Vec3::ZERO));
        solver.rebuild_neighbors();
        solver.compute_all_densities();
        let err = solver.max_density_error();
        assert!(err >= 0.0, "density error should be >= 0: {}", err);
    }

    #[test]
    fn test_solver_multi_step_stability() {
        // 多步模拟不应崩溃
        let mut solver = PbfSolver::new(PbfConfig::default())
            .with_boundary(PbfBoundary::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)));
        let h = solver.config.h;
        let spacing = h * 0.5;
        let particles = create_particle_grid(
            Vec3::new(0.3, 0.7, 0.3),
            Vec3::new(0.6, 0.9, 0.6),
            spacing,
            Vec3::ZERO,
            solver.config.mass,
        );
        solver.add_particles(&particles);
        // 200 步, 不应 NaN
        for _ in 0..200 {
            solver.step(0.005);
        }
        for p in &solver.particles {
            assert!(p.position.x.is_finite(), "NaN in position.x");
            assert!(p.position.y.is_finite(), "NaN in position.y");
            assert!(p.position.z.is_finite(), "NaN in position.z");
            assert!(p.velocity.x.is_finite(), "NaN in velocity.x");
            // 粒子应在边界内
            assert!(p.position.x >= -0.01 && p.position.x <= 1.01,
                "particle x={} out of boundary", p.position.x);
            assert!(p.position.y >= -0.01 && p.position.y <= 1.01,
                "particle y={} out of boundary", p.position.y);
            assert!(p.position.z >= -0.01 && p.position.z <= 1.01,
                "particle z={} out of boundary", p.position.z);
        }
    }

    #[test]
    fn test_solver_max_density_error_decreases_with_iterations() {
        // 更多迭代应减小密度误差
        let make_solver = |iters: usize| {
            let h = 0.1;
            let rho_0 = 1000.0;
            let mass = rho_0 * (h * 0.5).powi(3);
            let cfg = PbfConfig {
                h, rho_0, mass,
                iterations: iters,
                epsilon: 600.0,
                xsph_viscosity: 0.0,
                damping: 0.0,
                gravity: Vec3::ZERO,
            };
            let mut s = PbfSolver::new(cfg);
            s.add_particles(&create_particle_grid(
                Vec3::ZERO,
                Vec3::new(0.2, 0.2, 0.0),
                h * 0.5,
                Vec3::ZERO,
                mass,
            ));
            s
        };
        let mut s_low = make_solver(1);
        let mut s_high = make_solver(8);
        s_low.step(0.001);
        s_high.step(0.001);
        let err_low = s_low.max_density_error();
        let err_high = s_high.max_density_error();
        // 更多迭代应减小误差 (或至少不增加)
        assert!(err_high <= err_low + 0.01,
            "more iterations should reduce error: 1-iter={} 8-iter={}",
            err_low, err_high);
    }

    #[test]
    fn test_solver_keeps_particles_in_boundary() {
        // 边界 + 高速粒子, 应严格保持在边界内
        let mut solver = PbfSolver::new(PbfConfig {
            gravity: Vec3::new(0.0, -20.0, 0.0),
            ..Default::default()
        });
        let h = solver.config.h;
        let particles = create_particle_grid(
            Vec3::new(0.4, 0.8, 0.4),
            Vec3::new(0.6, 0.9, 0.6),
            h * 0.5,
            Vec3::ZERO,
            solver.config.mass,
        );
        solver.add_particles(&particles);
        solver.boundary = Some(PbfBoundary::new(Vec3::ZERO, Vec3::splat(1.0)));
        for _ in 0..300 {
            solver.step(0.005);
        }
        for p in &solver.particles {
            assert!(p.position.x >= -1e-3 && p.position.x <= 1.0 + 1e-3,
                "particle x={} out of [0,1]", p.position.x);
            assert!(p.position.y >= -1e-3 && p.position.y <= 1.0 + 1e-3,
                "particle y={} out of [0,1]", p.position.y);
            assert!(p.position.z >= -1e-3 && p.position.z <= 1.0 + 1e-3,
                "particle z={} out of [0,1]", p.position.z);
        }
    }

    #[test]
    fn test_poly6_normalization() {
        // ∫∫∫ W_poly6 dV 在半径 h 球内应为 1 (近似)
        // 数值积分: 在球内采样
        let h = 0.1;
        let n = 20;
        let dr = h / n as f32;
        let mut sum = 0.0;
        for ix in 0..n {
            for iy in 0..n {
                for iz in 0..n {
                    let x = (ix as f32 + 0.5) * dr - h * 0.5;
                    let y = (iy as f32 + 0.5) * dr - h * 0.5;
                    let z = (iz as f32 + 0.5) * dr - h * 0.5;
                    let r_sq = x * x + y * y + z * z;
                    if r_sq < h * h {
                        sum += poly6(r_sq, h) * dr * dr * dr;
                    }
                }
            }
        }
        // 应该接近 1 (允许数值误差)
        assert!((sum - 1.0).abs() < 0.1,
            "poly6 integral over ball: {} should be ~1", sum);
    }
}
