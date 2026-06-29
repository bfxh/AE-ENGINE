use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyOptimizer {
    pub design_space: DesignSpace,
    pub constraints: Vec<FunctionalConstraint>,
    pub objective: OptimizationObjective,
    pub iterations: usize,
    pub current_density: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSpace {
    pub resolution: [u32; 3],
    pub origin: Vec3,
    pub cell_size: f32,
    pub fixed_regions: Vec<FixedRegion>,
    pub void_regions: Vec<VoidRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedRegion {
    pub position: Vec3,
    pub radius: f32,
    pub density: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidRegion {
    pub position: Vec3,
    pub radius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalConstraint {
    pub constraint_type: ConstraintType,
    pub target_value: f32,
    pub weight: f32,
    pub direction: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    Mass,
    Stiffness,
    ThermalConductivity,
    ElectricalConductivity,
    Sharpness,
    Grip,
    ImpactResistance,
    Aerodynamic,
    Buoyancy,
    Flexibility,
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OptimizationObjective {
    MinimizeMass,
    MaximizeStiffness,
    MaximizeStrength,
    MinimizeThermalExpansion,
    MaximizeHeatDissipation,
    MultiObjective {
        weights: [f32; 4],
    },
}

impl TopologyOptimizer {
    pub fn new(design_space: DesignSpace) -> Self {
        let total = (design_space.resolution[0] * design_space.resolution[1] * design_space.resolution[2]) as usize;
        let mut density = vec![0.5f32; total];

        for region in &design_space.fixed_regions {
            let cx = ((region.position.x - design_space.origin.x) / design_space.cell_size) as i32;
            let cy = ((region.position.y - design_space.origin.y) / design_space.cell_size) as i32;
            let cz = ((region.position.z - design_space.origin.z) / design_space.cell_size) as i32;
            let r = (region.radius / design_space.cell_size).ceil() as i32;

            for z in (cz - r).max(0)..(cz + r).min(design_space.resolution[2] as i32) {
                for y in (cy - r).max(0)..(cy + r).min(design_space.resolution[1] as i32) {
                    for x in (cx - r).max(0)..(cx + r).min(design_space.resolution[0] as i32) {
                        let idx = (z as u32 * design_space.resolution[1] * design_space.resolution[0]
                            + y as u32 * design_space.resolution[0]
                            + x as u32) as usize;
                        if idx < density.len() {
                            density[idx] = region.density;
                        }
                    }
                }
            }
        }

        for region in &design_space.void_regions {
            let cx = ((region.position.x - design_space.origin.x) / design_space.cell_size) as i32;
            let cy = ((region.position.y - design_space.origin.y) / design_space.cell_size) as i32;
            let cz = ((region.position.z - design_space.origin.z) / design_space.cell_size) as i32;
            let r = (region.radius / design_space.cell_size).ceil() as i32;

            for z in (cz - r).max(0)..(cz + r).min(design_space.resolution[2] as i32) {
                for y in (cy - r).max(0)..(cy + r).min(design_space.resolution[1] as i32) {
                    for x in (cx - r).max(0)..(cx + r).min(design_space.resolution[0] as i32) {
                        let idx = (z as u32 * design_space.resolution[1] * design_space.resolution[0]
                            + y as u32 * design_space.resolution[0]
                            + x as u32) as usize;
                        if idx < density.len() {
                            density[idx] = 0.0;
                        }
                    }
                }
            }
        }

        Self {
            design_space,
            constraints: Vec::new(),
            objective: OptimizationObjective::MaximizeStiffness,
            iterations: 100,
            current_density: density,
        }
    }

    pub fn add_constraint(&mut self, constraint: FunctionalConstraint) {
        self.constraints.push(constraint);
    }

    pub fn optimize_step(&mut self) -> f32 {
        let res = self.design_space.resolution;
        let mut new_density = self.current_density.clone();
        let mut total_change = 0.0f32;

        for z in 0..res[2] {
            for y in 0..res[1] {
                for x in 0..res[0] {
                    let idx = (z * res[1] * res[0] + y * res[0] + x) as usize;
                    let current = self.current_density[idx];

                    let neighbor_sum = self.get_neighbor(z, y, x, res, -1)
                        + self.get_neighbor(z, y, x, res, 1);

                    let mut compliance = 0.0f32;
                    for constraint in &self.constraints {
                        compliance += constraint.weight * (current - constraint.target_value).abs();
                    }

                    let objective_gradient = match self.objective {
                        OptimizationObjective::MinimizeMass => -1.0,
                        OptimizationObjective::MaximizeStiffness => current * 2.0,
                        OptimizationObjective::MaximizeStrength => 1.0,
                        _ => 0.0,
                    };

                    let update = 0.1 * (neighbor_sum - current * 2.0)
                        - 0.05 * compliance
                        + 0.01 * objective_gradient;

                    new_density[idx] = (current + update).clamp(0.0, 1.0);
                    total_change += update.abs();
                }
            }
        }

        for region in &self.design_space.fixed_regions {
            let cx = ((region.position.x - self.design_space.origin.x) / self.design_space.cell_size) as i32;
            let cy = ((region.position.y - self.design_space.origin.y) / self.design_space.cell_size) as i32;
            let cz = ((region.position.z - self.design_space.origin.z) / self.design_space.cell_size) as i32;
            let r = (region.radius / self.design_space.cell_size).ceil() as i32;

            for z in (cz - r).max(0)..(cz + r).min(res[2] as i32) {
                for y in (cy - r).max(0)..(cy + r).min(res[1] as i32) {
                    for x in (cx - r).max(0)..(cx + r).min(res[0] as i32) {
                        let idx = (z as u32 * res[1] * res[0] + y as u32 * res[0] + x as u32) as usize;
                        if idx < new_density.len() {
                            new_density[idx] = region.density;
                        }
                    }
                }
            }
        }

        self.current_density = new_density;
        total_change / (res[0] * res[1] * res[2]) as f32
    }

    fn get_neighbor(&self, z: u32, y: u32, x: u32, res: [u32; 3], offset: i32) -> f32 {
        let nx = (x as i32 + offset).clamp(0, res[0] as i32 - 1) as u32;
        let ny = (y as i32 + offset).clamp(0, res[1] as i32 - 1) as u32;
        let nz = (z as i32 + offset).clamp(0, res[2] as i32 - 1) as u32;
        let idx = (nz * res[1] * res[0] + ny * res[0] + nx) as usize;
        self.current_density.get(idx).copied().unwrap_or(0.0)
    }

    pub fn optimize(&mut self) -> Vec<f32> {
        for _ in 0..self.iterations {
            let change = self.optimize_step();
            if change < 0.0001 {
                break;
            }
        }
        self.current_density.clone()
    }

    pub fn extract_surface(&self, threshold: f32) -> Vec<(Vec3, [f32; 3])> {
        let mut points = Vec::new();
        let res = self.design_space.resolution;

        for z in 0..res[2] {
            for y in 0..res[1] {
                for x in 0..res[0] {
                    let idx = (z * res[1] * res[0] + y * res[0] + x) as usize;
                    let density = self.current_density[idx];

                    if density >= threshold {
                        let pos = self.design_space.origin + Vec3::new(
                            x as f32 * self.design_space.cell_size,
                            y as f32 * self.design_space.cell_size,
                            z as f32 * self.design_space.cell_size,
                        );
                        let color = [density, density * 0.8, density * 0.5];
                        points.push((pos, color));
                    }
                }
            }
        }

        points
    }

    pub fn get_mass(&self) -> f32 {
        let cell_volume = self.design_space.cell_size.powi(3);
        self.current_density.iter().sum::<f32>() * cell_volume
    }

    pub fn get_compliance(&self) -> f32 {
        self.constraints.iter().map(|c| {
            let avg_density = self.current_density.iter().sum::<f32>() / self.current_density.len() as f32;
            c.weight * (avg_density - c.target_value).powi(2)
        }).sum()
    }
}