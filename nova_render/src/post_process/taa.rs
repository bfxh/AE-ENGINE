//! TAA 效果（借鉴 v1）

pub struct TaaEffect {
    pub blend_factor: f32,
    pub neighborhood_clamp: bool,
}

impl Default for TaaEffect {
    fn default() -> Self {
        Self { blend_factor: 0.1, neighborhood_clamp: true }
    }
}