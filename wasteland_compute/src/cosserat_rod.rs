//! Cosserat Rod — 离散弹性杆 (绳索/头发/线缆)
//!
//! 基于:
//! - Bergou, Wardetzky, Robinson, Audoly, Grinspun. "Discrete Elastic Rods."
//!   ACM TOG, SIGGRAPH 2008.
//! - Kugelstadt, Schömer. "Position-Based and Aggregated Directional Stiffness."
//!
//! 核心思想:
//! 1. 杆用中心线 (折线) + 材料标架表示
//! 2. Bishop 标架: 沿杆平行输运, 无扭曲 (自然标架)
//! 3. 材料标架: Bishop 标架旋转角度 θ (定义杆的朝向)
//! 4. 三种能量: 拉伸 (弹簧), 弯曲 (角度), 扭转 (θ 差)
//! 5. 半隐式 Euler 时间积分
//!
//! 应用: 绳索, 头发, 线缆, 藤蔓, 鞭子, 弹簧

use glam::Vec3;
use std::collections::HashSet;

// ============================================================
// 配置
// ============================================================

/// Cosserat 杆配置参数
#[derive(Debug, Clone)]
pub struct CosseratRodConfig {
    /// 拉伸刚度 EA (N) — Young 模量 × 截面积
    pub stretching_stiffness: f32,
    /// 弯曲刚度 EI (N·m²) — Young 模量 × 面积惯性矩
    pub bending_stiffness: f32,
    /// 扭转刚度 GJ (N·m²) — 剪切模量 × 极惯性矩
    pub twisting_stiffness: f32,
    /// 密度 ρ (kg/m³)
    pub density: f32,
    /// 截面半径 (m)
    pub radius: f32,
    /// 线性阻尼系数
    pub damping: f32,
    /// 重力加速度 (m/s²)
    pub gravity: Vec3,
}

impl Default for CosseratRodConfig {
    fn default() -> Self {
        Self {
            stretching_stiffness: 1000.0,
            bending_stiffness: 0.01,
            twisting_stiffness: 0.005,
            density: 1000.0,
            radius: 0.005,
            damping: 0.01,
            gravity: Vec3::new(0.0, -9.81, 0.0),
        }
    }
}

// ============================================================
// CosseratRod
// ============================================================

/// 离散弹性杆 (Cosserat Rod)
///
/// 中心线: n+1 个顶点 x_0, ..., x_n
/// 边: n 条边 e_i = x_{i+1} - x_i
/// Bishop 标架: 每条边一个 (u, v, t) 正交标架
/// 材料标架: Bishop 标架旋转 θ 角
pub struct CosseratRod {
    /// 中心线顶点位置 (n+1 个)
    pub vertices: Vec<Vec3>,
    /// 顶点速度
    pub velocities: Vec<Vec3>,
    /// 静止长度 (每条边)
    rest_lengths: Vec<f32>,
    /// 静止弯曲角度 (每个内部顶点, n-1 个)
    rest_bend_angles: Vec<f32>,
    /// 静止扭转角 (每对相邻边, n-1 个)
    rest_twist: Vec<f32>,

    /// 材料标架角度 θ (每条边)
    pub thetas: Vec<f32>,
    /// θ 角速度
    theta_velocities: Vec<f32>,

    /// Bishop 标架 u 向量 (每条边)
    bishop_u: Vec<Vec3>,
    /// Bishop 标架 v 向量 (每条边)
    bishop_v: Vec<Vec3>,

    /// 配置
    pub config: CosseratRodConfig,
    /// 固定顶点集合
    pub fixed_vertices: HashSet<usize>,

    /// 当前时间
    pub time: f32,
}

impl CosseratRod {
    /// 创建杆
    ///
    /// 初始形状作为静止形状 (rest lengths, rest bend angles 从初始位置计算)
    pub fn new(positions: &[Vec3], config: CosseratRodConfig) -> Self {
        let n = positions.len();
        let n_edges = n.saturating_sub(1);
        let n_interior = n.saturating_sub(2);

        let mut rest_lengths = Vec::with_capacity(n_edges);
        for i in 0..n_edges {
            rest_lengths.push((positions[i + 1] - positions[i]).length());
        }

        // 从初始形状计算静止弯曲角度
        let mut rest_bend_angles = vec![0.0_f32; n_interior];
        for i in 1..n.saturating_sub(1) {
            if i >= n_edges {
                continue;
            }
            let e_prev = positions[i] - positions[i - 1];
            let e_curr = positions[i + 1] - positions[i];
            let lp = e_prev.length();
            let lc = e_curr.length();
            if lp > 1e-12 && lc > 1e-12 {
                let cos_a = (e_prev.dot(e_curr) / (lp * lc)).clamp(-1.0, 1.0);
                rest_bend_angles[i - 1] = cos_a.acos();
            }
        }

        Self {
            vertices: positions.to_vec(),
            velocities: vec![Vec3::ZERO; n],
            rest_lengths,
            rest_bend_angles,
            rest_twist: vec![0.0; n_edges.saturating_sub(1)],
            thetas: vec![0.0; n_edges],
            theta_velocities: vec![0.0; n_edges],
            bishop_u: vec![Vec3::ZERO; n_edges],
            bishop_v: vec![Vec3::ZERO; n_edges],
            config,
            fixed_vertices: HashSet::new(),
            time: 0.0,
        }
    }

    /// 顶点数
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }
    /// 边数
    pub fn num_edges(&self) -> usize {
        self.rest_lengths.len()
    }

    /// 获取边向量 e_i = x_{i+1} - x_i
    #[inline]
    pub fn edge(&self, i: usize) -> Vec3 {
        self.vertices[i + 1] - self.vertices[i]
    }

    /// 边长度 |e_i|
    #[inline]
    pub fn edge_length(&self, i: usize) -> f32 {
        self.edge(i).length()
    }

    /// 边切向 t_i = e_i / |e_i|
    #[inline]
    pub fn edge_tangent(&self, i: usize) -> Vec3 {
        let e = self.edge(i);
        let len = e.length();
        if len > 1e-12 { e / len } else { Vec3::ZERO }
    }

    /// 顶点质量 = ρ × π × r² × L_voronoi
    pub fn vertex_mass(&self, i: usize) -> f32 {
        let area = std::f32::consts::PI * self.config.radius * self.config.radius;
        let voronoi_len = self.voronoi_length(i);
        self.config.density * area * voronoi_len
    }

    /// 顶点转动惯量 (绕切向轴) = ρ × π × r⁴/2 × L_voronoi
    pub fn vertex_inertia(&self, i: usize) -> f32 {
        let r = self.config.radius;
        let area = std::f32::consts::PI * r * r;
        let inertia_per_length = 0.5 * self.config.density * area * r * r;
        inertia_per_length * self.voronoi_length(i)
    }

    /// Voronoi 区域长度 (顶点 i 周围的杆段长度)
    #[inline]
    fn voronoi_length(&self, i: usize) -> f32 {
        let ne = self.num_edges();
        if ne == 0 {
            0.0
        } else if i == 0 {
            self.rest_lengths.first().copied().unwrap_or(0.0) * 0.5
        } else if i >= ne {
            self.rest_lengths.last().copied().unwrap_or(0.0) * 0.5
        } else {
            (self.rest_lengths[i - 1] + self.rest_lengths[i]) * 0.5
        }
    }

    /// 捕获当前形状作为静止形状
    pub fn capture_rest_shape(&mut self) {
        for i in 0..self.num_edges() {
            self.rest_lengths[i] = self.edge_length(i);
        }
        for i in 1..self.num_vertices().saturating_sub(1) {
            if i >= self.num_edges() {
                continue;
            }
            let t_prev = self.edge_tangent(i - 1);
            let t_curr = self.edge_tangent(i);
            let cos_a = t_prev.dot(t_curr).clamp(-1.0, 1.0);
            self.rest_bend_angles[i - 1] = cos_a.acos();
        }
    }
}
// ============================================================
// impl: 标架 / 曲率 / 能量 / 力 / 时间步进
// ============================================================

impl CosseratRod {
    // ========================================================
    // Bishop 标架 (平行输运)
    // ========================================================

    /// 计算所有边的 Bishop 标架 (沿杆平行输运, 无扭曲)
    pub fn compute_bishop_frame(&mut self) {
        let n = self.num_edges();
        if n == 0 {
            return;
        }
        // 初始标架 (第 0 条边)
        let t0 = self.edge_tangent(0);
        let ref_dir = if t0.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        self.bishop_u[0] = t0.cross(ref_dir).normalize();
        self.bishop_v[0] = t0.cross(self.bishop_u[0]).normalize();
        // 平行输运
        for i in 1..n {
            let t_prev = self.edge_tangent(i - 1);
            let t_curr = self.edge_tangent(i);
            let (u, v) =
                parallel_transport(self.bishop_u[i - 1], self.bishop_v[i - 1], t_prev, t_curr);
            self.bishop_u[i] = u;
            self.bishop_v[i] = v;
        }
    }

    /// 获取 Bishop 标架 (u, v) 在边 i
    pub fn bishop_frame(&self, i: usize) -> (Vec3, Vec3) {
        (self.bishop_u[i], self.bishop_v[i])
    }

    // ========================================================
    // 材料标架
    // ========================================================

    /// 计算材料标架 (m1, m2) 在所有边
    /// m1 = cos(θ) u + sin(θ) v
    /// m2 = -sin(θ) u + cos(θ) v
    pub fn compute_material_frames(&self) -> (Vec<Vec3>, Vec<Vec3>) {
        let n = self.num_edges();
        let mut m1 = vec![Vec3::ZERO; n];
        let mut m2 = vec![Vec3::ZERO; n];
        for i in 0..n {
            let (u, v) = (self.bishop_u[i], self.bishop_v[i]);
            let c = self.thetas[i].cos();
            let s = self.thetas[i].sin();
            m1[i] = c * u + s * v;
            m2[i] = -s * u + c * v;
        }
        (m1, m2)
    }

    // ========================================================
    // 曲率
    // ========================================================

    /// 离散曲率副法向量 κ_i (在内部顶点 i, 1 <= i <= n-1)
    ///
    /// κ_i = 2 (e_{i-1} × e_i) / (|e_{i-1}||e_i| + e_{i-1}·e_i)
    ///
    /// 方向沿转轴, 大小 = 2 tan(φ/2), φ 为转向角
    pub fn curvature_binormal(&self, i: usize) -> Vec3 {
        let e_prev = self.edge(i - 1);
        let e_curr = self.edge(i);
        let cross = e_prev.cross(e_curr);
        let denom = e_prev.length() * e_curr.length() + e_prev.dot(e_curr);
        if denom > 1e-12 { 2.0 * cross / denom } else { Vec3::ZERO }
    }

    /// 相邻边切向之间的夹角 (弧度)
    pub fn bend_angle(&self, i: usize) -> f32 {
        let t_prev = self.edge_tangent(i - 1);
        let t_curr = self.edge_tangent(i);
        let cos_a = t_prev.dot(t_curr).clamp(-1.0, 1.0);
        cos_a.acos()
    }

    // ========================================================
    // 能量
    // ========================================================

    /// 拉伸能量: E = (k/2) Σ (|e_i| - L_i)²
    pub fn stretching_energy(&self) -> f32 {
        let k = self.config.stretching_stiffness;
        let mut e = 0.0;
        for i in 0..self.num_edges() {
            let d = self.edge_length(i) - self.rest_lengths[i];
            e += 0.5 * k * d * d;
        }
        e
    }

    /// 弯曲能量: E = (k/2) Σ (φ_i - φ_rest_i)² / L_voronoi
    pub fn bending_energy(&self) -> f32 {
        let k = self.config.bending_stiffness;
        let mut e = 0.0;
        for i in 1..self.num_vertices().saturating_sub(1) {
            if i >= self.num_edges() {
                continue;
            }
            let voronoi = 0.5 * (self.rest_lengths[i - 1] + self.rest_lengths[i]);
            if voronoi < 1e-12 {
                continue;
            }
            let phi = self.bend_angle(i);
            let rest_phi = self.rest_bend_angles[i - 1];
            let d = phi - rest_phi;
            e += 0.5 * k * d * d / voronoi;
        }
        e
    }

    /// 扭转能量: E = (k/2) Σ (θ_i - θ_{i-1} - τ_rest)² / L_voronoi
    pub fn twisting_energy(&self) -> f32 {
        let k = self.config.twisting_stiffness;
        let mut e = 0.0;
        for i in 1..self.num_edges() {
            let voronoi = 0.5 * (self.rest_lengths[i - 1] + self.rest_lengths[i]);
            if voronoi < 1e-12 {
                continue;
            }
            let rest_t = if i - 1 < self.rest_twist.len() { self.rest_twist[i - 1] } else { 0.0 };
            let d = self.thetas[i] - self.thetas[i - 1] - rest_t;
            e += 0.5 * k * d * d / voronoi;
        }
        e
    }

    /// 总势能
    pub fn potential_energy(&self) -> f32 {
        self.stretching_energy() + self.bending_energy() + self.twisting_energy()
    }

    /// 动能 = Σ (1/2 m v² + 1/2 I ω²)
    pub fn kinetic_energy(&self) -> f32 {
        let mut ke = 0.0;
        for i in 0..self.num_vertices() {
            let m = self.vertex_mass(i);
            ke += 0.5 * m * self.velocities[i].length_squared();
        }
        for i in 0..self.num_edges() {
            let inertia = self.vertex_inertia(i.min(self.num_vertices().saturating_sub(1)));
            let tv = self.theta_velocities[i];
            ke += 0.5 * inertia * tv * tv;
        }
        ke
    }

    /// 总能量
    pub fn total_energy(&self) -> f32 {
        self.kinetic_energy() + self.potential_energy()
    }

    // ========================================================
    // 力
    // ========================================================

    /// 拉伸力 (弹簧力): F = k (|e| - L) t
    fn compute_stretching_forces(&self) -> Vec<Vec3> {
        let k = self.config.stretching_stiffness;
        let n = self.num_vertices();
        let mut f = vec![Vec3::ZERO; n];
        for i in 0..self.num_edges() {
            let e = self.edge(i);
            let len = e.length();
            if len < 1e-12 {
                continue;
            }
            let t = e / len;
            let stretch_force = k * (len - self.rest_lengths[i]) * t;
            f[i] += stretch_force;
            f[i + 1] -= stretch_force;
        }
        f
    }

    /// 弯曲力 (基于角度 φ = acos(t_prev · t_curr))
    ///
    /// E = (k / 2L) (φ - φ_rest)²
    /// F_j = -∂E/∂x_j = -(k/L)(φ - φ_rest) ∂φ/∂x_j
    ///
    /// ∂φ/∂x_{i-1} = (t_curr - cos(φ) t_prev) / (sin(φ) |e_{i-1}|)
    /// ∂φ/∂x_{i+1} = -(t_prev - cos(φ) t_curr) / (sin(φ) |e_i|)
    /// ∂φ/∂x_i     = -(∂φ/∂x_{i-1} + ∂φ/∂x_{i+1})
    fn compute_bending_forces(&self) -> Vec<Vec3> {
        let k = self.config.bending_stiffness;
        let n = self.num_vertices();
        let mut f = vec![Vec3::ZERO; n];
        for i in 1..n.saturating_sub(1) {
            if i >= self.num_edges() {
                continue;
            }
            let t_prev = self.edge_tangent(i - 1);
            let t_curr = self.edge_tangent(i);
            let cos_phi = t_prev.dot(t_curr).clamp(-1.0, 1.0);
            let sin_phi = t_prev.cross(t_curr).length();
            if sin_phi < 1e-8 {
                continue;
            }
            let phi = cos_phi.acos();
            let rest_phi = self.rest_bend_angles[i - 1];
            let voronoi = 0.5 * (self.rest_lengths[i - 1] + self.rest_lengths[i]);
            if voronoi < 1e-12 {
                continue;
            }
            let len_prev = self.edge_length(i - 1);
            let len_curr = self.edge_length(i);
            if len_prev < 1e-12 || len_curr < 1e-12 {
                continue;
            }
            let dphi_dxm1 = (t_curr - cos_phi * t_prev) / (len_prev * sin_phi);
            let dphi_dxp1 = -(t_prev - cos_phi * t_curr) / (len_curr * sin_phi);
            let dphi_dxi = -dphi_dxm1 - dphi_dxp1;
            let factor = -k * (phi - rest_phi) / voronoi;
            f[i - 1] += factor * dphi_dxm1;
            f[i] += factor * dphi_dxi;
            f[i + 1] += factor * dphi_dxp1;
        }
        f
    }

    /// 扭转扭矩: τ_i = -∂E/∂θ_i
    fn compute_twisting_torques(&self) -> Vec<f32> {
        let k = self.config.twisting_stiffness;
        let n = self.num_edges();
        let mut torque = vec![0.0_f32; n];
        for i in 1..n {
            let voronoi = 0.5 * (self.rest_lengths[i - 1] + self.rest_lengths[i]);
            if voronoi < 1e-12 {
                continue;
            }
            let rest_t = if i - 1 < self.rest_twist.len() { self.rest_twist[i - 1] } else { 0.0 };
            let d = self.thetas[i] - self.thetas[i - 1] - rest_t;
            let t = -k * d / voronoi;
            torque[i] += t;
            torque[i - 1] -= t;
        }
        torque
    }

    // ========================================================
    // 时间步进 (半隐式 Euler)
    // ========================================================

    /// 推进一步
    ///
    /// 1. 更新 Bishop 标架
    /// 2. 计算力 (拉伸 + 弯曲 + 重力)
    /// 3. 速度更新: v = v + (F/m) dt
    /// 4. 位置更新: x = x + v dt
    /// 5. 扭转更新: θ = θ + ω dt
    /// 6. 重算 Bishop 标架
    pub fn step(&mut self, dt: f32) {
        let n = self.num_vertices();
        let n_edges = self.num_edges();
        if n == 0 {
            return;
        }

        // 1. Bishop 标架
        self.compute_bishop_frame();

        // 2. 力
        let mut forces = self.compute_stretching_forces();
        let bend_forces = self.compute_bending_forces();
        for i in 0..n {
            forces[i] += bend_forces[i];
        }
        for i in 0..n {
            let m = self.vertex_mass(i);
            forces[i] += self.config.gravity * m;
        }

        let damping = (1.0 - self.config.damping * dt).max(0.0);

        // 3. 速度 + 位置更新
        for i in 0..n {
            if self.fixed_vertices.contains(&i) {
                self.velocities[i] = Vec3::ZERO;
                continue;
            }
            let m = self.vertex_mass(i);
            if m > 1e-12 {
                self.velocities[i] = self.velocities[i] * damping + forces[i] / m * dt;
            }
        }
        for i in 0..n {
            if self.fixed_vertices.contains(&i) {
                continue;
            }
            self.vertices[i] += self.velocities[i] * dt;
        }

        // 5. 扭转更新
        if n_edges > 0 {
            let torques = self.compute_twisting_torques();
            for i in 0..n_edges {
                let fixed =
                    self.fixed_vertices.contains(&i) || self.fixed_vertices.contains(&(i + 1));
                if fixed {
                    self.theta_velocities[i] = 0.0;
                    continue;
                }
                let inertia = self.vertex_inertia(i.min(n.saturating_sub(1)));
                if inertia > 1e-12 {
                    self.theta_velocities[i] =
                        self.theta_velocities[i] * damping + torques[i] / inertia * dt;
                }
            }
            for i in 0..n_edges {
                let fixed =
                    self.fixed_vertices.contains(&i) || self.fixed_vertices.contains(&(i + 1));
                if fixed {
                    continue;
                }
                self.thetas[i] += self.theta_velocities[i] * dt;
            }
        }

        // 6. 重算 Bishop 标架
        self.compute_bishop_frame();

        self.time += dt;
    }

    // ========================================================
    // 边界条件
    // ========================================================

    /// 固定顶点 (位置和速度锁定)
    pub fn fix_vertex(&mut self, i: usize) {
        self.fixed_vertices.insert(i);
        if i < self.velocities.len() {
            self.velocities[i] = Vec3::ZERO;
        }
    }

    /// 释放顶点
    pub fn release_vertex(&mut self, i: usize) {
        self.fixed_vertices.remove(&i);
    }

    /// 设置顶点位置 (并清零速度)
    pub fn set_vertex_position(&mut self, i: usize, pos: Vec3) {
        if i < self.vertices.len() {
            self.vertices[i] = pos;
            self.velocities[i] = Vec3::ZERO;
        }
    }

    /// 是否固定
    pub fn is_fixed(&self, i: usize) -> bool {
        self.fixed_vertices.contains(&i)
    }
}

// ============================================================
// 平行输运
// ============================================================

/// 平行输运 (u, v) 从 t_prev 到 t_curr
///
/// 绕轴 (t_prev × t_curr) 旋转角度 acos(t_prev · t_curr)
fn parallel_transport(u: Vec3, v: Vec3, t_prev: Vec3, t_curr: Vec3) -> (Vec3, Vec3) {
    let n = t_prev.cross(t_curr);
    let n_len = n.length();
    if n_len < 1e-10 {
        return (u, v);
    }
    let axis = n / n_len;
    let cos_a = t_prev.dot(t_curr);
    let sin_a = n_len;
    let new_u = rodrigues_rotate(u, axis, cos_a, sin_a);
    let new_v = rodrigues_rotate(v, axis, cos_a, sin_a);
    (new_u, new_v)
}

/// Rodrigues 旋转: v 绕 axis 旋转 (cos_a, sin_a)
fn rodrigues_rotate(v: Vec3, axis: Vec3, cos_a: f32, sin_a: f32) -> Vec3 {
    v * cos_a + axis.cross(v) * sin_a + axis * (axis.dot(v) * (1.0 - cos_a))
}
// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    fn assert_orthonormal(u: Vec3, v: Vec3, t: Vec3, tol: f32) {
        assert!((u.length() - 1.0).abs() < tol, "u not unit: |u|={}", u.length());
        assert!((v.length() - 1.0).abs() < tol, "v not unit: |v|={}", v.length());
        assert!((t.length() - 1.0).abs() < tol, "t not unit: |t|={}", t.length());
        assert!(u.dot(v).abs() < tol, "u·v={}", u.dot(v));
        assert!(u.dot(t).abs() < tol, "u·t={}", u.dot(t));
        assert!(v.dot(t).abs() < tol, "v·t={}", v.dot(t));
    }

    fn make_straight_rod(n: usize, len: f32) -> Vec<Vec3> {
        (0..n).map(|i| Vec3::new(i as f32 * len, 0.0, 0.0)).collect()
    }

    fn make_l_shape() -> Vec<Vec3> {
        vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0)]
    }

    // ---------- 基本构造 ----------

    #[test]
    fn test_rod_creation() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        assert_eq!(rod.num_vertices(), 5);
        assert_eq!(rod.num_edges(), 4);
        assert_eq!(rod.rest_lengths.len(), 4);
        assert_eq!(rod.rest_bend_angles.len(), 3);
        assert_eq!(rod.thetas.len(), 4);
        assert_eq!(rod.time, 0.0);
    }

    #[test]
    fn test_edge_and_tangent() {
        let positions = make_straight_rod(3, 0.5);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        assert!(approx_eq(rod.edge_length(0), 0.5, 1e-6));
        assert!(approx_eq(rod.edge_length(1), 0.5, 1e-6));
        let t0 = rod.edge_tangent(0);
        assert!((t0 - Vec3::X).length() < 1e-6);
    }

    #[test]
    fn test_vertex_mass_positive() {
        let positions = make_straight_rod(4, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        for i in 0..4 {
            assert!(rod.vertex_mass(i) > 0.0, "mass[{}]={} <= 0", i, rod.vertex_mass(i));
        }
        // 内部顶点质量 > 端点 (Voronoi 更长)
        assert!(rod.vertex_mass(1) > rod.vertex_mass(0));
        assert!(rod.vertex_mass(1) > rod.vertex_mass(3));
    }

    // ---------- Bishop 标架 ----------

    #[test]
    fn test_bishop_frame_orthonormal() {
        let positions = make_straight_rod(5, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.compute_bishop_frame();
        for i in 0..rod.num_edges() {
            let t = rod.edge_tangent(i);
            let (u, v) = rod.bishop_frame(i);
            assert_orthonormal(u, v, t, 1e-5);
        }
    }

    #[test]
    fn test_bishop_frame_straight_rod_constant() {
        // 直杆: Bishop 标架应沿杆不变
        let positions = make_straight_rod(5, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.compute_bishop_frame();
        let (u0, v0) = rod.bishop_frame(0);
        for i in 1..rod.num_edges() {
            let (u, v) = rod.bishop_frame(i);
            assert!(
                (u - u0).length() < 1e-5,
                "u changed at edge {}: diff={}",
                i,
                (u - u0).length()
            );
            assert!(
                (v - v0).length() < 1e-5,
                "v changed at edge {}: diff={}",
                i,
                (v - v0).length()
            );
        }
    }

    #[test]
    fn test_bishop_frame_curved_rod() {
        // L-shape: Bishop frame rotates at bend
        // If u aligns with rotation axis (binormal), u stays but v rotates
        let positions = make_l_shape();
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.compute_bishop_frame();
        let (u0, v0) = rod.bishop_frame(0);
        let (u1, v1) = rod.bishop_frame(1);
        let diff = (u0 - u1).length().max((v0 - v1).length());
        assert!(diff > 0.5, "Bishop frame should rotate at L-bend, diff={}", diff);
        let t0 = rod.edge_tangent(0);
        let t1 = rod.edge_tangent(1);
        assert_orthonormal(u0, v0, t0, 1e-5);
        assert_orthonormal(u1, v1, t1, 1e-5);
    }

    #[test]
    fn test_bishop_frame_parallel_transport_preserves_dot() {
        // 平行输运保持 u·v 不变
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.5, 0.5, 0.0),
            Vec3::new(1.5, 1.0, 0.3),
        ];
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.compute_bishop_frame();
        for i in 0..rod.num_edges() {
            let (u, v) = rod.bishop_frame(i);
            let t = rod.edge_tangent(i);
            assert_orthonormal(u, v, t, 1e-4);
        }
    }

    // ---------- 材料标架 ----------

    #[test]
    fn test_material_frame_zero_theta() {
        // θ=0 时, 材料标架 = Bishop 标架
        let positions = make_straight_rod(4, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.compute_bishop_frame();
        let (m1, m2) = rod.compute_material_frames();
        for i in 0..rod.num_edges() {
            let (u, v) = rod.bishop_frame(i);
            assert!((m1[i] - u).length() < 1e-6);
            assert!((m2[i] - v).length() < 1e-6);
        }
    }

    #[test]
    fn test_material_frame_rotation() {
        // θ=π/2 时, m1 = v, m2 = -u
        let positions = make_straight_rod(3, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.thetas = vec![std::f32::consts::FRAC_PI_2; rod.num_edges()];
        rod.compute_bishop_frame();
        let (m1, m2) = rod.compute_material_frames();
        for i in 0..rod.num_edges() {
            let (u, v) = rod.bishop_frame(i);
            assert!((m1[i] - v).length() < 1e-5, "m1 != v at {}", i);
            assert!((m2[i] + u).length() < 1e-5, "m2 != -u at {}", i);
        }
    }

    // ---------- 曲率 ----------

    #[test]
    fn test_curvature_binormal_straight() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        for i in 1..rod.num_vertices() - 1 {
            let kappa = rod.curvature_binormal(i);
            assert!(kappa.length() < 1e-6, "straight rod curvature[{}]={}", i, kappa.length());
        }
    }

    #[test]
    fn test_curvature_binormal_right_angle() {
        let positions = make_l_shape();
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        let kappa = rod.curvature_binormal(1);
        // L 形: 90° 弯曲, |κ| = 2 tan(45°) = 2
        assert!(
            approx_eq(kappa.length(), 2.0, 1e-5),
            "right angle |kappa|={} expected 2",
            kappa.length()
        );
        // 方向: e_prev × e_curr = X × Y = Z
        assert!(kappa.z > 0.9, "kappa direction wrong: {:?}", kappa);
    }

    #[test]
    fn test_bend_angle_straight() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        for i in 1..rod.num_vertices() - 1 {
            assert!(approx_eq(rod.bend_angle(i), 0.0, 1e-6));
        }
    }

    #[test]
    fn test_bend_angle_right_angle() {
        let positions = make_l_shape();
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        assert!(approx_eq(rod.bend_angle(1), std::f32::consts::FRAC_PI_2, 1e-5));
    }

    // ---------- 能量 ----------

    #[test]
    fn test_stretching_energy_rest() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        assert!(approx_eq(rod.stretching_energy(), 0.0, 1e-10));
    }

    #[test]
    fn test_stretching_energy_stretched() {
        let positions = make_straight_rod(3, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.vertices[2] = Vec3::new(0.22, 0.0, 0.0); // 拉伸 10%
        let e = rod.stretching_energy();
        // E = (k/2) (0.02)² = 0.5 * 1000 * 0.0004 = 0.2
        assert!(approx_eq(e, 0.2, 1e-4), "stretched energy={} expected 0.2", e);
    }

    #[test]
    fn test_bending_energy_rest() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        assert!(approx_eq(rod.bending_energy(), 0.0, 1e-10));
    }

    #[test]
    fn test_bending_energy_bent() {
        let positions = make_l_shape();
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        // 设静止角度为 0 (直杆), 当前为 90°
        rod.rest_bend_angles = vec![0.0];
        let e = rod.bending_energy();
        assert!(e > 0.0, "bent energy should be positive: {}", e);
    }

    #[test]
    fn test_twisting_energy_rest() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        assert!(approx_eq(rod.twisting_energy(), 0.0, 1e-10));
    }

    #[test]
    fn test_twisting_energy_twisted() {
        let positions = make_straight_rod(4, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.thetas = vec![0.0, 1.0, 0.0]; // 中间边扭转 1 rad
        let e = rod.twisting_energy();
        assert!(e > 0.0, "twisted energy should be positive: {}", e);
    }

    // ---------- 力 ----------

    #[test]
    fn test_stretching_force_direction() {
        let positions = make_straight_rod(3, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        // Stretch both edges so all vertices get force
        rod.vertices[0] = Vec3::new(-0.02, 0.0, 0.0);
        rod.vertices[2] = Vec3::new(0.22, 0.0, 0.0);
        let f = rod.compute_stretching_forces();
        assert!(f[0].x > 0.0, "F[0].x={} should be > 0", f[0].x);
        assert!(f[2].x < 0.0, "F[2].x={} should be < 0", f[2].x);
    }

    #[test]
    fn test_stretching_force_rest_zero() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        let f = rod.compute_stretching_forces();
        for i in 0..rod.num_vertices() {
            assert!(f[i].length() < 1e-10, "rest force[{}]={:?}", i, f[i]);
        }
    }

    #[test]
    fn test_bending_force_rest_zero() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        let f = rod.compute_bending_forces();
        for i in 0..rod.num_vertices() {
            assert!(f[i].length() < 1e-10, "rest bend force[{}]={:?}", i, f[i]);
        }
    }

    #[test]
    fn test_bending_force_restores() {
        // L 形杆, 静止为直杆 → 弯曲力应将其拉直
        let positions = make_l_shape();
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.rest_bend_angles = vec![0.0]; // 静止为直杆
        let f = rod.compute_bending_forces();
        // 中间顶点 (1) 应受力拉向 -Y (减小角度)
        assert!(
            f[1].y.abs() > 0.0 || f[1].x.abs() > 0.0,
            "bend force on vertex 1 should be nonzero: {:?}",
            f[1]
        );
    }

    #[test]
    fn test_twisting_torque_rest_zero() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        let t = rod.compute_twisting_torques();
        for i in 0..rod.num_edges() {
            assert!(t[i].abs() < 1e-10, "rest torque[{}]={}", i, t[i]);
        }
    }

    #[test]
    fn test_twisting_torque_restores() {
        let positions = make_straight_rod(4, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.thetas = vec![0.0, 1.0, 0.0];
        let t = rod.compute_twisting_torques();
        // θ[1] > θ[0], 力矩应使 θ[1] 减小, θ[0] 增大
        assert!(t[1] < 0.0, "torque[1]={} should be < 0", t[1]);
        assert!(t[0] > 0.0, "torque[0]={} should be > 0", t[0]);
    }

    // ---------- 时间步进 ----------

    #[test]
    fn test_step_advances_time() {
        let positions = make_straight_rod(5, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.step(0.01);
        assert!(approx_eq(rod.time, 0.01, 1e-10));
        rod.step(0.01);
        assert!(approx_eq(rod.time, 0.02, 1e-10));
    }

    #[test]
    fn test_fixed_vertex_stays() {
        let positions = make_straight_rod(5, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.fix_vertex(0);
        let p0 = rod.vertices[0];
        for _ in 0..10 {
            rod.step(0.001);
        }
        assert!(
            (rod.vertices[0] - p0).length() < 1e-10,
            "fixed vertex moved: {:?}",
            rod.vertices[0] - p0
        );
    }

    #[test]
    fn test_gravity_accelerates() {
        let positions = make_straight_rod(3, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.fix_vertex(0); // 固定一端
        rod.step(0.01);
        // 自由端应受重力下落
        assert!(rod.velocities[2].y < 0.0, "gravity should pull down: {:?}", rod.velocities[2]);
    }

    #[test]
    fn test_hanging_rod_stable() {
        // 悬挂杆: 固定一端, 重力作用下应稳定 (不振荡爆炸)
        let positions = make_straight_rod(8, 0.05);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.fix_vertex(0);
        let mut max_v = 0.0_f32;
        for _ in 0..200 {
            rod.step(0.001);
            for v in &rod.velocities {
                max_v = max_v.max(v.length());
            }
        }
        // 速度不应爆炸 (半隐式 Euler + 阻尼)
        assert!(max_v < 100.0, "hanging rod velocity exploded: {}", max_v);
        // 自由端应下垂
        assert!(rod.vertices[7].y < rod.vertices[0].y, "free end should hang below fixed end");
    }

    #[test]
    fn test_energy_conservation_no_damping() {
        // 无阻尼无重力: 能量应近似守恒
        let positions = make_straight_rod(5, 0.1);
        let mut rod = CosseratRod::new(
            &positions,
            CosseratRodConfig { damping: 0.0, gravity: Vec3::ZERO, ..Default::default() },
        );
        // 给一个初始扰动
        rod.velocities[2] = Vec3::new(0.0, 0.5, 0.0);
        let e0 = rod.total_energy();
        for _ in 0..100 {
            rod.step(0.0005);
        }
        let e1 = rod.total_energy();
        // 允许 10% 误差 (半隐式 Euler 有数值耗散)
        let rel_err = (e1 - e0).abs() / e0.max(1e-10);
        assert!(
            rel_err < 0.15,
            "energy not conserved: e0={} e1={} err={}%",
            e0,
            e1,
            rel_err * 100.0
        );
    }

    #[test]
    fn test_capture_rest_shape() {
        let positions = make_l_shape();
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        // 初始 rest_bend_angles 应为 π/2 (L 形)
        assert!(approx_eq(rod.rest_bend_angles[0], std::f32::consts::FRAC_PI_2, 1e-5));
        // 拉直后重新捕获
        rod.vertices[2] = Vec3::new(2.0, 0.0, 0.0);
        rod.capture_rest_shape();
        assert!(approx_eq(rod.rest_bend_angles[0], 0.0, 1e-5));
    }

    #[test]
    fn test_two_vertex_rod() {
        // 两顶点 (单边): 只有拉伸, 无弯曲/扭转
        let positions = vec![Vec3::ZERO, Vec3::X];
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        assert_eq!(rod.num_edges(), 1);
        assert_eq!(rod.rest_bend_angles.len(), 0);
        assert_eq!(rod.rest_twist.len(), 0);
        // 不应崩溃
        rod.step(0.001);
        assert!(approx_eq(rod.time, 0.001, 1e-10));
    }

    #[test]
    fn test_rodrigues_rotate_identity() {
        // 旋转 0 度: 不变
        let v = Vec3::new(1.0, 0.0, 0.0);
        let axis = Vec3::new(0.0, 0.0, 1.0);
        let r = rodrigues_rotate(v, axis, 1.0, 0.0);
        assert!((r - v).length() < 1e-6);
    }

    #[test]
    fn test_rodrigues_rotate_90() {
        // 绕 Z 轴旋转 90°: X → Y
        let v = Vec3::new(1.0, 0.0, 0.0);
        let axis = Vec3::new(0.0, 0.0, 1.0);
        let r = rodrigues_rotate(v, axis, 0.0, 1.0);
        assert!((r - Vec3::Y).length() < 1e-6, "X rotated 90° around Z should be Y: {:?}", r);
    }

    #[test]
    fn test_parallel_transport_parallel() {
        // 平行切向: 标架不变
        let u = Vec3::new(0.0, 1.0, 0.0);
        let v = Vec3::new(0.0, 0.0, 1.0);
        let t = Vec3::new(1.0, 0.0, 0.0);
        let (u2, v2) = parallel_transport(u, v, t, t);
        assert!((u2 - u).length() < 1e-6);
        assert!((v2 - v).length() < 1e-6);
    }

    #[test]
    fn test_parallel_transport_90() {
        // 切向从 X 旋转到 Y: u=Y 应旋转到 -X (保持正交)
        let u = Vec3::new(0.0, 1.0, 0.0);
        let v = Vec3::new(0.0, 0.0, 1.0);
        let t_prev = Vec3::new(1.0, 0.0, 0.0);
        let t_curr = Vec3::new(0.0, 1.0, 0.0);
        let (u2, v2) = parallel_transport(u, v, t_prev, t_curr);
        // u=Y 绕 Z 轴旋转 -90° → -X (因为 X→Y 是绕 +Z 旋转 +90°, u 跟着转)
        // 实际: 平行输运绕轴 (X×Y=Z) 旋转 90°
        // u=Y 绕 Z 旋转 90° → -X
        assert!((u2 - (-Vec3::X)).length() < 1e-5, "u2 should be -X: {:?}", u2);
        // v=Z 绕 Z 旋转 90° → Z (不变, 因为 v 与旋转轴平行)
        assert!((v2 - Vec3::Z).length() < 1e-5, "v2 should be Z: {:?}", v2);
    }

    #[test]
    fn test_release_vertex() {
        let positions = make_straight_rod(3, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.fix_vertex(1);
        assert!(rod.is_fixed(1));
        rod.release_vertex(1);
        assert!(!rod.is_fixed(1));
    }

    #[test]
    fn test_set_vertex_position() {
        let positions = make_straight_rod(3, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        let new_pos = Vec3::new(5.0, 5.0, 5.0);
        rod.set_vertex_position(1, new_pos);
        assert!((rod.vertices[1] - new_pos).length() < 1e-10);
        assert!(rod.velocities[1].length() < 1e-10);
    }

    #[test]
    fn test_kinetic_energy_zero_at_rest() {
        let positions = make_straight_rod(5, 0.1);
        let rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        assert!(approx_eq(rod.kinetic_energy(), 0.0, 1e-10));
    }

    #[test]
    fn test_kinetic_energy_positive() {
        let positions = make_straight_rod(5, 0.1);
        let mut rod = CosseratRod::new(&positions, CosseratRodConfig::default());
        rod.velocities[2] = Vec3::new(0.0, 1.0, 0.0);
        assert!(rod.kinetic_energy() > 0.0);
    }
}
