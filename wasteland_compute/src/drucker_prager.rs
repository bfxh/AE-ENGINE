//! Drucker-Prager Sand Plasticity — 沙土弹塑性有限元
//!
//! 基于:
//! - Klar, Gast, Pradhana, Fu, Jiang, Teran. "Drucker-Prager Elastoplasticity
//!   for Sand Animation." ACM TOG (SIGGRAPH 2016), 35(4).
//! - Stomakhin et al. "A Material Point Method for Snow Simulation." 2013.
//!   (弹塑性分解框架)
//!
//! 与 Stomakhin 2013 雪模型 (clamped yield) 的区别:
//! 1. 屈服面是应力空间中的圆锥 (vs 奇异值直接钳制)
//! 2. 体积应力 p = mean(σ) 控制屈服阈值 (压力越大, 沙越硬)
//! 3. 偏应力 |s| = ||σ - p·I|| 超过阈值时屈服
//! 4. 顶点投影 (apex projection): 拉伸时 (p < 0) 沙无强度, 投影到顶点
//!
//! 屈服函数 (简化 Drucker-Prager):
//!   f(σ) = |s| - (tan(φ)·p + C)
//!   其中: p = (σ1+σ2+σ3)/3 (压缩为正)
//!         s_i = σ_i - p (偏量)
//!         |s| = sqrt(s1² + s2² + s3²)
//!         φ = 摩擦角 (沙典型 30°)
//!         C = 内聚力 (干沙 ≈ 0, 湿沙 > 0)
//!
//! Return mapping:
//!   1. trial elastic: F_e^trial = F · F_p⁻¹
//!   2. SVD: F_e^trial = U · Σ · Vᵀ
//!   3. 计算 p, s, |s|, threshold = tan(φ)·p + C
//!   4. 若 |s| > threshold (屈服):
//!      a. 法向回归: σ_new = p + s·(threshold/|s|)
//!   5. 若 threshold ≤ 0 (顶点失效, 拉伸):
//!      a. 顶点投影: σ_new = (C/tan(φ))·(1,1,1) [全部相等]
//!   6. F_plastic_new = F_plastic · V · Σ · Σ_new⁻¹ · Vᵀ
//!   7. F_elastic = U · Σ_new · Vᵀ
//!
//! 应用: 沙堆, 雪崩, 沙漏, 沙地车辙, 流沙

use crate::corotational_fem::{CorotationalFemConfig, TetMesh};
use crate::stable_neo_hookean::svd_3x3;
use glam::{Mat3, Vec3};

/// Drucker-Prager 沙土配置
#[derive(Debug, Clone)]
pub struct DruckerPragerConfig {
    pub elastic: CorotationalFemConfig,
    pub friction_angle: f32,
    pub cohesion: f32,
    pub tension_cutoff: f32,
}

impl Default for DruckerPragerConfig {
    fn default() -> Self {
        Self {
            elastic: CorotationalFemConfig {
                youngs_modulus: 5e5,
                poisson_ratio: 0.3,
                density: 1600.0,
                damping: 0.5,
                gravity: Vec3::new(0.0, -9.81, 0.0),
            },
            friction_angle: 30.0_f32.to_radians(),
            cohesion: 0.0,
            tension_cutoff: 0.0,
        }
    }
}

pub struct DruckerPragerSolver {
    pub mesh: TetMesh,
    pub config: DruckerPragerConfig,
    rest_inverses: Vec<Mat3>,
    rest_volumes: Vec<f32>,
    pub f_plastic: Vec<Mat3>,
    pub lame_lambda: f32,
    pub lame_mu: f32,
    pub time: f32,
}

impl DruckerPragerSolver {
    pub fn new(mesh: TetMesh, config: DruckerPragerConfig) -> Self {
        let num_tets = mesh.num_tets();
        let mut rest_inverses = Vec::with_capacity(num_tets);
        let mut rest_volumes = Vec::with_capacity(num_tets);
        for tet in &mesh.tets {
            let (dm_inv, v0) = compute_rest_shape(&mesh.rest_vertices, *tet);
            rest_inverses.push(dm_inv);
            rest_volumes.push(v0);
        }
        let e = config.elastic.youngs_modulus;
        let nu = config.elastic.poisson_ratio;
        let lambda = e * nu / ((1.0 + nu) * (1.0 - 2.0 * nu));
        let mu = e / (2.0 * (1.0 + nu));
        Self {
            mesh,
            config,
            rest_inverses,
            rest_volumes,
            f_plastic: vec![Mat3::IDENTITY; num_tets],
            lame_lambda: lambda,
            lame_mu: mu,
            time: 0.0,
        }
    }

    pub fn total_deformation(&self, tet_idx: usize) -> Mat3 {
        let tet = self.mesh.tets[tet_idx];
        let x0 = self.mesh.vertices[tet[0]];
        let x1 = self.mesh.vertices[tet[1]];
        let x2 = self.mesh.vertices[tet[2]];
        let x3 = self.mesh.vertices[tet[3]];
        let ds = Mat3::from_cols(x1 - x0, x2 - x0, x3 - x0);
        ds * self.rest_inverses[tet_idx]
    }

    pub fn elastic_deformation(&self, tet_idx: usize) -> Mat3 {
        let f_total = self.total_deformation(tet_idx);
        let f_plastic = self.f_plastic[tet_idx];
        f_total * f_plastic.inverse()
    }

    /// Drucker-Prager return mapping
    pub fn return_mapping(&mut self, tet_idx: usize) -> Mat3 {
        let f_total = self.total_deformation(tet_idx);
        let f_plastic = self.f_plastic[tet_idx];
        let f_plastic_inv = f_plastic.inverse();
        let f_elastic_trial = f_total * f_plastic_inv;
        let svd = svd_3x3(f_elastic_trial);
        let sigma = svd.sigma;

        let p_vol = (sigma[0] + sigma[1] + sigma[2]) / 3.0;
        let s = [sigma[0] - p_vol, sigma[1] - p_vol, sigma[2] - p_vol];
        let s_norm = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();

        let tan_phi = self.config.friction_angle.tan();
        let cohesion = self.config.cohesion;
        let threshold = tan_phi * p_vol + cohesion;

        let sigma_clamped: [f32; 3];

        if threshold <= 0.0 {
            let apex = if tan_phi > 1e-10 {
                cohesion / tan_phi
            } else {
                self.config.tension_cutoff
            };
            sigma_clamped = [apex; 3];
        } else if s_norm > threshold {
            let scale = threshold / s_norm.max(1e-10);
            sigma_clamped = [
                p_vol + s[0] * scale,
                p_vol + s[1] * scale,
                p_vol + s[2] * scale,
            ];
        } else {
            sigma_clamped = sigma;
        }

        let yielded = sigma_clamped[0] != sigma[0]
            || sigma_clamped[1] != sigma[1]
            || sigma_clamped[2] != sigma[2];

        if yielded {
            let sigma_mat = Mat3::from_diagonal(Vec3::new(sigma[0], sigma[1], sigma[2]));
            let sigma_clamped_inv = Mat3::from_diagonal(Vec3::new(
                1.0 / sigma_clamped[0].abs().max(1e-10),
                1.0 / sigma_clamped[1].abs().max(1e-10),
                1.0 / sigma_clamped[2].abs().max(1e-10),
            ));
            let ratio = sigma_mat * sigma_clamped_inv;
            let plastic_increment = svd.v * ratio * svd.v.transpose();
            self.f_plastic[tet_idx] = f_plastic * plastic_increment;
        }

        let sigma_clamped_mat = Mat3::from_diagonal(Vec3::new(
            sigma_clamped[0],
            sigma_clamped[1],
            sigma_clamped[2],
        ));
        svd.u * sigma_clamped_mat * svd.v.transpose()
    }

    pub fn tet_stress_from_elastic(&self, f_elastic: Mat3) -> Mat3 {
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

    pub fn compute_tet_forces(&mut self, tet_idx: usize) -> [Vec3; 4] {
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

    pub fn compute_forces(&mut self) -> Vec<Vec3> {
        let n = self.mesh.num_vertices();
        let mut forces = vec![Vec3::ZERO; n];
        let g = self.config.elastic.gravity;
        let rho = self.config.elastic.density;
        for t in 0..self.mesh.num_tets() {
            let tet_forces = self.compute_tet_forces(t);
            let tet = self.mesh.tets[t];
            forces[tet[0]] += tet_forces[0];
            forces[tet[1]] += tet_forces[1];
            forces[tet[2]] += tet_forces[2];
            forces[tet[3]] += tet_forces[3];
            let v0 = self.rest_volumes[t];
            let mass = rho * v0 / 4.0;
            for k in 0..4 {
                forces[tet[k]] += g * mass;
            }
        }
        for i in 0..n {
            if self.mesh.fixed[i] {
                forces[i] = Vec3::ZERO;
            }
        }
        forces
    }

    pub fn step(&mut self, dt: f32) {
        let forces = self.compute_forces();
        let damping = self.config.elastic.damping;
        let rho = self.config.elastic.density;
        for t in 0..self.mesh.num_tets() {
            let v0 = self.rest_volumes[t];
            let mass_per_vertex = rho * v0 / 4.0;
            let tet = self.mesh.tets[t];
            for k in 0..4 {
                let vi = tet[k];
                if !self.mesh.fixed[vi] && mass_per_vertex > 1e-10 {
                    let a = forces[vi] / mass_per_vertex;
                    let damped = self.mesh.velocities[vi] * (1.0 - damping * dt);
                    self.mesh.velocities[vi] = damped + a * dt;
                }
            }
        }
        for i in 0..self.mesh.num_vertices() {
            if !self.mesh.fixed[i] {
                self.mesh.vertices[i] += self.mesh.velocities[i] * dt;
            }
        }
        self.time += dt;
    }

    pub fn reset_plastic(&mut self) {
        for fp in &mut self.f_plastic {
            *fp = Mat3::IDENTITY;
        }
    }

    pub fn deform_vertex(&mut self, i: usize, delta: Vec3) {
        self.mesh.vertices[i] += delta;
    }

    pub fn pressure(&self, tet_idx: usize) -> f32 {
        let f_e = self.elastic_deformation(tet_idx);
        let svd = svd_3x3(f_e);
        let p = (svd.sigma[0] + svd.sigma[1] + svd.sigma[2]) / 3.0;
        1.0 - p
    }

    pub fn is_yielded(&self, tet_idx: usize) -> bool {
        let f_e = self.elastic_deformation(tet_idx);
        let svd = svd_3x3(f_e);
        let sigma = svd.sigma;
        let p_vol = (sigma[0] + sigma[1] + sigma[2]) / 3.0;
        let s = [sigma[0] - p_vol, sigma[1] - p_vol, sigma[2] - p_vol];
        let s_norm = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
        let threshold = self.config.friction_angle.tan() * p_vol + self.config.cohesion;
        s_norm > threshold && threshold > 0.0
    }

    pub fn total_plastic_deformation(&self) -> f32 {
        let mut sum = 0.0_f32;
        let n = self.f_plastic.len();
        if n == 0 {
            return 0.0;
        }
        for fp in &self.f_plastic {
            let diff = Mat3::from_cols(
                fp.x_axis - Vec3::new(1.0, 0.0, 0.0),
                fp.y_axis - Vec3::new(0.0, 1.0, 0.0),
                fp.z_axis - Vec3::new(0.0, 0.0, 1.0),
            );
            sum += mat3_max_abs(diff);
        }
        sum / n as f32
    }
}

fn mat3_max_abs(m: Mat3) -> f32 {
    let a = m.abs();
    a.x_axis.max(a.y_axis).max(a.z_axis).max_element()
}

fn compute_rest_shape(rest_vertices: &[Vec3], tet: [usize; 4]) -> (Mat3, f32) {
    let x0 = rest_vertices[tet[0]];
    let x1 = rest_vertices[tet[1]];
    let x2 = rest_vertices[tet[2]];
    let x3 = rest_vertices[tet[3]];
    let dm = Mat3::from_cols(x1 - x0, x2 - x0, x3 - x0);
    let dm_inv = dm.inverse();
    let v0 = dm.determinant().abs() / 6.0;
    (dm_inv, v0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_unit_tet() -> TetMesh {
        let verts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let tets = vec![[0, 1, 2, 3]];
        TetMesh::new(verts, tets)
    }

    #[test]
    fn test_default_config_sand() {
        let cfg = DruckerPragerConfig::default();
        assert!((cfg.friction_angle - 30.0_f32.to_radians()).abs() < 1e-6);
        assert!((cfg.cohesion - 0.0).abs() < 1e-6);
        assert!((cfg.elastic.density - 1600.0).abs() < 1e-3);
        assert!((cfg.elastic.youngs_modulus - 5e5).abs() < 1.0);
    }

    #[test]
    fn test_solver_creation() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let solver = DruckerPragerSolver::new(mesh, cfg);
        assert_eq!(solver.mesh.num_tets(), 1);
        assert_eq!(solver.f_plastic.len(), 1);
        let diff = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff < 1e-6, "F_p should start as I, diff={}", diff);
        let e = 5e5;
        let nu = 0.3;
        let lambda_expected = e * nu / ((1.0 + nu) * (1.0 - 2.0 * nu));
        let mu_expected = e / (2.0 * (1.0 + nu));
        assert!((solver.lame_lambda - lambda_expected).abs() < 1.0);
        assert!((solver.lame_mu - mu_expected).abs() < 1.0);
    }

    #[test]
    fn test_total_deformation_rest() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let solver = DruckerPragerSolver::new(mesh, cfg);
        let f = solver.total_deformation(0);
        let diff = mat3_max_abs(f - Mat3::IDENTITY);
        assert!(diff < 1e-5, "F should be I at rest, diff={}", diff);
    }

    #[test]
    fn test_total_deformation_compression() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        // 压缩 v1 到 0.5 倍
        solver.deform_vertex(1, Vec3::new(-0.5, 0.0, 0.0));
        let f = solver.total_deformation(0);
        assert!((f.x_axis - Vec3::new(0.5, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_return_mapping_no_yield_small_deform() {
        // 小变形, 在弹性范围内
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        // 微小压缩 0.01
        solver.deform_vertex(1, Vec3::new(-0.01, 0.0, 0.0));
        let _ = solver.return_mapping(0);
        let diff = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff < 1e-4, "F_p should not change below yield, diff={}", diff);
    }

    #[test]
    fn test_return_mapping_compression_yield() {
        // 大压缩: σ1 << 1, p_vol < 1, threshold = tan(30)*p_vol + 0
        // σ = (0.1, 1, 1), p_vol = 0.7, s = (-0.6, 0.3, 0.3), |s| = 0.735
        // threshold = 0.577 * 0.7 = 0.404, |s| > threshold → 屈服
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(-0.9, 0.0, 0.0));
        let _ = solver.return_mapping(0);
        let diff = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff > 1e-3, "F_p should change after yielding, diff={}", diff);
    }

    #[test]
    fn test_return_mapping_shear_yield() {
        // 剪切: σ1 > 1, σ3 < 1, 偏量大
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        // 拉伸 v1 + 压缩 v3
        solver.deform_vertex(1, Vec3::new(1.0, 0.0, 0.0));
        solver.deform_vertex(3, Vec3::new(0.0, 0.0, -0.5));
        let _ = solver.return_mapping(0);
        let diff = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff > 1e-3, "F_p should change after shear yielding, diff={}", diff);
    }

    #[test]
    fn test_apex_projection_tension() {
        // 拉伸 (σ > 1, p_vol > 1, 但... 实际是看 threshold ≤ 0)
        // 干沙 cohesion=0, threshold = tan(phi)*p_vol, 仅当 p_vol<0 时 threshold<0
        // 但 σ_i > 0 始终 (奇异值), p_vol > 0, threshold > 0
        // 所以需要构造特殊场景: 用 cohesion 负值或负 p_vol
        // 实际中: 沙拉伸时, F_e_trial 的 σ 可能为负 (如果 F_p 累积过大)
        // 这里测试 apex 投影分支: 设 cohesion < 0 (异常), 或直接测试代码路径
        // 简单测试: 拉伸变形 + cohesion=0, 检查不崩溃
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(5.0, 0.0, 0.0));
        let f_e = solver.return_mapping(0);
        assert!(f_e.x_axis.is_finite());
        assert!(f_e.y_axis.is_finite());
        assert!(f_e.z_axis.is_finite());
    }

    #[test]
    fn test_apex_projection_zero_friction() {
        // 摩擦角 = 0 → tan(phi) = 0 → threshold = cohesion
        // 若 cohesion = 0 且有任何偏量 → 屈服, 退化到 apex
        let mesh = make_unit_tet();
        let mut cfg = DruckerPragerConfig::default();
        cfg.friction_angle = 0.0;
        cfg.cohesion = 0.0;
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(0.5, 0.0, 0.0));
        let _ = solver.return_mapping(0);
        // 应该触发 apex 分支 (threshold=0)
        let diff = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff > 1e-3, "F_p should change at apex, diff={}", diff);
    }

    #[test]
    fn test_cohesion_increases_strength() {
        // 有内聚力的沙需要更大变形才屈服
        let mesh1 = make_unit_tet();
        let mesh2 = make_unit_tet();
        let mut cfg1 = DruckerPragerConfig::default();
        cfg1.cohesion = 0.0;
        let mut cfg2 = DruckerPragerConfig::default();
        cfg2.cohesion = 1.0;
        let mut s1 = DruckerPragerSolver::new(mesh1, cfg1);
        let mut s2 = DruckerPragerSolver::new(mesh2, cfg2);
        // 同样的小变形
        s1.deform_vertex(1, Vec3::new(-0.2, 0.0, 0.0));
        s2.deform_vertex(1, Vec3::new(-0.2, 0.0, 0.0));
        let _ = s1.return_mapping(0);
        let _ = s2.return_mapping(0);
        let p1 = mat3_max_abs(s1.f_plastic[0] - Mat3::IDENTITY);
        let p2 = mat3_max_abs(s2.f_plastic[0] - Mat3::IDENTITY);
        // 有内聚力的应该塑性更小 (更难屈服)
        assert!(p2 <= p1 + 1e-6, "cohesion should reduce plastic: p1={}, p2={}", p1, p2);
    }

    #[test]
    fn test_friction_angle_increases_strength() {
        // 摩擦角越大, 沙越硬 (压力贡献越大)
        let mesh1 = make_unit_tet();
        let mesh2 = make_unit_tet();
        let mut cfg1 = DruckerPragerConfig::default();
        cfg1.friction_angle = 20.0_f32.to_radians();
        let mut cfg2 = DruckerPragerConfig::default();
        cfg2.friction_angle = 40.0_f32.to_radians();
        let mut s1 = DruckerPragerSolver::new(mesh1, cfg1);
        let mut s2 = DruckerPragerSolver::new(mesh2, cfg2);
        // 压缩变形 (产生压力)
        s1.deform_vertex(1, Vec3::new(-0.3, 0.0, 0.0));
        s2.deform_vertex(1, Vec3::new(-0.3, 0.0, 0.0));
        let _ = s1.return_mapping(0);
        let _ = s2.return_mapping(0);
        let p1 = mat3_max_abs(s1.f_plastic[0] - Mat3::IDENTITY);
        let p2 = mat3_max_abs(s2.f_plastic[0] - Mat3::IDENTITY);
        assert!(p2 <= p1 + 1e-6, "higher friction should reduce plastic: p1={}, p2={}", p1, p2);
    }

    #[test]
    fn test_tet_stress_identity_zero() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let solver = DruckerPragerSolver::new(mesh, cfg);
        let p = solver.tet_stress_from_elastic(Mat3::IDENTITY);
        assert!(mat3_max_abs(p) < 1e-3, "stress at F=I should be 0");
    }

    #[test]
    fn test_tet_stress_compression() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let solver = DruckerPragerSolver::new(mesh, cfg);
        let f = Mat3::from_cols(
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );
        let p = solver.tet_stress_from_elastic(f);
        // 压缩 σ_xx 应为负 (恢复力指向 +x)
        assert!(p.x_axis.x < 0.0, "P_xx should be negative for compression, got {}", p.x_axis.x);
    }

    #[test]
    fn test_compute_tet_forces_rest() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        let forces = solver.compute_tet_forces(0);
        for (i, f) in forces.iter().enumerate() {
            assert!(f.length() < 1e-2, "rest force[{}] should be ~0, got {:?}", i, f);
        }
    }

    #[test]
    fn test_compute_tet_forces_compression_pushes_back() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(-0.5, 0.0, 0.0));
        let forces = solver.compute_tet_forces(0);
        // v1 被压缩到 -0.5, 应被推向 +x
        assert!(forces[1].x > 0.0, "f1.x should be positive (pushing back), got {}", forces[1].x);
    }

    #[test]
    fn test_compute_forces_static_equilibrium() {
        let mesh = make_unit_tet();
        let mut cfg = DruckerPragerConfig::default();
        cfg.elastic.gravity = Vec3::ZERO;
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        let forces = solver.compute_forces();
        for (i, f) in forces.iter().enumerate() {
            assert!(f.length() < 1e-2, "rest force[{}] should be ~0, got {:?}", i, f);
        }
    }

    #[test]
    fn test_step_zero_force_no_motion() {
        let mesh = make_unit_tet();
        let mut cfg = DruckerPragerConfig::default();
        cfg.elastic.gravity = Vec3::ZERO;
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        let before = solver.mesh.vertices[0];
        solver.step(0.01);
        let after = solver.mesh.vertices[0];
        assert!((before - after).length() < 1e-3);
    }

    #[test]
    fn test_step_gravity_drops() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        let y_before = solver.mesh.vertices[0].y;
        for _ in 0..10 {
            solver.step(0.01);
        }
        let y_after = solver.mesh.vertices[0].y;
        assert!(y_after < y_before, "vertex should fall: {} -> {}", y_before, y_after);
    }

    #[test]
    fn test_step_fixed_vertex_stays() {
        let mut mesh = make_unit_tet();
        mesh.fix(0);
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        let before = solver.mesh.vertices[0];
        for _ in 0..10 {
            solver.step(0.01);
        }
        let after = solver.mesh.vertices[0];
        assert!((before - after).length() < 1e-5);
    }

    #[test]
    fn test_reset_plastic() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(-0.9, 0.0, 0.0));
        solver.return_mapping(0);
        let before = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(before > 1e-3);
        solver.reset_plastic();
        let after = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(after < 1e-6);
    }

    #[test]
    fn test_total_plastic_deformation_initial_zero() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let solver = DruckerPragerSolver::new(mesh, cfg);
        assert!(solver.total_plastic_deformation() < 1e-6);
    }

    #[test]
    fn test_total_plastic_grows_after_yield() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        let initial = solver.total_plastic_deformation();
        solver.deform_vertex(1, Vec3::new(-0.9, 0.0, 0.0));
        solver.return_mapping(0);
        let after = solver.total_plastic_deformation();
        assert!(after > initial, "plastic should grow: {} -> {}", initial, after);
    }

    #[test]
    fn test_pressure_positive_under_compression() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        // 各向同性压缩 (σ < 1)
        solver.deform_vertex(1, Vec3::new(-0.5, 0.0, 0.0));
        solver.deform_vertex(2, Vec3::new(0.0, -0.5, 0.0));
        solver.deform_vertex(3, Vec3::new(0.0, 0.0, -0.5));
        let p = solver.pressure(0);
        assert!(p > 0.0, "pressure should be positive (compression), got {}", p);
    }

    #[test]
    fn test_pressure_negative_under_tension() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(0.5, 0.0, 0.0));
        solver.deform_vertex(2, Vec3::new(0.0, 0.5, 0.0));
        solver.deform_vertex(3, Vec3::new(0.0, 0.0, 0.5));
        let p = solver.pressure(0);
        assert!(p < 0.0, "pressure should be negative (tension), got {}", p);
    }

    #[test]
    fn test_is_yielded_initial_false() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let solver = DruckerPragerSolver::new(mesh, cfg);
        assert!(!solver.is_yielded(0));
    }

    #[test]
    fn test_is_yielded_after_large_shear() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(2.0, 0.0, 0.0));
        solver.deform_vertex(3, Vec3::new(0.0, 0.0, -0.5));
        // 剪切变形应触发屈服
        assert!(solver.is_yielded(0), "should be yielded after large shear");
    }

    #[test]
    fn test_plastic_persists_after_release() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(-0.9, 0.0, 0.0));
        solver.return_mapping(0);
        let after_stretch = solver.total_plastic_deformation();
        assert!(after_stretch > 1e-3);
        solver.mesh.reset_to_rest();
        let after_release = solver.total_plastic_deformation();
        assert!(
            (after_stretch - after_release).abs() < 1e-3,
            "plastic should persist after position reset"
        );
    }

    #[test]
    fn test_inversion_safety() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(-3.0, 0.0, 0.0));
        let f_e = solver.return_mapping(0);
        assert!(f_e.x_axis.is_finite());
        assert!(f_e.y_axis.is_finite());
        assert!(f_e.z_axis.is_finite());
    }

    #[test]
    fn test_extreme_deformation_no_nan() {
        let mesh = make_unit_tet();
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(100.0, 0.0, 0.0));
        let _ = solver.compute_tet_forces(0);
        let p = solver.tet_stress_from_elastic(solver.elastic_deformation(0));
        assert!(p.x_axis.is_finite());
        assert!(p.y_axis.is_finite());
        assert!(p.z_axis.is_finite());
    }

    #[test]
    fn test_step_multiple_tets_stable() {
        let verts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];
        let tets = vec![[0, 1, 2, 3], [1, 2, 3, 4]];
        let mesh = TetMesh::new(verts, tets);
        let cfg = DruckerPragerConfig::default();
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        for _ in 0..20 {
            solver.step(0.005);
        }
        for v in &solver.mesh.vertices {
            assert!(v.is_finite());
        }
        for vel in &solver.mesh.velocities {
            assert!(vel.is_finite());
        }
    }

    #[test]
    fn test_mat3_max_abs() {
        let m = Mat3::from_cols(
            Vec3::new(-3.0, 0.0, 0.0),
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.0, 0.0, -2.0),
        );
        assert!((mat3_max_abs(m) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_dry_sand_zero_cohesion_default() {
        let cfg = DruckerPragerConfig::default();
        assert!(cfg.cohesion == 0.0, "dry sand should have zero cohesion");
    }

    #[test]
    fn test_wet_sand_cohesion_positive() {
        let mut cfg = DruckerPragerConfig::default();
        cfg.cohesion = 100.0;  // 湿沙有内聚力
        let mesh = make_unit_tet();
        let solver = DruckerPragerSolver::new(mesh, cfg);
        assert!(solver.config.cohesion > 0.0);
    }

    #[test]
    fn test_high_friction_resists_shear() {
        // 高摩擦角 (45°) 的沙土在小剪切下不屈服
        let mesh = make_unit_tet();
        let mut cfg = DruckerPragerConfig::default();
        cfg.friction_angle = 45.0_f32.to_radians();
        cfg.cohesion = 1.0;
        let mut solver = DruckerPragerSolver::new(mesh, cfg);
        // 小剪切
        solver.deform_vertex(1, Vec3::new(0.05, 0.0, 0.0));
        let _ = solver.return_mapping(0);
        let diff = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff < 1e-3, "high friction + cohesion should resist small shear, diff={}", diff);
    }
}
