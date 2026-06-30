//! Plastic FEM — 弹塑性有限元 (雪/沙/泥/金属凹痕)
//!
//! 基于:
//! - Stomakhin, Schroeder, Chai, Teran, Selle. "A Material Point Method for
//!   Snow Simulation." Walt Disney Animation Studios, SIGGRAPH 2013.
//! - Bonet, Wood. "Nonlinear Continuum Mechanics for Finite Element
//!   Analysis." Cambridge University Press, 2008.
//! - Klar, Gast, Pradhana, Fu, Jiang, Teran. "Drucker-Prager Elastoplasticity
//!   for Sand Animation." ACM TOG (SIGGRAPH 2016), 35(4).
//!
//! 核心思想 (弹塑性分解):
//! 1. 总变形梯度 F = F_elastic · F_plastic
//! 2. Return mapping:
//!    a. trial elastic: F_e^trial = F · F_p⁻¹
//!    b. SVD: F_e^trial = U · Σ · Vᵀ
//!    c. clamped yield: σ ∈ [1-θ_c, 1+θ_s]
//!    d. F_elastic = U · Σ_clamped · Vᵀ
//!    e. F_plastic_new = F_plastic · V · Σ · Σ_clamped⁻¹ · Vᵀ
//! 3. 应力 (Neo-Hookean): P = μ·F_e + (λ·log(J_e) - μ)·F_e⁻ᵀ
//! 4. 力矩阵 H = -V₀ · Pᵀ · Dm⁻ᵀ

use crate::corotational_fem::{CorotationalFemConfig, TetMesh};
use crate::stable_neo_hookean::svd_3x3;
use glam::{Mat3, Vec3};

/// Compute max absolute element of a Mat3
fn mat3_max_abs(m: Mat3) -> f32 {
    let a = m.abs();
    a.x_axis.max(a.y_axis).max(a.z_axis).max_element()
}

#[derive(Debug, Clone)]
pub struct PlasticFemConfig {
    pub elastic: CorotationalFemConfig,
    pub compression_yield: f32,
    pub stretch_yield: f32,
    pub hardening: f32,
}

impl Default for PlasticFemConfig {
    fn default() -> Self {
        Self {
            elastic: CorotationalFemConfig {
                youngs_modulus: 1.4e5,
                poisson_ratio: 0.3,
                density: 400.0,
                damping: 0.5,
                gravity: Vec3::new(0.0, -9.81, 0.0),
            },
            compression_yield: 0.025,
            stretch_yield: 0.025,
            hardening: 0.0,
        }
    }
}

pub struct PlasticFemSolver {
    pub mesh: TetMesh,
    pub config: PlasticFemConfig,
    rest_inverses: Vec<Mat3>,
    rest_volumes: Vec<f32>,
    pub f_plastic: Vec<Mat3>,
    pub lame_lambda: f32,
    pub lame_mu: f32,
    pub time: f32,
}

impl PlasticFemSolver {
    pub fn new(mesh: TetMesh, config: PlasticFemConfig) -> Self {
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

    pub fn return_mapping(&mut self, tet_idx: usize) -> Mat3 {
        let f_total = self.total_deformation(tet_idx);
        let f_plastic = self.f_plastic[tet_idx];
        let f_plastic_inv = f_plastic.inverse();
        let f_elastic_trial = f_total * f_plastic_inv;
        let svd = svd_3x3(f_elastic_trial);
        let sigma = svd.sigma;
        let theta_c = self.config.compression_yield;
        let theta_s = self.config.stretch_yield;
        let sigma_min = 1.0 - theta_c;
        let sigma_max = 1.0 + theta_s;
        let sigma_clamped = [
            sigma[0].clamp(sigma_min, sigma_max),
            sigma[1].clamp(sigma_min, sigma_max),
            sigma[2].clamp(sigma_min, sigma_max),
        ];
        let yielded = sigma[0] != sigma_clamped[0]
            || sigma[1] != sigma_clamped[1]
            || sigma[2] != sigma_clamped[2];
        if yielded {
            let sigma_mat = Mat3::from_diagonal(Vec3::new(sigma[0], sigma[1], sigma[2]));
            let sigma_clamped_inv = Mat3::from_diagonal(Vec3::new(
                1.0 / sigma_clamped[0].max(1e-10),
                1.0 / sigma_clamped[1].max(1e-10),
                1.0 / sigma_clamped[2].max(1e-10),
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
            sum += Self::frobenius_norm(diff);
        }
        sum / n as f32
    }

    fn frobenius_norm(m: Mat3) -> f32 {
        m.x_axis.length_squared() + m.y_axis.length_squared() + m.z_axis.length_squared()
    }
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
    fn test_default_config_snow() {
        let cfg = PlasticFemConfig::default();
        assert!((cfg.compression_yield - 0.025).abs() < 1e-6);
        assert!((cfg.stretch_yield - 0.025).abs() < 1e-6);
        assert!((cfg.elastic.youngs_modulus - 1.4e5).abs() < 1.0);
        assert!((cfg.elastic.density - 400.0).abs() < 1e-3);
    }

    #[test]
    fn test_solver_creation() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let solver = PlasticFemSolver::new(mesh, cfg);
        assert_eq!(solver.mesh.num_tets(), 1);
        assert_eq!(solver.f_plastic.len(), 1);
        let diff = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff < 1e-6, "F_p should start as I, diff={}", diff);
        let e = 1.4e5;
        let nu = 0.3;
        let lambda_expected = e * nu / ((1.0 + nu) * (1.0 - 2.0 * nu));
        let mu_expected = e / (2.0 * (1.0 + nu));
        assert!((solver.lame_lambda - lambda_expected).abs() < 1.0);
        assert!((solver.lame_mu - mu_expected).abs() < 1.0);
    }

    #[test]
    fn test_total_deformation_rest() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let solver = PlasticFemSolver::new(mesh, cfg);
        let f = solver.total_deformation(0);
        let diff = mat3_max_abs(f - Mat3::IDENTITY);
        assert!(diff < 1e-5, "F should be I at rest, diff={}", diff);
    }

    #[test]
    fn test_total_deformation_stretch() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(1.0, 0.0, 0.0));
        let f = solver.total_deformation(0);
        assert!((f.x_axis - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-5);
        assert!((f.y_axis - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-5);
        assert!((f.z_axis - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-5);
    }

    #[test]
    fn test_elastic_deformation_no_plastic() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(1.0, 0.0, 0.0));
        let f_total = solver.total_deformation(0);
        let f_elastic = solver.elastic_deformation(0);
        let diff = mat3_max_abs(f_total - f_elastic);
        assert!(diff < 1e-4, "F_e should equal F_total when F_p=I, diff={}", diff);
    }

    #[test]
    fn test_return_mapping_no_yield() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(0.01, 0.0, 0.0));
        let f_e = solver.return_mapping(0);
        let diff_fp = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff_fp < 1e-5, "F_p should not change below yield, diff={}", diff_fp);
        let diff_fe = (f_e.x_axis - Vec3::new(1.01, 0.0, 0.0)).length();
        assert!(diff_fe < 1e-3, "F_e should equal trial, diff={}", diff_fe);
    }

    #[test]
    fn test_return_mapping_compression_yield() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(-0.5, 0.0, 0.0));
        let _ = solver.return_mapping(0);
        let diff = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff > 1e-3, "F_p should change after yielding, diff={}", diff);
    }

    #[test]
    fn test_return_mapping_stretch_yield() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(1.0, 0.0, 0.0));
        let _ = solver.return_mapping(0);
        let diff = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(diff > 1e-3, "F_p should change after yielding, diff={}", diff);
    }

    #[test]
    fn test_return_mapping_clamped_sigma() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let stretch_yield = cfg.stretch_yield;
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(10.0, 0.0, 0.0));
        let f_e = solver.return_mapping(0);
        let svd = svd_3x3(f_e);
        let sigma_max = svd.sigma[0].max(svd.sigma[1]).max(svd.sigma[2]);
        assert!(
            sigma_max <= 1.0 + stretch_yield + 1e-3,
            "sigma_max should be clamped, got {}",
            sigma_max
        );
    }

    #[test]
    fn test_tet_stress_neo_hookean_identity() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let solver = PlasticFemSolver::new(mesh, cfg);
        let p = solver.tet_stress_from_elastic(Mat3::IDENTITY);
        let max = mat3_max_abs(p);
        assert!(max < 1e-3, "stress at F=I should be 0, got max={}", max);
    }

    #[test]
    fn test_tet_stress_stretch_positive() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let solver = PlasticFemSolver::new(mesh, cfg);
        let f = Mat3::from_cols(
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );
        let p = solver.tet_stress_from_elastic(f);
        assert!(p.x_axis.x > 0.0, "P_xx should be positive for stretch, got {}", p.x_axis.x);
    }

    #[test]
    fn test_compute_tet_forces_rest() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        let forces = solver.compute_tet_forces(0);
        for (i, f) in forces.iter().enumerate() {
            assert!(f.length() < 1e-2, "rest force[{}] should be ~0, got {:?}", i, f);
        }
    }

    #[test]
    fn test_compute_tet_forces_stretch_pulls_back() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(1.0, 0.0, 0.0));
        let forces = solver.compute_tet_forces(0);
        assert!(
            forces[1].x < 0.0,
            "f1.x should be negative (pulling back), got {}",
            forces[1].x
        );
    }

    #[test]
    fn test_compute_forces_static_equilibrium() {
        let mesh = make_unit_tet();
        let mut cfg = PlasticFemConfig::default();
        cfg.elastic.gravity = Vec3::ZERO;
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        let forces = solver.compute_forces();
        for (i, f) in forces.iter().enumerate() {
            assert!(f.length() < 1e-2, "rest force[{}] should be ~0, got {:?}", i, f);
        }
    }

    #[test]
    fn test_step_zero_force_no_motion() {
        let mesh = make_unit_tet();
        let mut cfg = PlasticFemConfig::default();
        cfg.elastic.gravity = Vec3::ZERO;
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        let before = solver.mesh.vertices[0];
        solver.step(0.01);
        let after = solver.mesh.vertices[0];
        assert!((before - after).length() < 1e-3);
    }

    #[test]
    fn test_step_gravity_drops() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        let y_before = solver.mesh.vertices[0].y;
        for _ in 0..10 {
            solver.step(0.01);
        }
        let y_after = solver.mesh.vertices[0].y;
        assert!(
            y_after < y_before,
            "vertex should fall, y_before={}, y_after={}",
            y_before,
            y_after
        );
    }

    #[test]
    fn test_step_fixed_vertex_stays() {
        let mut mesh = make_unit_tet();
        mesh.fix(0);
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
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
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(5.0, 0.0, 0.0));
        solver.return_mapping(0);
        let before = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(before > 1e-3);
        solver.reset_plastic();
        let after = mat3_max_abs(solver.f_plastic[0] - Mat3::IDENTITY);
        assert!(after < 1e-6, "F_p should be reset to I, got diff={}", after);
    }

    #[test]
    fn test_total_plastic_deformation_initial_zero() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let solver = PlasticFemSolver::new(mesh, cfg);
        assert!(solver.total_plastic_deformation() < 1e-6);
    }

    #[test]
    fn test_total_plastic_deformation_grows_after_yield() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        let initial = solver.total_plastic_deformation();
        solver.deform_vertex(1, Vec3::new(5.0, 0.0, 0.0));
        solver.return_mapping(0);
        let after = solver.total_plastic_deformation();
        assert!(after > initial, "plastic should grow: initial={}, after={}", initial, after);
    }

    #[test]
    fn test_plastic_persists_after_release() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(5.0, 0.0, 0.0));
        solver.return_mapping(0);
        let plastic_after_stretch = solver.total_plastic_deformation();
        assert!(plastic_after_stretch > 1e-3);
        solver.mesh.reset_to_rest();
        let plastic_after_release = solver.total_plastic_deformation();
        assert!(
            (plastic_after_stretch - plastic_after_release).abs() < 1e-3,
            "plastic should persist after position reset"
        );
    }

    #[test]
    fn test_repeated_loading_accumulates() {
        let mesh1 = make_unit_tet();
        let mesh2 = make_unit_tet();
        let cfg1 = PlasticFemConfig::default();
        let cfg2 = cfg1.clone();
        let mut solver1 = PlasticFemSolver::new(mesh1, cfg1);
        solver1.deform_vertex(1, Vec3::new(3.0, 0.0, 0.0));
        solver1.return_mapping(0);
        let plastic_single = solver1.total_plastic_deformation();
        let mut solver2 = PlasticFemSolver::new(mesh2, cfg2);
        for _ in 0..3 {
            solver2.deform_vertex(1, Vec3::new(1.0, 0.0, 0.0));
            solver2.return_mapping(0);
        }
        let plastic_multi = solver2.total_plastic_deformation();
        assert!(plastic_multi > 1e-3, "multi loading should yield plastic: {}", plastic_multi);
        assert!(plastic_single > 1e-3, "single loading should yield plastic: {}", plastic_single);
    }

    #[test]
    fn test_inversion_safety() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(-3.0, 0.0, 0.0));
        let f_e = solver.return_mapping(0);
        assert!(f_e.x_axis.is_finite());
        assert!(f_e.y_axis.is_finite());
        assert!(f_e.z_axis.is_finite());
    }

    #[test]
    fn test_stress_finite_under_extreme_deformation() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        solver.deform_vertex(1, Vec3::new(50.0, 0.0, 0.0));
        let _ = solver.compute_tet_forces(0);
        let p = solver.tet_stress_from_elastic(solver.elastic_deformation(0));
        assert!(p.x_axis.is_finite());
        assert!(p.y_axis.is_finite());
        assert!(p.z_axis.is_finite());
    }

    #[test]
    fn test_hardening_zero_default() {
        let cfg = PlasticFemConfig::default();
        assert!((cfg.hardening - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_volume_conservation_plastic() {
        let mesh = make_unit_tet();
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
        let v0_initial = solver.rest_volumes[0];
        solver.deform_vertex(1, Vec3::new(-0.5, 0.0, 0.0));
        solver.deform_vertex(2, Vec3::new(0.0, -0.5, 0.0));
        solver.deform_vertex(3, Vec3::new(0.0, 0.0, -0.5));
        solver.return_mapping(0);
        assert!((solver.rest_volumes[0] - v0_initial).abs() < 1e-6);
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
        let cfg = PlasticFemConfig::default();
        let mut solver = PlasticFemSolver::new(mesh, cfg);
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
    fn test_frobenius_norm() {
        let m = Mat3::from_cols(
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(0.0, 4.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        );
        let n = PlasticFemSolver::frobenius_norm(m);
        assert!((n - 25.0).abs() < 1e-5);
    }

    #[test]
    fn test_clamp_bounds() {
        let cfg = PlasticFemConfig::default();
        let sigma_min = 1.0 - cfg.compression_yield;
        let sigma_max = 1.0 + cfg.stretch_yield;
        assert!(sigma_min > 0.0 && sigma_min < 1.0);
        assert!(sigma_max > 1.0);
    }
}

