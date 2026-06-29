//! Camera

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// Camera Projection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraProjection {
    Perspective { fov: f32, aspect: f32, near: f32, far: f32 },
    Orthographic { left: f32, right: f32, top: f32, bottom: f32, near: f32, far: f32 },
}

impl CameraProjection {
    pub fn to_matrix(&self) -> Mat4 {
        match self {
            Self::Perspective { fov, aspect, near, far } => {
                Mat4::perspective_rh(*fov, *aspect, *near, *far)
            }
            Self::Orthographic { left, right, top, bottom, near, far } => {
                Mat4::orthographic_rh(*left, *right, *bottom, *top, *near, *far)
            }
        }
    }
}

/// Camera
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub projection: CameraProjection,
    pub jitter: [f32; 2],
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 5.0, 10.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            projection: CameraProjection::Perspective {
                fov: std::f32::consts::FRAC_PI_4,
                aspect: 16.0 / 9.0,
                near: 0.1,
                far: 1000.0,
            },
            jitter: [0.0, 0.0],
        }
    }
}

impl Camera {
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }
    pub fn proj_matrix(&self) -> Mat4 {
        let mut m = self.projection.to_matrix();
        // TAA jitter
        if self.jitter != [0.0, 0.0] {
            m.col_mut(2)[0] += self.jitter[0];
            m.col_mut(2)[1] += self.jitter[1];
        }
        m
    }
    pub fn view_proj(&self) -> Mat4 {
        self.proj_matrix() * self.view_matrix()
    }
}

/// GPU 上传 uniform
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub position: [f32; 4],
}