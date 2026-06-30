use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::scalar_field::BoundaryCondition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorField {
    pub name: String,
    pub resolution: [u32; 3],
    pub origin: Vec3,
    pub cell_size: f32,
    pub data: Vec<Vec3>,
    pub field_type: VectorFieldType,
    pub boundary_condition: BoundaryCondition,
    pub viscosity: f32,
    pub sources: Vec<VectorFieldSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorFieldType {
    Velocity,
    Force,
    Stress,
    MagneticField,
    ElectricField,
    Wind,
    InformationGradient,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFieldSource {
    pub position: Vec3,
    pub radius: f32,
    pub direction: Vec3,
    pub strength: f32,
    pub active: bool,
}

impl VectorField {
    pub fn new(
        name: String,
        resolution: [u32; 3],
        origin: Vec3,
        cell_size: f32,
        field_type: VectorFieldType,
        boundary: BoundaryCondition,
        viscosity: f32,
    ) -> Self {
        let total = (resolution[0] * resolution[1] * resolution[2]) as usize;
        Self {
            name,
            resolution,
            origin,
            cell_size,
            data: vec![Vec3::ZERO; total],
            field_type,
            boundary_condition: boundary,
            viscosity,
            sources: Vec::new(),
        }
    }

    fn index(&self, x: u32, y: u32, z: u32) -> usize {
        (z * self.resolution[1] * self.resolution[0] + y * self.resolution[0] + x) as usize
    }

    pub fn get(&self, x: u32, y: u32, z: u32) -> Vec3 {
        if x >= self.resolution[0] || y >= self.resolution[1] || z >= self.resolution[2] {
            return Vec3::ZERO;
        }
        self.data[self.index(x, y, z)]
    }

    pub fn set(&mut self, x: u32, y: u32, z: u32, value: Vec3) {
        if x < self.resolution[0] && y < self.resolution[1] && z < self.resolution[2] {
            let idx = self.index(x, y, z);
            self.data[idx] = value;
        }
    }

    pub fn sample(&self, world_pos: Vec3) -> Vec3 {
        let local = world_pos - self.origin;
        let fx = (local.x / self.cell_size).floor() as i32;
        let fy = (local.y / self.cell_size).floor() as i32;
        let fz = (local.z / self.cell_size).floor() as i32;

        if fx < 0
            || fx >= self.resolution[0] as i32
            || fy < 0
            || fy >= self.resolution[1] as i32
            || fz < 0
            || fz >= self.resolution[2] as i32
        {
            return Vec3::ZERO;
        }

        let x = fx as u32;
        let y = fy as u32;
        let z = fz as u32;

        let tx = local.x / self.cell_size - fx as f32;
        let ty = local.y / self.cell_size - fy as f32;
        let tz = local.z / self.cell_size - fz as f32;

        let c000 = self.get(x, y, z);
        let c100 = self.get(x + 1, y, z);
        let c010 = self.get(x, y + 1, z);
        let c110 = self.get(x + 1, y + 1, z);
        let c001 = self.get(x, y, z + 1);
        let c101 = self.get(x + 1, y, z + 1);
        let c011 = self.get(x, y + 1, z + 1);
        let c111 = self.get(x + 1, y + 1, z + 1);

        let c00 = c000 * (1.0 - tx) + c100 * tx;
        let c01 = c001 * (1.0 - tx) + c101 * tx;
        let c10 = c010 * (1.0 - tx) + c110 * tx;
        let c11 = c011 * (1.0 - tx) + c111 * tx;

        let c0 = c00 * (1.0 - ty) + c10 * ty;
        let c1 = c01 * (1.0 - ty) + c11 * ty;

        c0 * (1.0 - tz) + c1 * tz
    }

    pub fn divergence(&self, x: u32, y: u32, z: u32) -> f32 {
        let h = 2.0 * self.cell_size;
        let right = self.get(x + 1, y, z);
        let left = self.get(x.wrapping_sub(1), y, z);
        let top = self.get(x, y + 1, z);
        let bottom = self.get(x, y.wrapping_sub(1), z);
        let front = self.get(x, y, z + 1);
        let back = self.get(x, y, z.wrapping_sub(1));

        (right.x - left.x + top.y - bottom.y + front.z - back.z) / h
    }

    pub fn curl(&self, x: u32, y: u32, z: u32) -> Vec3 {
        let h = 2.0 * self.cell_size;
        let top = self.get(x, y + 1, z);
        let bottom = self.get(x, y.wrapping_sub(1), z);
        let front = self.get(x, y, z + 1);
        let back = self.get(x, y, z.wrapping_sub(1));
        let right = self.get(x + 1, y, z);
        let left = self.get(x.wrapping_sub(1), y, z);

        Vec3::new(
            (top.z - bottom.z - front.y + back.y) / h,
            (front.x - back.x - right.z + left.z) / h,
            (right.y - left.y - top.x + bottom.x) / h,
        )
    }

    pub fn advect(&mut self, velocity: &VectorField, dt: f32) {
        let mut new_data = vec![Vec3::ZERO; self.data.len()];

        for z in 0..self.resolution[2] {
            for y in 0..self.resolution[1] {
                for x in 0..self.resolution[0] {
                    let idx = self.index(x, y, z);
                    let cell_center = self.origin
                        + Vec3::new(
                            x as f32 * self.cell_size + self.cell_size * 0.5,
                            y as f32 * self.cell_size + self.cell_size * 0.5,
                            z as f32 * self.cell_size + self.cell_size * 0.5,
                        );

                    let vel = velocity.sample(cell_center);
                    let back_pos = cell_center - vel * dt;
                    new_data[idx] = self.sample(back_pos);
                }
            }
        }

        self.data = new_data;
    }

    pub fn apply_sources(&mut self, dt: f32) {
        for source in &self.sources {
            if !source.active {
                continue;
            }
            let radius_cells = (source.radius / self.cell_size).ceil() as i32;

            let cx = ((source.position.x - self.origin.x) / self.cell_size) as i32;
            let cy = ((source.position.y - self.origin.y) / self.cell_size) as i32;
            let cz = ((source.position.z - self.origin.z) / self.cell_size) as i32;

            for dx in -radius_cells..=radius_cells {
                for dy in -radius_cells..=radius_cells {
                    for dz in -radius_cells..=radius_cells {
                        let x = (cx + dx) as u32;
                        let y = (cy + dy) as u32;
                        let z = (cz + dz) as u32;
                        if x >= self.resolution[0]
                            || y >= self.resolution[1]
                            || z >= self.resolution[2]
                        {
                            continue;
                        }

                        let cell_center = self.origin
                            + Vec3::new(
                                x as f32 * self.cell_size + self.cell_size * 0.5,
                                y as f32 * self.cell_size + self.cell_size * 0.5,
                                z as f32 * self.cell_size + self.cell_size * 0.5,
                            );
                        let dist = (cell_center - source.position).length();
                        if dist > source.radius {
                            continue;
                        }

                        let factor = (1.0 - dist / source.radius).powi(2);
                        let idx = self.index(x, y, z);
                        self.data[idx] += source.direction * source.strength * factor * dt;
                    }
                }
            }
        }
    }

    pub fn streamlines(&self, start: Vec3, max_steps: usize, step_size: f32) -> Vec<Vec3> {
        let mut points = Vec::with_capacity(max_steps);
        let mut pos = start;

        for _ in 0..max_steps {
            points.push(pos);
            let vel = self.sample(pos);
            let mag = vel.length();
            if mag < 1e-6 {
                break;
            }
            pos += vel.normalize() * step_size;

            let local = pos - self.origin;
            if local.x < 0.0
                || local.x > self.resolution[0] as f32 * self.cell_size
                || local.y < 0.0
                || local.y > self.resolution[1] as f32 * self.cell_size
                || local.z < 0.0
                || local.z > self.resolution[2] as f32 * self.cell_size
            {
                break;
            }
        }

        points
    }
}
