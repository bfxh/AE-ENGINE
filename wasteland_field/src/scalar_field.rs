use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarField {
    pub name: String,
    pub resolution: [u32; 3],
    pub origin: Vec3,
    pub cell_size: f32,
    pub data: Vec<f32>,
    pub field_type: FieldType,
    pub boundary_condition: BoundaryCondition,
    pub diffusivity: f32,
    pub decay_rate: f32,
    pub sources: Vec<FieldSource>,
    pub sinks: Vec<FieldSink>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    Temperature,
    Density,
    Pressure,
    ChemicalConcentration {
        compound_id: u32,
    },
    BiologicalActivity,
    Radiation,
    StressScalar,
    ElectricPotential,
    #[allow(non_camel_case_types)]
    pH,
    Moisture,
    NutrientLevel,
    ToxinLevel,
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BoundaryCondition {
    Dirichlet(f32),
    Neumann(f32),
    Periodic,
    Absorbing,
    Reflecting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSource {
    pub position: Vec3,
    pub radius: f32,
    pub strength: f32,
    pub falloff: FalloffType,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FalloffType {
    Linear,
    Quadratic,
    Gaussian,
    Constant,
    Exponential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSink {
    pub position: Vec3,
    pub radius: f32,
    pub absorption_rate: f32,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct ScalarFieldConfig {
    pub name: String,
    pub resolution: [u32; 3],
    pub origin: Vec3,
    pub cell_size: f32,
    pub field_type: FieldType,
    pub boundary: BoundaryCondition,
    pub diffusivity: f32,
    pub decay_rate: f32,
    pub initial_value: Option<f32>,
}

impl ScalarField {
    pub fn new(config: ScalarFieldConfig) -> Self {
        let total = (config.resolution[0] * config.resolution[1] * config.resolution[2]) as usize;
        let initial_value = config.initial_value.unwrap_or(0.0);
        Self {
            name: config.name,
            resolution: config.resolution,
            origin: config.origin,
            cell_size: config.cell_size,
            data: vec![initial_value; total],
            field_type: config.field_type,
            boundary_condition: config.boundary,
            diffusivity: config.diffusivity,
            decay_rate: config.decay_rate,
            sources: Vec::new(),
            sinks: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_initial_value(
        name: String,
        resolution: [u32; 3],
        origin: Vec3,
        cell_size: f32,
        field_type: FieldType,
        boundary: BoundaryCondition,
        diffusivity: f32,
        decay_rate: f32,
        initial_value: f32,
    ) -> Self {
        Self::new(ScalarFieldConfig {
            name,
            resolution,
            origin,
            cell_size,
            field_type,
            boundary,
            diffusivity,
            decay_rate,
            initial_value: Some(initial_value),
        })
    }

    pub fn index(&self, x: u32, y: u32, z: u32) -> usize {
        (z * self.resolution[1] * self.resolution[0] + y * self.resolution[0] + x) as usize
    }

    pub fn sample(&self, world_pos: Vec3) -> f32 {
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
            return match self.boundary_condition {
                BoundaryCondition::Dirichlet(v) => v,
                BoundaryCondition::Neumann(_) => 0.0,
                _ => 0.0,
            };
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

    pub fn get(&self, x: u32, y: u32, z: u32) -> f32 {
        if x >= self.resolution[0] || y >= self.resolution[1] || z >= self.resolution[2] {
            return match self.boundary_condition {
                BoundaryCondition::Dirichlet(v) => v,
                _ => 0.0,
            };
        }
        self.data[self.index(x, y, z)]
    }

    pub fn set(&mut self, x: u32, y: u32, z: u32, value: f32) {
        if x < self.resolution[0] && y < self.resolution[1] && z < self.resolution[2] {
            let idx = self.index(x, y, z);
            self.data[idx] = value;
        }
    }

    pub fn add(&mut self, x: u32, y: u32, z: u32, value: f32) {
        if x < self.resolution[0] && y < self.resolution[1] && z < self.resolution[2] {
            let idx = self.index(x, y, z);
            self.data[idx] += value;
        }
    }

    pub fn gradient(&self, world_pos: Vec3) -> Vec3 {
        let h = self.cell_size * 0.5;
        let dx = self.sample(world_pos + Vec3::X * h) - self.sample(world_pos - Vec3::X * h);
        let dy = self.sample(world_pos + Vec3::Y * h) - self.sample(world_pos - Vec3::Y * h);
        let dz = self.sample(world_pos + Vec3::Z * h) - self.sample(world_pos - Vec3::Z * h);
        Vec3::new(dx, dy, dz) / (2.0 * h)
    }

    pub fn laplacian(&self, x: u32, y: u32, z: u32) -> f32 {
        let c = self.get(x, y, z);
        let sum = self.get(x + 1, y, z)
            + self.get(x.wrapping_sub(1), y, z)
            + self.get(x, y + 1, z)
            + self.get(x, y.wrapping_sub(1), z)
            + self.get(x, y, z + 1)
            + self.get(x, y, z.wrapping_sub(1));
        (sum - 6.0 * c) / (self.cell_size * self.cell_size)
    }

    pub fn apply_sources(&mut self, dt: f32) {
        let sources: Vec<FieldSource> = self.sources.to_vec();
        for source in &sources {
            if !source.active {
                continue;
            }
            let center = source.position;
            let radius_cells = (source.radius / self.cell_size).ceil() as i32;

            let cx = ((center.x - self.origin.x) / self.cell_size) as i32;
            let cy = ((center.y - self.origin.y) / self.cell_size) as i32;
            let cz = ((center.z - self.origin.z) / self.cell_size) as i32;

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
                        let dist = (cell_center - center).length();
                        if dist > source.radius {
                            continue;
                        }

                        let factor = match source.falloff {
                            FalloffType::Linear => 1.0 - dist / source.radius,
                            FalloffType::Quadratic => (1.0 - dist / source.radius).powi(2),
                            FalloffType::Gaussian => (-4.0 * (dist / source.radius).powi(2)).exp(),
                            FalloffType::Constant => 1.0,
                            FalloffType::Exponential => (-2.0 * dist / source.radius).exp(),
                        };

                        self.add(x, y, z, source.strength * factor * dt);
                    }
                }
            }
        }
    }

    pub fn apply_sinks(&mut self, dt: f32) {
        let sinks: Vec<FieldSink> = self.sinks.to_vec();
        for sink in &sinks {
            if !sink.active {
                continue;
            }
            let center = sink.position;
            let radius_cells = (sink.radius / self.cell_size).ceil() as i32;

            let cx = ((center.x - self.origin.x) / self.cell_size) as i32;
            let cy = ((center.y - self.origin.y) / self.cell_size) as i32;
            let cz = ((center.z - self.origin.z) / self.cell_size) as i32;

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
                        let dist = (cell_center - center).length();
                        if dist > sink.radius {
                            continue;
                        }

                        let factor = (1.0 - dist / sink.radius).powi(2);
                        let current = self.get(x, y, z);
                        let absorbed = (current * sink.absorption_rate * factor * dt).min(current);
                        self.add(x, y, z, -absorbed);
                    }
                }
            }
        }
    }

    pub fn diffuse(&mut self, dt: f32) {
        if self.diffusivity <= 0.0 {
            return;
        }

        let mut new_data = vec![0.0f32; self.data.len()];

        for z in 0..self.resolution[2] {
            for y in 0..self.resolution[1] {
                for x in 0..self.resolution[0] {
                    let idx = self.index(x, y, z);
                    let laplacian = self.laplacian(x, y, z);
                    let diffusion = self.diffusivity * laplacian * dt;
                    let decay = self.decay_rate * self.data[idx] * dt;
                    new_data[idx] = self.data[idx] + diffusion - decay;
                }
            }
        }

        self.data = new_data;
    }

    pub fn step(&mut self, dt: f32) {
        self.diffuse(dt);
        self.apply_sources(dt);
        self.apply_sinks(dt);
    }

    pub fn get_field_line(&self, start: Vec3, max_steps: usize, step_size: f32) -> Vec<Vec3> {
        let mut points = Vec::with_capacity(max_steps);
        let mut pos = start;

        for _ in 0..max_steps {
            points.push(pos);
            let grad = self.gradient(pos);
            let mag = grad.length();
            if mag < 1e-6 {
                break;
            }
            let dir = grad.normalize();
            pos += dir * step_size;

            if pos.x < self.origin.x
                || pos.x > self.origin.x + self.resolution[0] as f32 * self.cell_size
                || pos.y < self.origin.y
                || pos.y > self.origin.y + self.resolution[1] as f32 * self.cell_size
                || pos.z < self.origin.z
                || pos.z > self.origin.z + self.resolution[2] as f32 * self.cell_size
            {
                break;
            }
        }

        points
    }

    pub fn add_source(&mut self, source: FieldSource) {
        self.sources.push(source);
    }

    pub fn add_sink(&mut self, sink: FieldSink) {
        self.sinks.push(sink);
    }

    pub fn max_value(&self) -> f32 {
        self.data.iter().cloned().fold(0.0f32, f32::max)
    }

    pub fn min_value(&self) -> f32 {
        self.data.iter().cloned().fold(f32::MAX, f32::min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_field_creation() {
        let field = ScalarField::with_initial_value(
            "test".into(),
            [10, 10, 10],
            Vec3::ZERO,
            1.0,
            FieldType::Temperature,
            BoundaryCondition::Dirichlet(0.0),
            0.1,
            0.01,
            293.0,
        );
        assert_eq!(field.get(0, 0, 0), 293.0);
        assert_eq!(field.get(5, 5, 5), 293.0);
    }

    #[test]
    fn test_diffusion() {
        let mut field = ScalarField::with_initial_value(
            "diffuse".into(),
            [10, 10, 10],
            Vec3::ZERO,
            1.0,
            FieldType::Temperature,
            BoundaryCondition::Dirichlet(0.0),
            0.5,
            0.0,
            0.0,
        );
        field.set(5, 5, 5, 100.0);
        for _ in 0..10 {
            field.diffuse(0.1);
        }
        let center = field.get(5, 5, 5);
        let neighbor = field.get(6, 5, 5);
        assert!(center < 100.0);
        assert!(neighbor > 0.0);
    }
}
