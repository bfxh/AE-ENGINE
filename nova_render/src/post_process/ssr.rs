//! SSR 效果（借鉴 v1）

pub struct SsrEffect {
    pub max_steps: u32,
    pub thickness: f32,
    pub step_scale: f32,
    pub intensity: f32,
}

impl Default for SsrEffect {
    fn default() -> Self {
        Self { max_steps: 32, thickness: 0.5, step_scale: 1.0, intensity: 0.5 }
    }
}