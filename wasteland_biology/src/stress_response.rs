//! 应激反应模块 — HPA 轴、战斗或逃跑与异质性负荷
//!
//! 生物学背景:
//!   应激反应是机体面对威胁时的神经内分泌协同反应。HPA 轴（下丘脑-垂体-肾上腺轴）
//!   通过 CRH → ACTH → 皮质醇级联放大调控慢性应激；交感-肾上腺髓质系统则通过
//!   肾上腺素触发"战斗或逃跑"快速响应。长期慢性应激会导致异质性负荷 (Allostatic Load)
//!   累积，损伤心血管、免疫、神经、代谢系统。
//!
//! 论文来源:
//! - Selye, H. (1936). "A syndrome produced by diverse nocuous agents."
//!   Nature 138: 32. (一般适应综合征 GAS)
//! - Cannon, W. B. (1932). "The Wisdom of the Body." Norton. (战斗或逃跑反应)
//! - McEwen, B. S., Stellar, E. (1993). "Stress and the individual: mechanisms
//!   leading to disease." Arch. Intern. Med. 153(18): 2093-2101. (异质性负荷概念)
//! - Sapolsky, R. M., Romero, L. M., Munck, A. U. (2000). "How do glucocorticoids
//!   influence stress responses? Integrating permissive, suppressive, stimulatory,
//!   and preparative actions." Endocr. Rev. 21(1): 55-89.
//! - de Kloet, E. R., Joels, M., Holsboer, F. (2005). "Stress and the brain:
//!   from adaptation to disease." Nat. Rev. Neurosci. 6(6): 463-475.
//!
//! 物理量单位:
//!   - 时间: s
//!   - 激素浓度: ug/dL (皮质醇), pg/mL (ACTH), pg/mL (肾上腺素)
//!   - 异质性负荷: 无量纲累积指数

use serde::{Deserialize, Serialize};

/// 应激源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StressorType {
    /// 物理威胁 (捕食者、攻击)
    PhysicalThreat,
    /// 心理社会压力
    Psychosocial,
    /// 代谢压力 (饥饿、低血糖)
    Metabolic,
    /// 免疫挑战 (感染、炎症)
    Immune,
    /// 环境极端 (温度、辐射)
    Environmental,
    /// 训练/运动
    Exercise,
}

impl StressorType {
    /// 该应激源主要激活的轴
    pub fn primary_axis(&self) -> StressAxis {
        match self {
            Self::PhysicalThreat | Self::Exercise => StressAxis::Sympathoadrenal,
            Self::Psychosocial | Self::Metabolic | Self::Immune | Self::Environmental => {
                StressAxis::Hpa
            }
        }
    }

    /// 应激强度系数 (0..1)
    pub fn typical_intensity(&self) -> f32 {
        match self {
            Self::PhysicalThreat => 0.9,
            Self::Psychosocial => 0.5,
            Self::Metabolic => 0.6,
            Self::Immune => 0.7,
            Self::Environmental => 0.5,
            Self::Exercise => 0.4,
        }
    }
}

/// 应激反应轴
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StressAxis {
    /// HPA 轴: CRH → ACTH → Cortisol
    Hpa,
    /// 交感-肾上腺髓质轴: 肾上腺素/去甲肾上腺素
    Sympathoadrenal,
}

/// 应激源实例
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Stressor {
    pub kind: StressorType,
    /// 强度 (0..1)
    pub intensity: f32,
    /// 持续时间 (s)
    pub duration_s: f32,
}

impl Default for Stressor {
    fn default() -> Self {
        Self {
            kind: StressorType::Psychosocial,
            intensity: 0.5,
            duration_s: 600.0,
        }
    }
}

impl Stressor {
    /// 应激剂量 = 强度 × 持续时间 (无量纲)
    pub fn dose(&self) -> f32 {
        self.intensity.clamp(0.0, 1.0) * self.duration_s.max(0.0)
    }
}

/// HPA 轴状态 — 三级级联
/// 来源: Sapolsky 2000 Endocr. Rev.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HpaAxis {
    /// 下丘脑 CRH 浓度 (pg/mL)
    pub crh_pg_per_ml: f32,
    /// 垂体 ACTH 浓度 (pg/mL)
    pub acth_pg_per_ml: f32,
    /// 肾上腺皮质醇浓度 (ug/dL)
    pub cortisol_ug_per_dl: f32,
    /// 基线皮质醇 (晨峰参考 ~ 10-20 ug/dL)
    pub baseline_cortisol: f32,
    /// 昼夜节律当前小时 (0..24)
    pub circadian_hour: f32,
}

impl Default for HpaAxis {
    fn default() -> Self {
        Self {
            crh_pg_per_ml: 1.0,
            acth_pg_per_ml: 20.0,
            cortisol_ug_per_dl: 12.0,
            baseline_cortisol: 12.0,
            circadian_hour: 8.0,
        }
    }
}

impl HpaAxis {
    /// 皮质醇昼夜节律 — 晨峰 6-8 点, 夜间谷值
    /// 模型: 余弦波 + 偏移
    /// 来源: Weitzman 1971
    pub fn circadian_cortisol_ug_per_dl(hour: f32) -> f32 {
        let h = hour.rem_euclid(24.0);
        // 峰值在 08:00, 振幅 7, 基线 5
        let phase = (h - 8.0) * (2.0 * std::f32::consts::PI / 24.0);
        12.0 + 7.0 * phase.cos()
    }

    /// CRH 刺激 ACTH 分泌 — 显式 Euler 单步
    /// dACTH/dt = k1 * CRH - k2 * ACTH
    /// 来源: Sapolsky 2000 Eq. 3
    pub fn stimulate_acth(&mut self, dt_s: f32) {
        let k1 = 5.0e-3; // CRH → ACTH 转化率
        let k2 = 1.0e-3; // ACTH 清除率
        let d_acth = k1 * self.crh_pg_per_ml - k2 * self.acth_pg_per_ml;
        self.acth_pg_per_ml = (self.acth_pg_per_ml + d_acth * dt_s).max(0.0);
    }

    /// ACTH 刺激皮质醇分泌 — 显式 Euler 单步
    /// dCortisol/dt = k3 * ACTH - k4 * Cortisol
    /// 来源: Sapolsky 2000 Eq. 5
    pub fn stimulate_cortisol(&mut self, dt_s: f32) {
        let k3 = 2.0e-3; // ACTH → Cortisol 转化率
        let k4 = 5.0e-4; // Cortisol 清除率 (半衰期 ~ 60 min)
        let d_cort = k3 * self.acth_pg_per_ml - k4 * self.cortisol_ug_per_dl;
        self.cortisol_ug_per_dl = (self.cortisol_ug_per_dl + d_cort * dt_s).max(0.0);
    }

    /// 完整级联一步: CRH → ACTH → Cortisol
    pub fn step_cascade(&mut self, dt_s: f32) {
        self.stimulate_acth(dt_s);
        self.stimulate_cortisol(dt_s);
    }

    /// 应用一个应激源 — 增加 CRH 浓度
    pub fn apply_stressor(&mut self, stressor: &Stressor) {
        let crh_boost = stressor.dose() * 1.0e-2;
        self.crh_pg_per_ml += crh_boost;
    }

    /// 皮质醇水平相对基线
    pub fn cortisol_ratio(&self) -> f32 {
        if self.baseline_cortisol > 0.0 {
            self.cortisol_ug_per_dl / self.baseline_cortisol
        } else {
            0.0
        }
    }

    /// 是否处于皮质醇升高状态 (> 1.5 倍基线)
    pub fn is_elevated(&self) -> bool {
        self.cortisol_ratio() > 1.5
    }
}

/// 皮质醇水平评估
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CortisolLevel {
    /// 低 (< 5 ug/dL)
    Low,
    /// 正常 (5-25 ug/dL)
    Normal,
    /// 升高 (25-40 ug/dL)
    Elevated,
    /// 高 (> 40 ug/dL, Cushing 范围)
    High,
}

impl CortisolLevel {
    pub fn from_cortisol_ug_per_dl(c: f32) -> Self {
        if c < 5.0 {
            Self::Low
        } else if c < 25.0 {
            Self::Normal
        } else if c < 40.0 {
            Self::Elevated
        } else {
            Self::High
        }
    }
}

/// 肾上腺素战斗或逃跑响应
/// 来源: Cannon 1932
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AdrenalineResponse {
    /// 基线肾上腺素 (pg/mL, ~ 50)
    pub baseline_pg_per_ml: f32,
    /// 当前肾上腺素 (pg/mL)
    pub current_pg_per_ml: f32,
    /// 心率增量 (BPM)
    pub heart_rate_delta_bpm: f32,
    /// 瞳孔扩张 (0..1)
    pub pupil_dilation: f32,
}

impl Default for AdrenalineResponse {
    fn default() -> Self {
        Self {
            baseline_pg_per_ml: 50.0,
            current_pg_per_ml: 50.0,
            heart_rate_delta_bpm: 0.0,
            pupil_dilation: 0.0,
        }
    }
}

impl AdrenalineResponse {
    /// 触发战斗或逃跑反应 — 肾上腺素快速峰值 (秒级)
    /// 半衰期 ~ 2 分钟 (120 s)
    pub fn trigger_fight_or_flight(&mut self, intensity: f32) {
        let i = intensity.clamp(0.0, 1.0);
        // 峰值可达 500-1000 pg/mL (Cannon 1932)
        self.current_pg_per_ml = self.baseline_pg_per_ml + i * 950.0;
        self.heart_rate_delta_bpm = i * 80.0;
        self.pupil_dilation = i;
    }

    /// 显式 Euler 衰减一步 — 指数衰减 dE/dt = -k*E
    pub fn decay_step(&mut self, dt_s: f32) {
        let k = std::f32::consts::LN_2 / 120.0; // 半衰期 120 s
        let delta = -k * (self.current_pg_per_ml - self.baseline_pg_per_ml);
        self.current_pg_per_ml = (self.current_pg_per_ml + delta * dt_s).max(self.baseline_pg_per_ml);
        let heart_delta = -k * self.heart_rate_delta_bpm;
        self.heart_rate_delta_bpm = (self.heart_rate_delta_bpm + heart_delta * dt_s).max(0.0);
        let pupil_delta = -k * self.pupil_dilation;
        self.pupil_dilation = (self.pupil_dilation + pupil_delta * dt_s).max(0.0);
    }

    /// 是否处于战斗或逃跑激活状态
    pub fn is_active(&self) -> bool {
        self.current_pg_per_ml > self.baseline_pg_per_ml * 2.0
    }
}

/// 异质性负荷 (Allostatic Load) — 慢性应激累积损伤指数
/// 来源: McEwen & Stellar 1993
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AllostaticLoad {
    /// 累积负荷指数 (0..100+)
    pub cumulative_index: f32,
    /// 心血管损伤 (0..1)
    pub cardiovascular_damage: f32,
    /// 代谢损伤 (0..1)
    pub metabolic_damage: f32,
    /// 免疫损伤 (0..1)
    pub immune_damage: f32,
    /// 神经损伤 (0..1)
    pub neuro_damage: f32,
}

impl Default for AllostaticLoad {
    fn default() -> Self {
        Self {
            cumulative_index: 0.0,
            cardiovascular_damage: 0.0,
            metabolic_damage: 0.0,
            immune_damage: 0.0,
            neuro_damage: 0.0,
        }
    }
}

impl AllostaticLoad {
    /// 显式 Euler 累积一步 — 慢性应激增加负荷
    /// 来源: McEwen 1993, 简化线性模型
    pub fn accumulate_chronic(&mut self, cortisol_ratio: f32, dt_s: f32) {
        // 仅当皮质醇持续超过基线时累积
        if cortisol_ratio > 1.0 {
            let excess = cortisol_ratio - 1.0;
            let rate = excess * 1.0e-4; // 每秒累积率
            self.cumulative_index += rate * dt_s;
            self.cardiovascular_damage = (self.cardiovascular_damage + rate * 0.4 * dt_s).min(1.0);
            self.metabolic_damage = (self.metabolic_damage + rate * 0.3 * dt_s).min(1.0);
            self.immune_damage = (self.immune_damage + rate * 0.2 * dt_s).min(1.0);
            self.neuro_damage = (self.neuro_damage + rate * 0.1 * dt_s).min(1.0);
        }
    }

    /// 显式 Euler 恢复一步 — 急性应激后负荷部分可逆
    pub fn recover(&mut self, dt_s: f32) {
        let recovery_rate = 1.0e-5; // 每秒恢复率 (慢)
        self.cumulative_index = (self.cumulative_index - recovery_rate * dt_s).max(0.0);
    }

    /// 总损伤分数 (0..1, 各系统均值)
    pub fn total_damage_fraction(&self) -> f32 {
        (self.cardiovascular_damage
            + self.metabolic_damage
            + self.immune_damage
            + self.neuro_damage)
            / 4.0
    }

    /// 是否达到危险阈值 (累积 > 50)
    pub fn is_critical(&self) -> bool {
        self.cumulative_index > 50.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hpa_axis_default_state() {
        let h = HpaAxis::default();
        assert!((h.crh_pg_per_ml - 1.0).abs() < 1e-5);
        assert!((h.acth_pg_per_ml - 20.0).abs() < 1e-5);
        assert!((h.cortisol_ug_per_dl - 12.0).abs() < 1e-5);
        assert!((h.baseline_cortisol - 12.0).abs() < 1e-5);
    }

    #[test]
    fn test_hpa_axis_default_not_elevated() {
        let h = HpaAxis::default();
        assert!(!h.is_elevated());
    }

    #[test]
    fn test_hpa_axis_circadian_peak_at_8am() {
        let c_morning = HpaAxis::circadian_cortisol_ug_per_dl(8.0);
        let c_evening = HpaAxis::circadian_cortisol_ug_per_dl(20.0);
        // 08:00 应该是峰值 (12 + 7 = 19), 20:00 应该是 12 + 7*cos(pi) = 5
        assert!((c_morning - 19.0).abs() < 1e-3);
        assert!((c_evening - 5.0).abs() < 1e-3);
        assert!(c_morning > c_evening);
    }

    #[test]
    fn test_hpa_axis_circadian_midnight_low() {
        // 00:00 (午夜) = 12 + 7*cos((0-8)*pi/12) = 12 + 7*cos(-2pi/3) = 12 + 7*(-0.5) = 8.5
        let c = HpaAxis::circadian_cortisol_ug_per_dl(0.0);
        assert!((c - 8.5).abs() < 1e-2);
    }

    #[test]
    fn test_hpa_axis_crh_stimulates_acth() {
        let mut h = HpaAxis::default();
        let initial_acth = h.acth_pg_per_ml;
        // 提高 CRH 浓度后刺激 ACTH
        h.crh_pg_per_ml = 100.0;
        for _ in 0..1000 {
            h.stimulate_acth(1.0);
        }
        assert!(h.acth_pg_per_ml > initial_acth);
    }

    #[test]
    fn test_hpa_axis_acth_stimulates_cortisol() {
        let mut h = HpaAxis::default();
        let initial_cort = h.cortisol_ug_per_dl;
        // 提高 ACTH 浓度后刺激皮质醇
        h.acth_pg_per_ml = 200.0;
        for _ in 0..1000 {
            h.stimulate_cortisol(1.0);
        }
        assert!(h.cortisol_ug_per_dl > initial_cort);
    }

    #[test]
    fn test_hpa_axis_cascade_increases_cortisol() {
        let mut h = HpaAxis::default();
        let initial_cort = h.cortisol_ug_per_dl;
        // 模拟应激: 增加 CRH, 然后跑级联
        h.crh_pg_per_ml = 50.0;
        for _ in 0..5000 {
            h.step_cascade(1.0);
        }
        assert!(h.cortisol_ug_per_dl > initial_cort);
    }

    #[test]
    fn test_hpa_axis_apply_stressor_increases_crh() {
        let mut h = HpaAxis::default();
        let initial_crh = h.crh_pg_per_ml;
        let s = Stressor {
            kind: StressorType::PhysicalThreat,
            intensity: 0.9,
            duration_s: 600.0,
        };
        h.apply_stressor(&s);
        assert!(h.crh_pg_per_ml > initial_crh);
    }

    #[test]
    fn test_cortisol_level_classification() {
        assert_eq!(CortisolLevel::from_cortisol_ug_per_dl(2.0), CortisolLevel::Low);
        assert_eq!(CortisolLevel::from_cortisol_ug_per_dl(12.0), CortisolLevel::Normal);
        assert_eq!(CortisolLevel::from_cortisol_ug_per_dl(30.0), CortisolLevel::Elevated);
        assert_eq!(CortisolLevel::from_cortisol_ug_per_dl(50.0), CortisolLevel::High);
    }

    #[test]
    fn test_stressor_default_values() {
        let s = Stressor::default();
        assert_eq!(s.kind, StressorType::Psychosocial);
        assert!((s.intensity - 0.5).abs() < 1e-5);
        assert!((s.duration_s - 600.0).abs() < 1e-5);
    }

    #[test]
    fn test_stressor_dose_calculation() {
        let s = Stressor {
            kind: StressorType::PhysicalThreat,
            intensity: 0.5,
            duration_s: 1000.0,
        };
        assert!((s.dose() - 500.0).abs() < 1e-3);
    }

    #[test]
    fn test_stressor_dose_clamps_negative_duration() {
        let s = Stressor {
            kind: StressorType::PhysicalThreat,
            intensity: 0.5,
            duration_s: -100.0,
        };
        assert!((s.dose() - 0.0).abs() < 1e-3);
    }

    #[test]
    fn test_stressor_type_primary_axis() {
        assert_eq!(StressorType::PhysicalThreat.primary_axis(), StressAxis::Sympathoadrenal);
        assert_eq!(StressorType::Psychosocial.primary_axis(), StressAxis::Hpa);
        assert_eq!(StressorType::Exercise.primary_axis(), StressAxis::Sympathoadrenal);
    }

    #[test]
    fn test_adrenaline_response_default_baseline() {
        let a = AdrenalineResponse::default();
        assert!((a.baseline_pg_per_ml - 50.0).abs() < 1e-5);
        assert!((a.current_pg_per_ml - 50.0).abs() < 1e-5);
        assert!(!a.is_active());
    }

    #[test]
    fn test_adrenaline_trigger_fight_or_flight_rapid_peak() {
        let mut a = AdrenalineResponse::default();
        a.trigger_fight_or_flight(0.9);
        // 峰值应达 50 + 0.9*950 = 905 pg/mL
        assert!((a.current_pg_per_ml - 905.0).abs() < 1e-2);
        assert!((a.heart_rate_delta_bpm - 72.0).abs() < 1e-2);
        assert!(a.is_active());
    }

    #[test]
    fn test_adrenaline_decay_returns_to_baseline() {
        let mut a = AdrenalineResponse::default();
        a.trigger_fight_or_flight(1.0);
        // 跑足够长时间衰减应回到接近基线
        for _ in 0..10000 {
            a.decay_step(1.0);
        }
        assert!(a.current_pg_per_ml < a.baseline_pg_per_ml * 1.1);
        assert!(!a.is_active());
    }

    #[test]
    fn test_adrenaline_decay_never_below_baseline() {
        let mut a = AdrenalineResponse::default();
        a.decay_step(1000.0);
        assert!(a.current_pg_per_ml >= a.baseline_pg_per_ml);
    }

    #[test]
    fn test_allostatic_load_default_zero() {
        let al = AllostaticLoad::default();
        assert!((al.cumulative_index - 0.0).abs() < 1e-5);
        assert!(!al.is_critical());
        assert!((al.total_damage_fraction() - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_allostatic_load_accumulates_under_chronic_stress() {
        let mut al = AllostaticLoad::default();
        // 皮质醇 2 倍基线持续 1 天 (86400 s)
        al.accumulate_chronic(2.0, 86400.0);
        assert!(al.cumulative_index > 0.0);
        assert!(al.cardiovascular_damage > 0.0);
        assert!(al.metabolic_damage > 0.0);
        assert!(al.immune_damage > 0.0);
        assert!(al.neuro_damage > 0.0);
    }

    #[test]
    fn test_allostatic_load_no_accumulation_at_baseline() {
        let mut al = AllostaticLoad::default();
        al.accumulate_chronic(1.0, 86400.0);
        assert!((al.cumulative_index - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_allostatic_load_damage_capped_at_one() {
        let mut al = AllostaticLoad::default();
        // 极端皮质醇持续极长时间
        al.accumulate_chronic(10.0, 1e9);
        assert!(al.cardiovascular_damage <= 1.0);
        assert!(al.metabolic_damage <= 1.0);
        assert!(al.immune_damage <= 1.0);
        assert!(al.neuro_damage <= 1.0);
    }

    #[test]
    fn test_allostatic_load_recovery_decreases_index() {
        let mut al = AllostaticLoad {
            cumulative_index: 100.0,
            cardiovascular_damage: 0.5,
            metabolic_damage: 0.5,
            immune_damage: 0.5,
            neuro_damage: 0.5,
        };
        let initial = al.cumulative_index;
        al.recover(86400.0);
        assert!(al.cumulative_index < initial);
    }

    #[test]
    fn test_allostatic_load_critical_threshold() {
        let al = AllostaticLoad {
            cumulative_index: 75.0,
            cardiovascular_damage: 0.5,
            metabolic_damage: 0.5,
            immune_damage: 0.5,
            neuro_damage: 0.5,
        };
        assert!(al.is_critical());
    }
}
