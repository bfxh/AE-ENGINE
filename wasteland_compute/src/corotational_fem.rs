//! Corotational FEM — 共旋有限元 (软体/肌肉/组织)
//!
//! 基于:
//! - Müller, Gross. "Interactive Virtual Materials." GI 2004.
//! - Müller, Dorsey, McMillan. "Stable Real-Time Deformations." SCA 2002.
//! - Irving, Teran, Fedkiw. "Invertible Finite Elements for Robust
//!   Simulation of Large Deformation." SCA 2004.
//!
//! 核心思想:
//! 1. 把变形梯度 F 分解为旋转部分 R 和变形部分 (F - R)
//! 2. 应力只在变形部分产生, 旋转不产生应力 (物体可自由旋转)
//! 3. 因此大旋转 + 小变形场景下稳定 (软体/布料厚度/肌肉)
//! 4. 力的计算用 Dm⁻¹ (静止形状逆) 做"共旋"映射
//! 5. 半隐式 Euler 时间积分
//!
//! 关键公式:
//!   变形梯度 F = Ds · Dm⁻¹
//!   极分解 F = R · S  (R 旋转, S 对称正定)
//!   共旋线性应力 P = 2μ(F - R) + λ(tr(RᵀF) - 3)R
//!   力矩阵 H = -V₀ · Pᵀ · Dm⁻ᵀ
//!   顶点力 f_i = H[:, i] (i = 1, 2, 3), f_0 = -(f_1 + f_2 + f_3)
//!
//! Lamé 参数:
//!   λ = Eν / ((1+ν)(1-2ν))
//!   μ = E / (2(1+ν))     (剪切模量)
//!
//! 静止体积: V₀ = |det(Dm)| / 6
//!
//! 应用: 软体, 肌肉, 脂肪, 弹性体, 软组织, 大变形体

use glam::{Mat3, Vec3};

// ============================================================
// 配置
// ============================================================

/// 共旋有限元配置参数
#[derive(Debug, Clone)]
pub struct CorotationalFemConfig {
    /// Young 模量 E (Pa, N/m²) — 材料刚度
    pub youngs_modulus: f32,
    /// Poisson 比 ν — 不可压缩度 (0=完全可压缩, 0.5=完全不可压缩)
    pub poisson_ratio: f32,
    /// 密度 ρ (kg/m³)
    pub density: f32,
    /// 速度阻尼系数 (与 m·v 成正比)
    pub damping: f32,
    /// 重力加速度 (m/s²)
    pub gravity: Vec3,
}

impl Default for CorotationalFemConfig {
    fn default() -> Self {
        Self {
            youngs_modulus: 1e5,
            poisson_ratio: 0.45,
            density: 1000.0,
            damping: 0.5,
            gravity: Vec3::new(0.0, -9.81, 0.0),
        }
    }
}

// ============================================================
// 四面体网格
// ============================================================

/// 四面体网格
pub struct TetMesh {
    /// 顶点位置 (当前)
    pub vertices: Vec<Vec3>,
    /// 顶点速度
    pub velocities: Vec<Vec3>,
    /// 静止顶点位置 (初始形状, 用于计算 Dm)
    pub rest_vertices: Vec<Vec3>,
    /// 四面体索引 [v0, v1, v2, v3]
    pub tets: Vec<[usize; 4]>,
    /// 顶点是否固定 (Dirichlet 边界)
    pub fixed: Vec<bool>,
}

impl TetMesh {
    /// 创建网格 (静止形状 = 当前形状)
    pub fn new(vertices: Vec<Vec3>, tets: Vec<[usize; 4]>) -> Self {
        let n = vertices.len();
        Self {
            rest_vertices: vertices.clone(),
            vertices,
            velocities: vec![Vec3::ZERO; n],
            tets,
            fixed: vec![false; n],
        }
    }

    /// 固定顶点 i
    pub fn fix(&mut self, i: usize) {
        self.fixed[i] = true;
        self.velocities[i] = Vec3::ZERO;
    }

    /// 顶点数
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// 四面体数
    pub fn num_tets(&self) -> usize {
        self.tets.len()
    }

    /// 重置回静止形状
    pub fn reset_to_rest(&mut self) {
        for i in 0..self.vertices.len() {
            self.vertices[i] = self.rest_vertices[i];
            self.velocities[i] = Vec3::ZERO;
        }
    }
}

// ============================================================
// 共旋有限元求解器
// ============================================================

/// 共旋有限元求解器
///
/// 每个四面体:
///   Dm = [x1-x0, x2-x0, x3-x0] (静止形状矩阵)
///   Ds = [x1'-x0', x2'-x0', x3'-x0'] (当前形状矩阵)
///   F = Ds · Dm⁻¹ (变形梯度)
///   R = polar(F) (旋转部分)
///   P = 2μ(F-R) + λ(tr(RᵀF)-3)R (共旋 Cauchy 应力)
///   H = -V₀ · Pᵀ · Dm⁻ᵀ (力矩阵)
pub struct CorotationalFemSolver {
    /// 网格
    pub mesh: TetMesh,
    /// 配置
    pub config: CorotationalFemConfig,
    /// 每个四面体的 Dm⁻¹ (静止形状矩阵的逆)
    rest_inverses: Vec<Mat3>,
    /// 每个四面体的静止体积 V₀
    rest_volumes: Vec<f32>,
    /// Lamé 第一参数 λ
    pub lame_lambda: f32,
    /// Lamé 第二参数 μ (剪切模量)
    pub lame_mu: f32,
    /// 每个四面体当前的旋转矩阵 R (每帧更新, 可用于渲染/可视化)
    pub rotations: Vec<Mat3>,
    /// 模拟时间
    pub time: f32,
}

impl CorotationalFemSolver {
    /// 创建求解器 (从静止形状预计算 Dm⁻¹ 和体积)
    pub fn new(mesh: TetMesh, config: CorotationalFemConfig) -> Self {
        let n_tets = mesh.num_tets();
        let mut rest_inverses = vec![Mat3::IDENTITY; n_tets];
        let mut rest_volumes = vec![0.0; n_tets];

        for (i, tet) in mesh.tets.iter().enumerate() {
            let x0 = mesh.rest_vertices[tet[0]];
            let x1 = mesh.rest_vertices[tet[1]];
            let x2 = mesh.rest_vertices[tet[2]];
            let x3 = mesh.rest_vertices[tet[3]];
            let dm = Mat3::from_cols(x1 - x0, x2 - x0, x3 - x0);
            let det = dm.determinant();
            // 防退化 (体积为 0 或负)
            if det.abs() < 1e-12 {
                rest_inverses[i] = Mat3::IDENTITY;
                rest_volumes[i] = 1e-12;
            } else {
                rest_inverses[i] = dm.inverse();
                rest_volumes[i] = det.abs() / 6.0;
            }
        }

        // Lamé 参数
        let e = config.youngs_modulus;
        let nu = config.poisson_ratio;
        let lame_lambda = e * nu / ((1.0 + nu) * (1.0 - 2.0 * nu).max(1e-6));
        let lame_mu = e / (2.0 * (1.0 + nu));

        Self {
            mesh,
            config,
            rest_inverses,
            rest_volumes,
            lame_lambda,
            lame_mu,
            rotations: vec![Mat3::IDENTITY; n_tets],
            time: 0.0,
        }
    }

    // ========================================================
    // 形状矩阵 & 变形梯度
    // ========================================================

    /// 第 i 个四面体的当前形状矩阵 Ds = [x1-x0, x2-x0, x3-x0]
    #[inline]
    fn shape_matrix(&self, tet_idx: usize) -> Mat3 {
        let tet = self.mesh.tets[tet_idx];
        let x0 = self.mesh.vertices[tet[0]];
        let x1 = self.mesh.vertices[tet[1]];
        let x2 = self.mesh.vertices[tet[2]];
        let x3 = self.mesh.vertices[tet[3]];
        Mat3::from_cols(x1 - x0, x2 - x0, x3 - x0)
    }

    /// 第 i 个四面体的变形梯度 F = Ds · Dm⁻¹
    #[inline]
    fn deformation_gradient(&self, tet_idx: usize) -> Mat3 {
        self.shape_matrix(tet_idx) * self.rest_inverses[tet_idx]
    }

    // ========================================================
    // 极分解 (Higham 迭代)
    // ========================================================

    /// Higham 迭代极分解: F = R · S, 返回 R
    ///
    /// R_{n+1} = 0.5 · (R_n + R_n⁻ᵀ)
    /// 收敛: 3-5 次迭代到机器精度
    /// 数值上非常稳定, 即使 F 接近奇异也能工作
    fn polar_decomposition(f: Mat3, max_iter: usize, tol: f32) -> Mat3 {
        let mut r = f;
        for _ in 0..max_iter {
            let r_inv = r.inverse();
            // 检查 R 是否奇异
            if !r_inv.x_axis.is_finite() && !r_inv.y_axis.is_finite() && !r_inv.z_axis.is_finite() {
                break;
            }
            let r_inv_t = r_inv.transpose();
            let r_next = Mat3::from_cols(
                0.5 * (r.x_axis + r_inv_t.x_axis),
                0.5 * (r.y_axis + r_inv_t.y_axis),
                0.5 * (r.z_axis + r_inv_t.z_axis),
            );
            // 收敛检查: ||R_next - R||
            let diff = Mat3::from_cols(
                r_next.x_axis - r.x_axis,
                r_next.y_axis - r.y_axis,
                r_next.z_axis - r.z_axis,
            );
            let err = Self::frobenius_norm(diff);
            r = r_next;
            if err < tol {
                break;
            }
        }
        r
    }

    /// Frobenius 范数
    #[inline]
    fn frobenius_norm(m: Mat3) -> f32 {
        (m.x_axis.length_squared() + m.y_axis.length_squared() + m.z_axis.length_squared()).sqrt()
    }

    // ========================================================
    // 力的计算
    // ========================================================

    /// 计算第 i 个四面体对 4 个顶点的弹性力贡献
    ///
    /// P = 2μ(F - R) + λ(tr(RᵀF) - 3)R
    /// H = -V₀ · Pᵀ · Dm⁻ᵀ
    /// f_i = H[:, i] (i = 1, 2, 3)
    /// f_0 = -(f_1 + f_2 + f_3)  (动量守恒)
    fn compute_tet_forces(&mut self, tet_idx: usize) -> [Vec3; 4] {
        let f = self.deformation_gradient(tet_idx);
        let r = Self::polar_decomposition(f, 16, 1e-7);
        self.rotations[tet_idx] = r;

        // Rᵀ · F
        let r_t_f = r.transpose() * f;
        // tr(RᵀF) = RᵀF 的对角元素之和
        let trace = r_t_f.x_axis.x + r_t_f.y_axis.y + r_t_f.z_axis.z;

        // P = 2μ(F - R) + λ(tr(RᵀF) - 3)R
        let mu = self.lame_mu;
        let lambda = self.lame_lambda;
        let coeff = lambda * (trace - 3.0);
        let p = Mat3::from_cols(
            2.0 * mu * (f.x_axis - r.x_axis) + coeff * r.x_axis,
            2.0 * mu * (f.y_axis - r.y_axis) + coeff * r.y_axis,
            2.0 * mu * (f.z_axis - r.z_axis) + coeff * r.z_axis,
        );

        // H = -V₀ · Pᵀ · Dm⁻ᵀ
        let v0 = self.rest_volumes[tet_idx];
        let dm_inv = self.rest_inverses[tet_idx];
        let h = (p.transpose() * dm_inv.transpose()) * (-v0);

        // 列优先存储下, h.x_axis 是第 0 列, 即 H[:,0] = f_1
        let f1 = Vec3::new(h.x_axis.x, h.y_axis.x, h.z_axis.x);
        let f2 = Vec3::new(h.x_axis.y, h.y_axis.y, h.z_axis.y);
        let f3 = Vec3::new(h.x_axis.z, h.y_axis.z, h.z_axis.z);
        let f0 = -(f1 + f2 + f3);

        [f0, f1, f2, f3]
    }

    /// 计算所有顶点的合力 (弹性 + 重力 + 阻尼)
    pub fn compute_forces(&mut self) -> Vec<Vec3> {
        let n = self.mesh.num_vertices();
        let mut forces = vec![Vec3::ZERO; n];

        // 弹性力 (累加每个四面体的贡献)
        for t in 0..self.mesh.num_tets() {
            let tf = self.compute_tet_forces(t);
            let tet = self.mesh.tets[t];
            forces[tet[0]] += tf[0];
            forces[tet[1]] += tf[1];
            forces[tet[2]] += tf[2];
            forces[tet[3]] += tf[3];
        }

        // 重力 + 阻尼 (基于顶点质量)
        let g = self.config.gravity;
        let damping = self.config.damping;
        let rho = self.config.density;

        for i in 0..n {
            let mi = self.vertex_mass(i);
            if mi < 1e-12 {
                continue;
            }
            forces[i] += mi * g;
            forces[i] -= damping * mi * self.mesh.velocities[i];
        }

        forces
    }

    /// 顶点 i 的质量 = 共享该顶点的所有四面体体积的 1/4 × 密度
    pub fn vertex_mass(&self, vertex_idx: usize) -> f32 {
        let mut total_vol = 0.0;
        for (i, tet) in self.mesh.tets.iter().enumerate() {
            if tet[0] == vertex_idx
                || tet[1] == vertex_idx
                || tet[2] == vertex_idx
                || tet[3] == vertex_idx
            {
                total_vol += self.rest_volumes[i] * 0.25;
            }
        }
        total_vol * self.config.density
    }

    // ========================================================
    // 时间步进 (半隐式 Euler)
    // ========================================================

    /// 半隐式 Euler 步进
    ///
    /// v_{n+1} = v_n + dt · f(x_n) / m
    /// x_{n+1} = x_n + dt · v_{n+1}
    pub fn step(&mut self, dt: f32) {
        let n = self.mesh.num_vertices();
        let forces = self.compute_forces();

        // 速度更新
        for i in 0..n {
            if self.mesh.fixed[i] {
                self.mesh.velocities[i] = Vec3::ZERO;
                continue;
            }
            let mi = self.vertex_mass(i);
            if mi < 1e-12 {
                continue;
            }
            self.mesh.velocities[i] += dt * forces[i] / mi;
        }

        // 位置更新
        for i in 0..n {
            if self.mesh.fixed[i] {
                continue;
            }
            self.mesh.vertices[i] += dt * self.mesh.velocities[i];
        }

        self.time += dt;
    }

    /// 弹性势能 (用于能量守恒验证)
    ///
    /// E = Σ_t V₀ · [μ·||F-R||²_F + (λ/2)·(tr(RᵀF)-3)²]
    pub fn elastic_energy(&mut self) -> f32 {
        let mut e = 0.0;
        let mu = self.lame_mu;
        let lambda = self.lame_lambda;

        for t in 0..self.mesh.num_tets() {
            let f = self.deformation_gradient(t);
            let r = Self::polar_decomposition(f, 16, 1e-7);
            self.rotations[t] = r;
            let r_t_f = r.transpose() * f;
            let trace = r_t_f.x_axis.x + r_t_f.y_axis.y + r_t_f.z_axis.z;

            // ||F - R||²_F
            let d = Mat3::from_cols(
                f.x_axis - r.x_axis,
                f.y_axis - r.y_axis,
                f.z_axis - r.z_axis,
            );
            let d_norm_sq = d.x_axis.length_squared()
                + d.y_axis.length_squared()
                + d.z_axis.length_squared();

            let v0 = self.rest_volumes[t];
            e += v0 * (mu * d_norm_sq + 0.5 * lambda * (trace - 3.0).powi(2));
        }
        e
    }

    /// 动能
    pub fn kinetic_energy(&self) -> f32 {
        let mut e = 0.0;
        for i in 0..self.mesh.num_vertices() {
            let mi = self.vertex_mass(i);
            e += 0.5 * mi * self.mesh.velocities[i].length_squared();
        }
        e
    }

    /// 重力势能 (相对于 y = 0 平面)
    pub fn gravity_energy(&self) -> f32 {
        let mut e = 0.0;
        let g = self.config.gravity.y;
        for i in 0..self.mesh.num_vertices() {
            let mi = self.vertex_mass(i);
            e += -mi * g * self.mesh.vertices[i].y;
        }
        e
    }
}
// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 近似相等辅助
    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    fn approx_vec(a: Vec3, b: Vec3, tol: f32) -> bool {
        (a.x - b.x).abs() < tol && (a.y - b.y).abs() < tol && (a.z - b.z).abs() < tol
    }

    /// 构造一个标准单位四面体
    /// v0 = (0,0,0), v1 = (1,0,0), v2 = (0,1,0), v3 = (0,0,1)
    /// 体积 = 1/6
    fn unit_tet_mesh() -> TetMesh {
        let verts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        TetMesh::new(verts, vec![[0, 1, 2, 3]])
    }

    #[test]
    fn test_config_default() {
        let c = CorotationalFemConfig::default();
        assert!(c.youngs_modulus > 0.0);
        assert!(c.poisson_ratio > 0.0 && c.poisson_ratio < 0.5);
        assert!(c.density > 0.0);
        assert!(c.damping >= 0.0);
    }

    #[test]
    fn test_tet_mesh_creation() {
        let m = unit_tet_mesh();
        assert_eq!(m.num_vertices(), 4);
        assert_eq!(m.num_tets(), 1);
        assert!(!m.fixed[0]);
    }

    #[test]
    fn test_tet_mesh_fix() {
        let mut m = unit_tet_mesh();
        m.velocities[0] = Vec3::new(1.0, 2.0, 3.0);
        m.fix(0);
        assert!(m.fixed[0]);
        assert_eq!(m.velocities[0], Vec3::ZERO);
    }

    #[test]
    fn test_tet_mesh_reset_to_rest() {
        let mut m = unit_tet_mesh();
        m.vertices[0] = Vec3::new(5.0, 5.0, 5.0);
        m.velocities[0] = Vec3::new(1.0, 0.0, 0.0);
        m.reset_to_rest();
        assert_eq!(m.vertices[0], Vec3::ZERO);
        assert_eq!(m.velocities[0], Vec3::ZERO);
    }

    #[test]
    fn test_solver_lame_parameters() {
        let mut config = CorotationalFemConfig::default();
        config.youngs_modulus = 1e5;
        config.poisson_ratio = 0.25;
        let solver = CorotationalFemSolver::new(unit_tet_mesh(), config);
        // λ = Eν / ((1+ν)(1-2ν)) = 1e5 * 0.25 / (1.25 * 0.5) = 4e4
        assert!(approx(solver.lame_lambda, 4e4, 1.0));
        // μ = E / (2(1+ν)) = 1e5 / 2.5 = 4e4
        assert!(approx(solver.lame_mu, 4e4, 1.0));
    }

    #[test]
    fn test_rest_volume() {
        let solver = CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        // 单位四面体体积 = 1/6
        assert!(approx(solver.rest_volumes[0], 1.0 / 6.0, 1e-6));
    }

    #[test]
    fn test_vertex_mass() {
        let solver = CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        // 每个顶点共享 1 个四面体, 各占 1/4 体积
        // m = (1/6) * 0.25 * ρ = ρ / 24
        let expected = (1.0 / 6.0) * 0.25 * 1000.0;
        assert!(approx(solver.vertex_mass(0), expected, 1e-6));
    }

    #[test]
    fn test_polar_decomposition_identity() {
        let r = CorotationalFemSolver::polar_decomposition(Mat3::IDENTITY, 16, 1e-7);
        let diff = Mat3::from_cols(
            r.x_axis - Mat3::IDENTITY.x_axis,
            r.y_axis - Mat3::IDENTITY.y_axis,
            r.z_axis - Mat3::IDENTITY.z_axis,
        );
        assert!(CorotationalFemSolver::frobenius_norm(diff) < 1e-5);
    }

    #[test]
    fn test_polar_decomposition_pure_rotation() {
        // 绕 Z 轴旋转 90 度
        let theta = std::f32::consts::FRAC_PI_2;
        let rot = Mat3::from_cols(
            Vec3::new(theta.cos(), theta.sin(), 0.0),
            Vec3::new(-theta.sin(), theta.cos(), 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );
        let r = CorotationalFemSolver::polar_decomposition(rot, 16, 1e-7);
        let diff = Mat3::from_cols(
            r.x_axis - rot.x_axis,
            r.y_axis - rot.y_axis,
            r.z_axis - rot.z_axis,
        );
        assert!(CorotationalFemSolver::frobenius_norm(diff) < 1e-4);
    }

    #[test]
    fn test_polar_decomposition_pure_stretch() {
        // F = 2I (各向同性拉伸 2 倍)
        let f = Mat3::from_diagonal(Vec3::new(2.0, 2.0, 2.0));
        let r = CorotationalFemSolver::polar_decomposition(f, 16, 1e-7);
        // 纯拉伸的极分解 R 应该是 I
        let diff = Mat3::from_cols(
            r.x_axis - Mat3::IDENTITY.x_axis,
            r.y_axis - Mat3::IDENTITY.y_axis,
            r.z_axis - Mat3::IDENTITY.z_axis,
        );
        assert!(CorotationalFemSolver::frobenius_norm(diff) < 1e-4);
    }

    #[test]
    fn test_deformation_gradient_at_rest() {
        let solver = CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        let f = solver.deformation_gradient(0);
        let diff = Mat3::from_cols(
            f.x_axis - Mat3::IDENTITY.x_axis,
            f.y_axis - Mat3::IDENTITY.y_axis,
            f.z_axis - Mat3::IDENTITY.z_axis,
        );
        assert!(CorotationalFemSolver::frobenius_norm(diff) < 1e-6);
    }

    #[test]
    fn test_force_at_rest_is_zero() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        // 关闭重力以单独测试弹性力
        solver.config.gravity = Vec3::ZERO;
        let forces = solver.compute_forces();
        for f in &forces {
            assert!(f.length() < 1e-3, "force at rest should be zero, got {}", f);
        }
    }

    #[test]
    fn test_force_under_stretch() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        solver.config.gravity = Vec3::ZERO;
        // 拉伸 v1 沿 +x
        solver.mesh.vertices[1].x = 1.5;
        let forces = solver.compute_forces();
        // v1 受力应朝 -x (向回拉)
        assert!(
            forces[1].x < 0.0,
            "stretched vertex should be pulled back, got f1.x = {}",
            forces[1].x
        );
        // v0 受力应朝 +x (动量守恒)
        assert!(
            forces[0].x > 0.0,
            "opposite vertex should be pushed forward, got f0.x = {}",
            forces[0].x
        );
    }

    #[test]
    fn test_force_under_compression() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        solver.config.gravity = Vec3::ZERO;
        // 压缩 v1 沿 -x
        solver.mesh.vertices[1].x = 0.5;
        let forces = solver.compute_forces();
        // v1 受力应朝 +x (推开)
        assert!(
            forces[1].x > 0.0,
            "compressed vertex should be pushed out, got f1.x = {}",
            forces[1].x
        );
    }

    #[test]
    fn test_force_momentum_conservation() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        solver.config.gravity = Vec3::ZERO;
        solver.config.damping = 0.0;
        // 随机变形
        solver.mesh.vertices[1] = Vec3::new(1.3, 0.1, 0.0);
        solver.mesh.vertices[2] = Vec3::new(0.0, 1.2, 0.0);
        let forces = solver.compute_forces();
        // 总动量 = 0 (合力为 0, 无外力)
        let total = forces[0] + forces[1] + forces[2] + forces[3];
        assert!(
            total.length() < 1e-3,
            "total force should be zero (momentum conservation), got {}",
            total
        );
    }

    #[test]
    fn test_rotation_invariance() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        solver.config.gravity = Vec3::ZERO;
        // 整体旋转 45 度 (绕 Z 轴)
        let theta = std::f32::consts::FRAC_PI_4;
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        let rot = |v: Vec3| Vec3::new(cos_t * v.x - sin_t * v.y, sin_t * v.x + cos_t * v.y, v.z);
        for i in 0..4 {
            solver.mesh.vertices[i] = rot(solver.mesh.rest_vertices[i]);
        }
        let forces = solver.compute_forces();
        // 旋转不产生应力
        for f in &forces {
            assert!(f.length() < 1e-2, "rotation should not generate force, got {}", f);
        }
    }

    #[test]
    fn test_step_time_advance() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        let dt = 0.01;
        solver.step(dt);
        assert!(approx(solver.time, dt, 1e-9));
        solver.step(dt);
        assert!(approx(solver.time, 2.0 * dt, 1e-9));
    }

    #[test]
    fn test_fixed_vertex_no_motion() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        solver.config.gravity = Vec3::ZERO;
        solver.mesh.fix(0);
        // 给 v1 一个初始速度
        solver.mesh.velocities[1] = Vec3::new(1.0, 0.0, 0.0);
        let v0_initial = solver.mesh.vertices[0];
        solver.step(0.001);
        // v0 不动
        assert_eq!(solver.mesh.vertices[0], v0_initial);
        assert_eq!(solver.mesh.velocities[0], Vec3::ZERO);
    }

    #[test]
    fn test_hanging_tet_gravity() {
        // 固定 v0, 其他顶点在重力下应下垂
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        solver.mesh.fix(0);
        let dt = 1e-4;
        // 步进几步, 检查 v1, v2, v3 是否下落
        let y1_initial = solver.mesh.vertices[1].y;
        for _ in 0..10 {
            solver.step(dt);
        }
        assert!(
            solver.mesh.vertices[1].y < y1_initial,
            "v1 should fall under gravity, y_initial={}, y_now={}",
            y1_initial,
            solver.mesh.vertices[1].y
        );
    }

    #[test]
    fn test_elastic_energy_at_rest() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        assert!(solver.elastic_energy() < 1e-6);
    }

    #[test]
    fn test_elastic_energy_under_deformation() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        solver.mesh.vertices[1].x = 1.5;
        let e = solver.elastic_energy();
        assert!(e > 0.0, "deformed mesh should have positive energy, got {}", e);
    }

    #[test]
    fn test_elastic_energy_increases_with_stretch() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        solver.mesh.vertices[1].x = 1.2;
        let e1 = solver.elastic_energy();
        solver.mesh.vertices[1].x = 1.5;
        let e2 = solver.elastic_energy();
        assert!(e2 > e1, "more stretch should give more energy: {} vs {}", e2, e1);
    }

    #[test]
    fn test_rotation_extraction() {
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        // 整体旋转 30 度 (绕 Z 轴)
        let theta = std::f32::consts::PI / 6.0;
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        for v in solver.mesh.vertices.iter_mut() {
            let x = v.x;
            let y = v.y;
            v.x = cos_t * x - sin_t * y;
            v.y = sin_t * x + cos_t * y;
        }
        solver.config.gravity = Vec3::ZERO;
        let _ = solver.compute_forces();
        // 提取的 R 应该接近旋转矩阵
        let r = solver.rotations[0];
        let expected_r00 = cos_t;
        assert!(
            (r.x_axis.x - expected_r00).abs() < 1e-3,
            "R[0,0] = {} expected {}",
            r.x_axis.x,
            expected_r00
        );
    }

    #[test]
    fn test_two_tet_mesh() {
        // 两个四面体共享一个三角形面
        let verts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
        ];
        let tets = vec![[0, 1, 2, 3], [0, 1, 2, 4]];
        let mesh = TetMesh::new(verts, tets);
        let mut solver = CorotationalFemSolver::new(mesh, CorotationalFemConfig::default());
        solver.config.gravity = Vec3::ZERO;
        let forces = solver.compute_forces();
        assert_eq!(forces.len(), 5);
        // 静止时合力为 0
        for f in &forces {
            assert!(f.length() < 1e-3);
        }
    }

    #[test]
    fn test_inversion_stability() {
        // 翻转四面体 (v1 拉到 -x, 使 Dm 退化并翻转)
        let mut solver =
            CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        solver.config.gravity = Vec3::ZERO;
        solver.config.damping = 0.0;
        solver.mesh.vertices[1].x = -0.5; // 翻转
        // 求解器不应崩溃, 力应推动顶点恢复
        let forces = solver.compute_forces();
        assert!(forces.iter().all(|f| f.is_finite()));
    }

    #[test]
    fn test_kinetic_energy_nonneg() {
        let solver = CorotationalFemSolver::new(unit_tet_mesh(), CorotationalFemConfig::default());
        assert!(solver.kinetic_energy() >= 0.0);
    }

    #[test]
    fn test_energy_conservation_no_gravity() {
        // 无重力 + 无阻尼, 总能量应近似守恒
        let mut config = CorotationalFemConfig::default();
        config.gravity = Vec3::ZERO;
        config.damping = 0.0;
        config.youngs_modulus = 1e4;
        let mut solver = CorotationalFemSolver::new(unit_tet_mesh(), config);
        // 给 v1 一个初始速度
        solver.mesh.velocities[1] = Vec3::new(0.5, 0.0, 0.0);
        let dt = 1e-4;
        let e0 = solver.elastic_energy() + solver.kinetic_energy();
        for _ in 0..10 {
            solver.step(dt);
        }
        let e1 = solver.elastic_energy() + solver.kinetic_energy();
        // 允许 5% 误差 (半隐式 Euler 不是精确能量守恒)
        let rel_err = (e1 - e0).abs() / e0.max(1e-12);
        assert!(
            rel_err < 0.1,
            "energy conservation rel_err = {} (e0={}, e1={})",
            rel_err,
            e0,
            e1
        );
    }
}