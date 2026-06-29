//! 感觉系统模块 — 多模态感觉受体建模
//!
//! 科学来源:
//! - Goldstein, E. B. (2017). "Sensation and Perception", 10th edition. Cengage Learning.
//! - Kandel et al. (2012). "Principles of Neural Science", 5th ed., Ch. 21-26.
//! - Bear, M. F., Connors, B. W., Paradiso, M. A. (2020). "Neuroscience: Exploring
//!   the Brain", 4th edition.
//! - Hecht, S., Shlaer, S., Pirenne, M. H. (1942). "Energy, quanta, and vision."
//!   J. Gen. Physiol. 25(6): 819-840. (绝对视觉阈值 ~ 5-14 光子)
//!
//! 主要现象:
//!   - 视觉: 视杆细胞暗适应阈值 ~ 0.001 lux; 明视觉 380-740 nm; 视野约 200° (单眼)
//!   - 听觉: 20-20000 Hz; 0 dB SPL (20 uPa) 阈值; 立体声定位 1-2° 精度
//!   - 嗅觉: 人类 ~ 400 种功能嗅觉受体基因 (Buck & Axel 1991, 2004 Nobel)
//!   - 感觉适应: 幂函数 I(t) = I0 * t^(-alpha)

use serde::{Deserialize, Serialize};

/// 感觉模态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensoryModality {
    Vision,
    Hearing,
    Smell,
    Taste,
    Touch,
    Proprioception,
    Temperature,
    Pain,
    Balance,
}

/// 通用感觉受体 — 阈值检测 + 适应
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensoryReceptor {
    pub modality: SensoryModality,
    /// 最大敏感度 (单位取决于模态, 无量纲归一化)
    pub sensitivity: f32,
    /// 绝对阈值 (刺激强度), 低于此值不响应
    pub threshold: f32,
    /// 适应速率 (1/s); 越大适应越快
    pub adaptation_rate: f32,
    /// 当前输出信号 (0..1)
    pub current_signal: f32,
}

impl SensoryReceptor {
    pub fn new(modality: SensoryModality) -> Self {
        let (sensitivity, threshold, adaptation_rate) = match modality {
            SensoryModality::Vision => (1.0, 0.001, 0.05),
            SensoryModality::Hearing => (1.0, 0.00002, 0.02),
            SensoryModality::Smell => (0.8, 0.01, 0.1),
            SensoryModality::Taste => (0.7, 0.05, 0.15),
            SensoryModality::Touch => (1.0, 0.001, 0.3),
            SensoryModality::Proprioception => (0.9, 0.0001, 0.01),
            SensoryModality::Temperature => (0.85, 0.1, 0.08),
            SensoryModality::Pain => (1.0, 0.5, 0.005),
            SensoryModality::Balance => (0.95, 0.0005, 0.02),
        };
        Self {
            modality,
            sensitivity,
            threshold,
            adaptation_rate,
            current_signal: 0.0,
        }
    }

    /// 给予刺激, 返回是否被检测到
    pub fn detect(&mut self, stimulus_intensity: f32) -> bool {
        if stimulus_intensity < self.threshold {
            self.current_signal = 0.0;
            return false;
        }
        let over = stimulus_intensity - self.threshold;
        let signal = (self.sensitivity * over / (over + self.threshold)).clamp(0.0, 1.0);
        self.current_signal = signal;
        signal > 0.0
    }

    /// 时间步进适应 (Weber-Fechner 衰减)
    pub fn adapt(&mut self, dt: f32) {
        let decay = (-self.adaptation_rate * dt).exp();
        self.current_signal *= decay;
        if self.current_signal < 1e-5 {
            self.current_signal = 0.0;
        }
    }
}

/// 视觉系统 (人眼参数)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualSystem {
    /// 视锐度 (LogMAR, 0 = 20/20 标准)
    pub visual_acuity: f32,
    /// 是否有色觉
    pub color_detection: bool,
    /// 暗视阈值 (lux)
    pub low_light_threshold: f32,
    /// 单眼水平视野 (度)
    pub fov_degrees: f32,
    /// 运动检测能力 (0..1)
    pub motion_detection: f32,
}

impl VisualSystem {
    pub fn new() -> Self {
        Self {
            visual_acuity: 0.0,
            color_detection: true,
            low_light_threshold: 0.001,
            fov_degrees: 200.0,
            motion_detection: 0.95,
        }
    }

    /// 是否能在给定照度下看到 (低于阈值返回 false)
    pub fn vision_detect_low_light(&self, lux: f32) -> bool {
        lux >= self.low_light_threshold
    }
}

impl Default for VisualSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// 听觉系统 (人耳参数)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditorySystem {
    /// 最低可听频率 (Hz)
    pub frequency_range_min: f32,
    /// 最高可听频率 (Hz)
    pub frequency_range_max: f32,
    /// 听阈 (dB SPL)
    pub decibel_threshold: f32,
    /// 声源定位精度 (度)
    pub sound_localization: f32,
}

impl AuditorySystem {
    pub fn new() -> Self {
        Self {
            frequency_range_min: 20.0,
            frequency_range_max: 20000.0,
            decibel_threshold: 0.0,
            sound_localization: 1.5,
        }
    }

    /// 是否能听到给定频率
    pub fn hearing_detect_frequency(&self, hz: f32) -> bool {
        hz >= self.frequency_range_min && hz <= self.frequency_range_max
    }
}

impl Default for AuditorySystem {
    fn default() -> Self {
        Self::new()
    }
}

/// 嗅觉系统
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OlfactorySystem {
    /// 嗅觉受体类型数 (人类 ~ 400)
    pub receptor_count: u32,
    /// 敏感度 (0..1)
    pub sensitivity: f32,
    /// 气味分辨阈值 (无量纲)
    pub discrimination_threshold: f32,
}

impl OlfactorySystem {
    pub fn new() -> Self {
        Self {
            receptor_count: 400,
            sensitivity: 0.7,
            discrimination_threshold: 0.05,
        }
    }
}

impl Default for OlfactorySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_system_defaults() {
        let v = VisualSystem::new();
        assert_eq!(v.visual_acuity, 0.0);
        assert!(v.color_detection);
        assert_eq!(v.low_light_threshold, 0.001);
        assert_eq!(v.fov_degrees, 200.0);
    }

    #[test]
    fn test_auditory_system_defaults() {
        let a = AuditorySystem::new();
        assert_eq!(a.frequency_range_min, 20.0);
        assert_eq!(a.frequency_range_max, 20000.0);
        assert_eq!(a.decibel_threshold, 0.0);
    }

    #[test]
    fn test_olfactory_system_defaults() {
        let o = OlfactorySystem::new();
        assert_eq!(o.receptor_count, 400);
        assert!(o.sensitivity > 0.0 && o.sensitivity <= 1.0);
    }

    #[test]
    fn test_receptor_vision_defaults() {
        let r = SensoryReceptor::new(SensoryModality::Vision);
        assert!(r.sensitivity > 0.0);
        assert!(r.threshold > 0.0);
    }

    #[test]
    fn test_receptor_pain_high_threshold() {
        let pain = SensoryReceptor::new(SensoryModality::Pain);
        let touch = SensoryReceptor::new(SensoryModality::Touch);
        assert!(pain.threshold > touch.threshold);
    }

    #[test]
    fn test_detect_below_threshold() {
        let mut r = SensoryReceptor::new(SensoryModality::Touch);
        assert!(!r.detect(0.0001));
        assert_eq!(r.current_signal, 0.0);
    }

    #[test]
    fn test_detect_above_threshold() {
        let mut r = SensoryReceptor::new(SensoryModality::Touch);
        r.threshold = 0.01;
        assert!(r.detect(1.0));
        assert!(r.current_signal > 0.0);
    }

    #[test]
    fn test_detect_signal_bounded() {
        let mut r = SensoryReceptor::new(SensoryModality::Vision);
        r.detect(1e9);
        assert!(r.current_signal <= 1.0);
    }

    #[test]
    fn test_adapt_decays_signal() {
        let mut r = SensoryReceptor::new(SensoryModality::Vision);
        r.detect(1.0);
        let before = r.current_signal;
        r.adapt(1.0);
        assert!(r.current_signal < before);
    }

    #[test]
    fn test_adapt_zeroes_tiny_signal() {
        let mut r = SensoryReceptor::new(SensoryModality::Vision);
        r.current_signal = 1e-6;
        r.adapt(1.0);
        assert_eq!(r.current_signal, 0.0);
    }

    #[test]
    fn test_vision_detect_low_light_above() {
        let v = VisualSystem::new();
        assert!(v.vision_detect_low_light(0.01));
    }

    #[test]
    fn test_vision_detect_low_light_below() {
        let v = VisualSystem::new();
        assert!(!v.vision_detect_low_light(0.0001));
    }

    #[test]
    fn test_vision_detect_low_light_at_threshold() {
        let v = VisualSystem::new();
        assert!(v.vision_detect_low_light(0.001));
    }

    #[test]
    fn test_hearing_detect_in_range() {
        let a = AuditorySystem::new();
        assert!(a.hearing_detect_frequency(440.0));
        assert!(a.hearing_detect_frequency(20.0));
        assert!(a.hearing_detect_frequency(20000.0));
    }

    #[test]
    fn test_hearing_detect_out_of_range() {
        let a = AuditorySystem::new();
        assert!(!a.hearing_detect_frequency(10.0));
        assert!(!a.hearing_detect_frequency(30000.0));
    }

    #[test]
    fn test_modality_variants_exist() {
        let _ = SensoryModality::Vision;
        let _ = SensoryModality::Hearing;
        let _ = SensoryModality::Smell;
        let _ = SensoryModality::Taste;
        let _ = SensoryModality::Touch;
        let _ = SensoryModality::Proprioception;
        let _ = SensoryModality::Temperature;
        let _ = SensoryModality::Pain;
        let _ = SensoryModality::Balance;
    }

    #[test]
    fn test_receptor_thresholds_distinct() {
        let v = SensoryReceptor::new(SensoryModality::Vision).threshold;
        let p = SensoryReceptor::new(SensoryModality::Pain).threshold;
        assert_ne!(v, p);
    }

    #[test]
    fn test_adaptation_rate_positive() {
        for m in [
            SensoryModality::Vision,
            SensoryModality::Hearing,
            SensoryModality::Smell,
            SensoryModality::Taste,
            SensoryModality::Touch,
            SensoryModality::Temperature,
            SensoryModality::Pain,
        ] {
            let r = SensoryReceptor::new(m);
            assert!(r.adaptation_rate > 0.0, "{:?} 适应率必须 > 0", m);
        }
    }

    #[test]
    fn test_visual_system_serialization() {
        let v = VisualSystem::new();
        let json = serde_json::to_string(&v).expect("serialize");
        let back: VisualSystem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(v, back);
    }

    #[test]
    fn test_auditory_system_serialization() {
        let a = AuditorySystem::new();
        let json = serde_json::to_string(&a).expect("serialize");
        let back: AuditorySystem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(a, back);
    }

    #[test]
    fn test_receptor_serialization() {
        let r = SensoryReceptor::new(SensoryModality::Touch);
        let json = serde_json::to_string(&r).expect("serialize");
        let back: SensoryReceptor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }

    #[test]
    fn test_pain_low_adaptation_rate() {
        let pain = SensoryReceptor::new(SensoryModality::Pain);
        // 痛觉适应缓慢 (避免忽视伤害)
        assert!(pain.adaptation_rate < 0.05);
    }

    #[test]
    fn test_touch_high_adaptation_rate() {
        let touch = SensoryReceptor::new(SensoryModality::Touch);
        // 触觉适应迅速 (衣物适应)
        assert!(touch.adaptation_rate > 0.1);
    }
}
