//! SSAO 效果（借鉴 v1）

pub struct SsaoEffect {
    pub radius: f32,
    pub bias: f32,
    pub intensity: f32,
    pub kernel_size: u32,
}

impl Default for SsaoEffect {
    fn default() -> Self {
        Self { radius: 0.5, bias: 0.025, intensity: 1.0, kernel_size: 64 }
    }
}