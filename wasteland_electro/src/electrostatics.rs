use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::properties::{ElectromagneticProperties, VACUUM_PERMITTIVITY};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointCharge {
    pub position: Vec3,
    pub charge: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElectrostaticSolver {
    pub time_step: f32,
    pub convergence_threshold: f32,
    pub max_iterations: u32,
    pub ambient_potential: f32,
}

impl Default for ElectrostaticSolver {
    fn default() -> Self {
        Self {
            time_step: 1.0 / 60.0,
            convergence_threshold: 1e-6,
            max_iterations: 1000,
            ambient_potential: 0.0,
        }
    }
}

impl ElectrostaticSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn coulomb_force(&self, q1: f32, q2: f32, r_vec: Vec3, epsilon_r: f32) -> Vec3 {
        let r = r_vec.length();
        if r < 1e-9 {
            return Vec3::ZERO;
        }
        let k = 1.0 / (4.0 * std::f32::consts::PI * VACUUM_PERMITTIVITY * epsilon_r);
        let force_mag = k * q1 * q2 / (r * r);
        r_vec.normalize() * force_mag
    }

    pub fn electric_field(
        &self,
        charge: f32,
        source_pos: Vec3,
        target_pos: Vec3,
        epsilon_r: f32,
    ) -> Vec3 {
        let r_vec = target_pos - source_pos;
        let r = r_vec.length();
        if r < 1e-9 {
            return Vec3::ZERO;
        }
        let k = 1.0 / (4.0 * std::f32::consts::PI * VACUUM_PERMITTIVITY * epsilon_r);
        let e_mag = k * charge / (r * r);
        r_vec.normalize() * e_mag
    }

    pub fn electric_field_from_charges(
        &self,
        charges: &[PointCharge],
        target_pos: Vec3,
        epsilon_r: f32,
    ) -> Vec3 {
        let mut e = Vec3::ZERO;
        for pc in charges {
            e += self.electric_field(pc.charge, pc.position, target_pos, epsilon_r);
        }
        e
    }

    pub fn potential(
        &self,
        charge: f32,
        source_pos: Vec3,
        target_pos: Vec3,
        epsilon_r: f32,
    ) -> f32 {
        let r = (target_pos - source_pos).length();
        if r < 1e-9 {
            return 0.0;
        }
        let k = 1.0 / (4.0 * std::f32::consts::PI * VACUUM_PERMITTIVITY * epsilon_r);
        k * charge / r
    }

    pub fn potential_from_charges(
        &self,
        charges: &[PointCharge],
        target_pos: Vec3,
        epsilon_r: f32,
    ) -> f32 {
        charges.iter().map(|pc| self.potential(pc.charge, pc.position, target_pos, epsilon_r)).sum()
    }

    pub fn solve_poisson_3d(
        &self,
        potential: &mut [f32],
        charges: &[PointCharge],
        dimensions: (usize, usize, usize),
        spacing: Vec3,
        epsilon_r: f32,
    ) -> u32 {
        let (nx, ny, nz) = dimensions;
        let total = nx * ny * nz;
        let dx2 = spacing.x * spacing.x;
        let dy2 = spacing.y * spacing.y;
        let dz2 = spacing.z * spacing.z;
        let denominator = 2.0 / dx2 + 2.0 / dy2 + 2.0 / dz2;

        assert_eq!(potential.len(), total);

        for _ in 0..self.max_iterations {
            let mut max_diff = 0.0f32;

            for k in 1..nz - 1 {
                for j in 1..ny - 1 {
                    for i in 1..nx - 1 {
                        let idx = i + j * nx + k * nx * ny;
                        let idx_xp = (i + 1) + j * nx + k * nx * ny;
                        let idx_xm = (i - 1) + j * nx + k * nx * ny;
                        let idx_yp = i + (j + 1) * nx + k * nx * ny;
                        let idx_ym = i + (j - 1) * nx + k * nx * ny;
                        let idx_zp = i + j * nx + (k + 1) * nx * ny;
                        let idx_zm = i + j * nx + (k - 1) * nx * ny;

                        let pos = Vec3::new(
                            i as f32 * spacing.x,
                            j as f32 * spacing.y,
                            k as f32 * spacing.z,
                        );
                        let rho = self.charge_density_at(pos, charges);

                        let new_val = ((potential[idx_xp] + potential[idx_xm]) / dx2
                            + (potential[idx_yp] + potential[idx_ym]) / dy2
                            + (potential[idx_zp] + potential[idx_zm]) / dz2
                            + rho / (VACUUM_PERMITTIVITY * epsilon_r))
                            / denominator;

                        let diff = (new_val - potential[idx]).abs();
                        max_diff = max_diff.max(diff);
                        potential[idx] = new_val;
                    }
                }
            }

            if max_diff < self.convergence_threshold {
                return 0;
            }
        }

        1
    }

    fn charge_density_at(&self, pos: Vec3, charges: &[PointCharge]) -> f32 {
        charges
            .iter()
            .map(|pc| {
                let dist = (pos - pc.position).length();
                if dist < 1e-6 { pc.charge * 1e12 } else { 0.0 }
            })
            .sum()
    }

    pub fn compute_electric_field_from_grid(
        &self,
        potential: &[f32],
        dimensions: (usize, usize, usize),
        spacing: Vec3,
        target_pos: Vec3,
    ) -> Vec3 {
        let (nx, ny, nz) = dimensions;
        let i = (target_pos.x / spacing.x) as usize;
        let j = (target_pos.y / spacing.y) as usize;
        let k = (target_pos.z / spacing.z) as usize;

        if i == 0 || i >= nx - 1 || j == 0 || j >= ny - 1 || k == 0 || k >= nz - 1 {
            return Vec3::ZERO;
        }

        let idx = |x: usize, y: usize, z: usize| x + y * nx + z * nx * ny;

        let ex = -(potential[idx(i + 1, j, k)] - potential[idx(i - 1, j, k)]) / (2.0 * spacing.x);
        let ey = -(potential[idx(i, j + 1, k)] - potential[idx(i, j - 1, k)]) / (2.0 * spacing.y);
        let ez = -(potential[idx(i, j, k + 1)] - potential[idx(i, j, k - 1)]) / (2.0 * spacing.z);

        Vec3::new(ex, ey, ez)
    }

    pub fn solve_dielectric_interface(
        &self,
        e_incident: Vec3,
        normal: Vec3,
        epsilon_r1: f32,
        epsilon_r2: f32,
    ) -> (Vec3, Vec3) {
        let n = normal.normalize();
        let cos_theta = -e_incident.normalize().dot(n);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let e_parallel = e_incident - e_incident.dot(n) * n;
        let e_perp = e_incident.dot(n) * n;

        let reflection_coeff_parallel = (epsilon_r2 * cos_theta
            - epsilon_r1 * (epsilon_r2 / epsilon_r1 - sin_theta * sin_theta).sqrt())
            / (epsilon_r2 * cos_theta
                + epsilon_r1 * (epsilon_r2 / epsilon_r1 - sin_theta * sin_theta).sqrt());
        let reflection_coeff_perp = (epsilon_r1 * cos_theta
            - epsilon_r1 * (epsilon_r2 / epsilon_r1 - sin_theta * sin_theta).sqrt())
            / (epsilon_r1 * cos_theta
                + epsilon_r1 * (epsilon_r2 / epsilon_r1 - sin_theta * sin_theta).sqrt());

        let e_reflected =
            e_parallel * (-reflection_coeff_parallel) + e_perp * (-reflection_coeff_perp);
        let e_transmitted = e_incident + e_reflected;

        (e_reflected, e_transmitted)
    }

    pub fn capacitance_parallel_plate(&self, area: f32, separation: f32, epsilon_r: f32) -> f32 {
        epsilon_r * VACUUM_PERMITTIVITY * area / separation.max(1e-9)
    }

    pub fn energy_density(&self, e_field: Vec3, epsilon_r: f32) -> f32 {
        0.5 * epsilon_r * VACUUM_PERMITTIVITY * e_field.length_squared()
    }

    pub fn breakdown_voltage(&self, separation: f32, props: &ElectromagneticProperties) -> f32 {
        props.dielectric_strength * separation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coulomb_force_repulsion() {
        let solver = ElectrostaticSolver::new();
        let f = solver.coulomb_force(1.0, 1.0, Vec3::new(1.0, 0.0, 0.0), 1.0);
        assert!(f.x > 0.0);
    }

    #[test]
    fn test_coulomb_force_attraction() {
        let solver = ElectrostaticSolver::new();
        let f = solver.coulomb_force(1.0, -1.0, Vec3::new(1.0, 0.0, 0.0), 1.0);
        assert!(f.x < 0.0);
    }

    #[test]
    fn test_coulomb_inverse_square() {
        let solver = ElectrostaticSolver::new();
        let f1 = solver.coulomb_force(1.0, 1.0, Vec3::new(1.0, 0.0, 0.0), 1.0);
        let f2 = solver.coulomb_force(1.0, 1.0, Vec3::new(2.0, 0.0, 0.0), 1.0);
        let ratio = f1.x / f2.x;
        assert!(ratio > 3.8 && ratio < 4.2);
    }

    #[test]
    fn test_zero_distance() {
        let solver = ElectrostaticSolver::new();
        let f = solver.coulomb_force(1.0, 1.0, Vec3::ZERO, 1.0);
        assert_eq!(f, Vec3::ZERO);
    }

    #[test]
    fn test_electric_field_superposition() {
        let solver = ElectrostaticSolver::new();
        let charges = vec![
            PointCharge { position: Vec3::new(1.0, 0.0, 0.0), charge: 1.0 },
            PointCharge { position: Vec3::new(-1.0, 0.0, 0.0), charge: 1.0 },
        ];
        let e = solver.electric_field_from_charges(&charges, Vec3::ZERO, 1.0);
        assert!(e.x.abs() < 1e-6);
    }

    #[test]
    fn test_poisson_solver_convergence() {
        let solver = ElectrostaticSolver::new();
        let (nx, ny, nz) = (10, 10, 10);
        let mut potential = vec![0.0f32; nx * ny * nz];
        let charges = vec![PointCharge { position: Vec3::new(5.0, 5.0, 5.0), charge: 1e-9 }];
        let status =
            solver.solve_poisson_3d(&mut potential, &charges, (nx, ny, nz), Vec3::splat(1.0), 1.0);
        assert_eq!(status, 0);
    }

    #[test]
    fn test_capacitance() {
        let solver = ElectrostaticSolver::new();
        let c = solver.capacitance_parallel_plate(1.0, 0.001, 1.0);
        assert!(c > 0.0);
    }

    #[test]
    fn test_energy_density() {
        let solver = ElectrostaticSolver::new();
        let u = solver.energy_density(Vec3::new(1000.0, 0.0, 0.0), 1.0);
        assert!(u > 0.0);
    }
}
