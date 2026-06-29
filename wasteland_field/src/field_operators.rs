use glam::Vec3;

use crate::scalar_field::{ScalarField, ScalarFieldConfig};
use crate::vector_field::VectorField;

pub fn gradient_of(scalar: &ScalarField, name: &str) -> VectorField {
    let mut grad = VectorField::new(
        name.to_string(),
        scalar.resolution,
        scalar.origin,
        scalar.cell_size,
        crate::vector_field::VectorFieldType::InformationGradient,
        crate::scalar_field::BoundaryCondition::Dirichlet(0.0),
        0.0,
    );

    for z in 0..scalar.resolution[2] {
        for y in 0..scalar.resolution[1] {
            for x in 0..scalar.resolution[0] {
                let cell_center = scalar.origin
                    + Vec3::new(
                        x as f32 * scalar.cell_size + scalar.cell_size * 0.5,
                        y as f32 * scalar.cell_size + scalar.cell_size * 0.5,
                        z as f32 * scalar.cell_size + scalar.cell_size * 0.5,
                    );
                let g = scalar.gradient(cell_center);
                grad.set(x, y, z, g);
            }
        }
    }

    grad
}

pub fn divergence_of(vector: &VectorField) -> ScalarField {
    let mut div = ScalarField::new(ScalarFieldConfig {
        name: format!("div_{}", vector.name),
        resolution: vector.resolution,
        origin: vector.origin,
        cell_size: vector.cell_size,
        field_type: crate::scalar_field::FieldType::Custom(0),
        boundary: crate::scalar_field::BoundaryCondition::Dirichlet(0.0),
        diffusivity: 0.0,
        decay_rate: 0.0,
        initial_value: None,
    });

    for z in 0..vector.resolution[2] {
        for y in 0..vector.resolution[1] {
            for x in 0..vector.resolution[0] {
                let d = vector.divergence(x, y, z);
                div.set(x, y, z, d);
            }
        }
    }

    div
}

pub fn add_fields(a: &ScalarField, b: &ScalarField, name: &str) -> ScalarField {
    assert_eq!(a.resolution, b.resolution);
    let mut result = ScalarField::new(ScalarFieldConfig {
        name: name.to_string(),
        resolution: a.resolution,
        origin: a.origin,
        cell_size: a.cell_size,
        field_type: crate::scalar_field::FieldType::Custom(0),
        boundary: a.boundary_condition,
        diffusivity: a.diffusivity.max(b.diffusivity),
        decay_rate: (a.decay_rate + b.decay_rate) * 0.5,
        initial_value: None,
    });

    for i in 0..a.data.len() {
        result.data[i] = a.data[i] + b.data[i];
    }

    result
}

pub fn multiply_fields(a: &ScalarField, b: &ScalarField, name: &str) -> ScalarField {
    assert_eq!(a.resolution, b.resolution);
    let mut result = ScalarField::new(ScalarFieldConfig {
        name: name.to_string(),
        resolution: a.resolution,
        origin: a.origin,
        cell_size: a.cell_size,
        field_type: crate::scalar_field::FieldType::Custom(0),
        boundary: a.boundary_condition,
        diffusivity: a.diffusivity.max(b.diffusivity),
        decay_rate: (a.decay_rate + b.decay_rate) * 0.5,
        initial_value: None,
    });

    for i in 0..a.data.len() {
        result.data[i] = a.data[i] * b.data[i];
    }

    result
}

pub fn scale_field(field: &ScalarField, factor: f32, name: &str) -> ScalarField {
    let mut result = ScalarField::new(ScalarFieldConfig {
        name: name.to_string(),
        resolution: field.resolution,
        origin: field.origin,
        cell_size: field.cell_size,
        field_type: crate::scalar_field::FieldType::Custom(0),
        boundary: field.boundary_condition,
        diffusivity: field.diffusivity,
        decay_rate: field.decay_rate,
        initial_value: None,
    });

    for i in 0..field.data.len() {
        result.data[i] = field.data[i] * factor;
    }

    result
}

pub fn field_intersection(a: &ScalarField, b: &ScalarField, name: &str) -> ScalarField {
    let mut result = ScalarField::new(ScalarFieldConfig {
        name: name.to_string(),
        resolution: a.resolution,
        origin: a.origin,
        cell_size: a.cell_size,
        field_type: crate::scalar_field::FieldType::Custom(0),
        boundary: a.boundary_condition,
        diffusivity: 0.0,
        decay_rate: 0.0,
        initial_value: None,
    });

    for i in 0..a.data.len() {
        result.data[i] = a.data[i].min(b.data[i]);
    }

    result
}

pub fn field_union(a: &ScalarField, b: &ScalarField, name: &str) -> ScalarField {
    let mut result = ScalarField::new(ScalarFieldConfig {
        name: name.to_string(),
        resolution: a.resolution,
        origin: a.origin,
        cell_size: a.cell_size,
        field_type: crate::scalar_field::FieldType::Custom(0),
        boundary: a.boundary_condition,
        diffusivity: 0.0,
        decay_rate: 0.0,
        initial_value: None,
    });

    for i in 0..a.data.len() {
        result.data[i] = a.data[i].max(b.data[i]);
    }

    result
}
