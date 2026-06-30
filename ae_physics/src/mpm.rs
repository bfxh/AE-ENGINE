use serde::{Deserialize, Serialize};

use crate::fixed_point::{FixedMat3, FixedPoint, FixedVec3};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MpmMaterialModel {
    Elastic,
    ElastoPlastic,
    Granular { friction_angle: FixedPoint },
    Brittle { fracture_strain: FixedPoint },
    Fluid { viscosity: FixedPoint },
    Snow { hardening: FixedPoint },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpmConfig {
    pub grid_resolution: [u32; 3],
    pub cell_size: FixedPoint,
    pub particle_count: usize,
    pub youngs_modulus: FixedPoint,
    pub poissons_ratio: FixedPoint,
    pub yield_stress: FixedPoint,
    pub hardening: FixedPoint,
    pub density: FixedPoint,
    pub gravity: FixedVec3,
    pub dt: FixedPoint,
    pub substeps: u32,
    pub material_model: MpmMaterialModel,
    pub enable_fracture: bool,
    pub fracture_strain: FixedPoint,
}

impl Default for MpmConfig {
    fn default() -> Self {
        Self {
            grid_resolution: [64, 64, 64],
            cell_size: FixedPoint::from_f32(0.1),
            particle_count: 10000,
            youngs_modulus: FixedPoint::from_f32(1e6),
            poissons_ratio: FixedPoint::from_f32(0.3),
            yield_stress: FixedPoint::from_f32(1e4),
            hardening: FixedPoint::from_f32(0.1),
            density: FixedPoint::from_f32(1000.0),
            gravity: FixedVec3::from_f32(0.0, -9.81, 0.0),
            dt: FixedPoint::from_f32(1.0 / 240.0),
            substeps: 4,
            material_model: MpmMaterialModel::ElastoPlastic,
            enable_fracture: false,
            fracture_strain: FixedPoint::from_f32(0.15),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MpmParticle {
    pub position: FixedVec3,
    pub velocity: FixedVec3,
    pub deformation_gradient: FixedMat3,
    pub mass: FixedPoint,
    pub volume: FixedPoint,
    pub stress: FixedMat3,
    pub plastic_strain: FixedPoint,
    pub temperature: FixedPoint,
    pub active: bool,
    pub color: [f32; 4],
    pub accumulated_strain: FixedPoint,
    pub fractured: bool,
    pub parent_id: u64,
    pub connected_to: [u64; 8],
    pub num_connections: u8,
}

impl Default for MpmParticle {
    fn default() -> Self {
        Self {
            position: FixedVec3::ZERO,
            velocity: FixedVec3::ZERO,
            deformation_gradient: FixedMat3::IDENTITY,
            mass: FixedPoint::ONE,
            volume: FixedPoint::ONE,
            stress: FixedMat3::ZERO,
            plastic_strain: FixedPoint::ZERO,
            temperature: FixedPoint::from_f32(293.0),
            active: true,
            color: [0.8, 0.6, 0.4, 1.0],
            accumulated_strain: FixedPoint::ZERO,
            fractured: false,
            parent_id: 0,
            connected_to: [0; 8],
            num_connections: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MpmParticleSnapshot {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub stress_magnitude: f32,
    pub plastic_strain: f32,
    pub temperature: f32,
    pub color: [f32; 4],
    pub fractured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MpmGridCell {
    mass: FixedPoint,
    velocity: FixedVec3,
    active: bool,
}

impl Default for MpmGridCell {
    fn default() -> Self {
        Self { mass: FixedPoint::ZERO, velocity: FixedVec3::ZERO, active: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpmSimulation {
    pub config: MpmConfig,
    pub particles: Vec<MpmParticle>,
    pub time: f64,
    pub step_count: u64,
    grid: Vec<MpmGridCell>,
    grid_size: [usize; 3],
    next_particle_id: u64,
    pub fracture_events: Vec<FractureEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FractureEvent {
    pub time: f64,
    pub position: [f32; 3],
    pub strain_magnitude: f32,
    pub num_fragments: u32,
}

const FP_HALF: FixedPoint = FixedPoint::from_raw(1i64 << 31);
const FP_ONE_POINT_FIVE: FixedPoint = FixedPoint::from_raw(6442450944);
const FP_THREE_QUARTERS: FixedPoint = FixedPoint::from_raw(3221225472);
const FP_TWO: FixedPoint = FixedPoint::from_raw(8589934592);
const FP_THREE: FixedPoint = FixedPoint::from_raw(12884901888);
const FP_EPS: FixedPoint = FixedPoint::from_raw(10);

fn bspline_weight(f: FixedPoint) -> FixedPoint {
    let abs_f = f.abs();
    let x = FP_ONE_POINT_FIVE - abs_f;
    if abs_f < FP_HALF {
        FP_THREE_QUARTERS - f * f
    } else if abs_f < FP_ONE_POINT_FIVE {
        FP_HALF * x * x
    } else {
        FixedPoint::ZERO
    }
}

impl MpmSimulation {
    pub fn new(config: MpmConfig) -> Self {
        let grid_size = config.grid_resolution.map(|v| v as usize);
        let total_cells = grid_size[0] * grid_size[1] * grid_size[2];
        let grid = vec![MpmGridCell::default(); total_cells];
        let particle_count = config.particle_count;

        let mut sim = Self {
            config,
            particles: Vec::with_capacity(particle_count),
            time: 0.0,
            step_count: 0,
            grid,
            grid_size,
            next_particle_id: 1,
            fracture_events: Vec::new(),
        };
        sim.init_particles();
        sim
    }

    fn init_particles(&mut self) {
        if self.config.particle_count == 0 {
            return;
        }
        let res = self.config.grid_resolution.map(|v| FixedPoint::from_i32(v as i32));
        let cell = self.config.cell_size;
        let particles_per_dim = (self.config.particle_count as f32).cbrt().ceil() as u32;
        let spacing = cell / FixedPoint::from_i32(particles_per_dim as i32);

        for i in 0..particles_per_dim {
            for j in 0..particles_per_dim {
                for k in 0..particles_per_dim {
                    let x = (FixedPoint::from_i32(i as i32) + FP_HALF) * spacing;
                    let y = (FixedPoint::from_i32(j as i32) + FP_HALF) * spacing;
                    let z = (FixedPoint::from_i32(k as i32) + FP_HALF) * spacing;
                    let position =
                        FixedVec3::new(x, y + res[1] * cell * FixedPoint::from_f32(0.3), z);

                    let vol = spacing * spacing * spacing;
                    let particle = MpmParticle {
                        position,
                        mass: self.config.density * vol,
                        volume: vol,
                        parent_id: self.next_particle_id,
                        ..Default::default()
                    };
                    self.next_particle_id += 1;
                    self.particles.push(particle);
                }
            }
        }
    }

    pub fn add_particle(
        &mut self,
        position: FixedVec3,
        velocity: FixedVec3,
        mass: FixedPoint,
        color: [f32; 4],
    ) -> u64 {
        let id = self.next_particle_id;
        self.next_particle_id += 1;

        let particle = MpmParticle {
            position,
            velocity,
            mass,
            volume: mass / self.config.density,
            parent_id: id,
            color,
            ..Default::default()
        };
        self.particles.push(particle);
        id
    }

    pub fn remove_particle(&mut self, id: u64) -> bool {
        if let Some(pos) = self.particles.iter().position(|p| p.parent_id == id) {
            self.particles.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn step(&mut self) {
        let sub_dt = self.config.dt / FixedPoint::from_i32(self.config.substeps as i32);
        for _ in 0..self.config.substeps {
            self.substep(sub_dt);
        }
        self.time += self.config.dt.to_f64();
        self.step_count += 1;
    }

    fn substep(&mut self, dt: FixedPoint) {
        self.reset_grid();
        self.particles_to_grid();
        self.compute_grid_forces(dt);
        self.grid_to_particles(dt);
        self.update_deformation(dt);
        self.handle_constitutive_model();
        if self.config.enable_fracture {
            self.handle_fracture();
        }
    }

    fn grid_index(&self, x: i32, y: i32, z: i32) -> usize {
        Self::grid_index_static(x, y, z, self.grid_size)
    }

    fn grid_index_static(x: i32, y: i32, z: i32, grid_size: [usize; 3]) -> usize {
        let sx = grid_size[0] as i32;
        let sy = grid_size[1] as i32;
        if x < 0 || x >= sx || y < 0 || y >= sy || z < 0 || z >= grid_size[2] as i32 {
            return usize::MAX;
        }
        (x as usize) + (y as usize) * grid_size[0] + (z as usize) * grid_size[0] * grid_size[1]
    }

    fn reset_grid(&mut self) {
        for cell in &mut self.grid {
            cell.mass = FixedPoint::ZERO;
            cell.velocity = FixedVec3::ZERO;
            cell.active = false;
        }
    }

    fn particles_to_grid(&mut self) {
        let cell = self.config.cell_size;
        let inv_cell = FixedPoint::ONE / cell;

        for p in &self.particles {
            if !p.active {
                continue;
            }

            let base = (p.position * inv_cell).floor();
            let fx = p.position * inv_cell - base;

            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let gx = base.x.to_f32() as i32 + dx;
                        let gy = base.y.to_f32() as i32 + dy;
                        let gz = base.z.to_f32() as i32 + dz;
                        let idx = self.grid_index(gx, gy, gz);
                        if idx == usize::MAX {
                            continue;
                        }

                        let weight = bspline_weight(fx.x - FixedPoint::from_i32(dx))
                            * bspline_weight(fx.y - FixedPoint::from_i32(dy))
                            * bspline_weight(fx.z - FixedPoint::from_i32(dz));
                        if weight < FP_EPS {
                            continue;
                        }

                        let grid_pos = FixedVec3::new(
                            FixedPoint::from_i32(gx) * cell,
                            FixedPoint::from_i32(gy) * cell,
                            FixedPoint::from_i32(gz) * cell,
                        );
                        let rel_pos = grid_pos - p.position;
                        let stress_times_rel = p.stress * rel_pos;
                        let momentum =
                            p.mass * (p.velocity + stress_times_rel * inv_cell * inv_cell);

                        self.grid[idx].mass += p.mass * weight;
                        self.grid[idx].velocity += momentum * weight;
                        self.grid[idx].active = true;
                    }
                }
            }
        }
    }

    fn compute_grid_forces(&mut self, dt: FixedPoint) {
        for i in 0..self.grid.len() {
            let mass = self.grid[i].mass;
            if !self.grid[i].active || mass < FP_EPS {
                continue;
            }
            let force = self.config.gravity * dt * mass;
            self.grid[i].velocity = self.grid[i].velocity.saturating_add(force);
        }
    }

    fn grid_to_particles(&mut self, dt: FixedPoint) {
        let cell = self.config.cell_size;
        let inv_cell = FixedPoint::ONE / cell;
        let grid_size = self.grid_size;

        for p in &mut self.particles {
            if !p.active {
                continue;
            }

            let base = (p.position * inv_cell).floor();
            let fx = p.position * inv_cell - base;

            let mut new_velocity = FixedVec3::ZERO;
            let mut velocity_gradient = FixedMat3::ZERO;

            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let gx = base.x.to_f32() as i32 + dx;
                        let gy = base.y.to_f32() as i32 + dy;
                        let gz = base.z.to_f32() as i32 + dz;
                        let idx = Self::grid_index_static(gx, gy, gz, grid_size);
                        if idx == usize::MAX {
                            continue;
                        }

                        let weight = bspline_weight(fx.x - FixedPoint::from_i32(dx))
                            * bspline_weight(fx.y - FixedPoint::from_i32(dy))
                            * bspline_weight(fx.z - FixedPoint::from_i32(dz));
                        if weight < FP_EPS || !self.grid[idx].active || self.grid[idx].mass < FP_EPS
                        {
                            continue;
                        }

                        let grid_vel = self.grid[idx].velocity / self.grid[idx].mass;
                        new_velocity = new_velocity.saturating_add(grid_vel * weight);

                        let grad_dir = FixedVec3::new(
                            FixedPoint::from_i32(dx) - fx.x,
                            FixedPoint::from_i32(dy) - fx.y,
                            FixedPoint::from_i32(dz) - fx.z,
                        );
                        for a in 0..3 {
                            let gv_a = match a {
                                0 => grid_vel.x,
                                1 => grid_vel.y,
                                _ => grid_vel.z,
                            };
                            let mut col = *velocity_gradient.col(a);
                            for b in 0..3 {
                                let gd_b = match b {
                                    0 => grad_dir.x,
                                    1 => grad_dir.y,
                                    _ => grad_dir.z,
                                };
                                let contrib = gv_a * weight * gd_b * inv_cell;
                                match b {
                                    0 => col.x += contrib,
                                    1 => col.y += contrib,
                                    _ => col.z += contrib,
                                }
                            }
                            *velocity_gradient.col_mut(a) = col;
                        }
                    }
                }
            }

            p.velocity = new_velocity;
            p.position = p.position.saturating_add(new_velocity * dt);

            let mut new_f = p.deformation_gradient;
            new_f = new_f.saturating_add(velocity_gradient * new_f * dt);
            p.deformation_gradient = new_f;

            let det = velocity_gradient.determinant();
            let strain =
                if det > FixedPoint::ONE { det - FixedPoint::ONE } else { FixedPoint::ONE - det };
            p.accumulated_strain = p.accumulated_strain.saturating_add(strain);
        }
    }

    fn update_deformation(&mut self, _dt: FixedPoint) {
        let two = FP_TWO;
        let mu =
            self.config.youngs_modulus / (two * (FixedPoint::ONE + self.config.poissons_ratio));
        let lambda = self.config.youngs_modulus * self.config.poissons_ratio
            / ((FixedPoint::ONE + self.config.poissons_ratio)
                * (FixedPoint::ONE - two * self.config.poissons_ratio));

        for p in &mut self.particles {
            if !p.active {
                continue;
            }

            let f = p.deformation_gradient;
            let j = f.determinant();
            if j.abs() < FP_EPS {
                continue;
            }

            let f_inv_t = f.inverse().transpose();
            let j_clamped = j.clamp(FixedPoint::from_f32(0.01), FixedPoint::from_f32(100.0));
            let p_kirchhoff = mu * (f - f_inv_t) + lambda * j_clamped.ln() * f_inv_t;

            p.stress = p_kirchhoff * f.transpose() / j_clamped;
        }
    }

    fn handle_constitutive_model(&mut self) {
        match self.config.material_model {
            MpmMaterialModel::Elastic => {},
            MpmMaterialModel::ElastoPlastic => self.handle_plasticity(),
            MpmMaterialModel::Granular { friction_angle } => self.handle_granular(friction_angle),
            MpmMaterialModel::Brittle { fracture_strain } => {
                self.handle_plasticity();
                self.config.fracture_strain = fracture_strain;
                self.config.enable_fracture = true;
            },
            MpmMaterialModel::Fluid { viscosity } => self.handle_fluid(viscosity),
            MpmMaterialModel::Snow { hardening } => {
                self.config.hardening = hardening;
                self.handle_snow_plasticity();
            },
        }
    }

    fn handle_plasticity(&mut self) {
        let yield_stress = self.config.yield_stress;
        let hardening = self.config.hardening;

        for p in &mut self.particles {
            if !p.active {
                continue;
            }

            let trace = p.stress.x_axis.x + p.stress.y_axis.y + p.stress.z_axis.z;
            let dev_stress = p.stress - FixedMat3::IDENTITY * (trace / FP_THREE);
            let dev_norm = (dev_stress.x_axis.x * dev_stress.x_axis.x
                + dev_stress.y_axis.y * dev_stress.y_axis.y
                + dev_stress.z_axis.z * dev_stress.z_axis.z
                + FP_TWO
                    * (dev_stress.x_axis.y * dev_stress.x_axis.y
                        + dev_stress.x_axis.z * dev_stress.x_axis.z
                        + dev_stress.y_axis.z * dev_stress.y_axis.z))
                .sqrt();
            let von_mises = (FP_ONE_POINT_FIVE * dev_norm).sqrt();

            let plastic_limit = yield_stress + hardening * p.plastic_strain;
            if von_mises > plastic_limit {
                let scale = plastic_limit / von_mises;
                p.stress = FixedMat3::IDENTITY * (trace / FP_THREE) + dev_stress * scale;
                p.plastic_strain += (von_mises - plastic_limit) / self.config.youngs_modulus;
            }
        }
    }

    fn handle_granular(&mut self, friction_angle: FixedPoint) {
        let mu_s = friction_angle.to_f32().to_radians().tan();
        let mu_s_fp = FixedPoint::from_f32(mu_s);
        let yield_stress = self.config.yield_stress;

        for p in &mut self.particles {
            if !p.active {
                continue;
            }

            let trace = p.stress.x_axis.x + p.stress.y_axis.y + p.stress.z_axis.z;
            let pressure = -trace / FP_THREE;

            if pressure < FixedPoint::ZERO {
                p.stress = FixedMat3::ZERO;
                continue;
            }

            let dev_stress = p.stress - FixedMat3::IDENTITY * (trace / FP_THREE);
            let dev_norm = (dev_stress.x_axis.x * dev_stress.x_axis.x
                + dev_stress.y_axis.y * dev_stress.y_axis.y
                + dev_stress.z_axis.z * dev_stress.z_axis.z
                + FP_TWO
                    * (dev_stress.x_axis.y * dev_stress.x_axis.y
                        + dev_stress.x_axis.z * dev_stress.x_axis.z
                        + dev_stress.y_axis.z * dev_stress.y_axis.z))
                .sqrt();

            let max_shear = mu_s_fp * pressure + yield_stress;

            if dev_norm > max_shear {
                let scale = max_shear / dev_norm;
                p.stress = FixedMat3::IDENTITY * (trace / FP_THREE) + dev_stress * scale;
                p.plastic_strain += (dev_norm - max_shear) / self.config.youngs_modulus;
            }
        }
    }

    fn handle_fluid(&mut self, viscosity: FixedPoint) {
        for p in &mut self.particles {
            if !p.active {
                continue;
            }

            let trace = p.stress.x_axis.x + p.stress.y_axis.y + p.stress.z_axis.z;
            let pressure = -trace / FP_THREE;

            if pressure < FixedPoint::ZERO {
                p.stress = FixedMat3::ZERO;
            } else {
                p.stress = -FixedMat3::IDENTITY * pressure;
            }

            let vel_mag = p.velocity.length();
            if vel_mag > FixedPoint::ZERO {
                let damping = (-viscosity * vel_mag).exp();
                p.velocity *= damping;
            }
        }
    }

    fn handle_snow_plasticity(&mut self) {
        let hardening = self.config.hardening;
        let mu =
            self.config.youngs_modulus / (FP_TWO * (FixedPoint::ONE + self.config.poissons_ratio));
        let lambda = self.config.youngs_modulus * self.config.poissons_ratio
            / ((FixedPoint::ONE + self.config.poissons_ratio)
                * (FixedPoint::ONE - FP_TWO * self.config.poissons_ratio));

        let critical_compression = FixedPoint::ONE - FixedPoint::from_f32(2.5e-2);
        let critical_stretch = FixedPoint::ONE + FixedPoint::from_f32(7.5e-3);

        for p in &mut self.particles {
            if !p.active {
                continue;
            }

            let f = p.deformation_gradient;
            let (u, mut s, v) = fixed_mat3_svd(f);

            for i in 0..3 {
                let si = s.col(i).to_owned();
                let si_val = match i {
                    0 => si.x,
                    1 => si.y,
                    _ => si.z,
                };
                if si_val < critical_compression {
                    match i {
                        0 => s.x_axis.x = critical_compression,
                        1 => s.y_axis.y = critical_compression,
                        _ => s.z_axis.z = critical_compression,
                    }
                } else if si_val > critical_stretch {
                    match i {
                        0 => s.x_axis.x = critical_stretch,
                        1 => s.y_axis.y = critical_stretch,
                        _ => s.z_axis.z = critical_stretch,
                    }
                }
            }

            p.deformation_gradient = u * s * v.transpose();

            let f = p.deformation_gradient;
            let j = f.determinant();
            let j_clamped = j.clamp(FixedPoint::from_f32(0.01), FixedPoint::from_f32(100.0));

            let f_inv_t = f.inverse().transpose();
            let p_kirchhoff = mu * (f - f_inv_t) + lambda * j_clamped.ln() * f_inv_t;

            p.stress = p_kirchhoff * f.transpose() / j_clamped;

            let trace = p.stress.x_axis.x.saturating_add(p.stress.y_axis.y).saturating_add(p.stress.z_axis.z);
            let hydrostatic_trace = trace.saturating_div(FP_THREE);
            let dev_stress = p.stress.saturating_sub(FixedMat3::IDENTITY * hydrostatic_trace);
            let dev_norm_sq = dev_stress.x_axis.x.saturating_mul(dev_stress.x_axis.x)
                .saturating_add(dev_stress.y_axis.y.saturating_mul(dev_stress.y_axis.y))
                .saturating_add(dev_stress.z_axis.z.saturating_mul(dev_stress.z_axis.z))
                .saturating_add(FP_TWO.saturating_mul(
                    dev_stress.x_axis.y.saturating_mul(dev_stress.x_axis.y)
                    .saturating_add(dev_stress.x_axis.z.saturating_mul(dev_stress.x_axis.z))
                    .saturating_add(dev_stress.y_axis.z.saturating_mul(dev_stress.y_axis.z))
                ));
            let dev_norm = dev_norm_sq.sqrt();
            let von_mises = FP_ONE_POINT_FIVE.saturating_mul(dev_norm).sqrt();

            if von_mises > hardening {
                let scale = hardening.saturating_div(von_mises);
                let hydrostatic = FixedMat3::IDENTITY * hydrostatic_trace;
                let dev_scaled = dev_stress.saturating_mul(scale);
                p.stress = hydrostatic.saturating_add(dev_scaled);
                p.plastic_strain = p.plastic_strain.saturating_add((von_mises - hardening).saturating_div(self.config.youngs_modulus));
            }
        }
    }

    fn handle_fracture(&mut self) {
        let threshold = self.config.fracture_strain;
        let num_particles = self.particles.len();

        for i in 0..num_particles {
            if !self.particles[i].active || self.particles[i].fractured {
                continue;
            }

            let principal_strain = {
                let f = self.particles[i].deformation_gradient;
                let green = f.transpose() * f;
                let trace = green.x_axis.x + green.y_axis.y + green.z_axis.z;
                trace / FP_THREE
            };

            let strain = if principal_strain > FixedPoint::ONE {
                principal_strain - FixedPoint::ONE
            } else {
                FixedPoint::ONE - principal_strain
            };

            if strain > threshold {
                let pos = self.particles[i].position;
                let mass = self.particles[i].mass;
                let vel = self.particles[i].velocity;
                let color = self.particles[i].color;
                let parent = self.particles[i].parent_id;

                self.particles[i].fractured = true;

                let strain_ratio = strain / threshold;
                let num_fragments: u32 = strain_ratio.to_f32().min(8.0) as u32 + 1;
                let num_fragments_fp = FixedPoint::from_i32(num_fragments as i32 + 1);

                for fi in 0..num_fragments {
                    let angle = fi as f32 * std::f32::consts::TAU / num_fragments as f32;
                    let cell_half = self.config.cell_size * FixedPoint::from_f32(0.5);
                    let offset = FixedVec3::new(
                        FixedPoint::from_f32(angle.cos()) * cell_half,
                        FixedPoint::from_f32(angle.sin())
                            * self.config.cell_size
                            * FixedPoint::from_f32(0.2),
                        FixedPoint::from_f32((fi as f32 * 1.7).sin()) * cell_half,
                    );

                    let mut frag = MpmParticle {
                        position: pos + offset,
                        velocity: vel + offset * FP_TWO,
                        mass: mass / num_fragments_fp,
                        volume: self.particles[i].volume / num_fragments_fp,
                        color,
                        parent_id: self.next_particle_id,
                        ..Default::default()
                    };
                    self.next_particle_id += 1;

                    for c in 0..(num_fragments.min(7) as u8) {
                        frag.connected_to[c as usize] = parent + c as u64 + 1;
                    }
                    frag.num_connections = num_fragments.min(8) as u8;

                    self.particles.push(frag);
                }

                self.particles[i].mass /= num_fragments_fp;

                self.fracture_events.push(FractureEvent {
                    time: self.time,
                    position: pos.to_glam().to_array(),
                    strain_magnitude: strain.to_f32(),
                    num_fragments,
                });
            }
        }
    }

    pub fn snapshots(&self) -> Vec<MpmParticleSnapshot> {
        self.particles
            .iter()
            .filter(|p| p.active)
            .map(|p| {
                let stress_mag = (p.stress.x_axis.x * p.stress.x_axis.x
                    + p.stress.y_axis.y * p.stress.y_axis.y
                    + p.stress.z_axis.z * p.stress.z_axis.z
                    + FP_TWO
                        * (p.stress.x_axis.y * p.stress.x_axis.y
                            + p.stress.x_axis.z * p.stress.x_axis.z
                            + p.stress.y_axis.z * p.stress.y_axis.z))
                    .sqrt();
                MpmParticleSnapshot {
                    position: p.position.to_glam().to_array(),
                    velocity: p.velocity.to_glam().to_array(),
                    stress_magnitude: stress_mag.to_f32(),
                    plastic_strain: p.plastic_strain.to_f32(),
                    temperature: p.temperature.to_f32(),
                    color: p.color,
                    fractured: p.fractured,
                }
            })
            .collect()
    }

    pub fn active_particles(&self) -> usize {
        self.particles.iter().filter(|p| p.active).count()
    }

    pub fn fractured_count(&self) -> usize {
        self.particles.iter().filter(|p| p.fractured).count()
    }

    pub fn total_mass(&self) -> FixedPoint {
        self.particles.iter().filter(|p| p.active).fold(FixedPoint::ZERO, |acc, p| acc + p.mass)
    }

    pub fn average_temperature(&self) -> FixedPoint {
        let active: Vec<&MpmParticle> = self.particles.iter().filter(|p| p.active).collect();
        let count = active.len();
        if count == 0 {
            return FixedPoint::from_f32(293.0);
        }
        let sum: FixedPoint = active.iter().fold(FixedPoint::ZERO, |acc, p| acc + p.temperature);
        sum / FixedPoint::from_i32(count as i32)
    }

    pub fn kinetic_energy(&self) -> FixedPoint {
        self.particles
            .iter()
            .filter(|p| p.active)
            .fold(FixedPoint::ZERO, |acc, p| acc + FP_HALF * p.mass * p.velocity.length_squared())
    }

    pub fn total_strain_energy(&self) -> FixedPoint {
        self.particles
            .iter()
            .filter(|p| p.active)
            .fold(FixedPoint::ZERO, |acc, p| acc + p.accumulated_strain)
    }

    pub fn set_material_model(&mut self, model: MpmMaterialModel) {
        self.config.material_model = model;
    }

    pub fn enable_fracture(&mut self, threshold: FixedPoint) {
        self.config.enable_fracture = true;
        self.config.fracture_strain = threshold;
    }

    pub fn disable_fracture(&mut self) {
        self.config.enable_fracture = false;
    }
}

fn fixed_mat3_svd(m: FixedMat3) -> (FixedMat3, FixedMat3, FixedMat3) {
    let a = [
        [m.x_axis.x, m.y_axis.x, m.z_axis.x],
        [m.x_axis.y, m.y_axis.y, m.z_axis.y],
        [m.x_axis.z, m.y_axis.z, m.z_axis.z],
    ];

    let (u, s, vt) = fixed_svd_3x3(a);

    let u_mat = FixedMat3::from_cols(
        FixedVec3::new(u[0][0], u[1][0], u[2][0]),
        FixedVec3::new(u[0][1], u[1][1], u[2][1]),
        FixedVec3::new(u[0][2], u[1][2], u[2][2]),
    );

    let s_mat = FixedMat3::from_cols(
        FixedVec3::new(s[0], FixedPoint::ZERO, FixedPoint::ZERO),
        FixedVec3::new(FixedPoint::ZERO, s[1], FixedPoint::ZERO),
        FixedVec3::new(FixedPoint::ZERO, FixedPoint::ZERO, s[2]),
    );

    let v = FixedMat3::from_cols(
        FixedVec3::new(vt[0][0], vt[0][1], vt[0][2]),
        FixedVec3::new(vt[1][0], vt[1][1], vt[1][2]),
        FixedVec3::new(vt[2][0], vt[2][1], vt[2][2]),
    );

    (u_mat, s_mat, v)
}

fn fixed_svd_3x3(
    a: [[FixedPoint; 3]; 3],
) -> ([[FixedPoint; 3]; 3], [FixedPoint; 3], [[FixedPoint; 3]; 3]) {
    let mut u = [[FixedPoint::ZERO; 3]; 3];
    let mut s = [FixedPoint::ZERO; 3];
    let mut v = [[FixedPoint::ZERO; 3]; 3];

    let ata = [
        [
            a[0][0] * a[0][0] + a[1][0] * a[1][0] + a[2][0] * a[2][0],
            a[0][0] * a[0][1] + a[1][0] * a[1][1] + a[2][0] * a[2][1],
            a[0][0] * a[0][2] + a[1][0] * a[1][2] + a[2][0] * a[2][2],
        ],
        [
            a[0][1] * a[0][0] + a[1][1] * a[1][0] + a[2][1] * a[2][0],
            a[0][1] * a[0][1] + a[1][1] * a[1][1] + a[2][1] * a[2][1],
            a[0][1] * a[0][2] + a[1][1] * a[1][2] + a[2][1] * a[2][2],
        ],
        [
            a[0][2] * a[0][0] + a[1][2] * a[1][0] + a[2][2] * a[2][0],
            a[0][2] * a[0][1] + a[1][2] * a[1][1] + a[2][2] * a[2][1],
            a[0][2] * a[0][2] + a[1][2] * a[1][2] + a[2][2] * a[2][2],
        ],
    ];

    let (eigenvectors, eigenvalues) = fixed_symmetric_eigen_3x3(ata);

    for i in 0..3 {
        s[i] = if eigenvalues[i] > FixedPoint::ZERO {
            eigenvalues[i].sqrt()
        } else {
            FixedPoint::ZERO
        };
    }

    for i in 0..3 {
        for j in 0..3 {
            v[i][j] = eigenvectors[j][i];
        }
    }

    for j in 0..3 {
        let si = if s[j] > FP_EPS { FixedPoint::ONE / s[j] } else { FixedPoint::ZERO };
        for i in 0..3 {
            let mut sum = FixedPoint::ZERO;
            for k in 0..3 {
                sum += a[i][k] * v[k][j];
            }
            u[i][j] = sum * si;
        }
    }

    let det_u = u[0][0] * (u[1][1] * u[2][2] - u[1][2] * u[2][1])
        - u[0][1] * (u[1][0] * u[2][2] - u[1][2] * u[2][0])
        + u[0][2] * (u[1][0] * u[2][1] - u[1][1] * u[2][0]);

    if det_u < FixedPoint::ZERO {
        for row in &mut u {
            row[2] = -row[2];
        }
        s[2] = -s[2];
    }

    (u, s, v)
}

fn fixed_symmetric_eigen_3x3(
    mut a: [[FixedPoint; 3]; 3],
) -> ([[FixedPoint; 3]; 3], [FixedPoint; 3]) {
    let mut v = [
        [FixedPoint::ONE, FixedPoint::ZERO, FixedPoint::ZERO],
        [FixedPoint::ZERO, FixedPoint::ONE, FixedPoint::ZERO],
        [FixedPoint::ZERO, FixedPoint::ZERO, FixedPoint::ONE],
    ];
    let mut d = [a[0][0], a[1][1], a[2][2]];

    for _iter in 0..50 {
        let off_diag = a[0][1].abs() + a[0][2].abs() + a[1][2].abs();
        if off_diag < FP_EPS {
            break;
        }

        for p in 0..2 {
            for q in (p + 1)..3 {
                if a[p][q].abs() < FP_EPS {
                    continue;
                }

                let denom = d[p] - d[q];
                if denom.raw == 0 {
                    continue;
                }
                let theta =
                    FP_HALF * FixedPoint::atan2((FP_TWO * a[p][q]) / denom, FixedPoint::ONE);
                let c = theta.cos();
                let s = theta.sin();

                let mut a_new = a;
                a_new[p][p] = c * c * a[p][p] - FP_TWO * s * c * a[p][q] + s * s * a[q][q];
                a_new[q][q] = s * s * a[p][p] + FP_TWO * s * c * a[p][q] + c * c * a[q][q];
                a_new[p][q] = FixedPoint::ZERO;
                a_new[q][p] = FixedPoint::ZERO;

                for r in 0..3 {
                    if r != p && r != q {
                        a_new[p][r] = c * a[p][r] - s * a[q][r];
                        a_new[r][p] = a_new[p][r];
                        a_new[q][r] = s * a[p][r] + c * a[q][r];
                        a_new[r][q] = a_new[q][r];
                    }
                }

                a = a_new;
                d[p] = a[p][p];
                d[q] = a[q][q];

                for r in 0..3 {
                    let vpr = v[r][p];
                    let vqr = v[r][q];
                    v[r][p] = c * vpr - s * vqr;
                    v[r][q] = s * vpr + c * vqr;
                }
            }
        }
    }

    (a, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_config() -> MpmConfig {
        MpmConfig {
            grid_resolution: [4, 4, 4],
            cell_size: FixedPoint::from_f32(0.5),
            particle_count: 8,
            youngs_modulus: FixedPoint::from_f32(50.0),
            density: FixedPoint::from_f32(1.0),
            dt: FixedPoint::from_f32(1.0 / 60.0),
            substeps: 1,
            ..Default::default()
        }
    }

    const TEST_STEPS: usize = 5;

    #[test]
    fn test_mpm_initialization() {
        let config = small_config();
        let sim = MpmSimulation::new(config);
        assert_eq!(sim.particles.len(), 8);
        assert!(sim.active_particles() == 8);
        for p in &sim.particles {
            assert!(p.active);
            assert!(p.mass > FixedPoint::ZERO);
            assert!(p.volume > FixedPoint::ZERO);
        }
    }

    #[test]
    fn test_mpm_substep_no_panic() {
        let config = small_config();
        let mut sim = MpmSimulation::new(config);
        for _ in 0..10 {
            sim.step();
        }
        assert_eq!(sim.step_count, 10);
        assert_eq!(sim.active_particles(), 8);
    }

    #[test]
    fn test_mpm_elastic() {
        let mut config = small_config();
        config.material_model = MpmMaterialModel::Elastic;
        let mut sim = MpmSimulation::new(config);
        let initial_energy = sim.kinetic_energy();
        for _ in 0..TEST_STEPS {
            sim.step();
        }
        assert!(sim.active_particles() > 0);
        let final_energy = sim.kinetic_energy();
        assert!(final_energy > initial_energy);
    }

    #[test]
    fn test_mpm_fluid() {
        let mut config = small_config();
        config.material_model = MpmMaterialModel::Fluid { viscosity: FixedPoint::from_f32(0.01) };
        let mut sim = MpmSimulation::new(config);
        for _ in 0..TEST_STEPS {
            sim.step();
        }
        assert!(sim.active_particles() > 0);
    }

    #[test]
    fn test_mpm_granular() {
        let mut config = small_config();
        config.material_model =
            MpmMaterialModel::Granular { friction_angle: FixedPoint::from_f32(30.0) };
        let mut sim = MpmSimulation::new(config);
        for _ in 0..TEST_STEPS {
            sim.step();
        }
        assert!(sim.active_particles() > 0);
    }

    #[test]
    fn test_mpm_snow() {
        let mut config = small_config();
        config.material_model = MpmMaterialModel::Snow { hardening: FixedPoint::from_f32(0.05) };
        let mut sim = MpmSimulation::new(config);
        for _ in 0..TEST_STEPS {
            sim.step();
        }
        assert!(sim.active_particles() > 0);
    }

    #[test]
    fn test_mpm_fracture_enabled() {
        let mut config = small_config();
        config.enable_fracture = true;
        config.fracture_strain = FixedPoint::from_f32(0.01);
        let mut sim = MpmSimulation::new(config);
        for _ in 0..TEST_STEPS {
            sim.step();
        }
        assert!(sim.active_particles() > 0);
    }

    #[test]
    fn test_mass_conservation() {
        let config = small_config();
        let mut sim = MpmSimulation::new(config);
        let initial_mass = sim.total_mass();
        for _ in 0..TEST_STEPS {
            sim.step();
        }
        let final_mass = sim.total_mass();
        let diff = (initial_mass - final_mass).abs();
        let tolerance = initial_mass * FixedPoint::from_f32(0.001);
        assert!(
            diff < tolerance,
            "Mass not conserved: initial={}, final={}, diff={}",
            initial_mass,
            final_mass,
            diff
        );
    }

    #[test]
    fn test_add_remove_particle() {
        let config = small_config();
        let mut sim = MpmSimulation::new(config);
        let initial_count = sim.active_particles();
        let id = sim.add_particle(
            FixedVec3::from_f32(0.5, 0.5, 0.5),
            FixedVec3::ZERO,
            FixedPoint::from_f32(0.1),
            [1.0, 0.0, 0.0, 1.0],
        );
        assert_eq!(sim.active_particles(), initial_count + 1);
        assert!(sim.remove_particle(id));
        assert_eq!(sim.active_particles(), initial_count);
        assert!(!sim.remove_particle(99999));
    }

    #[test]
    fn test_snapshots() {
        let config = small_config();
        let sim = MpmSimulation::new(config);
        let snapshots = sim.snapshots();
        assert_eq!(snapshots.len(), 8);
        for s in &snapshots {
            assert!(!s.position[0].is_nan());
            assert!(!s.velocity[0].is_nan());
        }
    }

    #[test]
    fn test_material_model_switch() {
        let config = small_config();
        let mut sim = MpmSimulation::new(config);
        sim.set_material_model(MpmMaterialModel::Elastic);
        sim.step();
        sim.set_material_model(MpmMaterialModel::ElastoPlastic);
        sim.step();
        sim.set_material_model(MpmMaterialModel::Fluid { viscosity: FixedPoint::from_f32(0.01) });
        sim.step();
        assert_eq!(sim.step_count, 3);
    }

    #[test]
    fn test_fracture_enable_disable() {
        let config = small_config();
        let mut sim = MpmSimulation::new(config);
        assert!(!sim.config.enable_fracture);
        sim.enable_fracture(FixedPoint::from_f32(0.05));
        assert!(sim.config.enable_fracture);
        sim.disable_fracture();
        assert!(!sim.config.enable_fracture);
    }

    #[test]
    fn test_mpm_brittle() {
        let mut config = small_config();
        config.material_model =
            MpmMaterialModel::Brittle { fracture_strain: FixedPoint::from_f32(0.02) };
        let mut sim = MpmSimulation::new(config);
        for _ in 0..TEST_STEPS {
            sim.step();
        }
        assert!(sim.active_particles() > 0);
    }

    #[test]
    fn test_zero_particles() {
        let mut config = MpmConfig { particle_count: 0, ..Default::default() };
        config.grid_resolution = [4, 4, 4];
        let mut sim = MpmSimulation::new(config);
        sim.step();
        assert_eq!(sim.active_particles(), 0);
    }
}
