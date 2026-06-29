//! 排泄系统模块
//!
//! 基于: Vander, Renal Physiology (8th Edition)
//! 参考: 肾小球滤过、滤过分数、抗利尿激素 (ADH) 调节、尿液浓缩机制
//! 单位约定: GFR ml/min, RPF ml/min, 尿量 ml/min, 渗透压 mOsm/L,
//!           膀胱容量 ml, 排泄量 mmol/day
//!
//! 核心公式:
//!   - 滤过分数 FF = GFR / RPF (正常 ≈ 0.19)
//!   - 尿液生成 = 尿量速率 × dt
//!   - 脱水: ADH↑, 尿渗透压↑(最高 ~1400), 尿量↓
//!   - 水负荷: ADH↓, 尿渗透压↓(最低 ~50), 尿量↑ (水利尿)

use serde::{Deserialize, Serialize};

/// 肾脏生理状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RenalState {
    /// 正常
    Normal,
    /// 脱水 (ADH↑, 尿浓缩)
    Dehydration,
    /// 水过多 (ADH↓, 尿稀释)
    Overhydration,
    /// 肾功能衰竭 (GFR<30)
    RenalFailure,
}

/// 排泄系统状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcretorySystem {
    /// 肾小球滤过率 (ml/min)
    pub glomerular_filtration_rate: f32,
    /// 肾血浆流量 (ml/min)
    pub renal_plasma_flow: f32,
    /// 尿量速率 (ml/min)
    pub urine_output: f32,
    /// 钠排泄 (mmol/day)
    pub sodium_excretion: f32,
    /// 钾排泄 (mmol/day)
    pub potassium_excretion: f32,
    /// 尿渗透压 (mOsm/L)
    pub urine_osmolarity: f32,
    /// 膀胱容量 (ml)
    pub bladder_volume: f32,
    /// 膀胱最大容量 (ml)
    pub bladder_capacity: f32,
    /// 抗利尿激素水平 (0..1)
    pub adh_level: f32,
}

impl ExcretorySystem {
    /// 创建健康成人默认排泄系统
    /// GFR=125, RPF=660, 尿量=1ml/min, 渗透压=800, 膀胱容量=500, ADH=0.5
    pub fn new() -> Self {
        Self {
            glomerular_filtration_rate: 125.0,
            renal_plasma_flow: 660.0,
            urine_output: 1.0,
            sodium_excretion: 150.0,
            potassium_excretion: 80.0,
            urine_osmolarity: 800.0,
            bladder_volume: 0.0,
            bladder_capacity: 500.0,
            adh_level: 0.5,
        }
    }

    /// 滤过分数 FF = GFR / RPF (正常 ≈ 0.19)
    pub fn filtration_fraction(&self) -> f32 {
        if self.renal_plasma_flow > 0.0 {
            self.glomerular_filtration_rate / self.renal_plasma_flow
        } else {
            0.0
        }
    }

    /// 依据生理参数判定肾脏状态
    pub fn classify_state(&self) -> RenalState {
        if self.glomerular_filtration_rate < 30.0 {
            RenalState::RenalFailure
        } else if self.adh_level > 0.8 && self.urine_osmolarity > 1000.0 {
            RenalState::Dehydration
        } else if self.adh_level < 0.2 && self.urine_osmolarity < 200.0 {
            RenalState::Overhydration
        } else {
            RenalState::Normal
        }
    }

    /// 尿液生成 (dt 单位: 秒), 累积到膀胱, 返回该时段生成量
    pub fn urine_production(&mut self, dt: f32) -> f32 {
        let produced = self.urine_output * dt / 60.0;
        self.bladder_volume = (self.bladder_volume + produced).min(self.bladder_capacity * 1.5);
        produced
    }

    /// 模拟脱水: ADH↑, 尿渗透压↑, 尿量↓, GFR 轻度下降
    pub fn simulate_dehydration(&mut self, duration: f32) {
        let hours = duration / 3600.0;
        self.adh_level = (self.adh_level + 0.3 * duration / 60.0).min(1.0);
        self.urine_osmolarity = (self.urine_osmolarity + 200.0 * hours).min(1400.0);
        self.urine_output = (self.urine_output - 0.5 * hours).max(0.2);
        self.glomerular_filtration_rate =
            (self.glomerular_filtration_rate - 2.0 * hours).max(40.0);
    }

    /// 模拟水负荷 (饮水): ADH↓, 尿渗透压↓, 尿量↑
    pub fn simulate_water_load(&mut self, volume: f32) {
        self.adh_level = (self.adh_level - volume / 2000.0).max(0.0);
        self.urine_osmolarity = (self.urine_osmolarity - volume / 5.0).max(50.0);
        self.urine_output = (self.urine_output + volume / 500.0).min(10.0);
    }

    /// 排尿: 清空膀胱, 返回排出量
    pub fn urinate(&mut self) -> f32 {
        let voided = self.bladder_volume;
        self.bladder_volume = 0.0;
        voided
    }

    /// 每帧更新: 持续产尿, ADH 与 GFR 缓慢回归基线
    pub fn update(&mut self, dt: f32) {
        self.urine_production(dt);
        // ADH 向 0.5 漂移
        self.adh_level += (0.5 - self.adh_level) * 0.001 * dt;
        // GFR 向 125 恢复
        self.glomerular_filtration_rate +=
            (125.0 - self.glomerular_filtration_rate) * 0.001 * dt;
    }
}

impl Default for ExcretorySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_gfr() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.glomerular_filtration_rate, 125.0);
    }

    #[test]
    fn test_default_rpf() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.renal_plasma_flow, 660.0);
    }

    #[test]
    fn test_default_urine_output() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.urine_output, 1.0);
    }

    #[test]
    fn test_default_sodium_excretion() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.sodium_excretion, 150.0);
    }

    #[test]
    fn test_default_potassium_excretion() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.potassium_excretion, 80.0);
    }

    #[test]
    fn test_default_osmolarity() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.urine_osmolarity, 800.0);
    }

    #[test]
    fn test_default_bladder_volume() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.bladder_volume, 0.0);
    }

    #[test]
    fn test_default_bladder_capacity() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.bladder_capacity, 500.0);
    }

    #[test]
    fn test_default_adh() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.adh_level, 0.5);
    }

    #[test]
    fn test_filtration_fraction_default() {
        // FF = 125/660 ≈ 0.1894
        let sys = ExcretorySystem::new();
        let ff = sys.filtration_fraction();
        assert!((ff - 0.1894).abs() < 0.001);
    }

    #[test]
    fn test_filtration_fraction_zero_rpf() {
        let mut sys = ExcretorySystem::new();
        sys.renal_plasma_flow = 0.0;
        assert_eq!(sys.filtration_fraction(), 0.0);
    }

    #[test]
    fn test_classify_normal() {
        let sys = ExcretorySystem::new();
        assert_eq!(sys.classify_state(), RenalState::Normal);
    }

    #[test]
    fn test_classify_dehydration() {
        let mut sys = ExcretorySystem::new();
        sys.adh_level = 0.9;
        sys.urine_osmolarity = 1100.0;
        assert_eq!(sys.classify_state(), RenalState::Dehydration);
    }

    #[test]
    fn test_classify_overhydration() {
        let mut sys = ExcretorySystem::new();
        sys.adh_level = 0.1;
        sys.urine_osmolarity = 100.0;
        assert_eq!(sys.classify_state(), RenalState::Overhydration);
    }

    #[test]
    fn test_classify_renal_failure() {
        let mut sys = ExcretorySystem::new();
        sys.glomerular_filtration_rate = 20.0;
        assert_eq!(sys.classify_state(), RenalState::RenalFailure);
    }

    #[test]
    fn test_urine_production_adds_to_bladder() {
        let mut sys = ExcretorySystem::new();
        let produced = sys.urine_production(60.0); // 1 分钟
        // 1 ml/min * 60s / 60 = 1 ml
        assert!((produced - 1.0).abs() < 0.01);
        assert!((sys.bladder_volume - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_urine_production_respects_overflow() {
        let mut sys = ExcretorySystem::new();
        sys.bladder_volume = 740.0; // 接近 1.5x 容量上限
        sys.urine_production(600.0);
        // 不应超过 500 * 1.5 = 750
        assert!(sys.bladder_volume <= 750.0 + 0.01);
    }

    #[test]
    fn test_dehydration_raises_adh() {
        let mut sys = ExcretorySystem::new();
        let before = sys.adh_level;
        sys.simulate_dehydration(300.0); // 5 分钟
        assert!(sys.adh_level > before);
    }

    #[test]
    fn test_dehydration_raises_osmolarity() {
        let mut sys = ExcretorySystem::new();
        let before = sys.urine_osmolarity;
        sys.simulate_dehydration(3600.0); // 1 小时
        assert!(sys.urine_osmolarity > before);
    }

    #[test]
    fn test_dehydration_lowers_urine_output() {
        let mut sys = ExcretorySystem::new();
        let before = sys.urine_output;
        sys.simulate_dehydration(3600.0);
        assert!(sys.urine_output < before);
    }

    #[test]
    fn test_water_load_lowers_adh() {
        let mut sys = ExcretorySystem::new();
        let before = sys.adh_level;
        sys.simulate_water_load(1000.0); // 饮 1L 水
        assert!(sys.adh_level < before);
    }

    #[test]
    fn test_water_load_raises_urine_output() {
        let mut sys = ExcretorySystem::new();
        let before = sys.urine_output;
        sys.simulate_water_load(1000.0);
        assert!(sys.urine_output > before);
    }

    #[test]
    fn test_urinate_empties_bladder() {
        let mut sys = ExcretorySystem::new();
        sys.bladder_volume = 300.0;
        let voided = sys.urinate();
        assert!((voided - 300.0).abs() < 0.01);
        assert_eq!(sys.bladder_volume, 0.0);
    }

    #[test]
    fn test_update_normalizes_adh() {
        let mut sys = ExcretorySystem::new();
        sys.adh_level = 0.9;
        sys.update(600.0);
        assert!(sys.adh_level < 0.9);
    }

    #[test]
    fn test_serialization_round_trip() {
        let sys = ExcretorySystem::new();
        let json = serde_json::to_string(&sys).unwrap();
        let restored: ExcretorySystem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.glomerular_filtration_rate, sys.glomerular_filtration_rate);
        assert_eq!(restored.adh_level, sys.adh_level);
    }
}
