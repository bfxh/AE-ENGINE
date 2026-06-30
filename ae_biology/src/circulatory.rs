//! 循环系统模块
//!
//! 基于: Guyton & Hall, Textbook of Medical Physiology (14th Edition)
//! 参考: Frank-Starling 心脏定律、Ohm 定律应用于血流动力学
//! 单位约定: 心率 bpm, 血压 mmHg, 心输出量 L/min, 血容量 L,
//!           血管阻力 mmHg·min/L, 每搏输出量 ml
//!
//! 核心公式:
//!   - 平均动脉压 MAP = DBP + (SBP - DBP) / 3
//!   - 脉压 PP = SBP - DBP
//!   - 心输出量 CO = HR × SV / 1000 (L/min)
//!   - 血管阻力 TPR = MAP / CO

use serde::{Deserialize, Serialize};

/// 血压分级 (依据 ACC/AHA 2017 指南)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BloodPressureCategory {
    /// 低血压 SBP<90 或 DBP<60
    Hypotension,
    /// 正常 SBP<120 且 DBP<80
    Normal,
    /// 升高 SBP 120-129 且 DBP<80
    Elevated,
    /// 1期高血压 SBP 130-139 或 DBP 80-89
    Stage1Hypertension,
    /// 2期高血压 SBP>=140 或 DBP>=90
    Stage2Hypertension,
    /// 高血压危象 SBP>=180 或 DBP>=120
    HypertensiveCrisis,
}

/// 循环系统状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CirculatorySystem {
    /// 心率 (beats per minute)
    pub heart_rate: f32,
    /// 收缩压 (mmHg)
    pub blood_pressure_systolic: f32,
    /// 舒张压 (mmHg)
    pub blood_pressure_diastolic: f32,
    /// 心输出量 (L/min)
    pub cardiac_output: f32,
    /// 总血容量 (L)
    pub blood_volume: f32,
    /// 总外周血管阻力 (mmHg·min/L)
    pub total_vascular_resistance: f32,
    /// 每搏输出量 (ml)
    pub stroke_volume: f32,
}

impl CirculatorySystem {
    /// 创建健康成人的默认循环系统
    /// HR=75, BP=120/80, CO=5L/min, BV=5L, SV≈66.7ml
    pub fn new() -> Self {
        Self {
            heart_rate: 75.0,
            blood_pressure_systolic: 120.0,
            blood_pressure_diastolic: 80.0,
            cardiac_output: 5.0,
            blood_volume: 5.0,
            total_vascular_resistance: 18.7,
            stroke_volume: 66.7,
        }
    }

    /// 脉压 = 收缩压 - 舒张压
    pub fn pulse_pressure(&self) -> f32 {
        self.blood_pressure_systolic - self.blood_pressure_diastolic
    }

    /// 平均动脉压 = DBP + PP/3
    pub fn mean_arterial_pressure(&self) -> f32 {
        self.blood_pressure_diastolic + self.pulse_pressure() / 3.0
    }

    /// 依据 ACC/AHA 指南对当前血压进行分级
    pub fn classify_bp(&self) -> BloodPressureCategory {
        let s = self.blood_pressure_systolic;
        let d = self.blood_pressure_diastolic;
        if s >= 180.0 || d >= 120.0 {
            BloodPressureCategory::HypertensiveCrisis
        } else if s >= 140.0 || d >= 90.0 {
            BloodPressureCategory::Stage2Hypertension
        } else if s >= 130.0 || d >= 80.0 {
            BloodPressureCategory::Stage1Hypertension
        } else if s >= 120.0 {
            BloodPressureCategory::Elevated
        } else if s < 90.0 || d < 60.0 {
            BloodPressureCategory::Hypotension
        } else {
            BloodPressureCategory::Normal
        }
    }

    /// 模拟失血 (volume_lost 单位: L)
    /// 失血时: 血容量↓, 每搏输出量↓, 心率代偿性↑, 血压↓
    pub fn simulate_hemorrhage(&mut self, volume_lost: f32) {
        let loss = volume_lost.clamp(0.0, self.blood_volume);
        self.blood_volume -= loss;
        let frac_lost = loss / 5.0; // 相对正常 5L 血容量的比例

        // 静脉回流下降 → 每搏输出量下降 (Frank-Starling)
        self.stroke_volume *= (1.0 - frac_lost * 0.7).max(0.2);

        // 代偿性心动过速
        self.heart_rate = (self.heart_rate + frac_lost * 60.0).clamp(40.0, 200.0);

        // 血压下降
        let drop = frac_lost * 40.0;
        self.blood_pressure_systolic = (self.blood_pressure_systolic - drop).max(40.0);
        self.blood_pressure_diastolic = (self.blood_pressure_diastolic - drop * 0.7).max(30.0);

        // 重算心输出量 CO = HR × SV / 1000
        self.cardiac_output = self.heart_rate * self.stroke_volume / 1000.0;

        // 外周血管阻力代偿性升高
        self.total_vascular_resistance *= 1.0 + frac_lost * 0.3;
    }

    /// 每帧更新: 心率与血容量缓慢回归基线
    pub fn update(&mut self, dt: f32) {
        // 心率向基线 75 漂移
        self.heart_rate += (75.0 - self.heart_rate) * 0.01 * dt;
        // 血容量向 5L 恢复 (组织液重分布)
        self.blood_volume += (5.0 - self.blood_volume) * 0.005 * dt;
        // 重算心输出量
        self.cardiac_output = self.heart_rate * self.stroke_volume / 1000.0;
    }
}

impl Default for CirculatorySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_heart_rate() {
        let sys = CirculatorySystem::new();
        assert_eq!(sys.heart_rate, 75.0);
    }

    #[test]
    fn test_default_systolic() {
        let sys = CirculatorySystem::new();
        assert_eq!(sys.blood_pressure_systolic, 120.0);
    }

    #[test]
    fn test_default_diastolic() {
        let sys = CirculatorySystem::new();
        assert_eq!(sys.blood_pressure_diastolic, 80.0);
    }

    #[test]
    fn test_default_cardiac_output() {
        let sys = CirculatorySystem::new();
        assert_eq!(sys.cardiac_output, 5.0);
    }

    #[test]
    fn test_default_blood_volume() {
        let sys = CirculatorySystem::new();
        assert_eq!(sys.blood_volume, 5.0);
    }

    #[test]
    fn test_default_stroke_volume() {
        let sys = CirculatorySystem::new();
        assert!((sys.stroke_volume - 66.7).abs() < 0.1);
    }

    #[test]
    fn test_pulse_pressure_default() {
        let sys = CirculatorySystem::new();
        assert_eq!(sys.pulse_pressure(), 40.0);
    }

    #[test]
    fn test_mean_arterial_pressure_formula() {
        let sys = CirculatorySystem::new();
        // MAP = 80 + 40/3 = 93.333
        let map = sys.mean_arterial_pressure();
        assert!((map - 93.333).abs() < 0.01);
    }

    #[test]
    fn test_classify_normal() {
        let mut sys = CirculatorySystem::new();
        sys.blood_pressure_systolic = 110.0;
        sys.blood_pressure_diastolic = 70.0;
        assert_eq!(sys.classify_bp(), BloodPressureCategory::Normal);
    }

    #[test]
    fn test_classify_hypotension() {
        let mut sys = CirculatorySystem::new();
        sys.blood_pressure_systolic = 85.0;
        sys.blood_pressure_diastolic = 55.0;
        assert_eq!(sys.classify_bp(), BloodPressureCategory::Hypotension);
    }

    #[test]
    fn test_classify_elevated() {
        let mut sys = CirculatorySystem::new();
        sys.blood_pressure_systolic = 125.0;
        sys.blood_pressure_diastolic = 78.0;
        assert_eq!(sys.classify_bp(), BloodPressureCategory::Elevated);
    }

    #[test]
    fn test_classify_stage1() {
        let mut sys = CirculatorySystem::new();
        sys.blood_pressure_systolic = 135.0;
        sys.blood_pressure_diastolic = 82.0;
        assert_eq!(sys.classify_bp(), BloodPressureCategory::Stage1Hypertension);
    }

    #[test]
    fn test_classify_stage2() {
        let mut sys = CirculatorySystem::new();
        sys.blood_pressure_systolic = 150.0;
        sys.blood_pressure_diastolic = 95.0;
        assert_eq!(sys.classify_bp(), BloodPressureCategory::Stage2Hypertension);
    }

    #[test]
    fn test_classify_crisis() {
        let mut sys = CirculatorySystem::new();
        sys.blood_pressure_systolic = 185.0;
        sys.blood_pressure_diastolic = 125.0;
        assert_eq!(sys.classify_bp(), BloodPressureCategory::HypertensiveCrisis);
    }

    #[test]
    fn test_hemorrhage_reduces_blood_volume() {
        let mut sys = CirculatorySystem::new();
        sys.simulate_hemorrhage(1.0);
        assert!((sys.blood_volume - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_hemorrhage_increases_heart_rate() {
        let mut sys = CirculatorySystem::new();
        let before = sys.heart_rate;
        sys.simulate_hemorrhage(1.0);
        assert!(sys.heart_rate > before);
    }

    #[test]
    fn test_hemorrhage_reduces_blood_pressure() {
        let mut sys = CirculatorySystem::new();
        let before_s = sys.blood_pressure_systolic;
        sys.simulate_hemorrhage(1.5);
        assert!(sys.blood_pressure_systolic < before_s);
    }

    #[test]
    fn test_hemorrhage_reduces_cardiac_output() {
        let mut sys = CirculatorySystem::new();
        sys.simulate_hemorrhage(2.0);
        assert!(sys.cardiac_output < 5.0);
    }

    #[test]
    fn test_hemorrhage_zero_no_change() {
        let mut sys = CirculatorySystem::new();
        let before = sys.blood_volume;
        sys.simulate_hemorrhage(0.0);
        assert_eq!(sys.blood_volume, before);
    }

    #[test]
    fn test_hemorrhage_clamped_at_volume() {
        let mut sys = CirculatorySystem::new();
        sys.simulate_hemorrhage(100.0);
        // 不能失血超过总血量
        assert!(sys.blood_volume >= 0.0);
    }

    #[test]
    fn test_update_drifts_heart_rate_to_baseline() {
        let mut sys = CirculatorySystem::new();
        sys.heart_rate = 120.0;
        sys.update(60.0);
        assert!(sys.heart_rate < 120.0);
    }

    #[test]
    fn test_update_recovers_blood_volume() {
        let mut sys = CirculatorySystem::new();
        sys.blood_volume = 4.0;
        sys.update(60.0);
        assert!(sys.blood_volume > 4.0);
    }

    #[test]
    fn test_severe_hemorrhage_triggers_hypotension() {
        let mut sys = CirculatorySystem::new();
        sys.simulate_hemorrhage(3.0);
        // 失血 3L (60% 血容量) 后血压应显著下降
        assert!(sys.blood_pressure_systolic < 100.0);
        assert!(sys.blood_pressure_diastolic < 70.0);
    }

    #[test]
    fn test_total_vascular_resistance_increases_on_hemorrhage() {
        let mut sys = CirculatorySystem::new();
        let before = sys.total_vascular_resistance;
        sys.simulate_hemorrhage(1.0);
        assert!(sys.total_vascular_resistance > before);
    }

    #[test]
    fn test_serialization_round_trip() {
        let sys = CirculatorySystem::new();
        let json = serde_json::to_string(&sys).unwrap();
        let restored: CirculatorySystem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.heart_rate, sys.heart_rate);
        assert_eq!(restored.blood_pressure_systolic, sys.blood_pressure_systolic);
    }
}
