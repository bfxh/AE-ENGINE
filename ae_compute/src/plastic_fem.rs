//! Plastic FEM — 弹塑性有限元 (永久变形: 雪/沙/泥/金属凹痕)
//!
//! 基于:
//! - Stomakhin, Schroeder, Chai, Teran, Selle. "A Material Point Method for
//!   Snow Simulation." Walt Disney Animation Studios, SIGGRAPH 2013.
//! - Bargmann, Klarmann, von Danwitz. "Modeling of Frozen Soils: Phase
//!   Transition, Pore Ice and Plasticity." 2018.
//! - Bonet, Wood. "Nonlinear Continuum Mechanics for Finite Element
//!   Analysis." Cambridge Univ. Press, 2008. (基础塑性理论)
//!
//! 核心思想:
//! 1. 弹塑性分解: 总变形 F = F_elastic · F_plastic
//!    - F_elastic: 可恢复弹性变形 (产生应力)
//!    - F_plastic: 永久塑性变形 (无应力)
//! 2. 弹性能量只用 F_elastic 计算 (与 Neo-Hookean 相同公式)
//! 3. 屈服准则: SVD 钳制 (snow plasticity, Stomakhin 2013)
//!    - F_elastic = U · Σ · Vᵀ
//!    - Σ_clamped = clamp(Σ, [1-θ_c, 1+θ_s])
//!      - θ_c: 压缩屈服 (典型 0.025, 雪可被压缩到 97.5%)
//!      - θ_s: 拉伸屈服 (典型 0.025, 雪拉伸 2.5% 即断裂)
//!    - 超出部分转移到 F_plastic
//! 4. 塑性更新:
//!    F_plastic_new = F_plastic · V · Σ · Σ_clamped⁻¹ · Vᵀ
//!    F_elastic_new = U · Σ_clamped · Vᵀ
//!
//! 应用: 雪, 沙, 泥, 金属凹痕, 牙膏, 黏土, 塑料形变

use crate::corotational_fem::{CorotationalFemConfig, TetMesh};
use crate::stable_neo_hookean::svd_3x3;
use glam::{Mat3, Vec3};

// ============================================================
// 配置
// ============================================================

/// 弹塑性 FEM 配置
#[derive(Debug, Clone)]
pub struct PlasticFemConfig {
    /// 基础弹性参数
    pub elastic: CorotationalFemConfig,
    /// 压缩屈服 θ_c ∈ [0, 1] — σ_min = 1 - θ_c
    /// 雪: 0.025 (可被压缩到 97.5%)
    /// 沙: 0.1 (可被压缩到 90%)
    /// 金属: 0.001 (几乎不可压缩塑性)
    pub compression_yield: f32,
    /// 拉伸屈服 θ_s ∈ [0, 1] — σ_max = 1 + θ_s
    /// 雪: 0.025 (拉伸 2.5% 即断裂)
    /// 金属: 0.05 (可拉伸 5%)
    pub stretch_yield: f32,
    /// 硬化系数 (塑性累积增加屈服面) — 0 = 完美塑性
    pub hardening: f32,
}

impl Default for PlasticFemConfig {
    fn default() -> Self {
        Self {
            elastic: CorotationalFemConfig::default(),
            compression_yield: 0.025,
            stretch_yield: 0.025,
            hardening: 0.0,
        }
    }
}

// ============================================================
// 弹塑性求解器
// ============================================================

/// 弹塑性 FEM 求解器
///
/// 继承 Stable Neo-Hookean 的能量/应力计算, 添加 F_plastic 跟踪和屈服更新
pub struct PlasticFemSolver {
    /// 网格
    pub mesh: TetMesh,
    /// 配置
    pub config: PlasticFemConfig,
    /// 每个四面体的 Dm⁻¹
    rest_inverses: Vec<Mat3>,
    /// 每个四面体的静止体积
    rest_volumes: Vec<f32>,
    /// 每个四面体的塑性变形 F_plastic (累积)
    pub f_plastic: Vec<Mat3>,
    /// Lamé 第一参数
    pub lame_lambda: f32,
    /// Lamé 第二参数 (剪切模量)
    pub lame_mu: f32,
    /// 模拟时间
    pub time: f32,
}

impl PlasticFemSolver {
    /// 创建求解器
    pub fn new(mesh: TetMesh, config: PlasticFemConfig) -> Self {
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

        let e = config.elastic.youngs_modulus;
        let nu = config.elastic.poisson_ratio;
        let lame_lambda = e * nu / ((1.0 + nu) * (1.0 - 2.0 * nu).max(1e-6));
        let lame_mu = e / (2.0 * (1.0 + nu));

        Self {
            mesh,
            config,
            rest_inverses,
            rest_volumes,
            f_plastic: vec![Mat3::IDENTITY; n_tets],
            lame_lambda,
            lame_mu,
            time: 0.0,
        }
    }

    /// 当前形状矩阵
    #[inline]
    fn shape_matrix(&self, tet_idx: usize) -> Mat3 {
        let tet = self.mesh.tets[tet_idx];
        let x0 = self.mesh.vertices[tet[0]];
        let x1 = self.mesh.vertices[tet[1]];
        let x2 = self.mesh.vertices[tet[2]];
        let x3 = self.mesh.vertices[tet[3]];
        Mat3::from_cols(x1 - x0, x2 - x0, x3 - x0)
    }

    /// 总变形梯度 F = Ds · Dm⁻¹
    pub fn total_deformation(&self, tet_idx: usize) -> Mat3 {
        self.shape_matrix(tet_idx) * self.rest_inverses[tet_idx]
    }

    /// 弹性变形 F_elastic = F · F_plastic⁻¹
    pub fn elastic_deformation(&self, tet_idx: usize) -> Mat3 {
        let f_total = self.total_deformation(tet_idx);
        let f_plastic_inv = self.f_plastic[tet_idx].inverse();
        f_total * f_plastic_inv
    }

    /// 屈服处理 (SVD 钳制 + 塑性更新)
    ///
    /// 输入: 当前 F_total 和 F_plastic
    /// 输出: 更新后的 F_elastic (用于应力计算) 和 F_plastic
    pub fn return_mapping(&mut self, tet_idx: usize) -> Mat3 {
        let f_total = self.total_deformation(tet_idx);
        let f_plastic = self.f_plastic[tet_idx];
        let f_plastic_inv = f_plastic.inverse();
        let f_elastic_trial = f_total * f_plastic_inv;

        // SVD 分解
        let svd = svd_3x3(f_elastic_trial);
        let sigma = svd.sigma;

        // 钳制奇异值到屈服面 [1-θ_c, 1+θ_s]
        let sigma_min = 1.0 - self.config.compression_yield;
        let sigma_max = 1.0 + self.config.stretch_yield;
        let sigma_clamped = [
            sigma[0].clamp(sigma_min, sigma_max),
            sigma[1].clamp(sigma_min, sigma_max),
            sigma[2].clamp(sigma_min, sigma_max),
        ];

        // 检查是否发生屈服 (任一 σ 超出范围)
        let yielded = sigma[0] < sigma_min
            || sigma[0] > sigma_max
            || sigma[1] < sigma_min
            || sigma[1] > sigma_max
            || sigma[2] < sigma_min
            || sigma[2] > sigma_max;

        if !yielded {
            // 未屈服: F_elastic = trial, F_plastic 不变
            return f_elastic_trial;
        }

        // 塑性更新 (Stomakhin 2013):
        // F_plastic_new = F_plastic · V · Σ · Σ_clamped⁻¹ · Vᵀ
        // F_elastic_new = U · Σ_clamped · Vᵀ
        let sigma_mat = Mat3::from_diagonal(Vec3::from(sigma));
        let sigma_clamped_mat = Mat3::from_diagonal(Vec3::from(sigma_clamped));
        let sigma_clamped_inv = Mat3::from_diagonal(Vec3::new(
            1.0 / sigma_clamped[0].max(1e-10),
            1.0 / sigma_clamped[1].max(1e-10),
            1.0 / sigma_clamped[2].max(1e-10),
        ));

        // 塑性增量: V · Σ · Σ_clamped⁻¹ · Vᵀ
        let plastic_increment = svd.v * sigma_mat * sigma_clamped_inv * svd.v.transpose();
        self.f_plastic[tet_idx] = f_plastic * plastic_increment;

        // 弹性变形 (已校正)
        svd.u * sigma_clamped_mat * svd.v.transpose()
    }

    /// Neo-Hookean 应力 (用 F_elastic)
    fn tet_stress_from_elastic(&self, f_elastic: Mat3) -> Mat3 {
        let mu = self.lame_mu;
        let lambda = self.lame_lambda;

        let j = f_elastic.determinant().abs().max(1e-10);
        let log_j = j.ln();
        let f_inv_t = f_elastic.inverse().transpose();

        let coeff = lambda * log_j - mu;
        Mat3::from_cols(
            mu * f_elastic.x_axis + coeff * f_inv_t.x_axis,
            mu * f_elastic.y_axis + coeff * f_inv_t.y_axis,
            mu * f_elastic.z_axis + coeff * f_inv_t.z_axis,
        )
    }

    /// 计算单个四面体的力
    fn compute_tet_forces(&mut self, tet_idx: usize) -> [Vec3; 4] {
        // 先做 return mapping (更新 F_plastic, 返回 F_elastic)
        let f_elastic = self.return_mapping(tet_idx);
        let p = self.tet_stress_from_elastic(f_elastic);

        let v0 = self.rest_volumes[tet_idx];
        let dm_inv = self.rest_inverses[tet_idx];
        let h = (p.transpose() * dm_inv.transpose()) * (-v0);

        let f1 = Vec3::new(h.x_axis.x, h.y_axis.x, h.z_axis.x);
        let f2 = Vec3::new(h.x_axis.y, h.y_axis.y, h.z_axis.y);
        let f3 = Vec3::new(h.x_axis.z, h.y_axis.z, h.z_axis.z);
        let f0 = -(f1 + f2 + f3);

        [f0, f1, f2, f3]
    }

    /// 顶点质量
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
        total_vol * self.config.elastic.density
    }

    /// 计算所有顶点合力
    pub fn compute_forces(&mut self) -> Vec<Vec3> {
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

        let g = self.config.elastic.gravity;
        let damping = self.config.elastic.damping;
        let rho = self.config.elastic.density;

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

    /// 主动施加变形 (用于测试塑性累积)
    pub fn deform_vertex(&mut self, idx: usize, new_pos: Vec3) {
        self.mesh.vertices[idx] = new_pos;
    }

    /// 重置塑性 (清空 F_plastic = I)
    pub fn reset_plastic(&mut self) {
        for fp in &mut self.f_plastic {
            *fp = Mat3::IDENTITY;
        }
    }

    /// 总塑性变形量 (用于诊断)
    pub fn total_plastic_deformation(&self) -> f32 {
        let mut total = 0.0;
        for (i, fp) in self.f_plastic.iter().enumerate() {
            let diff = Mat3::from_cols(
                fp.x_axis - Mat3::IDENTITY.x_axis,
                fp.y_axis - Mat3::IDENTITY.y_axis,
                fp.z_axis - Mat3::IDENTITY.z_axis,
            );
            let norm = (diff.x_axis.length_squared()
                + diff.y_axis.length_squared()
                + diff.z_axis.length_squared())
            .sqrt();
            total += self.rest_volumes[i] * norm;
        }
        total
    }
}
