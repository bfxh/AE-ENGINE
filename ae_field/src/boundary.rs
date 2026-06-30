use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::scalar_field::BoundaryCondition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryShape {
    pub shape_type: BoundaryShapeType,
    pub position: Vec3,
    pub scale: Vec3,
    pub rotation: glam::Quat,
    pub boundary: BoundaryCondition,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BoundaryShapeType {
    Sphere,
    Box,
    Cylinder,
    Mesh {
        vertex_count: u32,
    },
    #[serde(skip)]
    Implicit(fn(Vec3) -> f32),
}

impl PartialEq for BoundaryShapeType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}
impl Eq for BoundaryShapeType {}

impl BoundaryShape {
    pub fn sample(&self, world_pos: Vec3) -> f32 {
        let local = world_pos - self.position;
        match self.shape_type {
            BoundaryShapeType::Sphere => {
                let r = self.scale.x;
                local.length() - r
            },
            BoundaryShapeType::Box => {
                let d = glam::vec3(local.x.abs(), local.y.abs(), local.z.abs()) - self.scale;
                d.max(Vec3::ZERO).length() + d.max_element().min(0.0)
            },
            BoundaryShapeType::Cylinder => {
                let dxz = glam::Vec2::new(local.x, local.z).length() - self.scale.x;
                let dy = local.y.abs() - self.scale.y;
                glam::vec2(dxz, dy).max(glam::Vec2::ZERO).length()
                    + dxz.max(0.0).max(dy.max(0.0)).min(0.0)
            },
            BoundaryShapeType::Mesh { .. } => local.length() - self.scale.x,
            BoundaryShapeType::Implicit(f) => f(local),
        }
    }

    pub fn is_inside(&self, world_pos: Vec3) -> bool {
        self.sample(world_pos) <= 0.0
    }

    pub fn distance(&self, world_pos: Vec3) -> f32 {
        self.sample(world_pos)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundarySystem {
    pub boundaries: Vec<BoundaryShape>,
}

impl BoundarySystem {
    pub fn new() -> Self {
        Self { boundaries: Vec::new() }
    }

    pub fn add_boundary(&mut self, boundary: BoundaryShape) {
        self.boundaries.push(boundary);
    }

    pub fn get_boundary_value(&self, world_pos: Vec3) -> Option<f32> {
        for boundary in &self.boundaries {
            let d = boundary.distance(world_pos);
            if d <= 0.0 {
                return match boundary.boundary {
                    BoundaryCondition::Dirichlet(v) => Some(v),
                    _ => Some(0.0),
                };
            }
        }
        None
    }

    pub fn nearest_distance(&self, world_pos: Vec3) -> Option<f32> {
        self.boundaries
            .iter()
            .map(|b| b.distance(world_pos))
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }
}

impl Default for BoundarySystem {
    fn default() -> Self {
        Self::new()
    }
}
