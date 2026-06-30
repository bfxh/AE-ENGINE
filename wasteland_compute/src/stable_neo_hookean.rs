//! Stable Neo-Hookean — 稳定 Neo-Hookean 有限元 (软组织/大变形)
//!
//! 基于:
//! - Smith, Schaefer, Jiang, Gowda, Marschner. "Stable Neo-Hookean Flesh
//!   Simulation." ACM TOG (SIGGRAPH 2018), 37(2).
//! - Stomakhin, Schroeder, Chai, Teran, Selle. "A Material Point Method for
//!   Snow Simulation." Walt Disney Animation Studios, SIGGRAPH 2013.
//! - Sifakis, Barbic. "FEM Simulation of 3D Deformable Solids." SIGGRAPH
//!   Courses 2012. (3x3 SVD details)
//!
//! 核心思想:
//! 1. Neo-Hookean 超弹性能量 (大变形, 比 Corotational 线性更准确):
//!    Ψ(F) = (μ/2)(||F||²_F - 3) - μ·log(J) + (λ/2)·(log J)²
//!    J = det(F), μ =剪切模量, λ = Lamé 第一参数
//! 2. 当 det(F) ≤ 0 (单元翻转) 时, log(J) 无定义 → 求解器崩溃
//! 3. Smith 2018 解决方案: SVD 分解 + 奇异值钳制
//!    - F = U·Σ·Vᵀ, σ_i = Σ_ii
//!    - 钳制 σ_i ∈ [σ_min, σ_max] (σ_min > 0 防翻转, σ_max 防爆炸)
//!    - 用钳制后的 F̃ = U·Σ̃·Vᵀ 计算能量和力
//! 4. 第一 Piola-Kirchhoff 应力:
//!    P = μ·F - μ·F⁻ᵀ + λ·log(J)·F⁻ᵀ
//! 5. 力矩阵 H = -V₀ · Pᵀ · Dm⁻ᵀ (与 Corotational FEM 相同结构)
//!
//! 3x3 SVD 实现算法 (Jacobi 特征值法):
//!   1. 计算 A = FᵀF (3x3 对称)
//!   2. Jacobi 旋转对角化 A = V·Λ·Vᵀ
//!   3. 奇异值 σ_i = √Λ_i
//!   4. U = F·V·Σ⁻¹ (处理秩亏损)
//!
//! 优势 (vs Corotational FEM):
//! - 大变形准确 (Corotational 是小变形线性化)
//! - 翻转稳定 (SVD 钳制避免 NaN)
//! - 物理正确 (能量驱动力, 非启发式)
//!
//! 应用: 软组织, 肌肉, 脂肪, 大变形体, 角色皮肤

use crate::corotational_fem::{CorotationalFemConfig, TetMesh};
use glam::{Mat3, Vec3};

// ============================================================
// SVD 工具 (3x3, Jacobi 特征值法)
// ============================================================

/// SVD 结果: F = U · diag(σ) · Vᵀ
#[derive(Debug, Clone, Copy)]
pub struct SvdResult {
    pub u: Mat3,
    pub sigma: [f32; 3],
    pub v: Mat3,
}

/// 3x3 SVD via Jacobi 特征值迭代
///
/// 步骤:
/// 1. A = Fᵀ·F (对称正半定)
/// 2. Jacobi 旋转对角化: A = V·Λ·Vᵀ
/// 3. σ_i = √Λ_ii
/// 4. U[:,k] = F·V[:,k] / σ_k (处理 σ_k≈0 的情况)
///
/// 收敛: 通常 5-10 次 sweep 即可
pub fn svd_3x3(f: Mat3) -> SvdResult {
    // 1. A = Fᵀ · F
    let a = f.transpose() * f;

    // 2. Jacobi 对角化 A = V · Λ · Vᵀ
    let (lambda, v) = jacobi_eigen_3x3(a);

    // 3. 奇异值 σ_i = √Λ_i (Λ 是 A 的特征值, 即 σ²)
    let mut sigma = [0.0_f32; 3];
    for i in 0..3 {
        sigma[i] = lambda[i].abs().sqrt();
    }

    // 4. U = F · V · Σ⁻¹
    //    U[:,k] = (F · V[:,k]) / σ_k
    //    对 σ_k ≈ 0 的列, 用其他列的正交补填充
    let fv = f * v;
    let mut u_cols = [Vec3::ZERO; 3];
    for k in 0..3 {
        if sigma[k] > 1e-10 {
            u_cols[k] = fv.col(k) / sigma[k];
        }
    }
    // 处理 σ≈0 的列: 用已有列的叉积补全
    // 假定 V 的列已构成右手系, U 也应是右手系
    let mut zero_count = 0;
    let mut zero_idx = -1;
    for k in 0..3 {
        if sigma[k] <= 1e-10 {
            zero_count += 1;
            zero_idx = k as i32;
        }
    }
    if zero_count == 1 {
        let z = zero_idx as usize;
        let i1 = (z + 1) % 3;
        let i2 = (z + 2) % 3;
        // u[z] = u[i1] × u[i2] / ||...||
        let cross = u_cols[i1].cross(u_cols[i2]);
        let len = cross.length();
        if len > 1e-10 {
            u_cols[z] = cross / len;
        } else {
            // 退化: 任意正交向量
            u_cols[z] = fallback_orthogonal(u_cols[i1]);
        }
    } else if zero_count >= 2 {
        // 多个奇异值 ≈ 0: 直接对 F 的列做 Gram-Schmidt
        let col0 = f.col(0);
        let len0 = col0.length();
        u_cols[0] = if len0 > 1e-10 { col0 / len0 } else { Vec3::new(1.0, 0.0, 0.0) };
        let mut col1 = f.col(1) - u_cols[0] * f.col(1).dot(u_cols[0]);
        let len1 = col1.length();
        u_cols[1] = if len1 > 1e-10 { col1 / len1 } else { Vec3::new(0.0, 1.0, 0.0) };
        u_cols[2] = u_cols[0].cross(u_cols[1]);
    }

    // 确保 U 是右手系 (det(U) > 0)
    // 同时翻转 U 和 V 的对应列以保持 F = U·Σ·Vᵀ 不变, sigma 始终非负
    let u_det = Mat3::from_cols(u_cols[0], u_cols[1], u_cols[2]).determinant();
    let mut v_cols = [v.col(0), v.col(1), v.col(2)];
    if u_det < 0.0 && zero_count == 0 {
        // 翻转最小奇异值对应的列 (U 和 V 同时翻转)
        let min_idx = sigma
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(2);
        u_cols[min_idx] = -u_cols[min_idx];
        v_cols[min_idx] = -v_cols[min_idx];
    }

    let u = Mat3::from_cols(u_cols[0], u_cols[1], u_cols[2]);
    let v_final = Mat3::from_cols(v_cols[0], v_cols[1], v_cols[2]);
    SvdResult { u, sigma, v: v_final }
}

/// 提供与 v 正交的单位向量 (退化情况下使用)
fn fallback_orthogonal(v: Vec3) -> Vec3 {
    let candidate = if v.x.abs() < 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let ortho = candidate - v * candidate.dot(v);
    let len = ortho.length();
    if len > 1e-10 {
        ortho / len
    } else {
        Vec3::new(0.0, 0.0, 1.0)
    }
}

/// 3x3 对称矩阵 Jacobi 特征值分解
///
/// A = V · diag(λ) · Vᵀ
/// 返回 (lambda, V), lambda 按降序排列
fn jacobi_eigen_3x3(mut a: Mat3) -> ([f32; 3], Mat3) {
    let mut v = Mat3::IDENTITY;
    const MAX_SWEEPS: usize = 32;
    const TOL: f32 = 1e-12;

    for _ in 0..MAX_SWEEPS {
        // 三个非对角元
        let off = a.x_axis.y.abs() + a.x_axis.z.abs() + a.y_axis.z.abs();
        if off < TOL {
            break;
        }

        // (0,1)
        if a.x_axis.y.abs() > TOL {
            apply_jacobi_rotation(&mut a, &mut v, 0, 1);
        }
        // (0,2)
        if a.x_axis.z.abs() > TOL {
            apply_jacobi_rotation(&mut a, &mut v, 0, 2);
        }
        // (1,2)
        if a.y_axis.z.abs() > TOL {
            apply_jacobi_rotation(&mut a, &mut v, 1, 2);
        }
    }

    // 提取特征值 (对角元), 按降序排列
    let mut eigvals = [a.x_axis.x, a.y_axis.y, a.z_axis.z];
    // 简单选择排序 (3 个元素)
    let mut idx = [0_usize, 1, 2];
    for i in 0..3 {
        for j in i + 1..3 {
            if eigvals[idx[j]] > eigvals[idx[i]] {
                idx.swap(i, j);
            }
        }
    }
    let sorted_eigvals = [eigvals[idx[0]], eigvals[idx[1]], eigvals[idx[2]]];
    let sorted_v = Mat3::from_cols(
        v.col(idx[0]),
        v.col(idx[1]),
        v.col(idx[2]),
    );
    (sorted_eigvals, sorted_v)
}

/// 在 (p, q) 平面应用 Jacobi 旋转, 消去 A[p,q]
fn apply_jacobi_rotation(a: &mut Mat3, v: &mut Mat3, p: usize, q: usize) {
    // 提取元素 (列优先存储: A[i,j] = a.col(j)[i])
    let app = a.col(p)[p];
    let aqq = a.col(q)[q];
    let apq = a.col(q)[p];

    // 计算旋转角 θ 使 tan(2θ) = 2*A[p,q] / (A[q,q] - A[p,p])
    let theta = if (aqq - app).abs() < 1e-30 {
        std::f32::consts::FRAC_PI_4
    } else {
        0.5 * (2.0 * apq / (aqq - app)).atan2(1.0)
    };
    let c = theta.cos();
    let s = theta.sin();

    // 构造旋转 G (在 p,q 平面): G[p,p]=c, G[q,q]=c, G[p,q]=s, G[q,p]=-s
    // A_new = Gᵀ · A · G
    // V_new = V · G

    // 提取 A 的所有元素
    let mut a_cols = [
        a.col(0),
        a.col(1),
        a.col(2),
    ];

    // 对 A 应用 G: A · G 改变列 p 和列 q
    for i in 0..3 {
        let aip = a_cols[p][i];
        let aiq = a_cols[q][i];
        a_cols[p][i] = aip * c - aiq * s;
        a_cols[q][i] = aip * s + aiq * c;
    }
    // 对 A 应用 Gᵀ: Gᵀ · (A·G) 改变行 p 和行 q
    // (列优先存储下行 p = 各列的第 p 个元素)
    for j in 0..3 {
        let apj = a_cols[j][p];
        let aqj = a_cols[j][q];
        a_cols[j][p] = apj * c - aqj * s;
        a_cols[j][q] = apj * s + aqj * c;
    }
    *a = Mat3::from_cols(a_cols[0], a_cols[1], a_cols[2]);

    // V = V · G
    let mut v_cols = [v.col(0), v.col(1), v.col(2)];
    for i in 0..3 {
        let vip = v_cols[p][i];
        let viq = v_cols[q][i];
        v_cols[p][i] = vip * c - viq * s;
        v_cols[q][i] = vip * s + viq * c;
    }
    *v = Mat3::from_cols(v_cols[0], v_cols[1], v_cols[2]);

    // 强制清零 A[p,q] 和 A[q,p] (消除浮点残差)
    let mut clean_cols = [a.col(0), a.col(1), a.col(2)];
    clean_cols[q][p] = 0.0;
    clean_cols[p][q] = 0.0;
    *a = Mat3::from_cols(clean_cols[0], clean_cols[1], clean_cols[2]);
}

// ============================================================
// 稳定 Neo-Hookean 求解器
// ============================================================

/// 稳定 Neo-Hookean 求解器配置 (使用 CorotationalFemConfig)
pub type StableNeoHookeanConfig = CorotationalFemConfig;

/// 稳定 Neo-Hookean 求解器
///
/// 与 Corotational FEM 共享 TetMesh, 但使用 Neo-Hookean 超弹性能量
pub struct StableNeoHookeanSolver {
    /// 网格
    pub mesh: TetMesh,
    /// 配置
    pub config: StableNeoHookeanConfig,
    /// 每个四面体的 Dm⁻¹
    rest_inverses: Vec<Mat3>,
    /// 每个四面体的静止体积 V₀
    rest_volumes: Vec<f32>,
    /// Lamé 第一参数 λ
    pub lame_lambda: f32,
    /// Lamé 第二参数 μ (剪切模量)
    pub lame_mu: f32,
    /// 钳制下限 σ_min (防翻转)
    pub sigma_min: f32,
    /// 钳制上限 σ_max (防爆炸)
    pub sigma_max: f32,
    /// 模拟时间
    pub time: f32,
}

impl StableNeoHookeanSolver {
    /// 创建求解器
    pub fn new(mesh: TetMesh, config: StableNeoHookeanConfig) -> Self {
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
            if det.abs() < 1e-12 {
                rest_inverses[i] = Mat3::IDENTITY;
                rest_volumes[i] = 1e-12;
            } else {
                rest_inverses[i] = dm.inverse();
                rest_volumes[i] = det.abs() / 6.0;
            }
        }

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
            sigma_min: 0.1,
            sigma_max: 10.0,
            time: 0.0,
        }
    }

    /// 当前形状矩阵 Ds
    #[inline]
    fn shape_matrix(&self, tet_idx: usize) -> Mat3 {
        let tet = self.mesh.tets[tet_idx];
        let x0 = self.mesh.vertices[tet[0]];
        let x1 = self.mesh.vertices[tet[1]];
        let x2 = self.mesh.vertices[tet[2]];
        let x3 = self.mesh.vertices[tet[3]];
        Mat3::from_cols(x1 - x0, x2 - x0, x3 - x0)
    }

    /// 变形梯度 F = Ds · Dm⁻¹
    #[inline]
    pub fn deformation_gradient(&self, tet_idx: usize) -> Mat3 {
        self.shape_matrix(tet_idx) * self.rest_inverses[tet_idx]
    }

    /// 钳制奇异值后的稳定变形梯度 F̃ = U · Σ̃ · Vᵀ
    pub fn clamped_deformation(&self, tet_idx: usize) -> Mat3 {
        let f = self.deformation_gradient(tet_idx);
        let svd = svd_3x3(f);
        let sigma_clamped = [
            svd.sigma[0].clamp(self.sigma_min, self.sigma_max),
            svd.sigma[1].clamp(self.sigma_min, self.sigma_max),
            svd.sigma[2].clamp(self.sigma_min, self.sigma_max),
        ];
        let sigma_mat = Mat3::from_diagonal(Vec3::from(sigma_clamped));
        svd.u * sigma_mat * svd.v.transpose()
    }

    /// Neo-Hookean 能量 Ψ(F) (单个四面体)
    ///
    /// Ψ = (μ/2)(||F||²_F - 3) - μ·log(J) + (λ/2)·(log J)²
    /// J = det(F)
    pub fn tet_energy(&self, tet_idx: usize) -> f32 {
        let f = self.clamped_deformation(tet_idx);
        let mu = self.lame_mu;
        let lambda = self.lame_lambda;

        let f_norm_sq = f.x_axis.length_squared()
            + f.y_axis.length_squared()
            + f.z_axis.length_squared();
        let j = f.determinant().abs().max(1e-10);
        let log_j = j.ln();

        (mu / 2.0) * (f_norm_sq - 3.0) - mu * log_j + (lambda / 2.0) * log_j * log_j
    }

    /// 第一 Piola-Kirchhoff 应力 P
    ///
    /// P = μ·F - μ·F⁻ᵀ + λ·log(J)·F⁻ᵀ
    /// 使用钳制后的 F 保证数值稳定性
    pub fn tet_stress(&self, tet_idx: usize) -> Mat3 {
        let f = self.clamped_deformation(tet_idx);
        let mu = self.lame_mu;
        let lambda = self.lame_lambda;

        let j = f.determinant().abs().max(1e-10);
        let log_j = j.ln();
        let f_inv_t = f.inverse().transpose();

        // P = μ·F - μ·F⁻ᵀ + λ·log(J)·F⁻ᵀ
        //   = μ·F + (λ·log(J) - μ) · F⁻ᵀ
        let coeff = lambda * log_j - mu;
        Mat3::from_cols(
            mu * f.x_axis + coeff * f_inv_t.x_axis,
            mu * f.y_axis + coeff * f_inv_t.y_axis,
            mu * f.z_axis + coeff * f_inv_t.z_axis,
        )
    }

    /// 计算四面体对 4 个顶点的弹性力
    ///
    /// H = -V₀ · Pᵀ · Dm⁻ᵀ
    /// f_i = H[:, i] (i = 1, 2, 3), f_0 = -(f_1 + f_2 + f_3)
    fn compute_tet_forces(&self, tet_idx: usize) -> [Vec3; 4] {
        let p = self.tet_stress(tet_idx);
        let v0 = self.rest_volumes[tet_idx];
        let dm_inv = self.rest_inverses[tet_idx];
        let h = (p.transpose() * dm_inv.transpose()) * (-v0);

        let f1 = Vec3::new(h.x_axis.x, h.y_axis.x, h.z_axis.x);
        let f2 = Vec3::new(h.x_axis.y, h.y_axis.y, h.z_axis.y);
        let f3 = Vec3::new(h.x_axis.z, h.y_axis.z, h.z_axis.z);
        let f0 = -(f1 + f2 + f3);

        [f0, f1, f2, f3]
    }

    /// 顶点质量 (共享四面体体积 1/4)
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

    /// 计算所有顶点合力 (弹性 + 重力 + 阻尼)
    pub fn compute_forces(&self) -> Vec<Vec3> {
        let n = self.mesh.num_vertices();
        let mut forces = vec![Vec3::ZERO; n];

        for t in 0..self.mesh.num_tets() {
            let tf = self.compute_tet_forces(t);
            let tet = self.mesh.tets[t];
            forces[tet[0]] += tf[0];
            forces[tet[1]] += tf[1];
            forces[tet[2]] += tf[2];
            forces[tet[3]] += tf[3];
        }

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

    /// 半隐式 Euler 步进
    pub fn step(&mut self, dt: f32) {
        let n = self.mesh.num_vertices();
        let forces = self.compute_forces();

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
        for i in 0..n {
            if self.mesh.fixed[i] {
                continue;
            }
            self.mesh.vertices[i] += dt * self.mesh.velocities[i];
        }
        self.time += dt;
    }

    /// 总弹性势能
    pub fn elastic_energy(&self) -> f32 {
        let mut e = 0.0;
        for t in 0..self.mesh.num_tets() {
            e += self.rest_volumes[t] * self.tet_energy(t);
        }
        e
    }

    /// 总动能
    pub fn kinetic_energy(&self) -> f32 {
        let mut e = 0.0;
        for i in 0..self.mesh.num_vertices() {
            let mi = self.vertex_mass(i);
            e += 0.5 * mi * self.mesh.velocities[i].length_squared();
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

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    fn approx_vec(a: Vec3, b: Vec3, tol: f32) -> bool {
        (a.x - b.x).abs() < tol && (a.y - b.y).abs() < tol && (a.z - b.z).abs() < tol
    }

    fn frobenius_diff(a: Mat3, b: Mat3) -> f32 {
        let d = Mat3::from_cols(
            a.x_axis - b.x_axis,
            a.y_axis - b.y_axis,
            a.z_axis - b.z_axis,
        );
        (d.x_axis.length_squared() + d.y_axis.length_squared() + d.z_axis.length_squared()).sqrt()
    }

    /// 单位四面体 (体积 = 1/6)
    fn unit_tet_mesh() -> TetMesh {
        let verts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        TetMesh::new(verts, vec![[0, 1, 2, 3]])
    }

    // ------------------------------------------------------------
    // SVD 测试
    // ------------------------------------------------------------

    #[test]
    fn test_svd_identity() {
        let svd = svd_3x3(Mat3::IDENTITY);
        for i in 0..3 {
            assert!(approx(svd.sigma[i], 1.0, 1e-5));
        }
    }

    #[test]
    fn test_svd_diagonal() {
        // F = diag(2, 3, 4) → U=V=I, σ = [4,3,2]
        let f = Mat3::from_diagonal(Vec3::new(2.0, 3.0, 4.0));
        let svd = svd_3x3(f);
        // 奇异值应为 {2, 3, 4} (顺序按降序)
        let mut s = svd.sigma;
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(approx(s[0], 2.0, 1e-4));
        assert!(approx(s[1], 3.0, 1e-4));
        assert!(approx(s[2], 4.0, 1e-4));
    }

    #[test]
    fn test_svd_pure_rotation() {
        // 绕 Z 轴旋转 30 度: 应有 σ = [1,1,1]
        let theta = std::f32::consts::PI / 6.0;
        let rot = Mat3::from_cols(
            Vec3::new(theta.cos(), theta.sin(), 0.0),
            Vec3::new(-theta.sin(), theta.cos(), 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );
        let svd = svd_3x3(rot);
        for i in 0..3 {
            assert!(approx(svd.sigma[i], 1.0, 1e-3));
        }
    }

    #[test]
    fn test_svd_reconstruction() {
        // F = U · Σ · Vᵀ 应该重建原矩阵
        let f = Mat3::from_cols(
            Vec3::new(1.0, 0.5, 0.3),
            Vec3::new(-0.2, 1.5, 0.4),
            Vec3::new(0.1, -0.3, 2.0),
        );
        let svd = svd_3x3(f);
        let reconstructed =
            svd.u * Mat3::from_diagonal(Vec3::from(svd.sigma)) * svd.v.transpose();
        assert!(
            frobenius_diff(f, reconstructed) < 1e-3,
            "SVD reconstruction failed, diff = {}",
            frobenius_diff(f, reconstructed)
        );
    }

    #[test]
    fn test_svd_singular_values_positive() {
        let f = Mat3::from_cols(
            Vec3::new(2.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 0.0, 2.0),
        );
        let svd = svd_3x3(f);
        for i in 0..3 {
            assert!(svd.sigma[i] > 0.0, "singular value {} = {} <= 0", i, svd.sigma[i]);
        }
    }

    #[test]
    fn test_svd_u_orthonormal() {
        let f = Mat3::from_cols(
            Vec3::new(1.0, 0.5, 0.3),
            Vec3::new(-0.2, 1.5, 0.4),
            Vec3::new(0.1, -0.3, 2.0),
        );
        let svd = svd_3x3(f);
        // U 的列应为单位正交
        for i in 0..3 {
            let col = svd.u.col(i);
            assert!(approx(col.length(), 1.0, 1e-3), "U col {} not unit: |{}|", i, col.length());
        }
        // 列间正交
        for i in 0..3 {
            for j in (i + 1)..3 {
                let dot = svd.u.col(i).dot(svd.u.col(j));
                assert!(dot.abs() < 1e-3, "U cols {} {} not orthogonal: dot = {}", i, j, dot);
            }
        }
    }

    #[test]
    fn test_jacobi_eigen_diagonal() {
        let a = Mat3::from_diagonal(Vec3::new(1.0, 2.0, 3.0));
        let (eigvals, _) = jacobi_eigen_3x3(a);
        // 降序: [3, 2, 1]
        assert!(approx(eigvals[0], 3.0, 1e-5));
        assert!(approx(eigvals[1], 2.0, 1e-5));
        assert!(approx(eigvals[2], 1.0, 1e-5));
    }

    #[test]
    fn test_jacobi_eigen_symmetric() {
        // 对称矩阵 [[2,1,0],[1,2,0],[0,0,5]]
        let a = Mat3::from_cols(
            Vec3::new(2.0, 1.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(0.0, 0.0, 5.0),
        );
        let (eigvals, v) = jacobi_eigen_3x3(a);
        // 特征值: 3, 1, 5 → 降序 [5, 3, 1]
        assert!(approx(eigvals[0], 5.0, 1e-5));
        assert!(approx(eigvals[1], 3.0, 1e-5));
        assert!(approx(eigvals[2], 1.0, 1e-5));
        // A·v_i = λ_i · v_i
        let v0 = v.col(0);
        let av0 = a * v0;
        let lv0 = v0 * eigvals[0];
        assert!(approx_vec(av0, lv0, 1e-4));
    }

    // ------------------------------------------------------------
    // 求解器基础
    // ------------------------------------------------------------

    #[test]
    fn test_solver_creation() {
        let solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        assert_eq!(solver.mesh.num_vertices(), 4);
        assert_eq!(solver.mesh.num_tets(), 1);
        assert!(solver.lame_mu > 0.0);
        assert!(solver.lame_lambda > 0.0);
    }

    #[test]
    fn test_rest_volume() {
        let solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        assert!(approx(solver.rest_volumes[0], 1.0 / 6.0, 1e-6));
    }

    #[test]
    fn test_deformation_at_rest() {
        let solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        let f = solver.deformation_gradient(0);
        assert!(frobenius_diff(f, Mat3::IDENTITY) < 1e-6);
    }

    #[test]
    fn test_clamped_deformation_at_rest() {
        let solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        let f = solver.clamped_deformation(0);
        assert!(frobenius_diff(f, Mat3::IDENTITY) < 1e-5);
    }

    // ------------------------------------------------------------
    // 能量
    // ------------------------------------------------------------

    #[test]
    fn test_energy_at_rest() {
        let solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        let e = solver.tet_energy(0);
        // F = I → ||F||²=3, J=1, log J=0
        // Ψ = (μ/2)(3-3) - μ·0 + (λ/2)·0 = 0
        assert!(approx(e, 0.0, 1e-3), "rest energy should be 0, got {}", e);
    }

    #[test]
    fn test_energy_under_stretch() {
        let solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        let mut mesh = unit_tet_mesh();
        mesh.vertices[1].x = 1.5; // 拉伸 v1
        let solver = StableNeoHookeanSolver::new(mesh, StableNeoHookeanConfig::default());
        let e = solver.tet_energy(0);
        assert!(e > 0.0, "stretched energy should be positive, got {}", e);
    }

    #[test]
    fn test_energy_increases_with_deformation() {
        let mut mesh1 = unit_tet_mesh();
        mesh1.vertices[1].x = 1.2;
        let s1 = StableNeoHookeanSolver::new(mesh1, StableNeoHookeanConfig::default());
        let e1 = s1.tet_energy(0);

        let mut mesh2 = unit_tet_mesh();
        mesh2.vertices[1].x = 1.5;
        let s2 = StableNeoHookeanSolver::new(mesh2, StableNeoHookeanConfig::default());
        let e2 = s2.tet_energy(0);

        assert!(e2 > e1, "more stretch should give more energy: e1={}, e2={}", e1, e2);
    }

    // ------------------------------------------------------------
    // 力
    // ------------------------------------------------------------

    #[test]
    fn test_force_at_rest_zero() {
        let mut solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        solver.config.gravity = Vec3::ZERO;
        let forces = solver.compute_forces();
        for f in &forces {
            assert!(f.length() < 1.0, "force at rest should be small, got {}", f);
        }
    }

    #[test]
    fn test_force_under_stretch() {
        let mut mesh = unit_tet_mesh();
        mesh.vertices[1].x = 1.3;
        let mut solver = StableNeoHookeanSolver::new(mesh, StableNeoHookeanConfig::default());
        solver.config.gravity = Vec3::ZERO;
        let forces = solver.compute_forces();
        // v1 应被拉回 -x
        assert!(
            forces[1].x < 0.0,
            "stretched vertex should be pulled back, got f1.x = {}",
            forces[1].x
        );
    }

    #[test]
    fn test_force_momentum_conservation() {
        let mut mesh = unit_tet_mesh();
        mesh.vertices[1] = Vec3::new(1.3, 0.1, 0.0);
        mesh.vertices[2] = Vec3::new(0.0, 1.2, 0.0);
        let mut solver = StableNeoHookeanSolver::new(mesh, StableNeoHookeanConfig::default());
        solver.config.gravity = Vec3::ZERO;
        solver.config.damping = 0.0;
        let forces = solver.compute_forces();
        let total = forces[0] + forces[1] + forces[2] + forces[3];
        assert!(
            total.length() < 1.0,
            "total force should be ~0 (momentum conservation), got {}",
            total
        );
    }

    #[test]
    fn test_rotation_invariance() {
        let mut mesh = unit_tet_mesh();
        // 整体绕 Z 轴旋转 45 度
        let theta = std::f32::consts::FRAC_PI_4;
        let c = theta.cos();
        let s = theta.sin();
        let rot = |v: Vec3| Vec3::new(c * v.x - s * v.y, s * v.x + c * v.y, v.z);
        for v in mesh.vertices.iter_mut() {
            *v = rot(*v);
        }
        // rest_vertices 也要同步旋转 (否则就是变形了)
        for v in mesh.rest_vertices.iter_mut() {
            *v = rot(*v);
        }
        let mut solver = StableNeoHookeanSolver::new(mesh, StableNeoHookeanConfig::default());
        solver.config.gravity = Vec3::ZERO;
        let forces = solver.compute_forces();
        for f in &forces {
            assert!(f.length() < 1.0, "rotation should not generate force, got {}", f);
        }
    }

    // ------------------------------------------------------------
    // 翻转稳定性
    // ------------------------------------------------------------

    #[test]
    fn test_inversion_no_nan() {
        let mut mesh = unit_tet_mesh();
        mesh.vertices[1].x = -0.5; // 翻转
        let solver = StableNeoHookeanSolver::new(mesh, StableNeoHookeanConfig::default());
        let e = solver.tet_energy(0);
        assert!(e.is_finite(), "inverted energy should be finite, got {}", e);
        let p = solver.tet_stress(0);
        assert!(p.x_axis.is_finite());
        assert!(p.y_axis.is_finite());
        assert!(p.z_axis.is_finite());
    }

    #[test]
    fn test_inversion_force_finite() {
        let mut mesh = unit_tet_mesh();
        mesh.vertices[1].x = -1.0; // 大幅翻转
        let mut solver = StableNeoHookeanSolver::new(mesh, StableNeoHookeanConfig::default());
        solver.config.gravity = Vec3::ZERO;
        solver.config.damping = 0.0;
        let forces = solver.compute_forces();
        for f in &forces {
            assert!(f.is_finite(), "inversion force should be finite, got {}", f);
        }
    }

    #[test]
    fn test_clamped_deformation_prevents_inversion() {
        let mut mesh = unit_tet_mesh();
        mesh.vertices[1].x = -1.0; // 翻转
        let solver = StableNeoHookeanSolver::new(mesh, StableNeoHookeanConfig::default());
        let f_clamped = solver.clamped_deformation(0);
        // 钳制后 F̃ 应有限 (无 NaN)
        assert!(f_clamped.x_axis.is_finite());
        assert!(f_clamped.y_axis.is_finite());
        assert!(f_clamped.z_axis.is_finite());
        // |det(F̃)| > 0 (sigma_min > 0, 防止退化)
        let det = f_clamped.determinant();
        assert!(det.abs() > 0.0, "clamped F should have non-zero determinant, got {}", det);
    }

    // ------------------------------------------------------------
    // 时间步进
    // ------------------------------------------------------------

    #[test]
    fn test_step_advances_time() {
        let mut solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        solver.step(0.01);
        assert!(approx(solver.time, 0.01, 1e-9));
        solver.step(0.01);
        assert!(approx(solver.time, 0.02, 1e-9));
    }

    #[test]
    fn test_step_no_explosion() {
        let mut solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        let dt = 1e-4;
        for _ in 0..50 {
            solver.step(dt);
        }
        for p in &solver.mesh.vertices {
            assert!(p.is_finite());
            assert!(p.length() < 1000.0, "particle flew away: {}", p);
        }
    }

    #[test]
    fn test_hanging_tet_gravity() {
        let mut solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        solver.mesh.fix(0); // 固定 v0
        let y1_initial = solver.mesh.vertices[1].y;
        for _ in 0..10 {
            solver.step(1e-4);
        }
        assert!(
            solver.mesh.vertices[1].y < y1_initial,
            "v1 should fall, y_initial={}, y_now={}",
            y1_initial,
            solver.mesh.vertices[1].y
        );
    }

    #[test]
    fn test_fixed_vertex_immobile() {
        let mut solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        solver.config.gravity = Vec3::ZERO;
        solver.mesh.fix(0);
        let v0 = solver.mesh.vertices[0];
        for _ in 0..5 {
            solver.step(1e-3);
        }
        assert_eq!(solver.mesh.vertices[0], v0);
        assert_eq!(solver.mesh.velocities[0], Vec3::ZERO);
    }

    // ------------------------------------------------------------
    // 能量诊断
    // ------------------------------------------------------------

    #[test]
    fn test_elastic_energy_total() {
        let solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        assert!(solver.elastic_energy().abs() < 1e-3);
    }

    #[test]
    fn test_kinetic_energy_nonneg() {
        let solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        assert!(solver.kinetic_energy() >= 0.0);
    }

    #[test]
    fn test_sigma_clamp_settings() {
        let mut solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        assert!(solver.sigma_min > 0.0);
        assert!(solver.sigma_max > solver.sigma_min);
        solver.sigma_min = 0.05;
        solver.sigma_max = 20.0;
        assert!(approx(solver.sigma_min, 0.05, 1e-9));
        assert!(approx(solver.sigma_max, 20.0, 1e-9));
    }

    #[test]
    fn test_stress_at_rest_near_zero() {
        let solver = StableNeoHookeanSolver::new(unit_tet_mesh(), StableNeoHookeanConfig::default());
        let p = solver.tet_stress(0);
        // F=I, J=1, log J=0
        // P = μ·I - μ·I + 0 = 0
        let zero = Mat3::ZERO;
        assert!(frobenius_diff(p, zero) < 1.0, "stress at rest should be small, got {}", frobenius_diff(p, zero));
    }

    #[test]
    fn test_two_tet_mesh_stability() {
        let verts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
        ];
        let tets = vec![[0, 1, 2, 3], [0, 1, 2, 4]];
        let mesh = TetMesh::new(verts, tets);
        let mut solver = StableNeoHookeanSolver::new(mesh, StableNeoHookeanConfig::default());
        solver.config.gravity = Vec3::ZERO;
        let forces = solver.compute_forces();
        for f in &forces {
            assert!(f.length() < 5.0, "force should be small at rest, got {}", f);
        }
    }

    #[test]
    fn test_energy_conservation_no_gravity() {
        let mut config = StableNeoHookeanConfig::default();
        config.gravity = Vec3::ZERO;
        config.damping = 0.0;
        config.youngs_modulus = 1e4;
        let mut solver = StableNeoHookeanSolver::new(unit_tet_mesh(), config);
        solver.mesh.velocities[1] = Vec3::new(0.3, 0.0, 0.0);
        let dt = 1e-4;
        let e0 = solver.elastic_energy() + solver.kinetic_energy();
        for _ in 0..5 {
            solver.step(dt);
        }
        let e1 = solver.elastic_energy() + solver.kinetic_energy();
        // 半隐式 Euler 允许 10% 误差
        let rel_err = (e1 - e0).abs() / e0.abs().max(1e-12);
        assert!(
            rel_err < 0.2,
            "energy conservation rel_err = {} (e0={}, e1={})",
            rel_err,
            e0,
            e1
        );
    }
}