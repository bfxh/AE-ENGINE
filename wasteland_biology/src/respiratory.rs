//! 呼吸系统模块
//!
//! 基于: West, Respiratory Physiology: The Essentials (11th Edition)
//! 参考: 肺泡通气方程、氧解离曲线、高海拔生理学、窒息动力学
//! 单位约定: 呼吸频率 breaths/min, 容积 ml, SpO2 百分比, PaCO2 mmHg,
//!           顺应性 ml/cmH2O, 气道阻力 cmH2O·L⁻¹·s
//!
//! 核心公式:
//!   - 分钟通气量 MV = RR × TV / 1000 (L/min)
//!   - 肺泡通气量 VA = RR × (TV - VD) / 1000 (L/min)
//!   - 氧输送 DO2 = CO × CaO2 × 10, CaO2 = 1.34 × Hb × SaO2 + 0.003 × PaO2
//!   - 大气压 PB(h) = 760 × exp(-h/8000)

use serde::{Deserialize, Serialize};

/// 呼吸系统生理状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RespiratoryState {
    /// 正常 RR 8-20, SpO2>=90, PaCO2<=45
    Normal,
    /// 低氧血症 SpO2<90
    Hypoxia,
    /// 高碳酸血症 PaCO2>45
    Hypercapnia,
    /// 窒息 RR=0
    Apnea,
    /// 呼吸急促 RR>20
    Tachypnea,
    /// 呼吸过缓 RR<8
    Bradypnea,
}

/// 呼吸系统状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespiratorySystem {
    /// 呼吸频率 (breaths/min)
    pub respiratory_rate: f32,
    /// 潮气量 (ml)
    pub tidal_volume: f32,
    /// 肺活量 (ml)
    pub vital_capacity: f32,
    /// 残气量 (ml)
    pub residual_volume: f32,
    /// 血氧饱和度 (百分比)
    pub oxygen_saturation: f32,
    /// 动脉二氧化碳分压 (mmHg)
    pub co2_level: f32,
    /// 肺顺应性 (ml/cmH2O)
    pub lung_compliance: f32,
    /// 气道阻力 (cmH2O·L⁻¹·s)
    pub airway_resistance: f32,
    /// 解剖死腔 (ml)
    pub anatomical_dead_space: f32,
}

impl RespiratorySystem {
    /// 创建健康成人默认呼吸系统
    /// RR=12, TV=500ml, VC=4500ml, RV=1200ml, SpO2=98%, PaCO2=40mmHg
    pub fn new() -> Self {
        Self {
            respiratory_rate: 12.0,
            tidal_volume: 500.0,
            vital_capacity: 4500.0,
            residual_volume: 1200.0,
            oxygen_saturation: 98.0,
            co2_level: 40.0,
            lung_compliance: 200.0,
            airway_resistance: 1.5,
            anatomical_dead_space: 150.0,
        }
    }

    /// 分钟通气量 MV = RR × TV / 1000 (L/min)
    pub fn minute_ventilation(&self) -> f32 {
        self.respiratory_rate * self.tidal_volume / 1000.0
    }

    /// 肺泡通气量 VA = RR × (TV - VD) / 1000 (L/min)
    pub fn alveolar_ventilation(&self) -> f32 {
        let effective = (self.tidal_volume - self.anatomical_dead_space).max(0.0);
        self.respiratory_rate * effective / 1000.0
    }

    /// 氧输送 DO2 = CO × CaO2 × 10 (ml O2/min)
    /// CaO2 = 1.34 × Hb × SaO2 + 0.003 × PaO2 (ml O2/dL)
    pub fn oxygen_delivery(&self, cardiac_output: f32, hemoglobin: f32) -> f32 {
        let sao2 = (self.oxygen_saturation / 100.0).clamp(0.0, 1.0);
        let pao2 = 95.0; // 假设动脉氧分压
        let cao2 = 1.34 * hemoglobin * sao2 + 0.003 * pao2;
        cardiac_output * cao2 * 10.0
    }

    /// 依据生理参数判定呼吸状态
    pub fn classify_state(&self) -> RespiratoryState {
        if self.respiratory_rate <= 0.0 {
            RespiratoryState::Apnea
        } else if self.respiratory_rate > 20.0 {
            RespiratoryState::Tachypnea
        } else if self.respiratory_rate < 8.0 {
            RespiratoryState::Bradypnea
        } else if self.oxygen_saturation < 90.0 {
            RespiratoryState::Hypoxia
        } else if self.co2_level > 45.0 {
            RespiratoryState::Hypercapnia
        } else {
            RespiratoryState::Normal
        }
    }

    /// 模拟窒息: 呼吸停止, SpO2 下降约 3.5%/分钟, PaCO2 上升约 4 mmHg/分钟
    pub fn simulate_apnea(&mut self, duration: f32) {
        self.respiratory_rate = 0.0;
        let drop = duration * 3.5;
        self.oxygen_saturation = (self.oxygen_saturation - drop).max(20.0);
        self.co2_level += duration * 4.0;
    }

    /// 模拟高海拔低氧: 大气压随海拔下降, SpO2 随之下降, 呼吸代偿性增快
    pub fn simulate_high_altitude(&mut self, altitude_meters: f32) {
        // 大气压 PB = 760 × exp(-h/8000)
        let pb = 760.0 * (-altitude_meters.max(0.0) / 8000.0).exp();
        // 吸入气氧分压 PIO2 = 0.21 × (PB - 47)
        let pio2 = 0.21 * (pb - 47.0).max(0.0);
        // 依据肺泡氧分压近似映射 SpO2 (氧解离曲线简化)
        let spo2 = match pio2 {
            p if p > 100.0 => 98.0,
            p if p > 80.0 => 95.0,
            p if p > 60.0 => 90.0,
            p if p > 50.0 => 85.0,
            p if p > 40.0 => 75.0,
            p if p > 30.0 => 60.0,
            _ => 40.0,
        };
        self.oxygen_saturation = spo2;
        // 缺氧性通气反应: 海拔每升高 1km, RR 约增加 1.5
        let alt_km = (altitude_meters / 1000.0).max(0.0);
        self.respiratory_rate = (self.respiratory_rate + alt_km * 1.5).clamp(8.0, 40.0);
    }

    /// 每帧更新: 通气参数缓慢回归基线
    pub fn update(&mut self, dt: f32) {
        // 呼吸频率向基线 12 漂移
        self.respiratory_rate += (12.0 - self.respiratory_rate) * 0.005 * dt;
        if self.respiratory_rate > 0.0 {
            // 有自主呼吸时 SpO2 向 98 恢复
            self.oxygen_saturation += (98.0 - self.oxygen_saturation) * 0.01 * dt;
            // PaCO2 向 40 恢复
            self.co2_level += (40.0 - self.co2_level) * 0.01 * dt;
        }
        self.oxygen_saturation = self.oxygen_saturation.clamp(0.0, 100.0);
        self.co2_level = self.co2_level.clamp(0.0, 150.0);
        self.respiratory_rate = self.respiratory_rate.clamp(0.0, 60.0);
    }
}

impl Default for RespiratorySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_respiratory_rate() {
        let sys = RespiratorySystem::new();
        assert_eq!(sys.respiratory_rate, 12.0);
    }

    #[test]
    fn test_default_tidal_volume() {
        let sys = RespiratorySystem::new();
        assert_eq!(sys.tidal_volume, 500.0);
    }

    #[test]
    fn test_default_vital_capacity() {
        let sys = RespiratorySystem::new();
        assert_eq!(sys.vital_capacity, 4500.0);
    }

    #[test]
    fn test_default_residual_volume() {
        let sys = RespiratorySystem::new();
        assert_eq!(sys.residual_volume, 1200.0);
    }

    #[test]
    fn test_default_oxygen_saturation() {
        let sys = RespiratorySystem::new();
        assert_eq!(sys.oxygen_saturation, 98.0);
    }

    #[test]
    fn test_default_co2_level() {
        let sys = RespiratorySystem::new();
        assert_eq!(sys.co2_level, 40.0);
    }

    #[test]
    fn test_minute_ventilation_default() {
        // MV = 12 * 500 / 1000 = 6.0 L/min
        let sys = RespiratorySystem::new();
        assert!((sys.minute_ventilation() - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_alveolar_ventilation_default() {
        // VA = 12 * (500 - 150) / 1000 = 4.2 L/min
        let sys = RespiratorySystem::new();
        assert!((sys.alveolar_ventilation() - 4.2).abs() < 0.001);
    }

    #[test]
    fn test_classify_normal() {
        let sys = RespiratorySystem::new();
        assert_eq!(sys.classify_state(), RespiratoryState::Normal);
    }

    #[test]
    fn test_classify_apnea() {
        let mut sys = RespiratorySystem::new();
        sys.respiratory_rate = 0.0;
        assert_eq!(sys.classify_state(), RespiratoryState::Apnea);
    }

    #[test]
    fn test_classify_tachypnea() {
        let mut sys = RespiratorySystem::new();
        sys.respiratory_rate = 25.0;
        assert_eq!(sys.classify_state(), RespiratoryState::Tachypnea);
    }

    #[test]
    fn test_classify_bradypnea() {
        let mut sys = RespiratorySystem::new();
        sys.respiratory_rate = 6.0;
        assert_eq!(sys.classify_state(), RespiratoryState::Bradypnea);
    }

    #[test]
    fn test_classify_hypoxia() {
        let mut sys = RespiratorySystem::new();
        sys.oxygen_saturation = 85.0;
        assert_eq!(sys.classify_state(), RespiratoryState::Hypoxia);
    }

    #[test]
    fn test_classify_hypercapnia() {
        let mut sys = RespiratorySystem::new();
        sys.co2_level = 55.0;
        assert_eq!(sys.classify_state(), RespiratoryState::Hypercapnia);
    }

    #[test]
    fn test_apnea_reduces_oxygen() {
        let mut sys = RespiratorySystem::new();
        let before = sys.oxygen_saturation;
        sys.simulate_apnea(60.0);
        assert!(sys.oxygen_saturation < before);
    }

    #[test]
    fn test_apnea_increases_co2() {
        let mut sys = RespiratorySystem::new();
        let before = sys.co2_level;
        sys.simulate_apnea(60.0);
        assert!(sys.co2_level > before);
    }

    #[test]
    fn test_apnea_sets_rate_to_zero() {
        let mut sys = RespiratorySystem::new();
        sys.simulate_apnea(30.0);
        assert_eq!(sys.respiratory_rate, 0.0);
    }

    #[test]
    fn test_high_altitude_reduces_oxygen() {
        let mut sys = RespiratorySystem::new();
        let before = sys.oxygen_saturation;
        sys.simulate_high_altitude(5500.0);
        assert!(sys.oxygen_saturation < before);
    }

    #[test]
    fn test_high_altitude_sea_level_no_change() {
        let mut sys = RespiratorySystem::new();
        let before = sys.oxygen_saturation;
        sys.simulate_high_altitude(0.0);
        assert_eq!(sys.oxygen_saturation, before);
    }

    #[test]
    fn test_high_altitude_increases_rate() {
        let mut sys = RespiratorySystem::new();
        let before = sys.respiratory_rate;
        sys.simulate_high_altitude(4000.0);
        assert!(sys.respiratory_rate > before);
    }

    #[test]
    fn test_extreme_altitude_very_low_oxygen() {
        let mut sys = RespiratorySystem::new();
        sys.simulate_high_altitude(8848.0); // 珠峰
        assert!(sys.oxygen_saturation < 80.0);
    }

    #[test]
    fn test_oxygen_delivery_positive() {
        let sys = RespiratorySystem::new();
        let do2 = sys.oxygen_delivery(5.0, 15.0);
        assert!(do2 > 0.0);
        // DO2 = 5 * (1.34*15*0.98 + 0.003*95) * 10 ≈ 5 * 19.94 * 10 ≈ 997
        assert!(do2 > 900.0);
    }

    #[test]
    fn test_update_recovers_oxygen_after_apnea() {
        let mut sys = RespiratorySystem::new();
        sys.simulate_apnea(120.0);
        let after_apnea = sys.oxygen_saturation;
        sys.respiratory_rate = 12.0; // 恢复呼吸
        sys.update(60.0);
        assert!(sys.oxygen_saturation > after_apnea);
    }

    #[test]
    fn test_update_normalizes_rate() {
        let mut sys = RespiratorySystem::new();
        sys.respiratory_rate = 25.0;
        sys.update(120.0);
        assert!(sys.respiratory_rate < 25.0);
    }

    #[test]
    fn test_serialization_round_trip() {
        let sys = RespiratorySystem::new();
        let json = serde_json::to_string(&sys).unwrap();
        let restored: RespiratorySystem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.respiratory_rate, sys.respiratory_rate);
        assert_eq!(restored.tidal_volume, sys.tidal_volume);
    }
}
