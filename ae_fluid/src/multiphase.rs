use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Liquid,
    Gas,
    Solid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseCell {
    pub volume_fraction: f32,
    pub phase: Phase,
    pub density: f32,
    pub viscosity: f32,
    pub surface_tension_coeff: f32,
}

impl Default for PhaseCell {
    fn default() -> Self {
        Self {
            volume_fraction: 0.0,
            phase: Phase::Gas,
            density: 1.225,
            viscosity: 1.8e-5,
            surface_tension_coeff: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub normal: Vec3,
    pub curvature: f32,
    pub position: Vec3,
    pub thickness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPhaseSolver {
    pub dimensions: (usize, usize, usize),
    pub spacing: f32,
    pub phases: Vec<Vec<PhaseCell>>,
    pub interfaces: Vec<Interface>,
    pub surface_tension_enabled: bool,
    pub num_phases: usize,
}

impl MultiPhaseSolver {
    pub fn new(dimensions: (usize, usize, usize), spacing: f32, num_phases: usize) -> Self {
        let (nx, ny, nz) = dimensions;
        let total = nx * ny * nz;
        let phases = (0..num_phases).map(|_| vec![PhaseCell::default(); total]).collect();
        Self {
            dimensions,
            spacing,
            phases,
            interfaces: Vec::new(),
            surface_tension_enabled: true,
            num_phases,
        }
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        let (nx, ny, _) = self.dimensions;
        x + y * nx + z * nx * ny
    }

    pub fn set_phase_region(
        &mut self,
        phase_idx: usize,
        origin: (usize, usize, usize),
        size: (usize, usize, usize),
        cell: PhaseCell,
    ) {
        let (nx, ny, nz) = self.dimensions;
        let (ox, oy, oz) = origin;
        let (sx, sy, sz) = size;
        for z in oz..(oz + sz).min(nz) {
            for y in oy..(oy + sy).min(ny) {
                for x in ox..(ox + sx).min(nx) {
                    let idx = self.index(x, y, z);
                    self.phases[phase_idx][idx] = cell.clone();
                    self.phases[phase_idx][idx].volume_fraction = 1.0;
                }
            }
        }
    }

    pub fn compute_interface_normals(&mut self) {
        let (nx, ny, nz) = self.dimensions;
        self.interfaces.clear();
        let h = self.spacing;

        for z in 1..nz - 1 {
            for y in 1..ny - 1 {
                for x in 1..nx - 1 {
                    let idx = self.index(x, y, z);
                    let f = self.phases[0][idx].volume_fraction;

                    if !(0.01..=0.99).contains(&f) {
                        continue;
                    }

                    let fx = (self.phases[0][self.index(x + 1, y, z)].volume_fraction
                        - self.phases[0][self.index(x - 1, y, z)].volume_fraction)
                        / (2.0 * h);
                    let fy = (self.phases[0][self.index(x, y + 1, z)].volume_fraction
                        - self.phases[0][self.index(x, y - 1, z)].volume_fraction)
                        / (2.0 * h);
                    let fz = (self.phases[0][self.index(x, y, z + 1)].volume_fraction
                        - self.phases[0][self.index(x, y, z - 1)].volume_fraction)
                        / (2.0 * h);

                    let gradient = Vec3::new(fx, fy, fz);
                    let grad_mag = gradient.length();
                    if grad_mag < 1e-6 {
                        continue;
                    }

                    let normal = gradient / grad_mag;

                    let fxx = (self.phases[0][self.index(x + 1, y, z)].volume_fraction - 2.0 * f
                        + self.phases[0][self.index(x - 1, y, z)].volume_fraction)
                        / (h * h);
                    let fyy = (self.phases[0][self.index(x, y + 1, z)].volume_fraction - 2.0 * f
                        + self.phases[0][self.index(x, y - 1, z)].volume_fraction)
                        / (h * h);
                    let fzz = (self.phases[0][self.index(x, y, z + 1)].volume_fraction - 2.0 * f
                        + self.phases[0][self.index(x, y, z - 1)].volume_fraction)
                        / (h * h);
                    let fxy = (self.phases[0][self.index(x + 1, y + 1, z)].volume_fraction
                        - self.phases[0][self.index(x - 1, y + 1, z)].volume_fraction
                        - self.phases[0][self.index(x + 1, y - 1, z)].volume_fraction
                        + self.phases[0][self.index(x - 1, y - 1, z)].volume_fraction)
                        / (4.0 * h * h);
                    let fxz = (self.phases[0][self.index(x + 1, y, z + 1)].volume_fraction
                        - self.phases[0][self.index(x - 1, y, z + 1)].volume_fraction
                        - self.phases[0][self.index(x + 1, y, z - 1)].volume_fraction
                        + self.phases[0][self.index(x - 1, y, z - 1)].volume_fraction)
                        / (4.0 * h * h);
                    let fyz = (self.phases[0][self.index(x, y + 1, z + 1)].volume_fraction
                        - self.phases[0][self.index(x, y - 1, z + 1)].volume_fraction
                        - self.phases[0][self.index(x, y + 1, z - 1)].volume_fraction
                        + self.phases[0][self.index(x, y - 1, z - 1)].volume_fraction)
                        / (4.0 * h * h);

                    let nx = normal.x;
                    let ny = normal.y;
                    let nz = normal.z;

                    let curvature =
                        (fxx * (1.0 - nx * nx) + fyy * (1.0 - ny * ny) + fzz * (1.0 - nz * nz)
                            - 2.0 * fxy * nx * ny
                            - 2.0 * fxz * nx * nz
                            - 2.0 * fyz * ny * nz)
                            / grad_mag;

                    self.interfaces.push(Interface {
                        normal,
                        curvature,
                        position: Vec3::new(x as f32 * h, y as f32 * h, z as f32 * h),
                        thickness: h,
                    });
                }
            }
        }
    }

    pub fn surface_tension_force(&self, idx: usize, sigma: f32) -> Vec3 {
        let (nx, ny, nz) = self.dimensions;
        let mut force = Vec3::ZERO;
        let z = idx / (nx * ny);
        let rem = idx % (nx * ny);
        let y = rem / nx;
        let x = rem % nx;

        if x == 0 || x >= nx - 1 || y == 0 || y >= ny - 1 || z == 0 || z >= nz - 1 {
            return force;
        }

        let f = self.phases[0][idx].volume_fraction;
        if !(0.01..=0.99).contains(&f) {
            return force;
        }

        let h = self.spacing;
        let fx = (self.phases[0][self.index(x + 1, y, z)].volume_fraction
            - self.phases[0][self.index(x - 1, y, z)].volume_fraction)
            / (2.0 * h);
        let fy = (self.phases[0][self.index(x, y + 1, z)].volume_fraction
            - self.phases[0][self.index(x, y - 1, z)].volume_fraction)
            / (2.0 * h);
        let fz = (self.phases[0][self.index(x, y, z + 1)].volume_fraction
            - self.phases[0][self.index(x, y, z - 1)].volume_fraction)
            / (2.0 * h);

        let grad = Vec3::new(fx, fy, fz);
        let grad_mag = grad.length();
        if grad_mag < 1e-6 {
            return force;
        }

        let normal = grad / grad_mag;
        let curvature = self.compute_curvature_at(x, y, z, h);

        let rho_avg = (self.phases[0][idx].density + self.phases[1][idx].density) * 0.5;
        force = sigma * curvature * normal * grad_mag * 2.0 / rho_avg;

        force
    }

    fn compute_curvature_at(&self, x: usize, y: usize, z: usize, h: f32) -> f32 {
        let f = self.phases[0][self.index(x, y, z)].volume_fraction;
        let fxx = (self.phases[0][self.index(x + 1, y, z)].volume_fraction - 2.0 * f
            + self.phases[0][self.index(x - 1, y, z)].volume_fraction)
            / (h * h);
        let fyy = (self.phases[0][self.index(x, y + 1, z)].volume_fraction - 2.0 * f
            + self.phases[0][self.index(x, y - 1, z)].volume_fraction)
            / (h * h);
        let fzz = (self.phases[0][self.index(x, y, z + 1)].volume_fraction - 2.0 * f
            + self.phases[0][self.index(x, y, z - 1)].volume_fraction)
            / (h * h);

        let fx = (self.phases[0][self.index(x + 1, y, z)].volume_fraction
            - self.phases[0][self.index(x - 1, y, z)].volume_fraction)
            / (2.0 * h);
        let fy = (self.phases[0][self.index(x, y + 1, z)].volume_fraction
            - self.phases[0][self.index(x, y - 1, z)].volume_fraction)
            / (2.0 * h);
        let fz = (self.phases[0][self.index(x, y, z + 1)].volume_fraction
            - self.phases[0][self.index(x, y, z - 1)].volume_fraction)
            / (2.0 * h);

        let grad_mag_sq = fx * fx + fy * fy + fz * fz;
        if grad_mag_sq < 1e-12 {
            return 0.0;
        }

        (fxx * (fy * fy + fz * fz) + fyy * (fx * fx + fz * fz) + fzz * (fx * fx + fy * fy)
            - 2.0
                * fx
                * fy
                * (self.phases[0][self.index(x + 1, y + 1, z)].volume_fraction
                    - self.phases[0][self.index(x - 1, y + 1, z)].volume_fraction
                    - self.phases[0][self.index(x + 1, y - 1, z)].volume_fraction
                    + self.phases[0][self.index(x - 1, y - 1, z)].volume_fraction)
                / (4.0 * h * h)
            - 2.0
                * fx
                * fz
                * (self.phases[0][self.index(x + 1, y, z + 1)].volume_fraction
                    - self.phases[0][self.index(x - 1, y, z + 1)].volume_fraction
                    - self.phases[0][self.index(x + 1, y, z - 1)].volume_fraction
                    + self.phases[0][self.index(x - 1, y, z - 1)].volume_fraction)
                / (4.0 * h * h)
            - 2.0
                * fy
                * fz
                * (self.phases[0][self.index(x, y + 1, z + 1)].volume_fraction
                    - self.phases[0][self.index(x, y - 1, z + 1)].volume_fraction
                    - self.phases[0][self.index(x, y + 1, z - 1)].volume_fraction
                    + self.phases[0][self.index(x, y - 1, z - 1)].volume_fraction)
                / (4.0 * h * h))
            / (grad_mag_sq * grad_mag_sq.sqrt())
    }

    pub fn advect_volume_fraction(&mut self, velocity_field: &[Vec3], dt: f32) {
        let (nx, ny, nz) = self.dimensions;
        let mut new_f = vec![0.0f32; self.phases[0].len()];
        let h = self.spacing;

        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = self.index(x, y, z);
                    let vel = velocity_field[idx];
                    let pos = Vec3::new(x as f32, y as f32, z as f32) * h;
                    let prev_pos = pos - vel * dt;
                    let prev_grid = prev_pos / h;

                    let ix = prev_grid.x.floor() as isize;
                    let iy = prev_grid.y.floor() as isize;
                    let iz = prev_grid.z.floor() as isize;

                    let tx = (prev_grid.x - ix as f32).clamp(0.0, 1.0);
                    let ty = (prev_grid.y - iy as f32).clamp(0.0, 1.0);
                    let tz = (prev_grid.z - iz as f32).clamp(0.0, 1.0);

                    let mut f = 0.0f32;
                    for dz in 0..=1 {
                        for dy in 0..=1 {
                            for dx in 0..=1 {
                                let cx = ix + dx as isize;
                                let cy = iy + dy as isize;
                                let cz = iz + dz as isize;
                                if cx < 0
                                    || cx >= nx as isize
                                    || cy < 0
                                    || cy >= ny as isize
                                    || cz < 0
                                    || cz >= nz as isize
                                {
                                    continue;
                                }
                                let c_idx = self.index(cx as usize, cy as usize, cz as usize);
                                let weight = (if dx == 0 { 1.0 - tx } else { tx })
                                    * (if dy == 0 { 1.0 - ty } else { ty })
                                    * (if dz == 0 { 1.0 - tz } else { tz });
                                f += self.phases[0][c_idx].volume_fraction * weight;
                            }
                        }
                    }
                    new_f[idx] = f.clamp(0.0, 1.0);
                }
            }
        }

        self.phases[0].iter_mut().zip(new_f.iter()).for_each(|(cell, &f)| cell.volume_fraction = f);

        let phase0_fractions: Vec<f32> = self.phases[0].iter().map(|c| c.volume_fraction).collect();
        let num_phases_minus_one = (self.num_phases - 1) as f32;
        for i in 1..self.num_phases {
            for (idx, cell) in self.phases[i].iter_mut().enumerate() {
                cell.volume_fraction = (1.0 - phase0_fractions[idx]) / num_phases_minus_one;
            }
        }
    }

    pub fn mixture_density_at(&self, idx: usize) -> f32 {
        self.phases.iter().map(|p| p[idx].volume_fraction * p[idx].density).sum()
    }

    pub fn mixture_viscosity_at(&self, idx: usize) -> f32 {
        self.phases.iter().map(|p| p[idx].volume_fraction * p[idx].viscosity).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurbulenceModel {
    pub enabled: bool,
    pub smagorinsky_constant: f32,
    pub filter_width: f32,
    pub turbulent_viscosity: Vec<f32>,
    pub dimensions: (usize, usize, usize),
}

impl TurbulenceModel {
    pub fn new(dimensions: (usize, usize, usize), enabled: bool) -> Self {
        let (nx, ny, nz) = dimensions;
        Self {
            enabled,
            smagorinsky_constant: 0.17,
            filter_width: 1.0,
            turbulent_viscosity: vec![0.0; nx * ny * nz],
            dimensions,
        }
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        let (nx, ny, _) = self.dimensions;
        x + y * nx + z * nx * ny
    }

    pub fn compute_eddy_viscosity(&mut self, velocity_field: &[Vec3], spacing: f32) {
        if !self.enabled {
            return;
        }

        let (nx, ny, nz) = self.dimensions;
        let delta = self.filter_width * spacing;
        let cs = self.smagorinsky_constant;
        let delta_sq = (cs * delta) * (cs * delta);

        for z in 1..nz - 1 {
            for y in 1..ny - 1 {
                for x in 1..nx - 1 {
                    let idx = self.index(x, y, z);
                    let h = 2.0 * spacing;

                    let du_dx = (velocity_field[self.index(x + 1, y, z)].x
                        - velocity_field[self.index(x - 1, y, z)].x)
                        / h;
                    let du_dy = (velocity_field[self.index(x, y + 1, z)].x
                        - velocity_field[self.index(x, y - 1, z)].x)
                        / h;
                    let du_dz = (velocity_field[self.index(x, y, z + 1)].x
                        - velocity_field[self.index(x, y, z - 1)].x)
                        / h;

                    let dv_dx = (velocity_field[self.index(x + 1, y, z)].y
                        - velocity_field[self.index(x - 1, y, z)].y)
                        / h;
                    let dv_dy = (velocity_field[self.index(x, y + 1, z)].y
                        - velocity_field[self.index(x, y - 1, z)].y)
                        / h;
                    let dv_dz = (velocity_field[self.index(x, y, z + 1)].y
                        - velocity_field[self.index(x, y, z - 1)].y)
                        / h;

                    let dw_dx = (velocity_field[self.index(x + 1, y, z)].z
                        - velocity_field[self.index(x - 1, y, z)].z)
                        / h;
                    let dw_dy = (velocity_field[self.index(x, y + 1, z)].z
                        - velocity_field[self.index(x, y - 1, z)].z)
                        / h;
                    let dw_dz = (velocity_field[self.index(x, y, z + 1)].z
                        - velocity_field[self.index(x, y, z - 1)].z)
                        / h;

                    let s11 = du_dx;
                    let s22 = dv_dy;
                    let s33 = dw_dz;
                    let s12 = 0.5 * (du_dy + dv_dx);
                    let s13 = 0.5 * (du_dz + dw_dx);
                    let s23 = 0.5 * (dv_dz + dw_dy);

                    let s_mag = (2.0
                        * (s11 * s11
                            + s22 * s22
                            + s33 * s33
                            + 2.0 * (s12 * s12 + s13 * s13 + s23 * s23)))
                        .sqrt();

                    self.turbulent_viscosity[idx] = delta_sq * s_mag;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiphase_solver_creation() {
        let solver = MultiPhaseSolver::new((32, 32, 32), 0.1, 2);
        assert_eq!(solver.phases.len(), 2);
        assert_eq!(solver.phases[0].len(), 32 * 32 * 32);
    }

    #[test]
    fn test_set_phase_region() {
        let mut solver = MultiPhaseSolver::new((32, 32, 32), 0.1, 2);
        let water = PhaseCell {
            volume_fraction: 1.0,
            phase: Phase::Liquid,
            density: 1000.0,
            viscosity: 0.001,
            surface_tension_coeff: 0.07,
        };
        solver.set_phase_region(0, (0, 0, 0), (16, 32, 32), water);
        let idx = solver.index(8, 16, 16);
        assert_eq!(solver.phases[0][idx].volume_fraction, 1.0);
        assert_eq!(solver.phases[0][idx].phase, Phase::Liquid);
    }

    #[test]
    fn test_interface_normals() {
        let mut solver = MultiPhaseSolver::new((32, 32, 32), 0.1, 2);
        let water = PhaseCell {
            volume_fraction: 1.0,
            phase: Phase::Liquid,
            density: 1000.0,
            viscosity: 0.001,
            surface_tension_coeff: 0.07,
        };
        let _air = PhaseCell {
            volume_fraction: 1.0,
            phase: Phase::Gas,
            density: 1.225,
            viscosity: 1.8e-5,
            surface_tension_coeff: 0.0,
        };
        solver.set_phase_region(0, (0, 0, 0), (10, 32, 32), water);
        let update_indices: Vec<(usize, f32)> = {
            let mut v = Vec::new();
            for z in 0..32usize {
                for y in 0..32usize {
                    for x in 10..22usize {
                        let t = (x - 10) as f32 / 12.0;
                        v.push((solver.index(x, y, z), 1.0 - t));
                    }
                }
            }
            v
        };
        for (idx, frac) in &update_indices {
            solver.phases[0][*idx].volume_fraction = *frac;
        }
        solver.compute_interface_normals();
        assert!(!solver.interfaces.is_empty());
    }

    #[test]
    fn test_surface_tension_force() {
        let mut solver = MultiPhaseSolver::new((32, 32, 32), 0.1, 2);
        let water = PhaseCell {
            volume_fraction: 1.0,
            phase: Phase::Liquid,
            density: 1000.0,
            viscosity: 0.001,
            surface_tension_coeff: 0.07,
        };
        let air = PhaseCell {
            volume_fraction: 1.0,
            phase: Phase::Gas,
            density: 1.225,
            viscosity: 1.8e-5,
            surface_tension_coeff: 0.0,
        };
        solver.set_phase_region(0, (0, 0, 0), (16, 32, 32), water);
        solver.set_phase_region(1, (16, 0, 0), (16, 32, 32), air);
        let idx = solver.index(16, 16, 16);
        let force = solver.surface_tension_force(idx, 0.07);
        assert!(force.length() >= 0.0);
    }

    #[test]
    fn test_turbulence_model() {
        let (nx, ny, nz) = (32, 32, 32);
        let mut model = TurbulenceModel::new((nx, ny, nz), true);
        let velocity: Vec<Vec3> = (0..nx * ny * nz)
            .map(|i| {
                let z = i / (nx * ny);
                let y = (i % (nx * ny)) / nx;
                let x = i % nx;
                Vec3::new((y as f32 * 0.1).sin(), (x as f32 * 0.1).cos(), (z as f32 * 0.05).sin())
            })
            .collect();
        model.compute_eddy_viscosity(&velocity, 0.1);
        let mid = model.index(16, 16, 16);
        assert!(model.turbulent_viscosity[mid] >= 0.0);
    }
}
