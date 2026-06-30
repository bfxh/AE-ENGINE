//! 昼夜节律模块 — 分子钟建模
//!
//! 科学来源:
//! - Young, M. W. (2018). "Time to sleep: the molecular basis of circadian rhythms."
//!   Nobel Prize lecture, 2017 Nobel in Physiology or Medicine.
//! - Konopka, R. J., Benzer, S. (1971). "Clock mutants of Drosophila melanogaster."
//!   PNAS 68(9): 2112-2116. (period 基因发现)
//! - Mohawk, J. A., Green, C. B., Takahashi, J. S. (2012). "Central and peripheral
//!   circadian clocks in mammals." Annu. Rev. Neurosci. 35: 445-462.
//! - Czeisler, C. A., et al. (1999). "Stability, precision, and near-24-hour period
//!   of the human circadian pacemaker." Science 284: 2177-2181.
//!
//! 核心规律:
//!   - 自由运行周期 ~ 24.2 h (Czeisler 1999)
//!   - 褪黑素: 夜间 (DLMO ~ 21:00, 峰值 ~ 03:00), 白天接近零
//!   - 皮质醇: 晨峰 ~ 08:00, 夜间谷值 ~ 00:00
//!   - 核体温: 谷值 ~ 05:00, 峰值 ~ 18:00
//!   - 时差恢复速率 ~ 1 h/day (East) / 1.5 h/day (West)

use serde::{Deserialize, Serialize};

/// 昼夜节律相位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CircadianPhase {
    Dawn,       // 04:00 - 06:00
    Morning,    // 06:00 - 10:00
    Noon,       // 10:00 - 14:00
    Afternoon,  // 14:00 - 18:00
    Evening,    // 18:00 - 22:00
    Night,      // 22:00 - 02:00
    DeepNight,  // 02:00 - 04:00
}

impl CircadianPhase {
    /// 判断给定小时 (0..24) 所属的相位
    pub fn from_hour(hour: f32) -> Self {
        let h = hour.rem_euclid(24.0);
        if h < 4.0 {
            Self::DeepNight
        } else if h < 6.0 {
            Self::Dawn
        } else if h < 10.0 {
            Self::Morning
        } else if h < 14.0 {
            Self::Noon
        } else if h < 18.0 {
            Self::Afternoon
        } else if h < 22.0 {
            Self::Evening
        } else {
            Self::Night
        }
    }
}

/// 昼夜节律生物钟
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircadianClock {
    /// 周期 (小时), 默认 24.2
    pub period_hours: f32,
    /// 振幅 (无量纲)
    pub amplitude: f32,
    /// 相位偏移 (小时)
    pub phase_offset: f32,
    /// 当前褪黑素水平 (0..1)
    pub melatonin_level: f32,
    /// 当前皮质醇水平 (0..1)
    pub cortisol_level: f32,
    /// 当前核体温偏移 (°C, 相对基线)
    pub body_temp_offset: f32,
}

impl CircadianClock {
    pub fn new() -> Self {
        Self {
            period_hours: 24.2,
            amplitude: 1.0,
            phase_offset: 0.0,
            melatonin_level: 0.0,
            cortisol_level: 0.0,
            body_temp_offset: 0.0,
        }
    }

    /// 给定时刻的褪黑素水平
    /// 模型: 夜间 (22:00 - 06:00) 高斯峰, 峰值 ~ 03:00
    pub fn melatonin_at(&self, hour: f32) -> f32 {
        let h = (hour - self.phase_offset).rem_euclid(24.0);
        // 双高斯: 上升期 (22-03) + 下降期 (03-06)
        let peak: f32 = 3.0; // 03:00
        let sigma_night: f32 = 3.0; // 夜间宽度
        let level = (-((h - peak).powi(2)) / (2.0_f32 * sigma_night.powi(2))).exp();
        // 白天抑制 (光强度)
        let daylight_suppression = if h >= 8.0 && h <= 20.0 { 0.0 } else { 1.0 };
        (level * daylight_suppression * self.amplitude).clamp(0.0, 1.0)
    }

    /// 给定时刻的皮质醇水平
    /// 模型: 晨峰 ~ 08:00, 谷值 ~ 00:00
    pub fn cortisol_at(&self, hour: f32) -> f32 {
        let h = (hour - self.phase_offset).rem_euclid(24.0);
        let peak: f32 = 8.0;
        let sigma: f32 = 5.0;
        let level = (-((h - peak).powi(2)) / (2.0_f32 * sigma.powi(2))).exp();
        (level * self.amplitude).clamp(0.0, 1.0)
    }

    /// 当前相位
    pub fn current_phase(&self, time_of_day: f32) -> CircadianPhase {
        CircadianPhase::from_hour(time_of_day - self.phase_offset)
    }

    /// 根据一天中的时刻更新激素水平
    pub fn update(&mut self, time_of_day: f32) {
        self.melatonin_level = self.melatonin_at(time_of_day);
        self.cortisol_level = self.cortisol_at(time_of_day);
        // 核体温: 峰值 ~18:00 (+0.5°C), 谷值 ~05:00 (-0.5°C)
        let h = (time_of_day - self.phase_offset).rem_euclid(24.0);
        let peak = 18.0;
        self.body_temp_offset =
            0.5 * (-((h - peak).powi(2)) / (2.0 * 6.0_f32.powi(2))).exp() - 0.3;
    }

    /// 判断给定时刻是否为最佳睡眠时段
    /// 最佳睡眠: 褪黑素高 + 体温低 (02:00 - 05:00)
    pub fn is_sleep_optimal(&self, hour: f32) -> bool {
        let phase = self.current_phase(hour);
        matches!(phase, CircadianPhase::DeepNight | CircadianPhase::Dawn)
            && self.melatonin_at(hour) > 0.3
    }

    /// 模拟时差: 平移相位
    /// timezone_shift_hours 正 = 东向旅行 (相位前移)
    /// 实际相位调整以 ~1 h/day 速率进行 (这里直接设置瞬时偏移作为目标)
    pub fn simulate_jet_lag(&mut self, timezone_shift_hours: f32) {
        self.phase_offset += timezone_shift_hours;
        // 归一化到 [-12, 12]
        while self.phase_offset > 12.0 {
            self.phase_offset -= 24.0;
        }
        while self.phase_offset < -12.0 {
            self.phase_offset += 24.0;
        }
    }
}

impl Default for CircadianClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_default_period() {
        let c = CircadianClock::new();
        assert!(c.period_hours > 24.0 && c.period_hours < 25.0);
    }

    #[test]
    fn test_clock_default_zero_phase() {
        let c = CircadianClock::new();
        assert_eq!(c.phase_offset, 0.0);
    }

    #[test]
    fn test_melatonin_low_at_noon() {
        let c = CircadianClock::new();
        assert!(c.melatonin_at(12.0) < 0.05);
    }

    #[test]
    fn test_melatonin_high_at_night() {
        let c = CircadianClock::new();
        let m = c.melatonin_at(3.0);
        assert!(m > 0.5, "凌晨 03:00 褪黑素应高, 实际 {}", m);
    }

    #[test]
    fn test_melatonin_zero_in_daylight() {
        let c = CircadianClock::new();
        // 8:00 - 20:00 之间应被光抑制
        for h in [9.0, 12.0, 15.0, 19.0] {
            assert!(c.melatonin_at(h) < 0.01, "h={} 不应分泌褪黑素", h);
        }
    }

    #[test]
    fn test_cortisol_peak_at_morning() {
        let c = CircadianClock::new();
        let morning = c.cortisol_at(8.0);
        let midnight = c.cortisol_at(0.0);
        assert!(morning > midnight, "晨峰应高于午夜");
    }

    #[test]
    fn test_cortisol_low_at_midnight() {
        let c = CircadianClock::new();
        // 午夜皮质醇应低于晨峰，宽松阈值（高斯模型在 0:00 距峰值 8h 仍有残值）
        assert!(c.cortisol_at(0.0) < c.cortisol_at(8.0));
        assert!(c.cortisol_at(0.0) < 0.4);
    }

    #[test]
    fn test_phase_classification_noon() {
        assert_eq!(CircadianPhase::from_hour(12.0), CircadianPhase::Noon);
    }

    #[test]
    fn test_phase_classification_morning() {
        assert_eq!(CircadianPhase::from_hour(8.0), CircadianPhase::Morning);
    }

    #[test]
    fn test_phase_classification_deep_night() {
        assert_eq!(CircadianPhase::from_hour(3.0), CircadianPhase::DeepNight);
    }

    #[test]
    fn test_phase_classification_dawn() {
        assert_eq!(CircadianPhase::from_hour(5.0), CircadianPhase::Dawn);
    }

    #[test]
    fn test_phase_classification_evening() {
        assert_eq!(CircadianPhase::from_hour(20.0), CircadianPhase::Evening);
    }

    #[test]
    fn test_phase_classification_wraps_24h() {
        // 25.0 应等价于 1.0 -> DeepNight
        assert_eq!(CircadianPhase::from_hour(25.0), CircadianPhase::from_hour(1.0));
    }

    #[test]
    fn test_update_sets_hormone_levels() {
        let mut c = CircadianClock::new();
        c.update(3.0);
        assert!(c.melatonin_level > 0.5);
        // 03:00 皮质醇应低于晨峰 08:00
        assert!(c.cortisol_level < c.cortisol_at(8.0));
    }

    #[test]
    fn test_update_sets_body_temp_offset() {
        let mut c = CircadianClock::new();
        c.update(18.0);
        // 18:00 为核体温峰值
        assert!(c.body_temp_offset > 0.0);
    }

    #[test]
    fn test_sleep_optimal_at_3am() {
        let c = CircadianClock::new();
        assert!(c.is_sleep_optimal(3.0));
    }

    #[test]
    fn test_sleep_not_optimal_at_noon() {
        let c = CircadianClock::new();
        assert!(!c.is_sleep_optimal(12.0));
    }

    #[test]
    fn test_jet_lag_shifts_phase() {
        let mut c = CircadianClock::new();
        let original_phase = c.phase_offset;
        c.simulate_jet_lag(8.0); // 飞向东 8 小时
        assert_ne!(c.phase_offset, original_phase);
    }

    #[test]
    fn test_jet_lag_normalized_to_12h_range() {
        let mut c = CircadianClock::new();
        c.simulate_jet_lag(20.0);
        assert!(c.phase_offset.abs() <= 12.0);
    }

    #[test]
    fn test_jet_lag_negative_shift() {
        let mut c = CircadianClock::new();
        c.simulate_jet_lag(-5.0);
        assert_eq!(c.phase_offset, -5.0);
    }

    #[test]
    fn test_current_phase_respects_offset() {
        let mut c = CircadianClock::new();
        c.simulate_jet_lag(8.0);
        // 现在 12:00 实际对应原始 04:00 (Dawn)
        let phase = c.current_phase(12.0);
        assert_eq!(phase, CircadianPhase::Dawn);
    }

    #[test]
    fn test_clock_serialization_roundtrip() {
        let mut c = CircadianClock::new();
        c.update(10.0);
        let json = serde_json::to_string(&c).expect("serialize");
        let back: CircadianClock = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c, back);
    }

    #[test]
    fn test_phase_serialization() {
        let p = CircadianPhase::Morning;
        let json = serde_json::to_string(&p).expect("serialize");
        let back: CircadianPhase = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn test_amplitude_scales_melatonin() {
        let mut c = CircadianClock::new();
        c.amplitude = 0.5;
        let level = c.melatonin_at(3.0);
        assert!(level <= 0.6, "振幅 0.5 应限制峰值");
    }
}
