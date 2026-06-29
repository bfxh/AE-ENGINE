//! Light

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// Light 类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightKind {
    Directional,
    Point { range: f32 },
    Spot { range: f32, inner_angle: f32, outer_angle: f32 },
}

/// Light
#[derive(Debug, Clone)]
pub struct Light {
    pub kind: LightKind,
    pub color: Vec3,
    pub intensity: f32,
    pub direction: Vec3,
    pub position: Vec3,
    pub cast_shadow: bool,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            kind: LightKind::Directional,
            color: Vec3::ONE,
            intensity: 1.0,
            direction: Vec3::new(-0.5, -1.0, -0.5).normalize(),
            position: Vec3::ZERO,
            cast_shadow: false,
        }
    }
}

/// GPU 上传 uniform
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct LightUniform {
    pub direction: [f32; 4],
    pub color: [f32; 4],
    pub position: [f32; 4],
    pub params: [f32; 4], // x=intensity, y=range, z=inner, w=outer
}