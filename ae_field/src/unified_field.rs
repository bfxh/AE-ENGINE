use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::field_solver::FieldSolver;
use crate::scalar_field::FieldType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldExcitation {
    pub position: Vec3,
    pub field_type: FieldType,
    pub intensity: f32,
    pub radius: f32,
    pub mass: f32,
    pub velocity: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoupledFieldSystem {
    pub solver: FieldSolver,
    pub coupling_coefficients: Vec<FieldCoupling>,
    pub excitations: Vec<FieldExcitation>,
    pub excitation_threshold: f32,
    pub min_excitation_radius: f32,
    pub max_excitations: usize,
    pub tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCoupling {
    pub source_field: String,
    pub target_field: String,
    pub coupling_type: CouplingType,
    pub coefficient: f32,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CouplingType {
    Linear,
    Threshold { threshold: f32 },
    GradientDriven,
    Nonlinear { exponent: f32 },
    Bidirectional,
    Decay,
}

impl CoupledFieldSystem {
    pub fn new() -> Self {
        Self {
            solver: FieldSolver::new(),
            coupling_coefficients: Vec::new(),
            excitations: Vec::new(),
            excitation_threshold: 0.8,
            min_excitation_radius: 0.05,
            max_excitations: 10000,
            tick: 0,
        }
    }

    pub fn add_coupling(
        &mut self,
        source: &str,
        target: &str,
        coupling_type: CouplingType,
        coefficient: f32,
    ) {
        self.coupling_coefficients.push(FieldCoupling {
            source_field: source.to_string(),
            target_field: target.to_string(),
            coupling_type,
            coefficient,
            active: true,
        });
    }

    pub fn set_default_thermodynamic_couplings(&mut self) {
        self.add_coupling("temperature", "density", CouplingType::Linear, -0.001);
        self.add_coupling("temperature", "pressure", CouplingType::Linear, 0.01);
        self.add_coupling("density", "pressure", CouplingType::Linear, 0.1);
        self.add_coupling("radiation", "temperature", CouplingType::Linear, 0.05);
        self.add_coupling("radiation", "biological_activity", CouplingType::Decay, 0.02);
        self.add_coupling("moisture", "temperature", CouplingType::GradientDriven, 0.005);
        self.add_coupling("toxin_level", "biological_activity", CouplingType::Decay, 0.1);
        self.add_coupling(
            "nutrient_level",
            "biological_activity",
            CouplingType::Threshold { threshold: 0.1 },
            0.05,
        );
    }

    pub fn apply_couplings(&mut self, dt: f32) {
        let coupling_count = self.coupling_coefficients.len();
        let mut gradient_data: Vec<Option<Vec<f32>>> = vec![None; coupling_count];
        let mut bidirectional_data: Vec<Option<Vec<f64>>> = vec![None; coupling_count];

        for (ci, coupling) in self.coupling_coefficients.iter().enumerate() {
            if !coupling.active {
                continue;
            }

            match coupling.coupling_type {
                CouplingType::GradientDriven => {
                    if let Some(source) = self.solver.scalar_fields.get(&coupling.source_field) {
                        let mut grad_mags = vec![0.0f32; source.data.len()];
                        for z in 0..source.resolution[2] {
                            for y in 0..source.resolution[1] {
                                for x in 0..source.resolution[0] {
                                    let cell_center = source.origin
                                        + Vec3::new(
                                            x as f32 * source.cell_size + source.cell_size * 0.5,
                                            y as f32 * source.cell_size + source.cell_size * 0.5,
                                            z as f32 * source.cell_size + source.cell_size * 0.5,
                                        );
                                    let grad = source.gradient(cell_center);
                                    let idx = source.index(x, y, z);
                                    grad_mags[idx] = grad.length();
                                }
                            }
                        }
                        gradient_data[ci] = Some(grad_mags);
                    }
                },
                CouplingType::Bidirectional => {
                    let (source_data, target_data) = {
                        let source = self.solver.scalar_fields.get(&coupling.source_field);
                        let target = self.solver.scalar_fields.get(&coupling.target_field);
                        match (source, target) {
                            (Some(s), Some(t)) if s.data.len() == t.data.len() => {
                                (s.data.clone(), t.data.clone())
                            },
                            _ => continue,
                        }
                    };
                    let len = source_data.len().min(target_data.len());
                    let mut exchanges = vec![0.0f64; len];
                    for i in 0..len {
                        exchanges[i] = (source_data[i] as f64 - target_data[i] as f64)
                            * coupling.coefficient as f64
                            * dt as f64;
                    }
                    bidirectional_data[ci] = Some(exchanges);
                },
                _ => {},
            }
        }

        let coupling_count = self.coupling_coefficients.len();
        for ci in 0..coupling_count {
            let coupling = &self.coupling_coefficients[ci];
            if !coupling.active {
                continue;
            }

            match coupling.coupling_type {
                CouplingType::GradientDriven => {
                    if let Some(grad_mags) = &gradient_data[ci] {
                        if let Some(target) =
                            self.solver.scalar_fields.get_mut(&coupling.target_field)
                        {
                            let len = grad_mags.len().min(target.data.len());
                            for (target_val, &grad_val) in
                                target.data.iter_mut().zip(grad_mags.iter()).take(len)
                            {
                                *target_val += grad_val * coupling.coefficient * dt;
                            }
                        }
                    }
                },
                CouplingType::Bidirectional => {
                    let exchange_clone = bidirectional_data[ci].clone();
                    if let Some(exchanges) = exchange_clone {
                        if let Some(source) =
                            self.solver.scalar_fields.get_mut(&coupling.source_field)
                        {
                            let len = exchanges.len().min(source.data.len());
                            for (source_val, &exchange_val) in
                                source.data.iter_mut().zip(exchanges.iter()).take(len)
                            {
                                *source_val += exchange_val as f32;
                            }
                        }
                        if let Some(target) =
                            self.solver.scalar_fields.get_mut(&coupling.target_field)
                        {
                            let len = exchanges.len().min(target.data.len());
                            for (target_val, &exchange_val) in
                                target.data.iter_mut().zip(exchanges.iter()).take(len)
                            {
                                *target_val -= exchange_val as f32;
                            }
                        }
                    }
                },
                _ => {
                    let (source_data, resolution) = {
                        let source = match self.solver.scalar_fields.get(&coupling.source_field) {
                            Some(f) => (f.data.clone(), f.resolution),
                            None => continue,
                        };
                        source
                    };

                    let target = match self.solver.scalar_fields.get_mut(&coupling.target_field) {
                        Some(f) => f,
                        None => continue,
                    };

                    if target.resolution != resolution {
                        continue;
                    }

                    match coupling.coupling_type {
                        CouplingType::Linear => {
                            for (target_val, &source_val) in
                                target.data.iter_mut().zip(source_data.iter())
                            {
                                *target_val += source_val * coupling.coefficient * dt;
                            }
                        },
                        CouplingType::Threshold { threshold } => {
                            for (target_val, &source_val) in
                                target.data.iter_mut().zip(source_data.iter())
                            {
                                if source_val > threshold {
                                    *target_val += source_val * coupling.coefficient * dt;
                                }
                            }
                        },
                        CouplingType::Nonlinear { exponent } => {
                            for (target_val, &source_val) in
                                target.data.iter_mut().zip(source_data.iter())
                            {
                                *target_val +=
                                    source_val.powf(exponent) * coupling.coefficient * dt;
                            }
                        },
                        CouplingType::Decay => {
                            for (target_val, &source_val) in
                                target.data.iter_mut().zip(source_data.iter())
                            {
                                *target_val -= source_val * coupling.coefficient * dt;
                                *target_val = target_val.max(0.0);
                            }
                        },
                        _ => {},
                    }
                },
            }
        }
    }

    pub fn detect_excitations(&mut self) -> Vec<FieldExcitation> {
        let mut new_excitations = Vec::new();

        for field in self.solver.scalar_fields.values() {
            if field.resolution[0] < 2 || field.resolution[1] < 2 || field.resolution[2] < 2 {
                continue;
            }

            for z in 1..field.resolution[2] - 1 {
                for y in 1..field.resolution[1] - 1 {
                    for x in 1..field.resolution[0] - 1 {
                        let val = field.get(x, y, z);

                        if val < self.excitation_threshold {
                            continue;
                        }

                        let is_peak = field.get(x - 1, y, z) < val
                            && field.get(x + 1, y, z) < val
                            && field.get(x, y - 1, z) < val
                            && field.get(x, y + 1, z) < val
                            && field.get(x, y, z - 1) < val
                            && field.get(x, y, z + 1) < val;

                        if !is_peak {
                            continue;
                        }

                        let cell_center = field.origin
                            + Vec3::new(
                                x as f32 * field.cell_size + field.cell_size * 0.5,
                                y as f32 * field.cell_size + field.cell_size * 0.5,
                                z as f32 * field.cell_size + field.cell_size * 0.5,
                            );

                        let laplacian = field.laplacian(x, y, z);

                        let radius = (val / laplacian.abs().max(1e-6))
                            .sqrt()
                            .max(self.min_excitation_radius)
                            .min(field.cell_size * 3.0);

                        let velocity = match self.solver.vector_fields.get("velocity") {
                            Some(vf) => vf.sample(cell_center),
                            None => Vec3::ZERO,
                        };

                        new_excitations.push(FieldExcitation {
                            position: cell_center,
                            field_type: field.field_type,
                            intensity: val,
                            radius,
                            mass: val * radius.powi(3) * 4.0 / 3.0 * std::f32::consts::PI,
                            velocity,
                        });
                    }
                }
            }
        }

        new_excitations.sort_by(|a, b| {
            b.intensity.partial_cmp(&a.intensity).unwrap_or(std::cmp::Ordering::Equal)
        });
        new_excitations.truncate(self.max_excitations);

        self.excitations = new_excitations.clone();
        new_excitations
    }

    pub fn step(&mut self, dt: f64) {
        let scaled_dt = dt as f32;

        self.apply_couplings(scaled_dt);

        for field in self.solver.scalar_fields.values_mut() {
            field.step(scaled_dt);
        }

        let velocity_field = self.solver.vector_fields.get("velocity").cloned();

        for field in self.solver.vector_fields.values_mut() {
            if let Some(ref vel) = velocity_field {
                let vel_clone = vel.clone();
                field.advect(&vel_clone, scaled_dt);
            }
            field.apply_sources(scaled_dt);
        }

        if self.tick.is_multiple_of(10) {
            self.detect_excitations();
        }

        self.solver.time += dt;
        self.tick += 1;
    }

    pub fn get_excitations_of_type(&self, field_type: FieldType) -> Vec<&FieldExcitation> {
        self.excitations.iter().filter(|e| e.field_type == field_type).collect()
    }

    pub fn stress_singularities(&self) -> Vec<Vec3> {
        self.excitations
            .iter()
            .filter(|e| {
                e.field_type == FieldType::StressScalar
                    && e.intensity > self.excitation_threshold * 1.5
            })
            .map(|e| e.position)
            .collect()
    }

    pub fn density_particles(&self) -> Vec<&FieldExcitation> {
        self.get_excitations_of_type(FieldType::Density)
    }

    pub fn chemical_hotspots(&self) -> Vec<&FieldExcitation> {
        self.excitations
            .iter()
            .filter(|e| matches!(e.field_type, FieldType::ChemicalConcentration { .. }))
            .collect()
    }
}

impl Default for CoupledFieldSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_solver::FieldConfiguration;

    #[test]
    fn test_coupling_system() {
        let mut system = CoupledFieldSystem::new();

        let config = FieldConfiguration {
            name: "temperature".into(),
            resolution: [10, 10, 10],
            origin: Vec3::ZERO,
            cell_size: 1.0,
        };
        system.solver.create_scalar_field(config, FieldType::Temperature);

        let config = FieldConfiguration {
            name: "density".into(),
            resolution: [10, 10, 10],
            origin: Vec3::ZERO,
            cell_size: 1.0,
        };
        system.solver.create_scalar_field(config, FieldType::Density);

        system.add_coupling("temperature", "density", CouplingType::Linear, 0.01);

        for i in 0..1000 {
            system.solver.scalar_fields.get_mut("temperature").unwrap().data[i] = 100.0;
        }

        system.step(0.1);

        let density = system.solver.scalar_fields.get("density").unwrap();
        assert!(density.data[0] > 0.0);
    }

    #[test]
    fn test_excitation_detection() {
        let mut system = CoupledFieldSystem::new();

        let config = FieldConfiguration {
            name: "density".into(),
            resolution: [10, 10, 10],
            origin: Vec3::ZERO,
            cell_size: 1.0,
        };
        let field = system.solver.create_scalar_field(config, FieldType::Density);
        field.set(5, 5, 5, 1.0);

        let excitations = system.detect_excitations();
        assert!(!excitations.is_empty());
        assert_eq!(excitations[0].field_type, FieldType::Density);
    }
}
