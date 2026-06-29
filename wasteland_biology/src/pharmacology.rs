//! 药理学模块
//!
//! 实现药代动力学（PK）和药效动力学（PD）模型，包括一室模型、
//! Michaelis-Menten 消除、ADME 过程和剂量-响应关系。
//!
//! # 生物学背景
//!
//! 药理学研究药物与生物系统的相互作用，分为两个主要分支：
//! - 药代动力学（PK）：研究药物在体内的吸收、分布、代谢和排泄（ADME）
//! - 药效动力学（PD）：研究药物对机体的作用及其机制
//!
//! # 核心模型
//!
//! - 一室模型：dC/dt = -k·C（一级消除）
//! - Michaelis-Menten：dC/dt = -V_max·C/(K_m + C)（酶介导消除）
//! - Hill 方程：E = E_max·C^n/(EC50^n + C^n)（剂量-响应）
//!
//! # 参考文献
//!
//! - Rowland M., Tozer T.N. (2011) "Clinical Pharmacokinetics and Pharmacodynamics."
//! - Gabrielsson J., Weiner D. (2016) "Pharmacokinetic and Pharmacodynamic Data Analysis."
//! - Atkinson A.J. et al. (2012) "Principles of Clinical Pharmacology."

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ADME 过程类型
///
/// 药物在体内的四个主要过程：吸收、分布、代谢、排泄。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdmeProcess {
    /// 吸收 - 药物进入血液循环
    Absorption,
    /// 分布 - 药物从血液分布到组织
    Distribution,
    /// 代谢 - 药物的生物转化
    Metabolism,
    /// 排泄 - 药物从体内清除
    Excretion,
}

/// ADME 参数记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmeParameters {
    /// 吸收速率常数 (ka, h^-1)
    pub absorption_rate: f32,
    /// 分布容积 (Vd, L/kg)
    pub volume_of_distribution: f32,
    /// 清除率 (CL, L/h/kg)
    pub clearance: f32,
    /// 半衰期 (t1/2, h)
    pub half_life: f32,
    /// 生物利用度 (F, 0.0-1.0)
    pub bioavailability: f32,
    /// 血浆蛋白结合率 (0.0-1.0)
    pub plasma_binding: f32,
}

impl Default for AdmeParameters {
    fn default() -> Self {
        Self {
            absorption_rate: 1.0,
            volume_of_distribution: 0.5,
            clearance: 0.1,
            half_life: 3.5, // ln(2)/0.1 ≈ 6.93 h，简化为典型值
            bioavailability: 0.8,
            plasma_binding: 0.9,
        }
    }
}

impl AdmeParameters {
    /// 计算半衰期
    ///
    /// t1/2 = ln(2) / k，其中 k = CL / Vd
    /// 参考：Rowland & Tozer (2011) Chapter 3
    pub fn calculate_half_life(&self) -> f32 {
        let k = self.clearance / self.volume_of_distribution;
        0.693 / k // ln(2) ≈ 0.693
    }

    /// 计算稳态浓度
    ///
    /// Css = F·Dose / (τ·CL)，τ 为给药间隔
    pub fn steady_state_concentration(&self, dose: f32, dosing_interval: f32) -> f32 {
        (self.bioavailability * dose) / (dosing_interval * self.clearance)
    }

    /// 计算达稳态时间
    ///
    /// 约需 4-5 个半衰期达到稳态
    pub fn time_to_steady_state(&self) -> f32 {
        4.0 * self.half_life
    }
}

/// 药代动力学模型
///
/// 一室模型：药物均匀分布，单指数消除。
/// 参考：Atkinson et al. (2012) Chapter 2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pharmacokinetics {
    /// ADME 参数
    pub adme: AdmeParameters,
    /// 消除速率常数 (k, h^-1)
    pub elimination_rate: f32,
    /// 当前血浆浓度 (mg/L)
    pub plasma_concentration: f32,
    /// 给药剂量 (mg/kg)
    pub dose: f32,
    /// 给药途径
    pub route: AdministrationRoute,
    /// 时间点记录
    pub concentration_time_curve: Vec<(f32, f32)>, // (time, concentration)
}

/// 给药途径
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdministrationRoute {
    /// 口服
    Oral,
    /// 静脉注射
    Intravenous,
    /// 静脉输注
    IntravenousInfusion,
    /// 肌内注射
    Intramuscular,
    /// 皮下注射
    Subcutaneous,
    /// 吸入
    Inhalation,
    /// 透皮
    Transdermal,
}

impl Default for Pharmacokinetics {
    fn default() -> Self {
        Self {
            adme: AdmeParameters::default(),
            elimination_rate: 0.2,
            plasma_concentration: 0.0,
            dose: 0.0,
            route: AdministrationRoute::Oral,
            concentration_time_curve: Vec::new(),
        }
    }
}

impl Pharmacokinetics {
    /// 创建新的一室模型
    pub fn new(dose: f32, route: AdministrationRoute, adme: AdmeParameters) -> Self {
        let elimination_rate = adme.clearance / adme.volume_of_distribution;
        let initial_concentration = match route {
            AdministrationRoute::Intravenous => dose / adme.volume_of_distribution,
            _ => 0.0,
        };
        Self {
            adme,
            elimination_rate,
            plasma_concentration: initial_concentration,
            dose,
            route,
            concentration_time_curve: vec![(0.0, initial_concentration)],
        }
    }

    /// 一室模型消除
    ///
    /// dC/dt = -k·C，解为 C(t) = C0·e^(-kt)
    /// 参考：Rowland & Tozer (2011) Eq 3-1
    pub fn simulate_elimination(&mut self, hours: f32) -> f32 {
        let c0 = self.plasma_concentration;
        let c_t = c0 * (-self.elimination_rate * hours).exp();
        self.plasma_concentration = c_t;
        self.concentration_time_curve.push((hours, c_t));
        c_t
    }

    /// 计算半衰期
    ///
    /// t1/2 = ln(2) / k
    pub fn half_life(&self) -> f32 {
        0.693 / self.elimination_rate
    }

    /// 计算曲线下面积 (AUC)
    ///
    /// AUC = C0 / k（一室模型）
    pub fn auc(&self) -> f32 {
        let c0 = self.dose * self.adme.bioavailability / self.adme.volume_of_distribution;
        c0 / self.elimination_rate
    }

    /// 达峰时间（口服给药）
    ///
    /// Tmax = ln(ka/k) / (ka - k)
    pub fn peak_time(&self) -> f32 {
        if self.route == AdministrationRoute::Oral {
            let ka = self.adme.absorption_rate;
            let k = self.elimination_rate;
            if ka > k {
                (ka / k).ln() / (ka - k)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// 峰浓度
    pub fn peak_concentration(&self) -> f32 {
        let c0 = self.dose * self.adme.bioavailability / self.adme.volume_of_distribution;
        if self.route == AdministrationRoute::Oral {
            c0 * (-self.elimination_rate * self.peak_time()).exp()
        } else {
            c0
        }
    }
}

/// Michaelis-Menten 消除模型
///
/// 酶介导的药物消除，在高浓度时接近零级动力学。
/// 参考：Gabrielsson & Weiner (2016) Chapter 7
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MichaelisMentenElimination {
    /// 最大消除速率 (Vmax, mg/h/kg)
    pub v_max: f32,
    /// Michaelis 常数 (Km, mg/L)
    pub k_m: f32,
    /// 当前浓度 (mg/L)
    pub concentration: f32,
}

impl Default for MichaelisMentenElimination {
    fn default() -> Self {
        Self {
            v_max: 10.0,
            k_m: 5.0,
            concentration: 0.0,
        }
    }
}

impl MichaelisMentenElimination {
    /// 创建新的 Michaelis-Menten 模型
    pub fn new(v_max: f32, k_m: f32, initial_concentration: f32) -> Self {
        Self {
            v_max,
            k_m,
            concentration: initial_concentration,
        }
    }

    /// 计算消除速率
    ///
    /// dC/dt = -Vmax·C/(Km + C)
    pub fn elimination_rate(&self) -> f32 {
        self.v_max * self.concentration / (self.k_m + self.concentration)
    }

    /// 模拟消除（简化 Euler 方法）
    ///
    /// 高浓度时接近零级消除（恒定速率），低浓度时接近一级消除。
    pub fn simulate(&mut self, hours: f32, dt: f32) -> f32 {
        let steps = (hours / dt).ceil() as i32;
        for _ in 0..steps {
            let rate = self.elimination_rate();
            self.concentration -= rate * dt;
            if self.concentration < 0.0 {
                self.concentration = 0.0;
                break;
            }
        }
        self.concentration
    }

    /// 判断是否为零级消除
    ///
    /// 当 C >> Km 时，消除速率接近 Vmax（恒定）
    pub fn is_zero_order(&self) -> bool {
        self.concentration > 10.0 * self.k_m
    }

    /// 判断是否为一级消除
    ///
    /// 当 C << Km 时，消除速率 ≈ (Vmax/Km)·C
    pub fn is_first_order(&self) -> bool {
        self.concentration <= 0.1 * self.k_m
    }

    /// 计算表观消除速率常数
    ///
    /// k_app = Vmax/(Km + C)
    pub fn apparent_rate(&self) -> f32 {
        self.v_max / (self.k_m + self.concentration)
    }
}

/// 药效动力学模型
///
/// Hill 方程描述药物浓度与效应的关系。
/// 参考：Gabrielsson & Weiner (2016) Chapter 5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pharmacodynamics {
    /// 最大效应 (Emax)
    pub e_max: f32,
    /// 半最大效应浓度 (EC50, mg/L)
    pub ec50: f32,
    /// Hill 系数（陡度因子）
    pub hill_coefficient: f32,
    /// 基线效应（无药物时的效应）
    pub baseline_effect: f32,
    /// 效应类型
    pub effect_type: EffectType,
}

/// 效应类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    /// 兴奋效应（增加）
    Excitation,
    /// 抑制效应（减少）
    Inhibition,
}

impl Default for Pharmacodynamics {
    fn default() -> Self {
        Self {
            e_max: 100.0,
            ec50: 10.0,
            hill_coefficient: 1.0,
            baseline_effect: 0.0,
            effect_type: EffectType::Excitation,
        }
    }
}

impl Pharmacodynamics {
    /// 创建新的 PD 模型
    pub fn new(e_max: f32, ec50: f32, hill_coefficient: f32, effect_type: EffectType) -> Self {
        Self {
            e_max,
            ec50,
            hill_coefficient,
            baseline_effect: 0.0,
            effect_type,
        }
    }

    /// Hill 方程计算效应
    ///
    /// E = Emax·C^n/(EC50^n + C^n)
    /// 参考：Hill A.V. (1910) "The possible effects of the aggregation of the molecules..."
    pub fn effect(&self, concentration: f32) -> f32 {
        let c_n = concentration.powf(self.hill_coefficient);
        let ec50_n = self.ec50.powf(self.hill_coefficient);
        let effect_magnitude = self.e_max * c_n / (ec50_n + c_n);

        match self.effect_type {
            EffectType::Excitation => self.baseline_effect + effect_magnitude,
            EffectType::Inhibition => self.baseline_effect - effect_magnitude,
        }
    }

    /// 计算半饱和浓度时的效应
    pub fn effect_at_ec50(&self) -> f32 {
        self.effect(self.ec50)
    }

    /// 计算效应为 50% Emax 时的浓度（即 EC50）
    pub fn concentration_for_effect(&self, target_effect: f32) -> f32 {
        let effect_ratio = match self.effect_type {
            EffectType::Excitation => (target_effect - self.baseline_effect) / self.e_max,
            EffectType::Inhibition => (self.baseline_effect - target_effect) / self.e_max,
        };
        // 从 Hill 方程反推浓度
        if effect_ratio <= 0.0 || effect_ratio >= 1.0 {
            return 0.0;
        }
        self.ec50 * (effect_ratio / (1.0 - effect_ratio)).powf(1.0 / self.hill_coefficient)
    }

    /// 计算陡度
    ///
    /// Hill 系数越大，曲线越陡峭
    pub fn steepness(&self) -> f32 {
        self.hill_coefficient
    }

    /// 计算 EC10 和 EC90 的比率
    ///
    /// 用于量化曲线陡度
    pub fn ec_ratio(&self) -> f32 {
        let ec10 = self.concentration_for_effect(self.e_max * 0.1);
        let ec90 = self.concentration_for_effect(self.e_max * 0.9);
        if ec10 > 0.0 {
            ec90 / ec10
        } else {
            0.0
        }
    }
}

/// 剂量-响应曲线
///
/// 整合 PK 和 PD 模型。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DoseResponse {
    /// PK 模型
    pub pk: Pharmacokinetics,
    /// PD 模型
    pub pd: Pharmacodynamics,
    /// 剂量-响应数据点
    pub curve_data: Vec<(f32, f32)>, // (dose, effect)
}

impl DoseResponse {
    /// 创建新的剂量-响应曲线
    pub fn new(pk: Pharmacokinetics, pd: Pharmacodynamics) -> Self {
        Self {
            pk,
            pd,
            curve_data: Vec::new(),
        }
    }

    /// 计算给定剂量的效应
    pub fn effect_at_dose(&self, dose: f32) -> f32 {
        let c_max = dose * self.pk.adme.bioavailability / self.pk.adme.volume_of_distribution;
        self.pd.effect(c_max)
    }

    /// 生成剂量-响应曲线
    pub fn generate_curve(&mut self, doses: &[f32]) {
        self.curve_data.clear();
        for dose in doses {
            let effect = self.effect_at_dose(*dose);
            self.curve_data.push((*dose, effect));
        }
    }

    /// 查找有效剂量 (ED)
    ///
    /// ED50 为产生 50% 最大效应的剂量
    pub fn effective_dose(&self, effect_percent: f32) -> f32 {
        let target_effect = self.pd.e_max * effect_percent / 100.0;
        let target_concentration = self.pd.concentration_for_effect(target_effect);
        target_concentration * self.pk.adme.volume_of_distribution / self.pk.adme.bioavailability
    }

    /// 查找治疗指数
    ///
    /// TI = TD50 / ED50（毒性剂量/有效剂量）
    pub fn therapeutic_index(&self, td50: f32) -> f32 {
        let ed50 = self.effective_dose(50.0);
        if ed50 > 0.0 {
            td50 / ed50
        } else {
            0.0
        }
    }
}

/// 房室模型
///
/// 多房室模型描述药物的分布动力学。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompartmentModel {
    /// 一室模型 - 药物均匀分布
    OneCompartment,
    /// 二室模型 - 中央室 + 外周室
    TwoCompartment,
    /// 三室模型 - 中央室 + 浅外周室 + 深外周室
    ThreeCompartment,
}

impl CompartmentModel {
    /// 获取房室数量
    pub fn compartment_count(&self) -> usize {
        match self {
            Self::OneCompartment => 1,
            Self::TwoCompartment => 2,
            Self::ThreeCompartment => 3,
        }
    }
}

/// 二室模型参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoCompartmentParams {
    /// 中央室容积 (V1, L/kg)
    pub v1: f32,
    /// 外周室容积 (V2, L/kg)
    pub v2: f32,
    /// 分布清除率 (Q, L/h/kg)
    pub intercompartmental_clearance: f32,
    /// 中央室清除率 (CL, L/h/kg)
    pub central_clearance: f32,
}

impl Default for TwoCompartmentParams {
    fn default() -> Self {
        Self {
            v1: 0.3,
            v2: 0.5,
            intercompartmental_clearance: 0.05,
            central_clearance: 0.1,
        }
    }
}

impl TwoCompartmentParams {
    /// 计算分布速率常数
    pub fn distribution_rate_constants(&self) -> (f32, f32) {
        let k12 = self.intercompartmental_clearance / self.v1;
        let k21 = self.intercompartmental_clearance / self.v2;
        (k12, k21)
    }

    /// 计算消除速率常数
    pub fn elimination_rate_constant(&self) -> f32 {
        self.central_clearance / self.v1
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ===== ADME 测试 =====

    #[test]
    fn test_adme_default() {
        let adme = AdmeParameters::default();
        assert!((adme.absorption_rate - 1.0).abs() < 0.01);
        assert!((adme.volume_of_distribution - 0.5).abs() < 0.01);
        assert!((adme.bioavailability - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_half_life_calculation() {
        let adme = AdmeParameters {
            clearance: 0.1,
            volume_of_distribution: 0.5,
            ..Default::default()
        };
        let t1_2 = adme.calculate_half_life();
        // ln(2)/(0.1/0.5) = 0.693/0.2 = 3.465
        assert!((t1_2 - 3.465).abs() < 0.1);
    }

    #[test]
    fn test_steady_state_concentration() {
        let adme = AdmeParameters {
            clearance: 0.1,
            bioavailability: 0.8,
            ..Default::default()
        };
        let css = adme.steady_state_concentration(100.0, 8.0);
        // Css = F·Dose / (τ·CL) = 0.8·100 / (8·0.1) = 100
        assert!((css - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_time_to_steady_state() {
        let adme = AdmeParameters {
            half_life: 6.0,
            ..Default::default()
        };
        let t_ss = adme.time_to_steady_state();
        assert!((t_ss - 24.0).abs() < 0.01); // 4 * 6 = 24
    }

    // ===== 一室模型测试 =====

    #[test]
    fn test_one_compartment_default() {
        let pk = Pharmacokinetics::default();
        assert!((pk.elimination_rate - 0.2).abs() < 0.01);
        assert_eq!(pk.route, AdministrationRoute::Oral);
    }

    #[test]
    fn test_one_compartment_iv_dose() {
        let adme = AdmeParameters {
            volume_of_distribution: 1.0,
            clearance: 0.2,
            ..Default::default()
        };
        let pk = Pharmacokinetics::new(100.0, AdministrationRoute::Intravenous, adme);
        // C0 = Dose/Vd = 100/1 = 100 mg/L
        assert!((pk.plasma_concentration - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_one_compartment_elimination() {
        let adme = AdmeParameters {
            volume_of_distribution: 1.0,
            clearance: 0.2,
            ..Default::default()
        };
        let mut pk = Pharmacokinetics::new(100.0, AdministrationRoute::Intravenous, adme);
        let k = 0.2; // CL/Vd

        // 模拟 3.465 小时（约一个半衰期）
        let c_t = pk.simulate_elimination(3.465);
        // C(t) = C0·e^(-kt) = 100·e^(-0.2·3.465) ≈ 50
        assert!((c_t - 50.0).abs() < 5.0);
    }

    #[test]
    fn test_one_compartment_half_life() {
        let adme = AdmeParameters {
            volume_of_distribution: 1.0,
            clearance: 0.2,
            ..Default::default()
        };
        let pk = Pharmacokinetics::new(100.0, AdministrationRoute::Intravenous, adme);
        let t1_2 = pk.half_life();
        // t1/2 = ln(2)/k = 0.693/0.2 = 3.465
        assert!((t1_2 - 3.465).abs() < 0.1);
    }

    #[test]
    fn test_auc_calculation() {
        let adme = AdmeParameters {
            volume_of_distribution: 1.0,
            clearance: 0.2,
            bioavailability: 1.0,
            ..Default::default()
        };
        let pk = Pharmacokinetics::new(100.0, AdministrationRoute::Intravenous, adme);
        // AUC = Dose/CL = 100/0.2 = 500
        assert!((pk.auc() - 500.0).abs() < 10.0);
    }

    // ===== Michaelis-Menten 测试 =====

    #[test]
    fn test_michaelis_menten_default() {
        let mm = MichaelisMentenElimination::default();
        assert!((mm.v_max - 10.0).abs() < 0.01);
        assert!((mm.k_m - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_michaelis_menten_elimination_rate() {
        let mm = MichaelisMentenElimination::new(10.0, 5.0, 50.0);
        // 当 C = 50, Km = 5: rate = 10·50/(5+50) = 9.09 mg/h
        let rate = mm.elimination_rate();
        assert!((rate - 9.09).abs() < 0.2);
    }

    #[test]
    fn test_michaelis_menten_zero_order() {
        let mm = MichaelisMentenElimination::new(10.0, 5.0, 100.0);
        // C >> Km (100 > 50)，接近零级
        assert!(mm.is_zero_order());
        let rate = mm.elimination_rate();
        assert!((rate - 10.0).abs() < 0.5); // 接近 Vmax
    }

    #[test]
    fn test_michaelis_menten_first_order() {
        let mm = MichaelisMentenElimination::new(10.0, 5.0, 0.5);
        // C = Km/10，接近一级消除
        assert!(mm.is_first_order());
        let apparent_rate = mm.apparent_rate();
        // k_app = Vmax/(Km+C) = 10/5.5 ≈ 1.818，渐近值 Vmax/Km = 2.0
        // C=Km/10 时一级近似偏差约 9%，容差 0.2 (10%)
        assert!((apparent_rate - 2.0).abs() < 0.2);
    }

    #[test]
    fn test_michaelis_menten_simulation() {
        let mut mm = MichaelisMentenElimination::new(10.0, 5.0, 50.0);
        let c_final = mm.simulate(1.0, 0.01);
        assert!(c_final > 0.0);
        assert!(c_final < 50.0);
    }

    // ===== Hill 方程测试 =====

    #[test]
    fn test_hill_equation_default() {
        let pd = Pharmacodynamics::default();
        assert!((pd.e_max - 100.0).abs() < 0.01);
        assert!((pd.ec50 - 10.0).abs() < 0.01);
        assert!((pd.hill_coefficient - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_hill_equation_at_ec50() {
        let pd = Pharmacodynamics::new(100.0, 10.0, 1.0, EffectType::Excitation);
        let effect = pd.effect(10.0); // EC50
        // E = 100·10/(10+10) = 50
        assert!((effect - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_hill_equation_high_concentration() {
        let pd = Pharmacodynamics::new(100.0, 10.0, 1.0, EffectType::Excitation);
        let effect = pd.effect(100.0);
        // E = 100·100/(10+100) = 90.9
        assert!((effect - 90.9).abs() < 1.0);
    }

    #[test]
    fn test_hill_equation_low_concentration() {
        let pd = Pharmacodynamics::new(100.0, 10.0, 1.0, EffectType::Excitation);
        let effect = pd.effect(1.0);
        // E = 100·1/(10+1) = 9.09
        assert!((effect - 9.09).abs() < 1.0);
    }

    #[test]
    fn test_hill_coefficient_steepness() {
        let pd_steep = Pharmacodynamics::new(100.0, 10.0, 3.0, EffectType::Excitation);
        let pd_flat = Pharmacodynamics::new(100.0, 10.0, 0.5, EffectType::Excitation);

        let e_steep_at_9 = pd_steep.effect(9.0);
        let e_flat_at_9 = pd_flat.effect(9.0);

        // C < EC50 时，Hill 系数高的效应更低（更陡地趋向 0 或 Emax）
        // 两者都 < 50（因为 C=9 < EC50=10），但 steep 应远低于 flat
        assert!(e_steep_at_9 < e_flat_at_9);
        assert!(e_steep_at_9 < 50.0);
    }

    #[test]
    fn test_inhibition_effect() {
        let mut pd = Pharmacodynamics::new(80.0, 10.0, 1.0, EffectType::Inhibition);
        pd.baseline_effect = 100.0;

        let effect = pd.effect(10.0);
        // E = 100 - 80·10/(10+10) = 100 - 40 = 60
        assert!((effect - 60.0).abs() < 1.0);
    }

    // ===== 剂量-响应测试 =====

    #[test]
    fn test_dose_response_default() {
        let dr = DoseResponse::default();
        assert!(dr.curve_data.is_empty());
    }

    #[test]
    fn test_dose_response_effect() {
        let adme = AdmeParameters {
            volume_of_distribution: 1.0,
            bioavailability: 1.0,
            ..Default::default()
        };
        let pk = Pharmacokinetics::new(100.0, AdministrationRoute::Intravenous, adme);
        let pd = Pharmacodynamics::new(100.0, 100.0, 1.0, EffectType::Excitation);
        let dr = DoseResponse::new(pk, pd);

        let effect = dr.effect_at_dose(100.0);
        // Cmax = 100 mg/L, EC50 = 100, E = 50
        assert!((effect - 50.0).abs() < 5.0);
    }

    #[test]
    fn test_effective_dose_ed50() {
        let adme = AdmeParameters {
            volume_of_distribution: 1.0,
            bioavailability: 1.0,
            ..Default::default()
        };
        let pk = Pharmacokinetics::new(100.0, AdministrationRoute::Intravenous, adme);
        let pd = Pharmacodynamics::new(100.0, 100.0, 1.0, EffectType::Excitation);
        let dr = DoseResponse::new(pk, pd);

        let ed50 = dr.effective_dose(50.0);
        // ED50 应接近 100 mg/kg
        assert!((ed50 - 100.0).abs() < 10.0);
    }

    #[test]
    fn test_therapeutic_index() {
        let adme = AdmeParameters {
            volume_of_distribution: 1.0,
            bioavailability: 1.0,
            ..Default::default()
        };
        let pk = Pharmacokinetics::new(100.0, AdministrationRoute::Intravenous, adme);
        let pd = Pharmacodynamics::new(100.0, 100.0, 1.0, EffectType::Excitation);
        let dr = DoseResponse::new(pk, pd);

        let ti = dr.therapeutic_index(500.0);
        // TI = 500/100 = 5
        assert!((ti - 5.0).abs() < 0.5);
    }

    // ===== 房室模型测试 =====

    #[test]
    fn test_compartment_count() {
        assert_eq!(CompartmentModel::OneCompartment.compartment_count(), 1);
        assert_eq!(CompartmentModel::TwoCompartment.compartment_count(), 2);
        assert_eq!(CompartmentModel::ThreeCompartment.compartment_count(), 3);
    }

    #[test]
    fn test_two_compartment_distribution_rates() {
        let params = TwoCompartmentParams::default();
        let (k12, k21) = params.distribution_rate_constants();
        // k12 = Q/V1 = 0.05/0.3 = 0.167
        assert!((k12 - 0.167).abs() < 0.01);
        // k21 = Q/V2 = 0.05/0.5 = 0.1
        assert!((k21 - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_two_compartment_elimination_rate() {
        let params = TwoCompartmentParams::default();
        let k10 = params.elimination_rate_constant();
        // k10 = CL/V1 = 0.1/0.3 = 0.333
        assert!((k10 - 0.333).abs() < 0.01);
    }
}