//! 次生代谢物模块
//!
//! 覆盖三大类次生代谢物（萜类 / 酚类 / 含氮化合物）及其子类、生态功能与生物合成途径。
//! 萜类细分为单萜、倍半萜、二萜、三萜、四萜、多萜；酚类细分为木质素、单宁、黄酮类、
//! 香豆素、木脂素、芪类；含氮化合物细分为生物碱、芥子油苷、含氰苷、非蛋白氨基酸。
//!
//! 本模块为废土植物学 (ae_botany) crate 的核心生化模块，描述植物在恶劣环境下
//! 产生的次生代谢物及其生态功能、合成动力学与化感作用。

use serde::{Deserialize, Serialize};

// ============================================================================
// 代谢物大类与子类
// ============================================================================

/// 次生代谢物三大类
///
/// 依据碳骨架与生物合成途径，植物次生代谢物主要分为萜类、酚类与含氮化合物三大类。
/// 每一大类下再细分为多个子类，对应不同的生态功能与化学性质。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecondaryMetaboliteClass {
    /// 萜类 — 由异戊二烯单位 (C5) 缩合而成的脂溶性化合物
    Terpenoid,
    /// 酚类 — 含有一个或多个羟基的芳香族化合物
    Phenolic,
    /// 含氮化合物 — 含氮元素的次生代谢物（生物碱、含氰苷等）
    NitrogenContaining,
}

/// 萜类子类（按异戊二烯单位数量分类）
///
/// 萜类由 C5 异戊二烯单位头尾缩合而成，按单位数可分为单萜至多萜。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerpenoidType {
    /// 单萜 C10 — 芳香油主要成分（薄荷醇、柠檬烯）
    Monoterpene,
    /// 倍半萜 C15 — 植物防御与信号分子（青蒿素、法尼烯）
    Sesquiterpene,
    /// 二萜 C20 — 树脂、赤霉素前体（紫杉醇、植醇）
    Diterpene,
    /// 三萜 C30 — 甾醇、皂苷（β-香树脂醇、豆甾醇）
    Triterpene,
    /// 四萜 C40 — 类胡萝卜素（β-胡萝卜素、番茄红素）
    Tetraterpene,
    /// 多萜 (C5)n — 橡胶、杜仲胶
    Polyterpene,
}

impl TerpenoidType {
    /// 返回该萜类子类对应的异戊二烯单位数（多萜返回估算平均值 5000）
    pub fn isoprene_units(self) -> f32 {
        match self {
            TerpenoidType::Monoterpene => 2.0,
            TerpenoidType::Sesquiterpene => 3.0,
            TerpenoidType::Diterpene => 4.0,
            TerpenoidType::Triterpene => 6.0,
            TerpenoidType::Tetraterpene => 8.0,
            TerpenoidType::Polyterpene => 5000.0,
        }
    }

    /// 返回碳原子数（每个异戊二烯单位含 5 个碳）
    pub fn carbon_count(self) -> f32 {
        self.isoprene_units() * 5.0
    }
}

/// 酚类子类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhenolicType {
    /// 木质素 — 维管植物细胞壁主要成分，赋予机械强度与疏水性
    Lignin,
    /// 单宁 — 分为水解单宁与缩合单宁，具收敛性，防御食草动物
    Tannin,
    /// 黄酮类 — 花色素苷、黄酮醇、异黄酮，UV 防护与传粉吸引
    Flavonoid,
    /// 香豆素 — 羟基肉桂酸内酯，具防御与信号功能
    Coumarin,
    /// 木脂素 — 苯丙素二聚体，植保素活性
    Lignan,
    /// 芪类 — 二苯乙烯骨架，白藜芦醇类，抗真菌植保素
    Stilbene,
}

/// 含氮化合物子类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NitrogenCompoundType {
    /// 生物碱 — 含氮碱性有机物，多具显著生理活性（吗啡、奎宁、咖啡因）
    Alkaloid,
    /// 芥子油苷 — 十字花科特有含硫葡萄糖苷，水解生成异硫氰酸酯
    Glucosinolate,
    /// 含氰苷 — 水解释放 HCN，具强烈防御作用（苦杏仁苷、亚麻苦苷）
    CyanogenicGlycoside,
    /// 非蛋白氨基酸 — 不参与蛋白质合成，多具毒性（刀豆氨酸、β-氨基丙腈）
    NonproteinAmino,
}

// ============================================================================
// 生态功能与合成途径
// ============================================================================

/// 生态功能
///
/// 次生代谢物在植物生态适应中发挥多重功能，包括防御、吸引、化感作用、
/// 紫外防护、胁迫耐受与信号传导。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EcologicalFunction {
    /// 防御 — 抵御食草动物与病原体侵染
    Defense,
    /// 吸引 — 吸引传粉者与种子传播者
    Attractant,
    /// 化感作用 — 抑制或促进邻近植物生长
    Allelopathy,
    /// 紫外防护 — 吸收紫外辐射，保护组织免受 DNA 损伤
    UVProtection,
    /// 胁迫耐受 — 提高对干旱、盐渍、低温等非生物胁迫的耐受
    StressTolerance,
    /// 信号传导 — 种内/种间化学通讯（如挥发物介导的预警）
    Signaling,
}

/// 生物合成途径
///
/// 次生代谢物合成通过若干核心途径衔接初生代谢与次生代谢。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiosyntheticPathway {
    /// 甲基赤藓醇磷酸途径 (MEP) — 质体途径，主要合成单萜、二萜、类胡萝卜素
    MEP,
    /// 甲羟戊酸途径 (MVA) — 细胞质途径，主要合成倍半萜、三萜、甾醇
    MVA,
    /// 莽草酸途径 — 合成芳香族氨基酸及多数酚类前体
    Shikimate,
    /// 乙酸途径 (丙二酰辅酶 A 途径) — 合成脂肪酸与多酮类黄酮骨架
    Acetate,
    /// 氨基酸途径 — 由氨基酸脱羧、甲基化等反应合成生物碱
    AminoAcid,
}

impl BiosyntheticPathway {
    /// 返回该途径的基础温度系数 Q10（温度每升高 10℃ 反应速率倍数）
    pub fn base_q10(self) -> f32 {
        match self {
            BiosyntheticPathway::MEP => 2.2,
            BiosyntheticPathway::MVA => 2.0,
            BiosyntheticPathway::Shikimate => 1.9,
            BiosyntheticPathway::Acetate => 2.1,
            BiosyntheticPathway::AminoAcid => 2.3,
        }
    }

    /// 返回该途径对光照的依赖系数（0..1，1 表示完全依赖光照）
    pub fn light_dependency(self) -> f32 {
        match self {
            // MEP 途径依赖质体中的光合产物，对光强敏感
            BiosyntheticPathway::MEP => 0.85,
            // 莽草酸途径需要磷酸烯醇式丙酮酸 (PEP) 与赤藓糖-4-磷酸，光合依赖
            BiosyntheticPathway::Shikimate => 0.75,
            // 乙酸途径依赖乙酰辅酶 A，间接依赖光合
            BiosyntheticPathway::Acetate => 0.55,
            // MVA 途径在细胞质中，依赖较少
            BiosyntheticPathway::MVA => 0.30,
            // 氨基酸途径依赖氮代谢，光依赖中等
            BiosyntheticPathway::AminoAcid => 0.40,
        }
    }

    /// 返回最适温度（℃）
    pub fn optimal_temperature(self) -> f32 {
        match self {
            BiosyntheticPathway::MEP => 28.0,
            BiosyntheticPathway::MVA => 26.0,
            BiosyntheticPathway::Shikimate => 25.0,
            BiosyntheticPathway::Acetate => 27.0,
            BiosyntheticPathway::AminoAcid => 24.0,
        }
    }
}

/// 合成途径单步反应
///
/// 描述酶促反应的动力学参数，遵循米氏动力学与催化常数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathwayStep {
    /// 酶名称（如 "DXS", "HMGR", "PAL"）
    pub enzyme: String,
    /// 底物名称
    pub substrate: String,
    /// 产物名称
    pub product: String,
    /// 米氏常数 Km (mmol/L)
    pub km: f32,
    /// 催化常数 kcat (1/s)
    pub kcat: f32,
    /// 调节因子 0..1（1 表示完全激活，0 表示完全抑制）
    pub regulation: f32,
}

impl PathwayStep {
    /// 计算该步反应的限速速率 (mmol/L/s)
    ///
    /// 假设底物饱和浓度为 1.0 mmol/L，使用简化米氏方程：
    /// v = kcat * [E] * [S] / (Km + [S]) * regulation
    /// 其中 [E] 取归一化 1.0，[S] 取 1.0 mmol/L。
    pub fn rate(&self) -> f32 {
        let substrate_conc = 1.0_f32;
        let enzyme_conc = 1.0_f32;
        let v = (self.kcat * enzyme_conc * substrate_conc) / (self.km + substrate_conc);
        v * self.regulation.clamp(0.0, 1.0)
    }
}

// ============================================================================
// 代谢物定义
// ============================================================================

/// 单个次生代谢物
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryMetabolite {
    /// 代谢物名称（中文/通用名）
    pub name: String,
    /// 大类
    pub class: SecondaryMetaboliteClass,
    /// 子类字符串描述（与子类枚举对应）
    pub subclass: String,
    /// 分子式（如 "C10H16O"）
    pub molecular_formula: String,
    /// 分子量 (g/mol)
    pub molecular_weight: f32,
    /// 含量 (mg/g 干重)
    pub concentration_mg_g: f32,
    /// 主要生态功能
    pub ecological_function: EcologicalFunction,
    /// 毒性水平 0..1（1 表示对哺乳动物剧毒）
    pub toxicity_level: f32,
}

// ============================================================================
// 核心计算函数
// ============================================================================

/// 估算次生代谢物合成速率
///
/// 综合考虑途径温度系数 (Q10)、最适温度偏差、光照依赖与途径限速步骤。
/// 返回归一化合成速率 (0..1+)，无单位。
///
/// # 参数
/// - `pathway`: 生物合成途径
/// - `steps`: 途径反应步骤序列
/// - `temp_c`: 环境温度 (℃)
/// - `light_intensity`: 光照强度 (0..1，1 为全日照)
///
/// # 算法
/// 1. 计算 Q10 温度系数修正：在低于最适温度时按 Q10 升高，高于最适则线性衰减
/// 2. 计算光照修正：rate_light = 1 - (1 - light) * light_dependency
/// 3. 取途径中速率最慢的步骤作为限速步骤 (rate-limiting step)
/// 4. 合成速率 = 限速速率 * 温度修正 * 光照修正
pub fn synthesis_rate(
    pathway: &BiosyntheticPathway,
    steps: &[PathwayStep],
    temp_c: f32,
    light_intensity: f32,
) -> f32 {
    if steps.is_empty() {
        return 0.0;
    }

    // 温度修正
    let optimal = pathway.optimal_temperature();
    let q10 = pathway.base_q10();
    let temp_factor = if temp_c <= optimal {
        // 低于最适温度按 Q10 估算
        let delta = (optimal - temp_c).max(0.0) / 10.0;
        q10.powf(-delta)
    } else {
        // 高于最适温度线性衰减，超过最适 +10℃ 后快速下降
        let excess = (temp_c - optimal).max(0.0);
        let penalty = (excess / 10.0).min(1.0);
        q10 * (1.0 - penalty * 0.7)
    };

    // 光照修正
    let light_factor = 1.0 - (1.0 - light_intensity.clamp(0.0, 1.0)) * pathway.light_dependency();

    // 限速步骤 — 取所有步骤中最小速率
    let limiting_rate = steps.iter().map(|s| s.rate()).fold(f32::INFINITY, f32::min);

    limiting_rate * temp_factor * light_factor
}

/// 萜类合成（根据类型和碳数）
///
/// 基于前体 IPP/DMAPP 浓度与异戊二烯基转移酶活性估算产量。
///
/// # 参数
/// - `terpene_type`: 萜类子类
/// - `precursor_concentration`: 前体浓度 (mmol/L)，IPP+DMAPP
/// - `enzyme_activity`: 酶活性 (0..1，1 为最大活性)
///
/// # 算法
/// 产量 = 前体浓度 / 异戊二烯单位数 * 酶活性 * 转化效率常数
/// 多萜因单位数极大，按饱和函数处理避免数值发散。
pub fn terpenoid_yield(
    terpene_type: TerpenoidType,
    precursor_concentration: f32,
    enzyme_activity: f32,
) -> f32 {
    if precursor_concentration <= 0.0 || enzyme_activity <= 0.0 {
        return 0.0;
    }

    let units = terpene_type.isoprene_units();
    let activity = enzyme_activity.clamp(0.0, 1.0);

    // 转化效率常数（mmol/L 速率系数）
    const CONVERSION_EFFICIENCY: f32 = 0.85;

    // 对于多萜 (units 极大)，使用饱和函数避免发散
    if units > 100.0 {
        // Michaelis-Menten 饱和形式
        let half_sat = 5.0_f32; // mmol/L
        let saturation = precursor_concentration / (half_sat + precursor_concentration);
        return saturation * activity * CONVERSION_EFFICIENCY * 100.0;
    }

    // 普通萜类：每个分子需要 units 个 IPP，故产物摩尔数与单位数成反比
    let yield_per_unit = precursor_concentration / units;
    yield_per_unit * activity * CONVERSION_EFFICIENCY
}

/// 酚类合成（受光照和胁迫诱导）
///
/// 酚类合成主要通过莽草酸途径，PAL 酶 (苯丙氨酸解氨酶) 是限速酶，
/// 受光照强烈诱导，UV-B 与生物胁迫可显著上调。
///
/// # 参数
/// - `pheno_type`: 酚类子类
/// - `light_intensity`: 光照强度 (0..1)
/// - `stress_level`: 胁迫水平 (0..1)
///
/// # 算法
/// - 木质素受胁迫诱导较强，光照中等
/// - 黄酮类受光照与 UV 强烈诱导
/// - 芪类为植保素，胁迫诱导为主
/// - 单宁、香豆素、木脂素介于其间
pub fn phenolic_yield(pheno_type: PhenolicType, light_intensity: f32, stress_level: f32) -> f32 {
    let light = light_intensity.clamp(0.0, 1.0);
    let stress = stress_level.clamp(0.0, 1.0);

    // 子类特异性的光照系数与胁迫系数
    let (light_coef, stress_coef, base_rate) = match pheno_type {
        // 木质素 — 持续合成，胁迫诱导次生壁加厚
        PhenolicType::Lignin => (0.5, 0.6, 1.2),
        // 黄酮类 — UV/光强诱导为主
        PhenolicType::Flavonoid => (0.9, 0.4, 0.8),
        // 单宁 — 防御性化合物，胁迫诱导
        PhenolicType::Tannin => (0.4, 0.7, 0.9),
        // 香豆素 — 光照与胁迫协同
        PhenolicType::Coumarin => (0.6, 0.5, 0.7),
        // 木脂素 — 防御诱导
        PhenolicType::Lignan => (0.5, 0.7, 0.6),
        // 芪类 — 植保素，强胁迫诱导
        PhenolicType::Stilbene => (0.3, 0.9, 0.3),
    };

    // 光照诱导因子
    let light_factor = light * light_coef + (1.0 - light_coef) * 0.3;
    // 胁迫诱导因子（非线性，胁迫阈值后陡升）
    let stress_factor = stress.powf(1.5) * stress_coef + (1.0 - stress_coef) * 0.2;

    base_rate * light_factor * stress_factor
}

/// 生物碱合成（受氮供应影响）
///
/// 生物碱合成依赖氨基酸前体（如鸟氨酸、赖氨酸、色氨酸、酪氨酸），
/// 氮供应充足时合成上调；植物幼年期合成较低，成熟期达峰值后下降。
///
/// # 参数
/// - `nitrogen_available`: 可利用氮 (mmol/L)
/// - `plant_age_day`: 植物日龄 (d)
///
/// # 算法
/// 1. 氮饱和函数（Michaelis-Menten）
/// 2. 年龄曲线：幼年低 → 成熟峰 → 衰老略降
/// 3. 综合产量 = 氮因子 * 年龄因子 * 系数
pub fn alkaloid_yield(nitrogen_available: f32, plant_age_day: f32) -> f32 {
    if nitrogen_available <= 0.0 || plant_age_day < 0.0 {
        return 0.0;
    }

    // 氮饱和 — 半饱和浓度 8 mmol/L
    let half_sat_n = 8.0_f32;
    let nitrogen_factor = nitrogen_available / (half_sat_n + nitrogen_available);

    // 年龄曲线 — 60 天达峰，钟形分布
    let peak_age = 60.0_f32;
    let sigma = 45.0_f32;
    let age_factor = (-(plant_age_day - peak_age).powi(2) / (2.0 * sigma * sigma)).exp();

    // 系数
    const BASE_RATE: f32 = 2.5; // mg/g 干重归一化
    BASE_RATE * nitrogen_factor * age_factor
}

/// 防御诱导（食草伤害诱导次生代谢物增加）
///
/// 食草动物取食后，植物通过茉莉酸 (JA) 信号途径诱导次生代谢物合成，
/// 通常在伤害后 12-24 小时达峰，随后逐步衰减。
///
/// # 参数
/// - `damage_level`: 伤害水平 (0..1)
/// - `time_since_damage_h`: 伤害后时间 (h)
///
/// # 算法
/// 使用钟形时间窗函数，峰值约在 18 小时；时间窗宽度随伤害水平增大。
pub fn defense_induction(damage_level: f32, time_since_damage_h: f32) -> f32 {
    if damage_level <= 0.0 || time_since_damage_h < 0.0 {
        return 0.0;
    }

    let damage = damage_level.clamp(0.0, 1.0);

    // 峰值时间与时间窗宽度
    let peak_time = 18.0_f32; // h
    let sigma = 6.0 + damage * 4.0; // 伤害越大，反应持续越久

    // 时间窗钟形函数
    let time_window =
        (-(time_since_damage_h - peak_time).powi(2) / (2.0 * sigma * sigma)).exp();

    // 最大诱导倍数 — 严重伤害可达 5 倍
    let max_induction = 1.0 + damage * 4.0;

    max_induction * time_window
}

/// 化感作用强度
///
/// 化感物质对目标植物的抑制/促进效应取决于浓度与目标物种敏感度。
/// 采用 Hill 方程形式，浓度越高效应越强，但存在饱和上限。
///
/// # 参数
/// - `concentration`: 化感物质浓度 (mg/L)
/// - `target_species_sensitivity`: 目标物种敏感度 (0..1，1 为极敏感)
///
/// # 返回
/// 抑制率 (0..1)，0 表示无抑制，1 表示完全抑制
pub fn allelopathic_effect(concentration: f32, target_species_sensitivity: f32) -> f32 {
    if concentration <= 0.0 {
        return 0.0;
    }

    let sensitivity = target_species_sensitivity.clamp(0.0, 1.0);

    // Hill 方程参数
    // EC50 半效应浓度 — 敏感度越高，EC50 越低
    let ec50 = 50.0 * (1.0 - sensitivity * 0.8); // 10..50 mg/L
    let hill_coef = 1.5_f32; // 协同系数

    // 抑制率 = C^n / (EC50^n + C^n)
    let c_n = concentration.powf(hill_coef);
    let ec50_n = ec50.powf(hill_coef);
    let inhibition = c_n / (ec50_n + c_n);

    // 敏感度上限调整
    inhibition * (0.4 + sensitivity * 0.6)
}

// ============================================================================
// 已知代谢物数据库
// ============================================================================

/// 返回已知次生代谢物数据库
///
/// 涵盖三大类代表性化合物，包含分子式、分子量、典型含量与生态功能。
pub fn known_metabolites() -> Vec<SecondaryMetabolite> {
    vec![
        // ---------------- 萜类 ----------------
        SecondaryMetabolite {
            name: "柠檬烯".to_string(),
            class: SecondaryMetaboliteClass::Terpenoid,
            subclass: "Monoterpene".to_string(),
            molecular_formula: "C10H16".to_string(),
            molecular_weight: 136.23,
            concentration_mg_g: 2.5,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.15,
        },
        SecondaryMetabolite {
            name: "薄荷醇".to_string(),
            class: SecondaryMetaboliteClass::Terpenoid,
            subclass: "Monoterpene".to_string(),
            molecular_formula: "C10H20O".to_string(),
            molecular_weight: 156.27,
            concentration_mg_g: 4.2,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.10,
        },
        SecondaryMetabolite {
            name: "青蒿素".to_string(),
            class: SecondaryMetaboliteClass::Terpenoid,
            subclass: "Sesquiterpene".to_string(),
            molecular_formula: "C15H22O5".to_string(),
            molecular_weight: 282.33,
            concentration_mg_g: 8.0,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.35,
        },
        SecondaryMetabolite {
            name: "紫杉醇".to_string(),
            class: SecondaryMetaboliteClass::Terpenoid,
            subclass: "Diterpene".to_string(),
            molecular_formula: "C47H51NO14".to_string(),
            molecular_weight: 853.91,
            concentration_mg_g: 0.5,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.85,
        },
        SecondaryMetabolite {
            name: "β-谷甾醇".to_string(),
            class: SecondaryMetaboliteClass::Terpenoid,
            subclass: "Triterpene".to_string(),
            molecular_formula: "C29H50O".to_string(),
            molecular_weight: 414.71,
            concentration_mg_g: 1.8,
            ecological_function: EcologicalFunction::StressTolerance,
            toxicity_level: 0.05,
        },
        SecondaryMetabolite {
            name: "β-胡萝卜素".to_string(),
            class: SecondaryMetaboliteClass::Terpenoid,
            subclass: "Tetraterpene".to_string(),
            molecular_formula: "C40H56".to_string(),
            molecular_weight: 536.87,
            concentration_mg_g: 0.8,
            ecological_function: EcologicalFunction::UVProtection,
            toxicity_level: 0.02,
        },
        // ---------------- 酚类 ----------------
        SecondaryMetabolite {
            name: "木质素".to_string(),
            class: SecondaryMetaboliteClass::Phenolic,
            subclass: "Lignin".to_string(),
            molecular_formula: "C9H10O2 (单体)".to_string(),
            molecular_weight: 150.17,
            concentration_mg_g: 200.0,
            ecological_function: EcologicalFunction::StressTolerance,
            toxicity_level: 0.01,
        },
        SecondaryMetabolite {
            name: "缩合单宁".to_string(),
            class: SecondaryMetaboliteClass::Phenolic,
            subclass: "Tannin".to_string(),
            molecular_formula: "C15H12O7 (单体)".to_string(),
            molecular_weight: 304.25,
            concentration_mg_g: 45.0,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.40,
        },
        SecondaryMetabolite {
            name: "槲皮素".to_string(),
            class: SecondaryMetaboliteClass::Phenolic,
            subclass: "Flavonoid".to_string(),
            molecular_formula: "C15H10O7".to_string(),
            molecular_weight: 302.24,
            concentration_mg_g: 3.5,
            ecological_function: EcologicalFunction::UVProtection,
            toxicity_level: 0.08,
        },
        SecondaryMetabolite {
            name: "花青素".to_string(),
            class: SecondaryMetaboliteClass::Phenolic,
            subclass: "Flavonoid".to_string(),
            molecular_formula: "C15H11O+".to_string(),
            molecular_weight: 207.25,
            concentration_mg_g: 5.0,
            ecological_function: EcologicalFunction::Attractant,
            toxicity_level: 0.03,
        },
        SecondaryMetabolite {
            name: "香豆素".to_string(),
            class: SecondaryMetaboliteClass::Phenolic,
            subclass: "Coumarin".to_string(),
            molecular_formula: "C9H6O2".to_string(),
            molecular_weight: 146.14,
            concentration_mg_g: 1.2,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.30,
        },
        SecondaryMetabolite {
            name: "白藜芦醇".to_string(),
            class: SecondaryMetaboliteClass::Phenolic,
            subclass: "Stilbene".to_string(),
            molecular_formula: "C14H12O3".to_string(),
            molecular_weight: 228.25,
            concentration_mg_g: 0.8,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.20,
        },
        // ---------------- 含氮化合物 ----------------
        SecondaryMetabolite {
            name: "咖啡因".to_string(),
            class: SecondaryMetaboliteClass::NitrogenContaining,
            subclass: "Alkaloid".to_string(),
            molecular_formula: "C8H10N4O2".to_string(),
            molecular_weight: 194.19,
            concentration_mg_g: 12.0,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.55,
        },
        SecondaryMetabolite {
            name: "吗啡".to_string(),
            class: SecondaryMetaboliteClass::NitrogenContaining,
            subclass: "Alkaloid".to_string(),
            molecular_formula: "C17H19NO3".to_string(),
            molecular_weight: 285.34,
            concentration_mg_g: 2.0,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.95,
        },
        SecondaryMetabolite {
            name: "奎宁".to_string(),
            class: SecondaryMetaboliteClass::NitrogenContaining,
            subclass: "Alkaloid".to_string(),
            molecular_formula: "C20H24N2O2".to_string(),
            molecular_weight: 324.42,
            concentration_mg_g: 6.5,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.60,
        },
        SecondaryMetabolite {
            name: "黑芥子苷".to_string(),
            class: SecondaryMetaboliteClass::NitrogenContaining,
            subclass: "Glucosinolate".to_string(),
            molecular_formula: "C10H16KNO9S2".to_string(),
            molecular_weight: 397.51,
            concentration_mg_g: 18.0,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.45,
        },
        SecondaryMetabolite {
            name: "苦杏仁苷".to_string(),
            class: SecondaryMetaboliteClass::NitrogenContaining,
            subclass: "CyanogenicGlycoside".to_string(),
            molecular_formula: "C20H27NO11".to_string(),
            molecular_weight: 457.42,
            concentration_mg_g: 3.0,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.90,
        },
        SecondaryMetabolite {
            name: "刀豆氨酸".to_string(),
            class: SecondaryMetaboliteClass::NitrogenContaining,
            subclass: "NonproteinAmino".to_string(),
            molecular_formula: "C5H12N4O3".to_string(),
            molecular_weight: 176.17,
            concentration_mg_g: 8.0,
            ecological_function: EcologicalFunction::Defense,
            toxicity_level: 0.70,
        },
    ]
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terpenoid_yield_monoterpene() {
        // 单萜 (C10, 2 个异戊二烯单位) — 前体浓度越高产量越高
        let low = terpenoid_yield(TerpenoidType::Monoterpene, 1.0, 0.8);
        let mid = terpenoid_yield(TerpenoidType::Monoterpene, 5.0, 0.8);
        let high = terpenoid_yield(TerpenoidType::Monoterpene, 10.0, 0.8);

        assert!(low > 0.0, "低浓度应仍产出正值");
        assert!(mid > low, "中浓度产量应高于低浓度");
        assert!(high > mid, "高浓度产量应高于中浓度");
    }

    #[test]
    fn test_terpenoid_yield_polyterpene_saturation() {
        // 多萜 (橡胶) 使用饱和函数，不应发散
        let y = terpenoid_yield(TerpenoidType::Polyterpene, 100.0, 1.0);
        assert!(y.is_finite(), "多萜产量应为有限值");
        assert!(y > 0.0, "多萜产量应为正");
        // 极高浓度也不应爆炸
        let y_extreme = terpenoid_yield(TerpenoidType::Polyterpene, 1_000_000.0, 1.0);
        assert!(y_extreme.is_finite(), "极高浓度产量仍应有限");
    }

    #[test]
    fn test_terpenoid_yield_zero_inputs() {
        assert_eq!(
            terpenoid_yield(TerpenoidType::Monoterpene, 0.0, 0.8),
            0.0,
            "前体为零应无产量"
        );
        assert_eq!(
            terpenoid_yield(TerpenoidType::Monoterpene, 5.0, 0.0),
            0.0,
            "酶活为零应无产量"
        );
    }

    #[test]
    fn test_defense_induction_bell_curve() {
        // 伤害后诱导应先升后降，约 18 小时达峰
        let damage = 0.8;
        let early = defense_induction(damage, 1.0);
        let peak = defense_induction(damage, 18.0);
        let late = defense_induction(damage, 72.0);

        assert!(peak > early, "18h 应高于 1h");
        assert!(peak > late, "18h 应高于 72h");
        // 峰值附近应有显著诱导 (大于 1)
        assert!(
            peak > 1.0,
            "高强度伤害峰值应大于基础水平 1.0，实际 {}",
            peak
        );
    }

    #[test]
    fn test_defense_induction_no_damage() {
        assert_eq!(defense_induction(0.0, 18.0), 0.0, "无伤害应无诱导");
        assert_eq!(
            defense_induction(0.5, 0.0),
            defense_induction(0.5, 0.0)
        );
        // 时间为 0 时（峰值在 18h）应有非零但较小的值
        let t0 = defense_induction(0.5, 0.0);
        assert!(t0 >= 0.0 && t0 < 1.0, "t=0 时诱导应较小");
    }

    #[test]
    fn test_phenolic_yield_light_enhancement() {
        // 光照增强酚类增加 — 以黄酮类为例
        let dark = phenolic_yield(PhenolicType::Flavonoid, 0.0, 0.3);
        let dim = phenolic_yield(PhenolicType::Flavonoid, 0.3, 0.3);
        let bright = phenolic_yield(PhenolicType::Flavonoid, 0.8, 0.3);
        let full = phenolic_yield(PhenolicType::Flavonoid, 1.0, 0.3);

        assert!(dim > dark, "弱光应高于黑暗");
        assert!(bright > dim, "强光应高于弱光");
        assert!(full >= bright, "全日照应不低于强光");
    }

    #[test]
    fn test_phenolic_yield_stilbene_stress_induced() {
        // 芪类为植保素，胁迫诱导为主
        let low_stress = phenolic_yield(PhenolicType::Stilbene, 0.5, 0.1);
        let high_stress = phenolic_yield(PhenolicType::Stilbene, 0.5, 0.9);
        assert!(
            high_stress > low_stress * 2.0,
            "高胁迫应显著高于低胁迫（芪类）"
        );
    }

    #[test]
    fn test_allelopathic_effect_concentration_response() {
        // 浓度越高效应越强
        let sensitivity = 0.7;
        let low = allelopathic_effect(1.0, sensitivity);
        let mid = allelopathic_effect(25.0, sensitivity);
        let high = allelopathic_effect(200.0, sensitivity);

        assert!(low < mid, "低浓度应弱于中浓度");
        assert!(mid < high, "中浓度应弱于高浓度");
        // 饱和上限
        assert!(
            high <= 1.0,
            "抑制率不应超过 1.0，实际 {}",
            high
        );
    }

    #[test]
    fn test_allelopathic_effect_zero_concentration() {
        assert_eq!(
            allelopathic_effect(0.0, 0.8),
            0.0,
            "零浓度应无化感效应"
        );
    }

    #[test]
    fn test_allelopathic_effect_sensitivity_ordering() {
        // 同浓度下，敏感度高的目标应受抑制更强
        let conc = 30.0;
        let resistant = allelopathic_effect(conc, 0.1);
        let sensitive = allelopathic_effect(conc, 0.9);
        assert!(
            sensitive > resistant,
            "敏感物种应受更强抑制"
        );
    }

    #[test]
    fn test_known_metabolites_nonempty() {
        let list = known_metabolites();
        assert!(!list.is_empty(), "已知代谢物列表不应为空");
        // 应覆盖三大类
        let has_terp = list
            .iter()
            .any(|m| m.class == SecondaryMetaboliteClass::Terpenoid);
        let has_phen = list
            .iter()
            .any(|m| m.class == SecondaryMetaboliteClass::Phenolic);
        let has_n = list
            .iter()
            .any(|m| m.class == SecondaryMetaboliteClass::NitrogenContaining);
        assert!(has_terp, "应包含萜类");
        assert!(has_phen, "应包含酚类");
        assert!(has_n, "应包含含氮化合物");
        // 所有毒性水平应在 0..1
        for m in &list {
            assert!(
                m.toxicity_level >= 0.0 && m.toxicity_level <= 1.0,
                "代谢物 {} 毒性水平应在 0..1，实际 {}",
                m.name,
                m.toxicity_level
            );
            assert!(
                m.molecular_weight > 0.0,
                "代谢物 {} 分子量应为正",
                m.name
            );
        }
    }

    #[test]
    fn test_synthesis_rate_pathway() {
        // 构造简单 MEP 途径两步反应
        let steps = vec![
            PathwayStep {
                enzyme: "DXS".to_string(),
                substrate: "丙酮酸+甘油醛-3-磷酸".to_string(),
                product: "DXP".to_string(),
                km: 0.5,
                kcat: 12.0,
                regulation: 0.9,
            },
            PathwayStep {
                enzyme: "DXR".to_string(),
                substrate: "DXP".to_string(),
                product: "MEP".to_string(),
                km: 0.3,
                kcat: 8.0,
                regulation: 0.85,
            },
        ];
        let rate_optimal = synthesis_rate(&BiosyntheticPathway::MEP, &steps, 28.0, 1.0);
        let rate_cold = synthesis_rate(&BiosyntheticPathway::MEP, &steps, 10.0, 1.0);
        let rate_dark = synthesis_rate(&BiosyntheticPathway::MEP, &steps, 28.0, 0.0);

        assert!(rate_optimal > 0.0, "最适条件下合成速率应大于 0");
        assert!(
            rate_cold < rate_optimal,
            "低温应使速率降低"
        );
        // MEP 对光强高度依赖，黑暗下应明显降低
        assert!(
            rate_dark < rate_optimal * 0.5,
            "黑暗下 MEP 途径速率应显著降低"
        );
    }

    #[test]
    fn test_synthesis_rate_empty_steps() {
        let rate = synthesis_rate(&BiosyntheticPathway::Shikimate, &[], 25.0, 1.0);
        assert_eq!(rate, 0.0, "无步骤应返回 0");
    }

    #[test]
    fn test_terpenoid_type_carbon_count() {
        assert_eq!(TerpenoidType::Monoterpene.carbon_count(), 10.0);
        assert_eq!(TerpenoidType::Sesquiterpene.carbon_count(), 15.0);
        assert_eq!(TerpenoidType::Diterpene.carbon_count(), 20.0);
        assert_eq!(TerpenoidType::Triterpene.carbon_count(), 30.0);
        assert_eq!(TerpenoidType::Tetraterpene.carbon_count(), 40.0);
    }

    #[test]
    fn test_alkaloid_yield_peak_age() {
        // 60 天达峰
        let young = alkaloid_yield(15.0, 20.0);
        let peak = alkaloid_yield(15.0, 60.0);
        let old = alkaloid_yield(15.0, 150.0);
        assert!(peak > young, "成熟期应高于幼年期");
        assert!(peak > old, "成熟期应高于衰老期");
    }

    #[test]
    fn test_alkaloid_yield_nitrogen_response() {
        // 氮供应增加，产量增加（饱和）
        let low_n = alkaloid_yield(1.0, 60.0);
        let mid_n = alkaloid_yield(10.0, 60.0);
        let high_n = alkaloid_yield(100.0, 60.0);
        assert!(mid_n > low_n, "中氮应高于低氮");
        assert!(high_n > mid_n, "高氮应高于中氮");
        // 饱和趋势 — 高氮与极高氮差距应小于中氮与低氮差距
        let extreme_n = alkaloid_yield(1000.0, 60.0);
        let gap_high = high_n - mid_n;
        let gap_extreme = extreme_n - high_n;
        assert!(
            gap_extreme < gap_high,
            "氮饱和后增量应递减"
        );
    }
}
