use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::scalar_field::{FieldType, ScalarField, ScalarFieldConfig};
use crate::vector_field::{VectorField, VectorFieldType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSolver {
    pub scalar_fields: HashMap<String, ScalarField>,
    pub vector_fields: HashMap<String, VectorField>,
    pub time: f64,
    pub time_scale: f32,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConfiguration {
    pub name: String,
    pub resolution: [u32; 3],
    pub origin: Vec3,
    pub cell_size: f32,
}

impl FieldSolver {
    pub fn new() -> Self {
        Self {
            scalar_fields: HashMap::new(),
            vector_fields: HashMap::new(),
            time: 0.0,
            time_scale: 1.0,
            paused: false,
        }
    }

    pub fn create_scalar_field(
        &mut self,
        config: FieldConfiguration,
        field_type: FieldType,
    ) -> &mut ScalarField {
        let field = ScalarField::new(ScalarFieldConfig {
            name: config.name.clone(),
            resolution: config.resolution,
            origin: config.origin,
            cell_size: config.cell_size,
            field_type,
            boundary: crate::scalar_field::BoundaryCondition::Dirichlet(0.0),
            diffusivity: 0.1,
            decay_rate: 0.01,
            initial_value: None,
        });
        self.scalar_fields.insert(config.name.clone(), field);
        self.scalar_fields.get_mut(&config.name).unwrap()
    }

    pub fn create_vector_field(
        &mut self,
        config: FieldConfiguration,
        field_type: VectorFieldType,
    ) -> &mut VectorField {
        let field = VectorField::new(
            config.name.clone(),
            config.resolution,
            config.origin,
            config.cell_size,
            field_type,
            crate::scalar_field::BoundaryCondition::Dirichlet(0.0),
            0.1,
        );
        self.vector_fields.insert(config.name.clone(), field);
        self.vector_fields.get_mut(&config.name).unwrap()
    }

    pub fn step(&mut self, dt: f64) {
        if self.paused {
            return;
        }

        let scaled_dt = (dt * self.time_scale as f64) as f32;

        for field in self.scalar_fields.values_mut() {
            field.step(scaled_dt);
        }

        self.time += dt * self.time_scale as f64;
    }

    pub fn get_combined_temperature_field(&self) -> Option<&ScalarField> {
        self.scalar_fields.values().find(|f| f.field_type == FieldType::Temperature)
    }

    pub fn get_combined_density_field(&self) -> Option<&ScalarField> {
        self.scalar_fields.values().find(|f| f.field_type == FieldType::Density)
    }

    pub fn get_biological_activity(&self) -> Option<&ScalarField> {
        self.scalar_fields.values().find(|f| f.field_type == FieldType::BiologicalActivity)
    }

    pub fn compute_overlap_zone(
        &self,
        field_a: &str,
        field_b: &str,
        threshold: f32,
    ) -> Vec<(Vec3, f32)> {
        let a = match self.scalar_fields.get(field_a) {
            Some(f) => f,
            None => return Vec::new(),
        };
        let b = match self.scalar_fields.get(field_b) {
            Some(f) => f,
            None => return Vec::new(),
        };

        if a.resolution != b.resolution {
            return Vec::new();
        }

        let mut overlap = Vec::new();
        for z in 0..a.resolution[2] {
            for y in 0..a.resolution[1] {
                for x in 0..a.resolution[0] {
                    let val_a = a.get(x, y, z);
                    let val_b = b.get(x, y, z);
                    if val_a > threshold && val_b > threshold {
                        let cell_center = a.origin
                            + Vec3::new(
                                x as f32 * a.cell_size + a.cell_size * 0.5,
                                y as f32 * a.cell_size + a.cell_size * 0.5,
                                z as f32 * a.cell_size + a.cell_size * 0.5,
                            );
                        overlap.push((cell_center, val_a * val_b));
                    }
                }
            }
        }

        overlap
    }

    pub fn interact_fields(
        &mut self,
        source_field: &str,
        target_field: &str,
        interaction_rate: f32,
        dt: f32,
    ) {
        let (source_data, resolution, _origin, _cell_size) = {
            let source = match self.scalar_fields.get(source_field) {
                Some(f) => (f.data.clone(), f.resolution, f.origin, f.cell_size),
                None => return,
            };
            source
        };

        let target = match self.scalar_fields.get_mut(target_field) {
            Some(f) => f,
            None => return,
        };

        if target.resolution != resolution {
            return;
        }

        for (target_val, &source_val) in target.data.iter_mut().zip(source_data.iter()) {
            *target_val += source_val * interaction_rate * dt;
            *target_val = target_val.max(0.0);
        }
    }
}

impl Default for FieldSolver {
    fn default() -> Self {
        Self::new()
    }
}
