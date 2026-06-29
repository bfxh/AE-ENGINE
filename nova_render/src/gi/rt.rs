//! 硬件光追 GI（未来）

pub struct RtGi {
    pub max_bounces: u32,
    pub samples_per_pixel: u32,
}

impl Default for RtGi {
    fn default() -> Self {
        Self { max_bounces: 2, samples_per_pixel: 1 }
    }
}