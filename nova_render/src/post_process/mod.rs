//! 后处理栈（借鉴 kajiya + bevy 乒乓纹理）
//!
//! kajiya 验证过的顺序：
//! TAA → 运动模糊 → 曝光 → 色调映射 → 眩光 → 锐化 → DPI 升采样
//!
//! 加上 v1 的：
//! Bloom / SSAO / SSR / FXAA / CAS

pub mod stack;
pub mod bloom;
pub mod tonemap;
pub mod taa;
pub mod ssao;
pub mod ssr;
pub mod motion_blur;
pub mod fxaa;
pub mod cas;

pub use stack::{EffectStack, EffectSlot};
pub use bloom::{BloomPass, BloomEffect, BloomUniform};
pub use tonemap::TonemapEffect;
pub use taa::TaaEffect;
pub use ssao::SsaoEffect;
pub use ssr::SsrEffect;
pub use motion_blur::MotionBlurEffect;
pub use fxaa::FxaaEffect;
pub use cas::CasEffect;
