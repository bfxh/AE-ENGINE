//! 体温调节模块 — 下丘脑负反馈、出汗/寒战/血流分布
//!
//! 生物学背景:
//!   人体核心温度由下丘脑 (hypothalamus) 通过负反馈维持在 (37.0 ± 0.5) °C。
//!   温度感受器 (皮肤与内脏的 TRP 通道) 将信号传至下丘脑视前区 (PO/AH),
//!   通过三种效应器调节:
//!     1. 出汗 (sweating) — 蒸发散热,最大可达 ~2 L/h ≈ 1400 W
//!     2. 寒战 (shivering) — 骨骼肌不自主收缩产热,最大 ~5× BMR
//!     3. 血管运动 (vasomotion) — 皮肤血流重分布,改变皮肤温度
//!
//! 热平衡方程 (热力学第一定律):
//!   Q_met + Q_rad + Q_conv + Q_cond - Q_evap = Q_stored
//!   Q_stored = m·c·dT/dt   (Q_met=基础代谢, 默认 80 W)
//!
//! 论文来源:
//!   - Benzinger T.H. (1969). "Heat regulation: homeostasis of central
//!     temperature in man." Physiol. Rev. 49:671-759. (下丘脑反馈)
//!   - Gagge A.P., Stolwijk J.A.J., Nishi Y. (1971). "An effective temperature
//!     scale based on a simple model of human physiological regulatory
//!     response." ASHRAE Trans. 77:247-262. (Gagge 两节点模型)
//!   - Rowell L.B. (1974). "Human cardiovascular adjustments to exercise and
//!     thermal stress." Physiol. Rev. 54:75-159. (血流分布)
//!   - ASHRAE Handbook (2017). "Fundamentals - Thermal Comfort."

use serde::{Deserialize, Serialize};

/// 下丘脑
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Hypothalamus {
    /// 温度设定点 (K), 人体 37 °C = 310.15 K
    pub setpoint_k: f32,
    /// 比例增益 (W/K), 出汗响应
    pub sweat_gain_w_k: f32,
    /// 比例增益 (W/K), 寒战响应
    pub shiver_gain_w_k: f32,
    /// 死区 (K), 阈值内不响应
    pub deadband_k: f32,
}

impl Hypothalamus {
    pub fn new() -> Self {
        Self {
            setpoint_k: 310.15,  // 37 °C
            sweat_gain_w_k: 200.0,
            shiver_gain_w_k: 100.0,
            deadband_k: 0.3,     // ±0.3 K
        }
    }

    /// 温度误差 (K), 正 = 太热
    pub fn error_k(&self, core_temp_k: f32) -> f32 {
        core_temp_k - self.setpoint_k
    }

    /// 是否在死区内
    pub fn in_deadband(&self, core_temp_k: f32) -> bool {
        self.error_k(core_temp_k).abs() <= self.deadband_k
    }
}

impl Default for Hypothalamus {
    fn default() -> Self { Self::new() }
}

/// 热交换分量 (W, 正 = 进入身体)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HeatExchange {
    /// 代谢产热 (W), BMR 默认 80 W
    pub q_met: f32,
    /// 辐射换热 (W), 负 = 散热
    pub q_rad: f32,
    /// 对流换热 (W)
    pub q_conv: f32,
    /// 传导换热 (W)
    pub q_cond: f32,
    /// 蒸发散热 (W), 总是负值 (散热)
    pub q_evap: f32,
}

impl HeatExchange {
    pub fn new() -> Self {
        Self {
            q_met: 80.0,        // 基础代谢 80 W
            q_rad: -40.0,
            q_conv: -30.0,
            q_cond: -10.0,
            q_evap: 0.0,
        }
    }

    /// 总热存储 Q_stored = Q_met + Q_rad + Q_conv + Q_cond - Q_evap (W)
    /// 注意: Q_evap 在我们的符号约定中为正数 (表示蒸发散热幅度),从 Q_stored 中扣除
    pub fn q_stored_w(&self) -> f32 {
        self.q_met + self.q_rad + self.q_conv + self.q_cond - self.q_evap
    }
}

impl Default for HeatExchange {
    fn default() -> Self { Self::new() }
}

/// 体温状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThermalState {
    /// 核心温度 (K)
    pub core_temp_k: f32,
    /// 皮肤温度 (K)
    pub skin_temp_k: f32,
    /// 出汗率 (g/s), 1 g 蒸发 ≈ 2426 J 潜热
    pub sweat_rate_g_s: f32,
    /// 寒战强度 (0..1)
    pub shiver_intensity: f32,
    /// 皮肤血流分数 (0..1, 占心输出量比例)
    pub skin_blood_flow_fraction: f32,
}

impl ThermalState {
    pub fn new() -> Self {
        Self {
            core_temp_k: 310.15,  // 37 °C
            skin_temp_k: 305.15,  // 32 °C
            sweat_rate_g_s: 0.0,
            shiver_intensity: 0.0,
            skin_blood_flow_fraction: skin_blood_fraction_default(),
        }
    }

    /// 蒸发散热 (W) — 出汗率 × 潜热
    pub fn evaporation_w(&self) -> f32 {
        let latent_heat_j_g = 2426.0; // 37 °C 水蒸发潜热
        self.sweat_rate_g_s * latent_heat_j_g
    }
}

fn skin_blood_fraction_default() -> f32 { 0.05 }

impl Default for ThermalState {
    fn default() -> Self { Self::new() }
}

/// 体温调节器
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Thermoregulator {
    pub hypothalamus: Hypothalamus,
    /// 身体质量 (kg)
    pub body_mass_kg: f32,
    /// 比热容 (J/(kg·K)), 人体 ~3500
    pub specific_heat_j_kg_k: f32,
    pub state: ThermalState,
    pub heat: HeatExchange,
}

impl Thermoregulator {
    pub fn new() -> Self {
        Self {
            hypothalamus: Hypothalamus::new(),
            body_mass_kg: 70.0,
            specific_heat_j_kg_k: 3500.0,
            state: ThermalState::new(),
            heat: HeatExchange::new(),
        }
    }

    /// 总热容 (J/K)
    pub fn thermal_capacity_j_k(&self) -> f32 {
        self.body_mass_kg * self.specific_heat_j_kg_k
    }

    /// 调节出汗率 (Benzinger 1969 — 出汗阈值约 37 °C+0.3)
    /// 显式更新出汗率与 Q_evap
    pub fn regulate_sweat(&mut self, dt: f32) {
        let err = self.hypothalamus.error_k(self.state.core_temp_k);
        if err > self.hypothalamus.deadband_k {
            // 太热 — 增加出汗
            let target_rate = (err - self.hypothalamus.deadband_k) * self.hypothalamus.sweat_gain_w_k / 2426.0;
            let k = 0.5; // 1/s
            self.state.sweat_rate_g_s += (target_rate - self.state.sweat_rate_g_s) * k * dt;
            self.state.sweat_rate_g_s = self.state.sweat_rate_g_s.max(0.0);
        } else {
            // 在死区或更冷 — 衰减出汗
            self.state.sweat_rate_g_s = (self.state.sweat_rate_g_s - 0.5 * dt).max(0.0);
        }
        self.heat.q_evap = self.state.evaporation_w();
    }

    /// 调节寒战 (冷响应,增加代谢产热)
    pub fn regulate_shiver(&mut self, dt: f32) {
        let err = self.hypothalamus.error_k(self.state.core_temp_k);
        if err < -self.hypothalamus.deadband_k {
            // 太冷 — 寒战产热
            let cold_stim = (-err - self.hypothalamus.deadband_k) / 2.0;
            let target = cold_stim.clamp(0.0, 1.0);
            let k = 0.5;
            self.state.shiver_intensity += (target - self.state.shiver_intensity) * k * dt;
            // 寒战增加代谢产热 (最大 4× BMR = 320 W 增量)
            self.heat.q_met = 80.0 + self.state.shiver_intensity * 320.0;
        } else {
            // 衰减寒战
            self.state.shiver_intensity = (self.state.shiver_intensity - 0.5 * dt).max(0.0);
            self.heat.q_met = 80.0 + self.state.shiver_intensity * 320.0;
        }
    }

    /// 调节皮肤血流 (Rowell 1974)
    pub fn regulate_blood_flow(&mut self, dt: f32) {
        let err = self.hypothalamus.error_k(self.state.core_temp_k);
        // 热时血管扩张 (高达 60% 心输出量), 冷时收缩 (< 1%)
        let target = if err > 0.0 {
            (0.05 + err * 0.5).min(0.6)
        } else {
            (0.05 + err * 0.5).max(0.005)
        };
        let k = 0.3;
        self.state.skin_blood_flow_fraction += (target - self.state.skin_blood_flow_fraction) * k * dt;
        self.state.skin_blood_flow_fraction = self.state.skin_blood_flow_fraction.clamp(0.005, 0.6);
    }

    /// 显式 Euler 积分: 更新核心温度
    /// dT/dt = Q_stored / (m·c)
    pub fn step(&mut self, dt: f32) {
        self.regulate_sweat(dt);
        self.regulate_shiver(dt);
        self.regulate_blood_flow(dt);

        let q_stored = self.heat.q_stored_w();
        let d_t = q_stored / self.thermal_capacity_j_k() * dt;
        self.state.core_temp_k += d_t;
    }
}

impl Default for Thermoregulator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- 默认值 ---

    #[test]
    fn test_hypothalamus_default_setpoint() {
        let h = Hypothalamus::default();
        // 37 °C = 310.15 K
        assert!((h.setpoint_k - 310.15).abs() < 1e-3);
    }

    #[test]
    fn test_thermal_state_default_core_temp() {
        let s = ThermalState::default();
        assert!((s.core_temp_k - 310.15).abs() < 1e-3);
        assert!(s.skin_temp_k < s.core_temp_k); // 皮肤比核心冷
        assert_eq!(s.sweat_rate_g_s, 0.0);
        assert_eq!(s.shiver_intensity, 0.0);
    }

    #[test]
    fn test_heat_exchange_default_q_met() {
        let h = HeatExchange::default();
        assert_eq!(h.q_met, 80.0);  // BMR 80 W
    }

    #[test]
    fn test_thermoregulator_default() {
        let t = Thermoregulator::default();
        assert_eq!(t.body_mass_kg, 70.0);
        assert_eq!(t.specific_heat_j_kg_k, 3500.0);
        assert!((t.state.core_temp_k - 310.15).abs() < 1e-3);
    }

    // --- 热平衡 ---

    #[test]
    fn test_q_stored_balanced_zero() {
        let h = HeatExchange {
            q_met: 80.0,
            q_rad: -40.0,
            q_conv: -30.0,
            q_cond: -10.0,
            q_evap: 0.0,
        };
        // 80 - 40 - 30 - 10 - 0 = 0
        assert!((h.q_stored_w()).abs() < 1e-6);
    }

    #[test]
    fn test_q_stored_positive_when_met_high() {
        let h = HeatExchange {
            q_met: 200.0,
            q_rad: -40.0,
            q_conv: -30.0,
            q_cond: -10.0,
            q_evap: 0.0,
        };
        assert!(h.q_stored_w() > 0.0);
    }

    #[test]
    fn test_q_stored_negative_with_evap() {
        let h = HeatExchange {
            q_met: 80.0,
            q_rad: -40.0,
            q_conv: -30.0,
            q_cond: -10.0,
            q_evap: 100.0, // 大量蒸发散热
        };
        // 80 - 40 - 30 - 10 - 100 = -100 W
        assert!(h.q_stored_w() < 0.0);
    }

    #[test]
    fn test_zero_storage_no_temp_change() {
        let mut t = Thermoregulator::default();
        // 设为完全平衡: q_met + q_rad + q_conv + q_cond - q_evap = 0
        t.heat = HeatExchange { q_met: 80.0, q_rad: -40.0, q_conv: -30.0, q_cond: -10.0, q_evap: 0.0 };
        let before = t.state.core_temp_k;
        // 在死区内,不会触发出汗/寒战
        t.step(60.0);
        // 温度变化应非常小 (因调节器可能略改变 q_evap/q_met,但死区内不应触发)
        let after = t.state.core_temp_k;
        assert!((after - before).abs() < 0.01, "temp drifted by {}", after - before);
    }

    // --- 出汗降温 (Benzinger 1969) ---

    #[test]
    fn test_sweat_activation_when_hot() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 312.0; // 38.85 °C, 远高于设定点
        let before = t.state.sweat_rate_g_s;
        t.regulate_sweat(1.0);
        assert!(t.state.sweat_rate_g_s > before);
        assert!(t.heat.q_evap > 0.0);
    }

    #[test]
    fn test_sweat_no_activation_in_deadband() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 310.30; // +0.15 K, 在死区内
        t.regulate_sweat(1.0);
        assert_eq!(t.state.sweat_rate_g_s, 0.0);
        assert_eq!(t.heat.q_evap, 0.0);
    }

    #[test]
    fn test_sweat_decays_when_cool() {
        let mut t = Thermoregulator::default();
        t.state.sweat_rate_g_s = 0.5;
        t.state.core_temp_k = 309.0; // 冷
        t.regulate_sweat(2.0);
        assert!(t.state.sweat_rate_g_s < 0.5);
    }

    #[test]
    fn test_evaporation_w_calculation() {
        let s = ThermalState { sweat_rate_g_s: 1.0, ..Default::default() };
        // 1 g/s × 2426 J/g = 2426 W
        assert!((s.evaporation_w() - 2426.0).abs() < 1.0);
    }

    // --- 寒战产热 ---

    #[test]
    fn test_shiver_activation_when_cold() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 308.0; // 34.85 °C
        let before = t.state.shiver_intensity;
        t.regulate_shiver(1.0);
        assert!(t.state.shiver_intensity > before);
        assert!(t.heat.q_met > 80.0);
    }

    #[test]
    fn test_shiver_no_activation_in_deadband() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 310.0; // 在死区内
        t.regulate_shiver(1.0);
        assert_eq!(t.state.shiver_intensity, 0.0);
        assert_eq!(t.heat.q_met, 80.0);
    }

    #[test]
    fn test_shiver_max_metabolic_increase() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 305.0; // 极冷
        // 让寒战接近最大
        for _ in 0..100 {
            t.regulate_shiver(0.1);
        }
        // 最大 q_met = 80 + 320 = 400 W
        assert!(t.heat.q_met <= 400.1);
        assert!(t.heat.q_met > 350.0);
    }

    // --- 血流重分布 (Rowell 1974) ---

    #[test]
    fn test_blood_flow_increases_when_hot() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 312.0;
        let before = t.state.skin_blood_flow_fraction;
        t.regulate_blood_flow(1.0);
        assert!(t.state.skin_blood_flow_fraction > before);
    }

    #[test]
    fn test_blood_flow_decreases_when_cold() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 308.0;
        let before = t.state.skin_blood_flow_fraction;
        t.regulate_blood_flow(1.0);
        assert!(t.state.skin_blood_flow_fraction < before);
    }

    #[test]
    fn test_blood_flow_caps_at_max() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 320.0; // 极热
        for _ in 0..100 {
            t.regulate_blood_flow(0.1);
        }
        assert!(t.state.skin_blood_flow_fraction <= 0.6 + 1e-6);
    }

    #[test]
    fn test_blood_flow_floors_at_min() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 290.0; // 极冷
        for _ in 0..100 {
            t.regulate_blood_flow(0.1);
        }
        assert!(t.state.skin_blood_flow_fraction >= 0.005 - 1e-6);
    }

    // --- 集成测试 ---

    #[test]
    fn test_step_cools_down_when_hot() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 312.0;
        let before = t.state.core_temp_k;
        for _ in 0..100 {
            t.step(1.0);
        }
        assert!(t.state.core_temp_k < before);
    }

    #[test]
    fn test_step_warms_up_when_cold() {
        let mut t = Thermoregulator::default();
        t.state.core_temp_k = 307.0;
        let before = t.state.core_temp_k;
        for _ in 0..100 {
            t.step(1.0);
        }
        assert!(t.state.core_temp_k > before);
    }

    #[test]
    fn test_thermal_capacity_calculation() {
        let t = Thermoregulator::default();
        // 70 kg × 3500 J/(kg·K) = 245000 J/K
        assert!((t.thermal_capacity_j_k() - 245000.0).abs() < 1.0);
    }

    #[test]
    fn test_hypothalamus_error_positive_when_hot() {
        let h = Hypothalamus::default();
        assert!(h.error_k(311.0) > 0.0);
        assert!(h.error_k(309.0) < 0.0);
    }

    #[test]
    fn test_hypothalamus_deadband() {
        let h = Hypothalamus::default();
        assert!(h.in_deadband(310.15));      // 设定点
        assert!(h.in_deadband(310.40));      // +0.25 K,在死区内
        assert!(!h.in_deadband(311.0));      // +0.85 K,超出死区
    }
}
