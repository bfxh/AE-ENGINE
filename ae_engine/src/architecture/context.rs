//! 世界上下文模块
//!
//! 提供系统更新时的共享上下文信息。

use glam::Vec3;

/// 全局世界状态，供系统更新时读取
#[derive(Debug, Clone)]
pub struct WorldContext {
    /// 当前模拟时间
    pub time: f64,
    /// 时间步长
    pub dt: f32,
    /// 时间缩放
    pub time_scale: f32,
    /// 是否暂停
    pub paused: bool,
    /// 当前 tick 计数
    pub tick_count: u64,
    /// 世界边界
    pub world_bounds: WorldBounds,
    /// 全局温度
    pub global_temperature: f32,
    /// 全局辐射
    pub global_radiation: f32,
    /// 风向
    pub wind: Vec3,
    /// 降水量
    pub precipitation: f32,
    /// 云层覆盖
    pub cloud_cover: f32,
}

/// 世界边界
#[derive(Debug, Clone, Copy)]
pub struct WorldBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for WorldBounds {
    fn default() -> Self {
        Self { min: Vec3::new(-100.0, -100.0, -100.0), max: Vec3::new(100.0, 100.0, 100.0) }
    }
}

impl Default for WorldContext {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldContext {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            dt: 1.0 / 60.0,
            time_scale: 1.0,
            paused: false,
            tick_count: 0,
            world_bounds: WorldBounds::default(),
            global_temperature: 293.0,
            global_radiation: 0.0,
            wind: Vec3::ZERO,
            precipitation: 0.0,
            cloud_cover: 0.3,
        }
    }

    /// 从 GameWorld 状态构建上下文
    #[allow(clippy::too_many_arguments)]
    pub fn from_world_state(
        time: f64,
        dt: f32,
        time_scale: f32,
        paused: bool,
        tick_count: u64,
        bounds_min: Vec3,
        bounds_max: Vec3,
        global_temperature: f32,
        global_radiation: f32,
        wind: Vec3,
        precipitation: f32,
        cloud_cover: f32,
    ) -> Self {
        Self {
            time,
            dt,
            time_scale,
            paused,
            tick_count,
            world_bounds: WorldBounds { min: bounds_min, max: bounds_max },
            global_temperature,
            global_radiation,
            wind,
            precipitation,
            cloud_cover,
        }
    }
}
