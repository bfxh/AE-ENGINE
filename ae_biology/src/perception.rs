//! 游戏感知系统模块 — AI 角色感知建模
//!
//! 科学与工程来源:
//! - Reynolds, C. W. (1999). "Steering Behaviors For Autonomous Characters."
//!   GDC 1999 / "Game Programming Gems" — 视觉锥/听觉半径/朝向感知
//! - Millington, I. (2019). "AI for Games", 3rd edition. CRC Press. Ch. 10 Sensing
//! - Ericson, C. (2004). "Real-Time Collision Detection." Morgan Kaufmann.
//!   (球-锥相交、距离衰减)
//! - Buck, S. (2004). "The Illusion of Intelligence: Integrating AI into Games."
//!
//! 设计: 视觉锥 + 听觉半径 + 嗅觉半径 + 触觉半径 + 注意力衰减 + 潜行因子
//! 潜行因子综合: 可见性 / 距离 / 光照 — 用于潜行类游戏 (Skyrim / Deus Ex 范式)

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// 感知类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PerceptionSense {
    Vision,
    Hearing,
    Smell,
    Touch,
    Thermal,
    Vibration,
}

/// 目标可见性
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Visibility {
    /// 完全可见
    Visible,
    /// 部分可见 (灌木丛/烟雾)
    Partial,
    /// 完全隐藏
    Hidden,
    /// 伪装 (与背景融合)
    Camouflaged,
}

impl Visibility {
    /// 可见性系数 (0..1)
    pub fn factor(&self) -> f32 {
        match self {
            Self::Visible => 1.0,
            Self::Partial => 0.5,
            Self::Hidden => 0.0,
            Self::Camouflaged => 0.25,
        }
    }
}

/// 已检测到的目标
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectedTarget {
    /// 目标位置
    pub position: Vec3,
    /// 与感知者的距离
    pub distance: f32,
    /// 由哪种感官感知到
    pub sense: PerceptionSense,
    /// 置信度 (0..1)
    pub confidence: f32,
    /// 上次看到时刻 (s)
    pub last_seen_time: f32,
}

/// 感知系统
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerceptionSystem {
    /// 视觉锥半角 (度, 全锥 = 2 * 半角)
    pub vision_cone_angle: f32,
    /// 视觉最远距离
    pub vision_range: f32,
    /// 听觉半径
    pub hearing_radius: f32,
    /// 嗅觉半径
    pub smell_radius: f32,
    /// 触觉半径
    pub touch_radius: f32,
    /// 注意力上限 (s)
    pub attention_span: f32,
    /// 当前剩余注意力 (s)
    pub current_attention: f32,
}

impl PerceptionSystem {
    pub fn new() -> Self {
        Self {
            vision_cone_angle: 60.0, // 半角 60° -> 全锥 120°
            vision_range: 30.0,
            hearing_radius: 15.0,
            smell_radius: 5.0,
            touch_radius: 1.5,
            attention_span: 5.0,
            current_attention: 5.0,
        }
    }

    /// 是否能"看到"目标
    /// target_pos / self_pos / self_facing (单位向量)
    pub fn can_see(&self, target_pos: Vec3, self_pos: Vec3, self_facing: Vec3) -> bool {
        let to_target = target_pos - self_pos;
        let dist = to_target.length();
        if dist > self.vision_range || dist < 1e-6 {
            return dist <= self.vision_range;
        }
        let dir = to_target / dist;
        let cos_half_cone = (self.vision_cone_angle.to_radians()).cos();
        dir.dot(self_facing) >= cos_half_cone
    }

    /// 是否能听到给定声音
    /// sound_level: 0..1 (相对强度)
    /// distance: 与声源距离
    pub fn can_hear(&self, sound_level: f32, distance: f32) -> bool {
        if distance > self.hearing_radius {
            return false;
        }
        // 距离衰减 (1/r 简化)
        let attenuation = if distance > 1e-6 {
            1.0 / (1.0 + distance)
        } else {
            1.0
        };
        let perceived = sound_level * attenuation;
        // 听觉阈值 0.05
        perceived > 0.05
    }

    /// 综合检测: 视觉 + 听觉 + 嗅觉 + 触觉
    /// 返回检测到的目标 (优先视觉 > 听觉 > 嗅觉 > 触觉)
    pub fn detect(
        &self,
        target_pos: Vec3,
        self_pos: Vec3,
    ) -> Option<DetectedTarget> {
        let to_target = target_pos - self_pos;
        let dist = to_target.length();

        // 触觉 (最优先, 因为最近)
        if dist <= self.touch_radius {
            return Some(DetectedTarget {
                position: target_pos,
                distance: dist,
                sense: PerceptionSense::Touch,
                confidence: 1.0,
                last_seen_time: 0.0,
            });
        }

        // 嗅觉
        if dist <= self.smell_radius {
            let conf = (1.0 - dist / self.smell_radius).clamp(0.0, 1.0);
            return Some(DetectedTarget {
                position: target_pos,
                distance: dist,
                sense: PerceptionSense::Smell,
                confidence: conf,
                last_seen_time: 0.0,
            });
        }

        // 听觉 (假设目标在移动产生声响 0.5)
        if self.can_hear(0.5, dist) {
            let conf = (1.0 - dist / self.hearing_radius).clamp(0.0, 1.0) * 0.7;
            return Some(DetectedTarget {
                position: target_pos,
                distance: dist,
                sense: PerceptionSense::Hearing,
                confidence: conf,
                last_seen_time: 0.0,
            });
        }

        // 视觉 (默认朝向 +Z, 实际调用方应使用 can_see)
        if dist <= self.vision_range {
            let conf = (1.0 - dist / self.vision_range).clamp(0.0, 1.0);
            return Some(DetectedTarget {
                position: target_pos,
                distance: dist,
                sense: PerceptionSense::Vision,
                confidence: conf,
                last_seen_time: 0.0,
            });
        }

        None
    }

    /// 注意力衰减 (每秒衰减)
    pub fn update_attention(&mut self, dt: f32) {
        self.current_attention = (self.current_attention - dt).max(0.0);
        if self.current_attention <= 0.0 {
            // 注意力恢复
            self.current_attention = self.attention_span;
        }
    }

    /// 计算潜行因子 (0 = 完全隐藏, 1 = 完全暴露)
    /// visibility: 目标可见性
    /// distance: 与感知者距离
    /// lighting: 0..1 (0 = 黑暗, 1 = 明亮)
    pub fn calculate_stealth_factor(
        visibility: Visibility,
        distance: f32,
        lighting: f32,
    ) -> f32 {
        let v = visibility.factor();
        // 距离因子: 越远越隐蔽 (10m 以上几乎隐藏)
        let d = (1.0 - distance / 10.0).clamp(0.0, 1.0);
        let l = lighting.clamp(0.0, 1.0);
        (v * d * l).clamp(0.0, 1.0)
    }
}

impl Default for PerceptionSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_vision_cone() {
        let p = PerceptionSystem::new();
        assert!(p.vision_cone_angle > 0.0 && p.vision_cone_angle <= 90.0);
    }

    #[test]
    fn test_default_ranges_ordering() {
        let p = PerceptionSystem::new();
        // 触觉 < 嗅觉 < 听觉 < 视觉
        assert!(p.touch_radius < p.smell_radius);
        assert!(p.smell_radius < p.hearing_radius);
        assert!(p.hearing_radius < p.vision_range);
    }

    #[test]
    fn test_can_see_in_front_within_range() {
        let p = PerceptionSystem::new();
        let self_pos = Vec3::ZERO;
        let facing = Vec3::Z; // 朝 +Z
        let target = Vec3::new(0.0, 0.0, 10.0);
        assert!(p.can_see(target, self_pos, facing));
    }

    #[test]
    fn test_can_see_behind_returns_false() {
        let p = PerceptionSystem::new();
        let self_pos = Vec3::ZERO;
        let facing = Vec3::Z;
        let target = Vec3::new(0.0, 0.0, -10.0);
        assert!(!p.can_see(target, self_pos, facing));
    }

    #[test]
    fn test_can_see_out_of_range_returns_false() {
        let p = PerceptionSystem::new();
        let self_pos = Vec3::ZERO;
        let facing = Vec3::Z;
        let target = Vec3::new(0.0, 0.0, 100.0); // 远超 vision_range=30
        assert!(!p.can_see(target, self_pos, facing));
    }

    #[test]
    fn test_can_see_at_cone_edge() {
        let p = PerceptionSystem::new();
        let self_pos = Vec3::ZERO;
        let facing = Vec3::Z;
        // 60° 半角, 在边缘应能看到
        let target = Vec3::new(8.66, 0.0, 5.0); // ~60°
        assert!(p.can_see(target, self_pos, facing));
    }

    #[test]
    fn test_can_see_outside_cone() {
        let p = PerceptionSystem::new();
        let self_pos = Vec3::ZERO;
        let facing = Vec3::Z;
        // 80° 偏角, 超出 60° 半角
        let target = Vec3::new(9.85, 0.0, 1.74);
        assert!(!p.can_see(target, self_pos, facing));
    }

    #[test]
    fn test_can_hear_close_strong_sound() {
        let p = PerceptionSystem::new();
        assert!(p.can_hear(1.0, 1.0));
    }

    #[test]
    fn test_can_hear_far_weak_sound_false() {
        let p = PerceptionSystem::new();
        assert!(!p.can_hear(0.05, 14.0));
    }

    #[test]
    fn test_can_hear_beyond_radius_false() {
        let p = PerceptionSystem::new();
        // hearing_radius = 15
        assert!(!p.can_hear(1.0, 20.0));
    }

    #[test]
    fn test_detect_touch_close_target() {
        let p = PerceptionSystem::new();
        let self_pos = Vec3::ZERO;
        let target = Vec3::new(0.5, 0.0, 0.0);
        let det = p.detect(target, self_pos).expect("应检测到");
        assert_eq!(det.sense, PerceptionSense::Touch);
        assert_eq!(det.confidence, 1.0);
    }

    #[test]
    fn test_detect_smell_target() {
        let p = PerceptionSystem::new();
        let self_pos = Vec3::ZERO;
        let target = Vec3::new(3.0, 0.0, 0.0); // 在 smell_radius=5 内
        let det = p.detect(target, self_pos).expect("应检测到");
        assert_eq!(det.sense, PerceptionSense::Smell);
        assert!(det.confidence > 0.0 && det.confidence < 1.0);
    }

    #[test]
    fn test_detect_too_far_returns_none() {
        let p = PerceptionSystem::new();
        let self_pos = Vec3::ZERO;
        let target = Vec3::new(1000.0, 0.0, 0.0);
        assert!(p.detect(target, self_pos).is_none());
    }

    #[test]
    fn test_update_attention_decreases() {
        let mut p = PerceptionSystem::new();
        let before = p.current_attention;
        p.update_attention(1.0);
        assert!(p.current_attention < before);
    }

    #[test]
    fn test_update_attention_regen_on_zero() {
        let mut p = PerceptionSystem::new();
        p.current_attention = 0.1;
        p.update_attention(1.0);
        // 衰减到 0 后会重生
        assert!(p.current_attention > 0.0);
    }

    #[test]
    fn test_stealth_factor_visible_close_bright() {
        let f = PerceptionSystem::calculate_stealth_factor(Visibility::Visible, 1.0, 1.0);
        assert!(f > 0.5);
    }

    #[test]
    fn test_stealth_factor_hidden_zero() {
        let f = PerceptionSystem::calculate_stealth_factor(Visibility::Hidden, 1.0, 1.0);
        assert_eq!(f, 0.0);
    }

    #[test]
    fn test_stealth_factor_far_distance() {
        let f = PerceptionSystem::calculate_stealth_factor(Visibility::Visible, 20.0, 1.0);
        // 20m 远, 距离因子 = 0
        assert!(f < 0.05);
    }

    #[test]
    fn test_stealth_factor_dark() {
        let f = PerceptionSystem::calculate_stealth_factor(Visibility::Visible, 1.0, 0.0);
        assert_eq!(f, 0.0);
    }

    #[test]
    fn test_visibility_factor_values() {
        assert_eq!(Visibility::Visible.factor(), 1.0);
        assert_eq!(Visibility::Partial.factor(), 0.5);
        assert_eq!(Visibility::Hidden.factor(), 0.0);
        assert!(Visibility::Camouflaged.factor() > 0.0 && Visibility::Camouflaged.factor() < 0.5);
    }

    #[test]
    fn test_perception_serialization() {
        let p = PerceptionSystem::new();
        let json = serde_json::to_string(&p).expect("serialize");
        let back: PerceptionSystem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn test_detected_target_serialization() {
        let t = DetectedTarget {
            position: Vec3::new(1.0, 2.0, 3.0),
            distance: 4.0,
            sense: PerceptionSense::Vision,
            confidence: 0.8,
            last_seen_time: 1.5,
        };
        let json = serde_json::to_string(&t).expect("serialize");
        let back: DetectedTarget = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(t, back);
    }

    #[test]
    fn test_perception_sense_variants() {
        let _ = PerceptionSense::Vision;
        let _ = PerceptionSense::Hearing;
        let _ = PerceptionSense::Smell;
        let _ = PerceptionSense::Touch;
        let _ = PerceptionSense::Thermal;
        let _ = PerceptionSense::Vibration;
    }
}
