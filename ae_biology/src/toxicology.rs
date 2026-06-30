//! 毒理学模块
//!
//! 实现毒理学核心概念，包括 LD50、剂量-响应曲线、器官毒性（肝/肾）
//! 和辐射毒性模型。
//!
//! # 生物学背景
//!
//! 毒理学研究化学物质对生物系统的有害作用，主要关注：
//! - 剂量-响应关系：毒性与剂量之间的定量关系
//! - LD50：中位致死剂量，引起 50% 死亡的剂量
//! - 器官特异性毒性：特定器官的损伤机制
//! - 辐射毒性：电离辐射的生物效应
//!
//! # 核心模型
//!
//! - LD50 = 中位致死剂量（概率单位分析）
//! - 剂量-响应 S 曲线：logistic 模型或 probit 模型
//! - 线性无阈值模型（LNT）：风险 = α·剂量（辐射）
//!
//! # 参考文献
//!
//! - Casarett L.J. et al. (2018) "Casarett and Doull's Toxicology."
//! - Eaton D.L., Gilbert S.G. (2013) "Principles of Toxicology."
//! - BEIR VII Report (2006) "Health Risks from Exposure to Low Levels of Ionizing Radiation."

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 毒物类型
///
/// 按毒理学分类的毒物类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToxicantClass {
    /// 化学毒物（有机/无机化合物）
    Chemical,
    /// 重金属毒物（铅、汞、镉等）
    HeavyMetal,
    /// 辐射毒物（电离辐射）
    Radiation,
    /// 生物毒素（细菌、真菌、植物毒素）
    Biotoxin,
    /// 药物毒性（治疗药物过量）
    DrugToxicity,
}

/// 毒物记录
///
/// 单个毒物的毒理学参数。
/// 参考：Casarett & Doull's Toxicology (2018) Chapter 2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toxicant {
    /// 毒物名称
    pub name: String,
    /// 毒物类型
    pub class: ToxicantClass,
    /// LD50 - 中位致死剂量 (mg/kg)
    pub ld50: f32,
    /// LD10 - 10% 死亡剂量 (mg/kg)
    pub ld10: f32,
    /// NOAEL - 无观察到有害效应剂量 (mg/kg)
    pub noael: f32,
    /// LOAEL - 最低观察到有害效应剂量 (mg/kg)
    pub loael: f32,
    /// MTD - 最大耐受剂量 (mg/kg)
    pub mtd: f32,
    /// 安全因子（用于计算 ADI）
    pub safety_factor: f32,
    /// 暴露途径
    pub exposure_route: ExposureRoute,
    /// 主要靶器官
    pub target_organs: Vec<TargetOrgan>,
}

/// 暴露途径
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExposureRoute {
    /// 口服摄入
    Oral,
    /// 吸入
    Inhalation,
    /// 皮肤接触
    Dermal,
    /// 静脉注射
    Intravenous,
    /// 眼接触
    Ocular,
}

/// 靶器官
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetOrgan {
    /// 肝脏
    Liver,
    /// 肾脏
    Kidney,
    /// 神经系统
    NervousSystem,
    /// 心血管系统
    Cardiovascular,
    /// 呼吸系统
    Respiratory,
    /// 生殖系统
    Reproductive,
    /// 免疫系统
    Immune,
    /// 皮肤
    Skin,
    /// 眼睛
    Eye,
    /// 造血系统
    Hematopoietic,
}

impl Default for Toxicant {
    fn default() -> Self {
        Self {
            name: String::new(),
            class: ToxicantClass::Chemical,
            ld50: 100.0,
            ld10: 30.0,
            noael: 10.0,
            loael: 20.0,
            mtd: 80.0,
            safety_factor: 100.0,
            exposure_route: ExposureRoute::Oral,
            target_organs: vec![TargetOrgan::Liver],
        }
    }
}

impl Toxicant {
    /// 创建新毒物
    pub fn new(name: String, class: ToxicantClass, ld50: f32) -> Self {
        Self {
            name,
            class,
            ld50,
            ld10: ld50 * 0.3,
            noael: ld50 * 0.1,
            loael: ld50 * 0.2,
            mtd: ld50 * 0.8,
            safety_factor: 100.0,
            exposure_route: ExposureRoute::Oral,
            target_organs: vec![TargetOrgan::Liver],
        }
    }

    /// 计算每日允许摄入量 (ADI)
    ///
    /// ADI = NOAEL / 安全因子
    /// 参考：Eaton & Gilbert (2013) Chapter 4
    pub fn adi(&self) -> f32 {
        self.noael / self.safety_factor
    }

    /// 计算致死概率
    ///
    /// 使用概率单位（probit）模型：P = Φ((log(D) - log(LD50))/σ)
    pub fn lethality_probability(&self, dose: f32, slope: f32) -> f32 {
        if dose <= 0.0 {
            return 0.0;
        }
        // 简化 logistic 模型（probit 5 对应 50% 死亡率）
        let log_ratio = (dose / self.ld50).ln();
        let probit = 5.0 + slope * log_ratio;
        // P = 1/(1 + exp(-(probit-5)))，probit=5 → P=0.5
        let prob = 1.0 / (1.0 + (-(probit - 5.0)).exp());
        prob.clamp(0.0, 1.0)
    }

    /// 判断剂量是否超过 LD50
    pub fn exceeds_ld50(&self, dose: f32) -> bool {
        dose >= self.ld50
    }

    /// 计算毒性等级
    ///
    /// 根据 GHS 分类标准
    pub fn toxicity_category(&self) -> ToxicityCategory {
        if self.ld50 <= 5.0 {
            ToxicityCategory::Category1
        } else if self.ld50 <= 50.0 {
            ToxicityCategory::Category2
        } else if self.ld50 <= 300.0 {
            ToxicityCategory::Category3
        } else if self.ld50 <= 2000.0 {
            ToxicityCategory::Category4
        } else {
            ToxicityCategory::Category5
        }
    }
}

/// GHS 毒性分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToxicityCategory {
    /// 类别 1 - 极毒 (LD50 ≤ 5 mg/kg)
    Category1,
    /// 类别 2 - 高毒 (LD50 ≤ 50 mg/kg)
    Category2,
    /// 类别 3 - 中毒 (LD50 ≤ 300 mg/kg)
    Category3,
    /// 类别 4 - 低毒 (LD50 ≤ 2000 mg/kg)
    Category4,
    /// 类别 5 - 相对无毒 (LD50 > 2000 mg/kg)
    Category5,
}

/// 剂量-响应曲线
///
/// 描述毒物剂量与效应概率的关系。
/// 参考：Casarett & Doull's Toxicology (2018) Chapter 3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoseResponseCurve {
    /// 曲线 ID
    pub id: u32,
    /// 毒物名称
    pub toxicant: String,
    /// 剂量数据点 (mg/kg)
    pub doses: Vec<f32>,
    /// 效应概率数据点 (0.0-1.0)
    pub responses: Vec<f32>,
    /// LD50
    pub ld50: f32,
    /// 曲线斜率（Hill 系数或 probit 斜率）
    pub slope: f32,
    /// 效应类型
    pub effect_type: DoseResponseType,
}

/// 剂量-响应类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoseResponseType {
    /// 死亡响应
    Mortality,
    /// 器官损伤
    OrganDamage,
    /// 功能障碍
    FunctionalImpairment,
    /// 生化改变
    BiochemicalChange,
}

impl Default for DoseResponseCurve {
    fn default() -> Self {
        Self {
            id: 0,
            toxicant: String::new(),
            doses: Vec::new(),
            responses: Vec::new(),
            ld50: 100.0,
            slope: 2.0,
            effect_type: DoseResponseType::Mortality,
        }
    }
}

impl DoseResponseCurve {
    /// 创建新的剂量-响应曲线
    pub fn new(id: u32, toxicant: String, ld50: f32, slope: f32) -> Self {
        Self {
            id,
            toxicant,
            doses: Vec::new(),
            responses: Vec::new(),
            ld50,
            slope,
            effect_type: DoseResponseType::Mortality,
        }
    }

    /// 计算 S 曲线响应
    ///
    /// 使用 logistic 模型：P = D^n/(LD50^n + D^n)
    /// 参考：Hill 方程在毒理学中的应用
    pub fn calculate_response(&self, dose: f32) -> f32 {
        if dose <= 0.0 {
            return 0.0;
        }
        let dose_n = dose.powf(self.slope);
        let ld50_n = self.ld50.powf(self.slope);
        dose_n / (ld50_n + dose_n)
    }

    /// 生成完整曲线数据
    pub fn generate_curve(&mut self, dose_range: (f32, f32), steps: usize) {
        self.doses.clear();
        self.responses.clear();

        let step_size = (dose_range.1 - dose_range.0) / steps as f32;
        for i in 0..=steps {
            let dose = dose_range.0 + step_size * i as f32;
            let response = self.calculate_response(dose);
            self.doses.push(dose);
            self.responses.push(response);
        }
    }

    /// 查找特定响应对应的剂量
    ///
    /// 如 ED10, ED50, ED90
    pub fn dose_for_response(&self, target_response: f32) -> f32 {
        if target_response <= 0.0 || target_response >= 1.0 {
            return 0.0;
        }
        // 从 logistic 模型反推
        let ratio = target_response / (1.0 - target_response);
        self.ld50 * ratio.powf(1.0 / self.slope)
    }

    /// 计算 ED10（10% 效应剂量）
    pub fn ed10(&self) -> f32 {
        self.dose_for_response(0.1)
    }

    /// 计算 ED90（90% 效应剂量）
    pub fn ed90(&self) -> f32 {
        self.dose_for_response(0.9)
    }

    /// 计算曲线陡度指标
    ///
    /// 使用 ED10/ED90 比率（0-1 之间），越接近 1 表示曲线越陡。
    /// Hill 系数高 → ED10 与 ED90 接近 → 比值大 → 更陡。
    pub fn steepness_index(&self) -> f32 {
        let ed10 = self.ed10();
        let ed90 = self.ed90();
        if ed90 > 0.0 {
            ed10 / ed90
        } else {
            0.0
        }
    }
}

/// 器官毒性记录
///
/// 特定器官的毒性效应和损伤程度。
/// 参考：Casarett & Doull's Toxicology (2018) Chapter 13-15
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganToxicity {
    /// 靶器官
    pub organ: TargetOrgan,
    /// 毒物名称
    pub toxicant: String,
    /// 损伤阈值剂量 (mg/kg)
    pub threshold_dose: f32,
    /// 当前损伤程度 (0.0-1.0)
    pub damage_level: f32,
    /// 可逆性
    pub reversible: bool,
    /// 损伤机制
    pub mechanism: ToxicityMechanism,
    /// 功能丧失比例 (0.0-1.0)
    pub functional_loss: f32,
}

/// 毒性机制
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToxicityMechanism {
    /// 直接细胞毒性
    DirectCytotoxicity,
    /// 氧化应激
    OxidativeStress,
    /// 代谢产物毒性
    MetabolicActivation,
    /// 免疫介导
    ImmuneMediated,
    /// DNA 损伤
    DnaDamage,
    /// 蛋白质损伤
    ProteinDamage,
    /// 膜损伤
    MembraneDamage,
    /// 干扰信号通路
    SignalInterference,
}

impl Default for OrganToxicity {
    fn default() -> Self {
        Self {
            organ: TargetOrgan::Liver,
            toxicant: String::new(),
            threshold_dose: 50.0,
            damage_level: 0.0,
            reversible: true,
            mechanism: ToxicityMechanism::DirectCytotoxicity,
            functional_loss: 0.0,
        }
    }
}

impl OrganToxicity {
    /// 创建新的器官毒性记录
    pub fn new(organ: TargetOrgan, toxicant: String, threshold_dose: f32) -> Self {
        Self {
            organ,
            toxicant,
            threshold_dose,
            damage_level: 0.0,
            reversible: true,
            mechanism: ToxicityMechanism::DirectCytotoxicity,
            functional_loss: 0.0,
        }
    }

    /// 应用剂量计算损伤
    ///
    /// 剂量超过阈值后开始产生损伤
    pub fn apply_dose(&mut self, dose: f32) -> f32 {
        if dose < self.threshold_dose {
            return 0.0;
        }

        // 损伤程度与超阈值剂量成正比
        let excess = dose - self.threshold_dose;
        let new_damage = excess / self.threshold_dose * 0.5;
        self.damage_level = (self.damage_level + new_damage).min(1.0);
        self.functional_loss = self.damage_level;

        self.damage_level
    }

    /// 判断是否达到毒性阈值
    pub fn exceeds_threshold(&self, dose: f32) -> bool {
        dose >= self.threshold_dose
    }

    /// 计算恢复速度
    ///
    /// 可逆损伤可逐步恢复
    pub fn recovery_rate(&self) -> f32 {
        if self.reversible {
            0.05 // 每单位时间恢复 5%
        } else {
            0.0
        }
    }

    /// 模拟恢复过程
    pub fn recover(&mut self, time_units: f32) -> f32 {
        if !self.reversible {
            return self.damage_level;
        }

        let recovery = self.recovery_rate() * time_units;
        self.damage_level = (self.damage_level - recovery).max(0.0);
        self.functional_loss = self.damage_level;

        self.damage_level
    }

    /// 判断器官是否严重受损
    pub fn is_severely_damaged(&self) -> bool {
        self.damage_level > 0.7
    }

    /// 判断器官是否功能衰竭
    pub fn is_failure(&self) -> bool {
        self.functional_loss > 0.9
    }
}

/// 肝毒性特化参数
///
/// 肝脏是药物代谢的主要器官，易受毒物损伤。
/// 参考：Casarett & Doull's Toxicology (2018) Chapter 13
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiverToxicity {
    /// 基础器官毒性
    pub base: OrganToxicity,
    /// 肝细胞坏死比例
    pub necrosis_ratio: f32,
    /// 肝脂肪变性程度
    pub steatosis_level: f32,
    /// 胆汁淤积程度
    pub cholestasis_level: f32,
    /// 肝酶升高倍数 (ALT/AST)
    pub enzyme_elevation: f32,
}

impl Default for LiverToxicity {
    fn default() -> Self {
        Self {
            base: OrganToxicity::new(TargetOrgan::Liver, String::new(), 50.0),
            necrosis_ratio: 0.0,
            steatosis_level: 0.0,
            cholestasis_level: 0.0,
            enzyme_elevation: 1.0,
        }
    }
}

impl LiverToxicity {
    /// 应用肝毒性剂量
    pub fn apply_dose(&mut self, dose: f32) {
        let damage = self.base.apply_dose(dose);
        self.necrosis_ratio = damage * 0.8;
        self.steatosis_level = damage * 0.5;
        self.enzyme_elevation = 1.0 + damage * 10.0;
    }

    /// 判断是否为脂肪肝
    pub fn is_fatty_liver(&self) -> bool {
        self.steatosis_level > 0.3
    }

    /// 判断是否为肝坏死
    pub fn is_necrosis(&self) -> bool {
        self.necrosis_ratio > 0.5
    }
}

/// 辐射毒性记录
///
/// 电离辐射的生物效应，使用线性无阈值模型（LNT）。
/// 参考：BEIR VII Report (2006) "Health Risks from Exposure to Low Levels of Ionizing Radiation."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiationToxicity {
    /// 累积剂量 (mGy)
    pub cumulative_dose: f32,
    /// 急性剂量 (mGy)
    pub acute_dose: f32,
    /// 辐射类型
    pub radiation_type: RadiationType,
    /// 暴露途径
    pub exposure_path: RadiationExposurePath,
    /// 癌症风险系数 (风险/mGy)
    pub cancer_risk_coefficient: f32,
    /// 急性辐射综合征阶段
    pub ars_stage: Option<ArsStage>,
}

/// 辐射类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadiationType {
    /// α 辐射（高 LET，局部损伤）
    Alpha,
    /// β 辐射（中 LET）
    Beta,
    /// γ 辐射（低 LET，穿透性）
    Gamma,
    /// X 射线（低 LET）
    XRay,
    /// 中子辐射（高 LET）
    Neutron,
}

impl RadiationType {
    /// 获取相对生物效应 (RBE)
    ///
    /// 参考：ICRP Publication 92
    pub fn rbe(&self) -> f32 {
        match self {
            Self::Alpha => 20.0,
            Self::Beta => 1.0,
            Self::Gamma => 1.0,
            Self::XRay => 1.0,
            Self::Neutron => 10.0,
        }
    }
}

/// 辐射暴露途径
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadiationExposurePath {
    /// 外照射
    External,
    /// 内照射（吸入）
    InternalInhalation,
    /// 内照射（摄入）
    InternalIngestion,
    /// 皮肤污染
    SkinContamination,
}

/// 急性辐射综合征阶段
///
/// 高剂量急性暴露后的典型病程。
/// 参考：Casarett & Doull's Toxicology (2018) Chapter 22
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArsStage {
    /// 前驱期（恶心、呕吐）
    Prodromal,
    /// 潜伏期（症状缓解）
    Latent,
    /// 极期（症状爆发）
    Critical,
    /// 恢复期
    Recovery,
}

impl Default for RadiationToxicity {
    fn default() -> Self {
        Self {
            cumulative_dose: 0.0,
            acute_dose: 0.0,
            radiation_type: RadiationType::Gamma,
            exposure_path: RadiationExposurePath::External,
            cancer_risk_coefficient: 0.05, // BEIR VII: ~5%/Sv = 0.05%/mGy
            ars_stage: None,
        }
    }
}

impl RadiationToxicity {
    /// 创建新的辐射毒性记录
    pub fn new(radiation_type: RadiationType) -> Self {
        Self {
            cumulative_dose: 0.0,
            acute_dose: 0.0,
            radiation_type,
            exposure_path: RadiationExposurePath::External,
            cancer_risk_coefficient: 0.05,
            ars_stage: None,
        }
    }

    /// 累积辐射剂量
    ///
    /// 剂量具有累积极性效应。
    pub fn add_dose(&mut self, dose: f32) {
        self.cumulative_dose += dose;
    }

    /// 计算等效剂量
    ///
    /// H = D × RBE
    pub fn equivalent_dose(&self) -> f32 {
        self.cumulative_dose * self.radiation_type.rbe()
    }

    /// 计算癌症风险（LNT 模型）
    ///
    /// 风险 = α × 等效剂量
    /// 参考：BEIR VII (2006) 线性无阈值模型
    pub fn cancer_risk(&self) -> f32 {
        let h = self.equivalent_dose();
        h * self.cancer_risk_coefficient / 100.0 // 转换为概率
    }

    /// 判断是否触发急性辐射综合征
    ///
    /// 通常需要 >1 Gy (1000 mGy) 的急性剂量
    pub fn check_ars(&mut self) {
        if self.acute_dose > 1000.0 {
            self.ars_stage = Some(ArsStage::Prodromal);
        } else if self.acute_dose > 2000.0 {
            self.ars_stage = Some(ArsStage::Critical);
        }
    }

    /// 判断急性剂量严重程度
    pub fn acute_severity(&self) -> RadiationSeverity {
        if self.acute_dose < 100.0 {
            RadiationSeverity::Minimal
        } else if self.acute_dose < 500.0 {
            RadiationSeverity::Mild
        } else if self.acute_dose < 1000.0 {
            RadiationSeverity::Moderate
        } else if self.acute_dose < 5000.0 {
            RadiationSeverity::Severe
        } else {
            RadiationSeverity::Fatal
        }
    }
}

/// 辐射严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadiationSeverity {
    /// 极轻微 (< 100 mGy)
    Minimal,
    /// 轻度 (100-500 mGy)
    Mild,
    /// 中度 (500-1000 mGy)
    Moderate,
    /// 重度 (1-5 Gy)
    Severe,
    /// 致命 (> 5 Gy)
    Fatal,
}

/// 毒代动力学
///
/// 毒物在体内的吸收、分布、代谢、排泄过程。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Toxicokinetics {
    /// 吸收分数 (0.0-1.0)
    pub absorption_fraction: f32,
    /// 分布容积 (L/kg)
    pub volume_of_distribution: f32,
    /// 清除率 (L/h/kg)
    pub clearance: f32,
    /// 代谢转化率 (0.0-1.0)
    pub metabolism_rate: f32,
    /// 毒物代谢产物
    pub metabolites: HashMap<String, f32>,
    /// 当前体内负荷 (mg/kg)
    pub body_burden: f32,
}

impl Toxicokinetics {
    /// 创建新的毒代动力学模型
    pub fn new(absorption: f32, vd: f32, clearance: f32) -> Self {
        Self {
            absorption_fraction: absorption,
            volume_of_distribution: vd,
            clearance,
            metabolism_rate: 0.5,
            metabolites: HashMap::new(),
            body_burden: 0.0,
        }
    }

    /// 吸收毒物
    pub fn absorb(&mut self, external_dose: f32) {
        let absorbed = external_dose * self.absorption_fraction;
        self.body_burden += absorbed / self.volume_of_distribution;
    }

    /// 消除毒物
    ///
    /// 一级消除：dC/dt = -k·C
    pub fn eliminate(&mut self, hours: f32) -> f32 {
        let k = self.clearance / self.volume_of_distribution;
        let c_new = self.body_burden * (-k * hours).exp();
        self.body_burden = c_new;
        c_new
    }

    /// 代谢转化
    ///
    /// 产生毒性代谢产物
    pub fn metabolize(&mut self) {
        let metabolized = self.body_burden * self.metabolism_rate;
        self.body_burden -= metabolized;

        // 增加代谢产物记录
        self.metabolites.insert("primary_metabolite".to_string(), metabolized * 0.7);
        self.metabolites.insert("secondary_metabolite".to_string(), metabolized * 0.3);
    }

    /// 计算半衰期
    pub fn half_life(&self) -> f32 {
        let k = self.clearance / self.volume_of_distribution;
        0.693 / k
    }

    /// 计算稳态体内负荷
    pub fn steady_state_burden(&self, daily_dose: f32) -> f32 {
        let absorbed_daily = daily_dose * self.absorption_fraction;
        absorbed_daily / (24.0 * self.clearance)
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 毒物基础测试 =====

    #[test]
    fn test_toxicant_default() {
        let tox = Toxicant::default();
        assert!((tox.ld50 - 100.0).abs() < 0.01);
        assert!((tox.noael - 10.0).abs() < 0.01);
        assert_eq!(tox.class, ToxicantClass::Chemical);
    }

    #[test]
    fn test_toxicant_ld50_creation() {
        let tox = Toxicant::new("TestToxin".to_string(), ToxicantClass::HeavyMetal, 50.0);
        assert!((tox.ld50 - 50.0).abs() < 0.01);
        assert!((tox.ld10 - 15.0).abs() < 0.01); // ld50 * 0.3
        assert!((tox.noael - 5.0).abs() < 0.01); // ld50 * 0.1
    }

    #[test]
    fn test_adi_calculation() {
        let tox = Toxicant {
            noael: 10.0,
            safety_factor: 100.0,
            ..Default::default()
        };
        let adi = tox.adi();
        assert!((adi - 0.1).abs() < 0.01); // 10/100
    }

    #[test]
    fn test_lethality_probability_at_ld50() {
        let tox = Toxicant::default();
        let prob = tox.lethality_probability(tox.ld50, 2.0);
        assert!((prob - 0.5).abs() < 0.1); // LD50 应产生 ~50% 死亡
    }

    #[test]
    fn test_lethality_probability_zero_dose() {
        let tox = Toxicant::default();
        let prob = tox.lethality_probability(0.0, 2.0);
        assert_eq!(prob, 0.0);
    }

    #[test]
    fn test_exceeds_ld50() {
        let tox = Toxicant::new("Test".to_string(), ToxicantClass::Chemical, 100.0);
        assert!(tox.exceeds_ld50(150.0));
        assert!(!tox.exceeds_ld50(50.0));
    }

    #[test]
    fn test_toxicity_category() {
        let cat1 = Toxicant::new("Cat1".to_string(), ToxicantClass::Chemical, 1.0);
        assert_eq!(cat1.toxicity_category(), ToxicityCategory::Category1);

        let cat3 = Toxicant::new("Cat3".to_string(), ToxicantClass::Chemical, 200.0);
        assert_eq!(cat3.toxicity_category(), ToxicityCategory::Category3);

        let cat5 = Toxicant::new("Cat5".to_string(), ToxicantClass::Chemical, 3000.0);
        assert_eq!(cat5.toxicity_category(), ToxicityCategory::Category5);
    }

    // ===== 剂量-响应曲线测试 =====

    #[test]
    fn test_dose_response_curve_default() {
        let curve = DoseResponseCurve::default();
        assert!((curve.ld50 - 100.0).abs() < 0.01);
        assert!((curve.slope - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_dose_response_s_curve() {
        let curve = DoseResponseCurve::new(1, "Test".to_string(), 100.0, 2.0);

        let resp_low = curve.calculate_response(10.0);
        assert!(resp_low < 0.1); // 低剂量低响应

        let resp_at_ld50 = curve.calculate_response(100.0);
        assert!((resp_at_ld50 - 0.5).abs() < 0.05); // LD50 处响应约 50%

        let resp_high = curve.calculate_response(500.0);
        assert!(resp_high > 0.9); // 高剂量高响应
    }

    #[test]
    fn test_dose_for_response() {
        let curve = DoseResponseCurve::new(1, "Test".to_string(), 100.0, 2.0);

        let ed10 = curve.ed10();
        assert!(ed10 < curve.ld50);

        let ed90 = curve.ed90();
        assert!(ed90 > curve.ld50);
    }

    #[test]
    fn test_curve_steepness() {
        let steep_curve = DoseResponseCurve::new(1, "Steep".to_string(), 100.0, 4.0);
        let flat_curve = DoseResponseCurve::new(2, "Flat".to_string(), 100.0, 1.0);

        let steep_index = steep_curve.steepness_index();
        let flat_index = flat_curve.steepness_index();

        // Hill 系数越大，曲线越陡，ED90/ED10 比率越大
        assert!(steep_index > flat_index);
    }

    #[test]
    fn test_curve_generation() {
        let mut curve = DoseResponseCurve::new(1, "Test".to_string(), 100.0, 2.0);
        curve.generate_curve((0.0, 200.0), 20);

        assert_eq!(curve.doses.len(), 21);
        assert_eq!(curve.responses.len(), 21);
    }

    // ===== 器官毒性测试 =====

    #[test]
    fn test_organ_toxicity_default() {
        let organ = OrganToxicity::default();
        assert_eq!(organ.organ, TargetOrgan::Liver);
        assert!((organ.damage_level).abs() < 0.01);
        assert!(organ.reversible);
    }

    #[test]
    fn test_organ_toxicity_threshold() {
        let organ = OrganToxicity::new(TargetOrgan::Kidney, "TestToxin".to_string(), 50.0);

        assert!(!organ.exceeds_threshold(30.0));
        assert!(organ.exceeds_threshold(60.0));
    }

    #[test]
    fn test_organ_damage_progression() {
        let mut organ = OrganToxicity::new(TargetOrgan::Liver, "TestToxin".to_string(), 50.0);

        let damage1 = organ.apply_dose(50.0); // 刚达阈值
        assert!((damage1).abs() < 0.01);

        let damage2 = organ.apply_dose(100.0); // 超阈值 50
        assert!(damage2 > 0.0);
        assert!(damage2 < 1.0);

        organ.apply_dose(200.0); // 大剂量
        assert!(organ.damage_level > 0.5);
    }

    #[test]
    fn test_organ_recovery() {
        let mut organ = OrganToxicity::new(TargetOrgan::Liver, "TestToxin".to_string(), 50.0);
        organ.reversible = true;
        organ.apply_dose(100.0);
        let initial_damage = organ.damage_level;

        organ.recover(10.0);
        assert!(organ.damage_level < initial_damage);
    }

    #[test]
    fn test_organ_non_reversible() {
        let mut organ = OrganToxicity::new(TargetOrgan::Liver, "TestToxin".to_string(), 50.0);
        organ.reversible = false;
        organ.apply_dose(100.0);
        let initial_damage = organ.damage_level;

        organ.recover(10.0);
        assert!((organ.damage_level - initial_damage).abs() < 0.01);
    }

    #[test]
    fn test_severe_damage() {
        let mut organ = OrganToxicity::new(TargetOrgan::Liver, "TestToxin".to_string(), 50.0);
        organ.apply_dose(200.0);
        assert!(organ.is_severely_damaged());
    }

    // ===== 肝毒性测试 =====

    #[test]
    fn test_liver_toxicity_default() {
        let liver = LiverToxicity::default();
        assert!((liver.necrosis_ratio).abs() < 0.01);
        assert!((liver.enzyme_elevation - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_liver_damage_application() {
        let mut liver = LiverToxicity::default();
        liver.base.threshold_dose = 50.0;
        liver.apply_dose(100.0);

        assert!(liver.necrosis_ratio > 0.0);
        assert!(liver.enzyme_elevation > 1.0);
    }

    #[test]
    fn test_fatty_liver_detection() {
        let mut liver = LiverToxicity::default();
        liver.steatosis_level = 0.5;
        assert!(liver.is_fatty_liver());
    }

    #[test]
    fn test_necrosis_detection() {
        let mut liver = LiverToxicity::default();
        liver.necrosis_ratio = 0.7;
        assert!(liver.is_necrosis());
    }

    // ===== 辐射毒性测试 =====

    #[test]
    fn test_radiation_toxicity_default() {
        let rad = RadiationToxicity::default();
        assert!((rad.cumulative_dose).abs() < 0.01);
        assert_eq!(rad.radiation_type, RadiationType::Gamma);
    }

    #[test]
    fn test_radiation_rbe() {
        assert!((RadiationType::Alpha.rbe() - 20.0).abs() < 0.01);
        assert!((RadiationType::Gamma.rbe() - 1.0).abs() < 0.01);
        assert!((RadiationType::Neutron.rbe() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_radiation_dose_accumulation() {
        let mut rad = RadiationToxicity::new(RadiationType::Gamma);
        rad.add_dose(100.0);
        rad.add_dose(50.0);

        assert!((rad.cumulative_dose - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_equivalent_dose() {
        let mut rad = RadiationToxicity::new(RadiationType::Alpha);
        rad.add_dose(10.0); // 10 mGy α 辐射

        let h = rad.equivalent_dose();
        // H = 10 × 20 = 200 mGy (等效)
        assert!((h - 200.0).abs() < 1.0);
    }

    #[test]
    fn test_cancer_risk_lnt() {
        let mut rad = RadiationToxicity::new(RadiationType::Gamma);
        rad.add_dose(100.0); // 100 mGy

        let risk = rad.cancer_risk();
        // 风险 ≈ 100 × 0.05 / 100 = 0.05 (5%)
        assert!(risk > 0.0);
        assert!(risk < 0.1);
    }

    #[test]
    fn test_radiation_severity() {
        let rad_minimal = RadiationToxicity { acute_dose: 50.0, ..Default::default() };
        assert_eq!(rad_minimal.acute_severity(), RadiationSeverity::Minimal);

        let rad_severe = RadiationToxicity { acute_dose: 3000.0, ..Default::default() };
        assert_eq!(rad_severe.acute_severity(), RadiationSeverity::Severe);

        let rad_fatal = RadiationToxicity { acute_dose: 10000.0, ..Default::default() };
        assert_eq!(rad_fatal.acute_severity(), RadiationSeverity::Fatal);
    }

    #[test]
    fn test_ars_trigger() {
        let mut rad = RadiationToxicity { acute_dose: 1500.0, ..Default::default() };
        rad.check_ars();
        assert!(rad.ars_stage.is_some());
    }

    // ===== 毒代动力学测试 =====

    #[test]
    fn test_toxicokinetics_absorption() {
        let mut tk = Toxicokinetics::new(0.8, 1.0, 0.1);
        tk.absorb(100.0);

        // 吸收 80 mg/kg，分布到 1 L/kg → 80 mg/L 体内负荷
        assert!(tk.body_burden > 0.0);
    }

    #[test]
    fn test_toxicokinetics_elimination() {
        let mut tk = Toxicokinetics::new(1.0, 1.0, 0.2);
        tk.body_burden = 100.0;

        let c_final = tk.eliminate(3.465); // 约一个半衰期
        // C = 100 × e^(-0.2×3.465) ≈ 50
        assert!((c_final - 50.0).abs() < 5.0);
    }

    #[test]
    fn test_toxicokinetics_metabolism() {
        let mut tk = Toxicokinetics::new(1.0, 1.0, 0.1);
        tk.metabolism_rate = 0.5;
        tk.body_burden = 100.0;

        tk.metabolize();
        assert!((tk.body_burden - 50.0).abs() < 1.0);
        assert!(tk.metabolites.contains_key("primary_metabolite"));
    }

    #[test]
    fn test_toxicokinetics_half_life() {
        let tk = Toxicokinetics::new(1.0, 1.0, 0.2);
        let t1_2 = tk.half_life();
        // ln(2)/0.2 = 3.465
        assert!((t1_2 - 3.465).abs() < 0.1);
    }
}