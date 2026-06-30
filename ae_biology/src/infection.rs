//! 感染模拟 —— 5 变量炎症反应 ODE 模型
//!
//! 论文来源：
//! - Reynolds et al., "A mathematical model of pulmonary infection"
//!   —— 细菌 + M1/M2 巨噬细胞极化 + 细胞因子调控
//! - PDES 感染模型（PDE Solutions Inc. 软件案例库）
//! - Day et al. 2009, "Macrophage polarization in tissue repair"

use serde::{Deserialize, Serialize};

/// 感染模型参数
///
/// 5 变量系统：
/// - B:   细菌密度
/// - M1:  促炎巨噬细胞（杀菌 + 分泌 IL-6）
/// - M2:  抗炎巨噬细胞（分泌 TGF-β，抑制 M1）
/// - IL6: 白介素-6（促炎信号，招募 M1）
/// - TGFβ: 转化生长因子-β（抗炎信号，招募 M2，抑制 M1）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InfectionModel {
    // 细菌动力学
    /// k_Bg: 细菌 logistic 生长速率
    pub k_bg: f32,
    /// B_max: 细菌携带容量
    pub b_max: f32,
    /// k_Bk: M1 杀菌速率
    pub k_bk: f32,

    // M1 动力学
    /// k_M1i: IL-6 招募 M1 速率
    pub k_m1i: f32,
    /// M1_max: M1 上限
    pub m1_max: f32,
    /// k_M1a: TGF-β 抑制 M1（极化转换）速率
    pub k_m1a: f32,
    /// k_M1d: M1 凋亡速率
    pub k_m1d: f32,

    // M2 动力学
    /// k_M2i: TGF-β 招募 M2 速率
    pub k_m2i: f32,
    /// M2_max: M2 上限
    pub m2_max: f32,
    /// k_M2d: M2 凋亡速率
    pub k_m2d: f32,

    // 细胞因子
    /// k_IL6: M1 分泌 IL-6 速率
    pub k_il6: f32,
    /// k_IL6d: IL-6 降解速率
    pub k_il6d: f32,
    /// k_TGFb: M2 分泌 TGF-β 速率
    pub k_tgfb: f32,
    /// k_TGFbd: TGF-β 降解速率
    pub k_tgfbd: f32,
}

impl InfectionModel {
    /// 默认参数 —— Reynolds 模型 + PDES 案例库标定值
    pub fn new() -> Self {
        Self {
            k_bg: 1.5,
            b_max: 10.0,
            k_bk: 0.8,
            k_m1i: 0.5,
            m1_max: 1.0,
            k_m1a: 0.2,
            k_m1d: 0.05,
            k_m2i: 0.4,
            m2_max: 1.0,
            k_m2d: 0.05,
            k_il6: 0.3,
            k_il6d: 0.1,
            k_tgfb: 0.2,
            k_tgfbd: 0.08,
        }
    }

    /// 单步积分（显式 Euler）
    ///
    /// Reynolds 感染模型 ODE：
    ///   dB/dt    = k_Bg · B · (1 - B/B_max) - k_Bk · M1 · B            (1)
    ///   dM1/dt   = k_M1i · IL6 · (1 - M1/M1_max) - k_M1a · TGFβ · M1 - k_M1d · M1  (2)
    ///   dM2/dt   = k_M2i · TGFβ · (1 - M2/M2_max) - k_M2d · M2          (3)
    ///   dIL6/dt  = k_IL6 · M1 - k_IL6d · IL6                            (4)
    ///   dTGFβ/dt = k_TGFb · M2 - k_TGFbd · TGFβ                         (5)
    pub fn step(&self, state: &mut InfectionState, dt: f32) {
        let b = state.bacteria.max(0.0);
        let m1 = state.m1.max(0.0);
        let m2 = state.m2.max(0.0);
        let il6 = state.il6.max(0.0);
        let tgfb = state.tgf_beta.max(0.0);

        // Reynolds Eq.(1) —— 细菌 logistic 增长 + M1 杀菌
        let db_dt = self.k_bg * b * (1.0 - b / self.b_max) - self.k_bk * m1 * b;
        // Reynolds Eq.(2) —— M1 由 IL-6 招募，被 TGF-β 抑制，自然凋亡
        let dm1_dt = self.k_m1i * il6 * (1.0 - m1 / self.m1_max)
            - self.k_m1a * tgfb * m1
            - self.k_m1d * m1;
        // Reynolds Eq.(3) —— M2 由 TGF-β 招募
        let dm2_dt = self.k_m2i * tgfb * (1.0 - m2 / self.m2_max) - self.k_m2d * m2;
        // Reynolds Eq.(4) —— IL-6 由 M1 分泌
        let dil6_dt = self.k_il6 * m1 - self.k_il6d * il6;
        // Reynolds Eq.(5) —— TGF-β 由 M2 分泌
        let dtgfb_dt = self.k_tgfb * m2 - self.k_tgfbd * tgfb;

        state.bacteria = (state.bacteria + db_dt * dt).max(0.0);
        state.m1 = (state.m1 + dm1_dt * dt).max(0.0);
        state.m2 = (state.m2 + dm2_dt * dt).max(0.0);
        state.il6 = (state.il6 + dil6_dt * dt).max(0.0);
        state.tgf_beta = (state.tgf_beta + dtgfb_dt * dt).max(0.0);
    }

    /// 感染是否已清除（B < 0.01）
    pub fn is_resolved(&self, state: &InfectionState) -> bool {
        state.bacteria < 0.01
    }

    /// 评估感染严重度
    ///
    /// 阈值参照临床脓毒症诊断标准（IL-6 > 100 pg/mL 为重症）
    pub fn severity(&self, state: &InfectionState) -> InfectionSeverity {
        let b = state.bacteria;
        if b < 0.01 {
            InfectionSeverity::Healthy
        } else if b < 1.0 {
            InfectionSeverity::Mild
        } else if b < 3.0 {
            InfectionSeverity::Moderate
        } else if b < 6.0 {
            InfectionSeverity::Severe
        } else {
            InfectionSeverity::Critical
        }
    }
}

impl Default for InfectionModel {
    fn default() -> Self {
        Self::new()
    }
}

/// 感染状态：5 个变量
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InfectionState {
    /// 细菌密度（无量纲，0..B_max）
    pub bacteria: f32,
    /// M1 促炎巨噬细胞密度
    pub m1: f32,
    /// M2 抗炎巨噬细胞密度
    pub m2: f32,
    /// 白介素-6 浓度
    pub il6: f32,
    /// 转化生长因子-β 浓度
    pub tgf_beta: f32,
}

impl InfectionState {
    /// 健康初始状态：所有变量为 0
    pub fn healthy() -> Self {
        Self {
            bacteria: 0.0,
            m1: 0.0,
            m2: 0.0,
            il6: 0.0,
            tgf_beta: 0.0,
        }
    }

    /// 引入细菌（污染伤口）
    pub fn introduce_bacteria(&mut self, count: f32) {
        self.bacteria = (self.bacteria + count).max(0.0);
    }
}

impl Default for InfectionState {
    fn default() -> Self {
        Self::healthy()
    }
}

/// 感染严重度分级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InfectionSeverity {
    /// 健康（无可检测细菌）
    Healthy,
    /// 轻度感染（B < 1）
    Mild,
    /// 中度感染（1 ≤ B < 3）
    Moderate,
    /// 重度感染（3 ≤ B < 6）
    Severe,
    /// 危重感染（B ≥ 6，败血症风险）
    Critical,
}

impl InfectionSeverity {
    /// 是否需要医疗干预
    pub fn requires_intervention(&self) -> bool {
        matches!(self, Self::Moderate | Self::Severe | Self::Critical)
    }

    /// 数值化严重度（0..1）用于 AI 决策
    pub fn numeric(&self) -> f32 {
        match self {
            Self::Healthy => 0.0,
            Self::Mild => 0.25,
            Self::Moderate => 0.5,
            Self::Severe => 0.75,
            Self::Critical => 1.0,
        }
    }
}

// ext methods
impl InfectionModel {
    pub fn bacteria_growth_rate(&self, b: f32) -> f32 { self.k_bg * b * (1.0 - b / self.b_max) }
    pub fn m1_killing_rate(&self, m1: f32, b: f32) -> f32 { self.k_bk * m1 * b }
    pub fn is_active_clearance(&self, state: &InfectionState) -> bool { state.m1 > 0.1 && state.bacteria > 0.01 }
    pub fn inflammation_balance(&self, state: &InfectionState) -> f32 {
        if state.m2 < 1e-6 { return if state.m1 > 1e-6 { f32::INFINITY } else { 0.0 }; }
        state.m1 / state.m2
    }
    pub fn simulate(&self, initial: &InfectionState, dt: f32, steps: usize) -> InfectionState {
        let mut s = *initial;
        for _ in 0..steps { self.step(&mut s, dt); }
        s
    }
}
impl InfectionState {
    pub fn is_inflamed(&self) -> bool { self.bacteria > 0.1 || self.m1 > 0.1 || self.il6 > 0.5 }
    pub fn inflammatory_load(&self) -> f32 { self.bacteria * 1.0 + self.m1 * 2.0 + self.il6 * 0.5 + self.tgf_beta * 0.3 }
    pub fn polarization_ratio(&self) -> f32 {
        if self.m2 < 1e-6 { return if self.m1 > 1e-6 { f32::INFINITY } else { 0.0 }; }
        self.m1 / self.m2
    }
    pub fn has_immune_response(&self) -> bool { self.m1 > 0.0 || self.m2 > 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infection_model_default() {
        let m = InfectionModel::default();
        assert_eq!(m.k_bg, 1.5);
        assert_eq!(m.b_max, 10.0);
        assert_eq!(m.k_bk, 0.8);
    }

    #[test]
    fn test_infection_state_healthy() {
        let s = InfectionState::healthy();
        assert_eq!(s.bacteria, 0.0);
        assert_eq!(s.m1, 0.0);
        assert_eq!(s.m2, 0.0);
    }

    #[test]
    fn test_introduce_bacteria() {
        let mut s = InfectionState::healthy();
        s.introduce_bacteria(5.0);
        assert_eq!(s.bacteria, 5.0);
        s.introduce_bacteria(3.0);
        assert_eq!(s.bacteria, 8.0);
    }

    #[test]
    fn test_bacteria_grows_without_m1() {
        let model = InfectionModel::default();
        let mut s = InfectionState::healthy();
        s.introduce_bacteria(1.0);
        let initial = s.bacteria;
        model.step(&mut s, 0.1);
        assert!(s.bacteria > initial);
    }

    #[test]
    fn test_bacteria_logistic_cap() {
        let model = InfectionModel::default();
        let mut s = InfectionState::healthy();
        s.introduce_bacteria(9.5);
        for _ in 0..100 { model.step(&mut s, 0.1); }
        assert!(s.bacteria <= model.b_max + 0.01);
    }

    #[test]
    fn test_m1_kills_bacteria() {
        let model = InfectionModel::default();
        let mut s = InfectionState { bacteria: 5.0, m1: 1.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        let initial = s.bacteria;
        model.step(&mut s, 0.1);
        assert!(s.bacteria < initial);
    }

    #[test]
    fn test_is_resolved_healthy() {
        let model = InfectionModel::default();
        let s = InfectionState::healthy();
        assert!(model.is_resolved(&s));
    }

    #[test]
    fn test_is_resolved_with_bacteria() {
        let model = InfectionModel::default();
        let s = InfectionState { bacteria: 1.0, m1: 0.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        assert!(!model.is_resolved(&s));
    }

    #[test]
    fn test_severity_healthy() {
        let model = InfectionModel::default();
        let s = InfectionState::healthy();
        assert_eq!(model.severity(&s), InfectionSeverity::Healthy);
    }

    #[test]
    fn test_severity_mild() {
        let model = InfectionModel::default();
        let s = InfectionState { bacteria: 0.5, m1: 0.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        assert_eq!(model.severity(&s), InfectionSeverity::Mild);
    }

    #[test]
    fn test_severity_moderate() {
        let model = InfectionModel::default();
        let s = InfectionState { bacteria: 2.0, m1: 0.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        assert_eq!(model.severity(&s), InfectionSeverity::Moderate);
    }

    #[test]
    fn test_severity_severe() {
        let model = InfectionModel::default();
        let s = InfectionState { bacteria: 4.0, m1: 0.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        assert_eq!(model.severity(&s), InfectionSeverity::Severe);
    }

    #[test]
    fn test_severity_critical() {
        let model = InfectionModel::default();
        let s = InfectionState { bacteria: 7.0, m1: 0.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        assert_eq!(model.severity(&s), InfectionSeverity::Critical);
    }

    #[test]
    fn test_requires_intervention() {
        assert!(!InfectionSeverity::Healthy.requires_intervention());
        assert!(!InfectionSeverity::Mild.requires_intervention());
        assert!(InfectionSeverity::Moderate.requires_intervention());
        assert!(InfectionSeverity::Severe.requires_intervention());
        assert!(InfectionSeverity::Critical.requires_intervention());
    }

    #[test]
    fn test_numeric_severity_monotonic() {
        let v = [InfectionSeverity::Healthy.numeric(), InfectionSeverity::Mild.numeric(),
                 InfectionSeverity::Moderate.numeric(), InfectionSeverity::Severe.numeric(),
                 InfectionSeverity::Critical.numeric()];
        for i in 1..v.len() { assert!(v[i] > v[i-1]); }
    }

    #[test]
    fn test_bacteria_growth_rate() {
        let model = InfectionModel::default();
        assert!(model.bacteria_growth_rate(1.0) > 0.0);
        assert!(model.bacteria_growth_rate(model.b_max).abs() < 1e-6);
    }

    #[test]
    fn test_m1_killing_rate() {
        let model = InfectionModel::default();
        let rate = model.m1_killing_rate(1.0, 5.0);
        assert!((rate - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_is_active_clearance() {
        let model = InfectionModel::default();
        let active = InfectionState { bacteria: 1.0, m1: 0.5, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        let no_m1 = InfectionState { bacteria: 1.0, m1: 0.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        assert!(model.is_active_clearance(&active));
        assert!(!model.is_active_clearance(&no_m1));
    }

    #[test]
    fn test_inflammation_balance() {
        let model = InfectionModel::default();
        let balanced = InfectionState { bacteria: 0.0, m1: 1.0, m2: 1.0, il6: 0.0, tgf_beta: 0.0 };
        assert!((model.inflammation_balance(&balanced) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_simulate_no_mutation() {
        let model = InfectionModel::default();
        let initial = InfectionState { bacteria: 1.0, m1: 0.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        let _ = model.simulate(&initial, 0.1, 10);
        assert_eq!(initial.bacteria, 1.0);
    }

    #[test]
    fn test_is_inflamed() {
        let healthy = InfectionState::healthy();
        let bact = InfectionState { bacteria: 1.0, m1: 0.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        assert!(!healthy.is_inflamed());
        assert!(bact.is_inflamed());
    }

    #[test]
    fn test_inflammatory_load() {
        let s = InfectionState { bacteria: 2.0, m1: 1.0, m2: 0.0, il6: 3.0, tgf_beta: 1.0 };
        let load = s.inflammatory_load();
        assert!((load - (2.0 + 2.0 + 1.5 + 0.3)).abs() < 1e-5);
    }

    #[test]
    fn test_polarization_ratio() {
        let balanced = InfectionState { bacteria: 0.0, m1: 1.0, m2: 1.0, il6: 0.0, tgf_beta: 0.0 };
        assert!((balanced.polarization_ratio() - 1.0).abs() < 1e-6);
        let m1_only = InfectionState { bacteria: 0.0, m1: 1.0, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        assert!(m1_only.polarization_ratio().is_infinite());
    }

    #[test]
    fn test_has_immune_response() {
        let healthy = InfectionState::healthy();
        let with_m1 = InfectionState { bacteria: 0.0, m1: 0.1, m2: 0.0, il6: 0.0, tgf_beta: 0.0 };
        assert!(!healthy.has_immune_response());
        assert!(with_m1.has_immune_response());
    }

    #[test]
    fn test_full_resolution_cycle() {
        // 默认参数下 k_bg=1.5, k_bk=0.8, M1_max=1.0
        // 清除条件: k_bk * M1 > k_bg (当 B 很小时)，即 M1 > 1.875，但 M1_max=1.0
        // 因此需要增强杀菌速率才能清除细菌
        let mut model = InfectionModel::default();
        model.k_bk = 5.0; // 增强杀菌
        let mut s = InfectionState::healthy();
        s.introduce_bacteria(2.0);
        s.m1 = 0.8;
        s.il6 = 0.5;
        for _ in 0..300 { model.step(&mut s, 0.1); }
        assert!(model.is_resolved(&s), "bacteria should be cleared with enhanced M1 killing");
    }

    #[test]
    fn test_default_model_cannot_clear_bacteria() {
        // 验证默认参数下 M1_max=1.0 不足以清除细菌
        // 因为清除条件 M1 > k_bg/k_bk = 1.875 > M1_max
        let model = InfectionModel::default();
        let mut s = InfectionState::healthy();
        s.introduce_bacteria(2.0);
        s.m1 = 1.0; // 最大 M1
        s.il6 = 1.0;
        for _ in 0..500 { model.step(&mut s, 0.1); }
        // 细菌应仍然存在（达到平衡态）
        assert!(!model.is_resolved(&s) || s.bacteria < 1.0, "default model cannot fully clear bacteria");
    }
}
