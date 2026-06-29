//! Culling

use glam::{Mat4, Vec4, Vec3};

/// 视锥
#[derive(Debug, Clone)]
pub struct Frustum {
    pub planes: [Vec4; 6],
}

impl Frustum {
    pub fn from_view_proj(view_proj: Mat4) -> Self {
        // glam 0.29 Mat4 是列主序：to_cols_array() 返回 16 个 f32，按列排列
        // cols[0..4] = col0, cols[4..8] = col1, cols[8..12] = col2, cols[12..16] = col3
        // 转置后取行：row_i = (col0[i], col1[i], col2[i], col3[i])
        let m = view_proj.to_cols_array();
        let row = |i: usize| Vec4::new(m[i], m[4 + i], m[8 + i], m[12 + i]);
        let r0 = row(0);
        let r1 = row(1);
        let r2 = row(2);
        let r3 = row(3);
        let planes = [
            r3 + r0, // left
            r3 - r0, // right
            r3 + r1, // bottom
            r3 - r1, // top
            r3 + r2, // near
            r3 - r2, // far
        ];
        let planes = planes.map(|p| p.normalize_or_zero());
        Self { planes }
    }

    pub fn contains_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.dot(Vec4::new(center.x, center.y, center.z, 1.0)) < -radius {
                return false;
            }
        }
        true
    }
}

/// Culling trait
pub trait Culling: Send + Sync {
    fn cull(&self, frustum: &Frustum) -> Vec<CullingResult>;
}

#[derive(Debug, Clone)]
pub struct CullingResult {
    pub node_id: u32,
    pub distance: f32,
}