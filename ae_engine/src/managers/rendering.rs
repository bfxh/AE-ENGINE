//! 渲染层管理器
//!
//! 管理视觉缓存、光谱渲染和渲染状态。

use std::sync::Arc;

use ae_optics::prelude::*;

use crate::systems::{RenderState, VisualCacheSystem, VisualUnit, VisualUnitId};

/// 渲染层管理器
pub struct RenderingManager {
    pub visual_cache: Arc<VisualCacheSystem>,
    pub spectral_renderer: SpectralRenderer,
    pub render_state: RenderState,
}

impl std::fmt::Debug for RenderingManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderingManager")
            .field("visual_cache", &self.visual_cache)
            .field("spectral_renderer", &self.spectral_renderer)
            .field("render_state", &self.render_state)
            .finish()
    }
}

impl RenderingManager {
    /// 创建新的渲染层管理器
    pub fn new() -> Self {
        Self {
            visual_cache: Arc::new(VisualCacheSystem::new(1000)),
            spectral_renderer: SpectralRenderer::new(),
            render_state: RenderState::default(),
        }
    }

    /// 更新渲染系统
    pub fn update(&mut self, _dt: f32, camera_position: glam::Vec3) {
        self.visual_cache.predictive_cache(camera_position, 50.0);
    }

    /// 定期清理缓存
    pub fn evict_unused(&self, max_age: f64) {
        self.visual_cache.evict_unused(max_age);
    }

    /// 添加视觉单元
    pub fn add_visual_unit(&self, unit: VisualUnit) {
        self.visual_cache.add_visual_unit(unit);
    }

    /// 预热缓存
    pub fn warm_cache(&self, unit_ids: Vec<VisualUnitId>) {
        for id in unit_ids {
            self.visual_cache.get_visual_unit(id);
        }
    }
}

impl Default for RenderingManager {
    fn default() -> Self {
        Self::new()
    }
}
