//! 根系系统模块
//!
//! 覆盖根类型分类、根构型、菌根共生、根际微生物、水分与营养吸收动力学、
//! 固氮根瘤及根分泌物等子模块。所有数值计算基于生理生态学经验模型，
//! 适用于荒漠植被在胁迫环境下的根系行为模拟。

use serde::{Deserialize, Serialize};

// ============================================================================
// 一、根类型与根构型
// ============================================================================

/// 根类型分类
///
/// 依据形态与功能差异划分，用于根系分段建模。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RootType {
    /// 主根（直根系）— 由胚根直接发育而成
    Taproot,
    /// 侧根 — 自主根或其分支上发生
    Lateral,
    /// 不定根 — 非胚根起源，可由茎、叶等部位发生
    Adventitious,
    /// 根毛 — 根表皮细胞突起，负责水分与离子吸收
    RootHair,
    /// 支柱根 — 提供机械支撑的不定根（如玉米支柱根）
    Prop,
    /// 气生根 — 暴露于空气中的根（如榕树气生根）
    Aerial,
    /// 储藏根 — 储藏养分的变态根（如胡萝卜、甘薯）
    Storage,
}

/// 根系构型类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RootArchitecture {
    /// 直根系（双子叶植物典型）— 主根发达，深扎土壤
    TaprootSystem,
    /// 须根系（单子叶植物典型）— 主根不发达，由不定根构成网络
    FibrousSystem,
    /// 二型根系 — 同时具备深根与浅根两套系统（如豆科植物）
    Dimorphic,
}

/// 根段（根系拓扑结构的最小单元）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootSegment {
    /// 段唯一 ID
    pub segment_id: u32,
    /// 父段 ID（None 表示根尖起点）
    pub parent_id: Option<u32>,
    /// 根类型
    pub root_type: RootType,
    /// 长度（cm）
    pub length_cm: f32,
    /// 半径（mm）
    pub radius_mm: f32,
    /// 深度（cm，地表为 0，向下为正）
    pub depth_cm: f32,
    /// 水平距离（cm，距植株主轴）
    pub horizontal_cm: f32,
    /// 年龄（天）
    pub age_days: f32,
    /// 生物量（mg）
    pub biomass_mg: f32,
    /// 一级侧根数量
    pub lateral_count: u32,
}

impl RootSegment {
    /// 计算该根段的表面积（cm²）
    ///
    /// 近似为圆柱体侧表面积：S = 2π·r·L
    /// 其中 r 单位为 mm，需转换为 cm。
    pub fn surface_area_cm2(&self) -> f32 {
        let radius_cm = self.radius_mm * 0.1;
        2.0 * std::f32::consts::PI * radius_cm * self.length_cm
    }

    /// 计算该根段体积（cm³）
    pub fn volume_cm3(&self) -> f32 {
        let radius_cm = self.radius_mm * 0.1;
        std::f32::consts::PI * radius_cm * radius_cm * self.length_cm
    }
}

/// 根系整体结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootSystem {
    /// 构型类型
    pub architecture: RootArchitecture,
    /// 所有根段
    pub segments: Vec<RootSegment>,
    /// 最大扎根深度（cm）
    pub max_depth_cm: f32,
    /// 最大水平扩展宽度（cm）
    pub max_width_cm: f32,
    /// 总长度（cm）
    pub total_length_cm: f32,
    /// 总表面积（cm²）
    pub surface_area_cm2: f32,
    /// 总生物量（g）
    pub biomass_g: f32,
}

impl RootSystem {
    /// 创建空的根系结构
    pub fn new(architecture: RootArchitecture) -> Self {
        Self {
            architecture,
            segments: Vec::new(),
            max_depth_cm: 0.0,
            max_width_cm: 0.0,
            total_length_cm: 0.0,
            surface_area_cm2: 0.0,
            biomass_g: 0.0,
        }
    }

    /// 重新计算派生统计量（深度、宽度、长度、表面积、生物量）
    pub fn recompute_stats(&mut self) {
        let mut max_depth = 0.0_f32;
        let mut max_width = 0.0_f32;
        let mut total_len = 0.0_f32;
        let mut total_surf = 0.0_f32;
        let mut total_bio = 0.0_f32;

        for seg in &self.segments {
            if seg.depth_cm > max_depth {
                max_depth = seg.depth_cm;
            }
            if seg.horizontal_cm > max_width {
                max_width = seg.horizontal_cm;
            }
            total_len += seg.length_cm;
            total_surf += seg.surface_area_cm2();
            total_bio += seg.biomass_mg;
        }

        self.max_depth_cm = max_depth;
        self.max_width_cm = max_width;
        self.total_length_cm = total_len;
        self.surface_area_cm2 = total_surf;
        self.biomass_g = total_bio * 0.001; // mg -> g
    }

    /// 按深度分层计算根段总长度（cm），返回 [0..max_depth] 的离散分段
    ///
    /// `layer_count` 为层数，等距划分。
    pub fn length_by_depth(&self, layer_count: usize) -> Vec<f32> {
        if self.segments.is_empty() || layer_count == 0 {
            return Vec::new();
        }
        let max_d = self.max_depth_cm.max(1.0);
        let step = max_d / layer_count as f32;
        let mut buckets = vec![0.0_f32; layer_count];
        for seg in &self.segments {
            let idx = ((seg.depth_cm / step).floor() as usize).min(layer_count - 1);
            buckets[idx] += seg.length_cm;
        }
        buckets
    }

    /// 根系密度（根长密度，RLD），单位 cm/cm³
    ///
    /// `soil_volume_cm3` 为根区土壤体积。
    pub fn root_length_density(&self, soil_volume_cm3: f32) -> f32 {
        if soil_volume_cm3 <= 0.0 {
            return 0.0;
        }
        self.total_length_cm / soil_volume_cm3
    }
}

// ============================================================================
// 二、菌根共生
// ============================================================================

/// 菌根类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MycorrhizaType {
    /// 无菌根共生
    None,
    /// 外生菌根（EcM）— 菌丝在根外形成套膜，皮层细胞间形成哈氏网
    Ectomycorrhiza,
    /// 丛枝菌根（AM）— 菌丝进入皮层细胞内形成丛枝与泡囊
    Arbuscular,
    /// 杜鹃花类菌根 — 发生于杜鹃花科等植物
    Ericoid,
    /// 兰科菌根 — 兰科植物种子萌发必需
    Orchid,
}

/// 菌根共生关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MycorrhizalAssociation {
    /// 菌根类型
    pub mycorrhiza_type: MycorrhizaType,
    /// 定殖率（%，0..100）
    pub colonization_pct: f32,
    /// 菌丝长度（m/g 土壤）
    pub hyphal_length_m_g: f32,
    /// 营养转移速率（mg/cm²/day，磷为主）
    pub nutrient_transfer_rate: f32,
    /// 碳代价（%，植物光合产物分配给菌根的比例）
    pub carbon_cost_pct: f32,
}

impl MycorrhizalAssociation {
    /// 创建默认无菌根的关联
    pub fn none() -> Self {
        Self {
            mycorrhiza_type: MycorrhizaType::None,
            colonization_pct: 0.0,
            hyphal_length_m_g: 0.0,
            nutrient_transfer_rate: 0.0,
            carbon_cost_pct: 0.0,
        }
    }

    /// 创建典型丛枝菌根关联
    pub fn typical_arbuscular() -> Self {
        Self {
            mycorrhiza_type: MycorrhizaType::Arbuscular,
            colonization_pct: 45.0,
            hyphal_length_m_g: 5.0,
            nutrient_transfer_rate: 0.8,
            carbon_cost_pct: 12.0,
        }
    }

    /// 有效定殖率（0..1）
    pub fn effective_colonization(&self) -> f32 {
        (self.colonization_pct / 100.0).clamp(0.0, 1.0)
    }

    /// 增强系数（对营养吸收的放大倍数）
    pub fn enhancement_factor(&self) -> f32 {
        match self.mycorrhiza_type {
            MycorrhizaType::None => 1.0,
            MycorrhizaType::Arbuscular => {
                // 丛枝菌根对磷吸收增强显著
                1.0 + 3.0 * self.effective_colonization()
            }
            MycorrhizaType::Ectomycorrhiza => {
                1.0 + 2.5 * self.effective_colonization()
            }
            MycorrhizaType::Ericoid => 1.0 + 1.5 * self.effective_colonization(),
            MycorrhizaType::Orchid => 1.0 + 0.8 * self.effective_colonization(),
        }
    }
}

// ============================================================================
// 三、根际微生物
// ============================================================================

/// 根际微生物组
///
/// 根际是受根系活动直接影响的土壤微域（通常距根表面 1-2 mm），
/// 微生物丰度显著高于非根际土壤（"根际效应"）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhizosphereMicrobiome {
    /// 细菌总数（CFU/g 干土）
    pub bacterial_count: f32,
    /// 真菌数（CFU/g 干土）
    pub fungal_count: f32,
    /// 固氮菌数量（CFU/g 干土）
    pub n_fixing_bacteria: f32,
    /// 解磷菌数量（CFU/g 干土）
    pub p_solubilizing_bacteria: f32,
    /// 病原菌负荷（0..1）
    pub pathogen_load: f32,
    /// 有益菌比例（0..1）
    pub beneficial_ratio: f32,
}

impl RhizosphereMicrobiome {
    /// 创建默认中性根际微生物组
    pub fn default_microbiome() -> Self {
        Self {
            bacterial_count: 1.0e7,
            fungal_count: 1.0e5,
            n_fixing_bacteria: 1.0e4,
            p_solubilizing_bacteria: 5.0e3,
            pathogen_load: 0.1,
            beneficial_ratio: 0.5,
        }
    }

    /// 健康度评分（0..1）
    ///
    /// 综合考虑有益菌比例与病原负荷。
    pub fn health_score(&self) -> f32 {
        let beneficial = self.beneficial_ratio.clamp(0.0, 1.0);
        let pathogen = self.pathogen_load.clamp(0.0, 1.0);
        (beneficial * 0.7 + (1.0 - pathogen) * 0.3).clamp(0.0, 1.0)
    }

    /// 总微生物量（CFU/g）
    pub fn total_count(&self) -> f32 {
        self.bacterial_count + self.fungal_count
    }
}

// ============================================================================
// 四、营养与水分吸收
// ============================================================================

/// 营养吸收日通量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrientUptake {
    /// 氮吸收（mg N/day）
    pub nitrogen_uptake_mg_day: f32,
    /// 磷吸收（mg P/day）
    pub phosphorus_uptake_mg_day: f32,
    /// 钾吸收（mg K/day）
    pub potassium_uptake_mg_day: f32,
    /// 水分吸收（ml/day）
    pub water_uptake_ml_day: f32,
}

impl NutrientUptake {
    /// 零吸收
    pub fn zero() -> Self {
        Self {
            nitrogen_uptake_mg_day: 0.0,
            phosphorus_uptake_mg_day: 0.0,
            potassium_uptake_mg_day: 0.0,
            water_uptake_ml_day: 0.0,
        }
    }
}

/// Michaelis-Menten 吸收动力学参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptakeKinetics {
    /// 最大吸收速率 Vmax（nmol/g/min）
    pub vmax: f32,
    /// 米氏常数 Km（μM）
    pub km: f32,
    /// 最低浓度阈值 Cmin（μM），低于此值不吸收
    pub cmin: f32,
}

impl UptakeKinetics {
    /// 氮吸收典型参数
    pub fn nitrogen() -> Self {
        Self {
            vmax: 12.0,
            km: 50.0,
            cmin: 2.0,
        }
    }
    /// 磷吸收典型参数（磷移动性低，Km 较小）
    pub fn phosphorus() -> Self {
        Self {
            vmax: 4.0,
            km: 8.0,
            cmin: 0.5,
        }
    }
    /// 钾吸收典型参数
    pub fn potassium() -> Self {
        Self {
            vmax: 15.0,
            km: 25.0,
            cmin: 1.0,
        }
    }
}

// ============================================================================
// 五、固氮根瘤
// ============================================================================

/// 固氮根瘤
///
/// 豆科植物与根瘤菌（Rhizobium 等）共生形成，
/// 通过固氮酶将 N₂ 还原为 NH₃。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootNodule {
    /// 根瘤数量
    pub count: u32,
    /// 总质量（mg）
    pub mass_mg: f32,
    /// 固氮酶活性（μmol C₂H₄/g/h，乙炔还原法测定）
    pub nitrogenase_activity: f32,
    /// 固氮量（mg N/day）
    pub nitrogen_fixed_mg_day: f32,
    /// 效率（0..1）
    pub efficiency: f32,
}

impl RootNodule {
    /// 无根瘤
    pub fn none() -> Self {
        Self {
            count: 0,
            mass_mg: 0.0,
            nitrogenase_activity: 0.0,
            nitrogen_fixed_mg_day: 0.0,
            efficiency: 0.0,
        }
    }

    /// 创建典型豆科植物根瘤
    pub fn typical_legume() -> Self {
        Self {
            count: 50,
            mass_mg: 200.0,
            nitrogenase_activity: 30.0,
            nitrogen_fixed_mg_day: 15.0,
            efficiency: 0.7,
        }
    }
}

// ============================================================================
// 六、关键函数实现
// ============================================================================

/// 温度响应函数（Q10 = 2.0 经验模型）
///
/// 在 `optimal_c` 附近响应最大，过高或过低都会抑制。
fn temperature_response(temp_c: f32, optimal_c: f32, q10: f32) -> f32 {
    if temp_c <= 0.0 {
        return 0.0;
    }
    if temp_c <= optimal_c {
        let q10_factor = q10.powf((temp_c - 20.0) / 10.0);
        return q10_factor.max(0.0).min(2.5);
    }
    // 高温段：从 optimal 线性下降到 0（optimal+15 时为 0）
    let excess = temp_c - optimal_c;
    let max_range = 15.0;
    if excess >= max_range {
        0.0
    } else {
        let q10_at_opt = q10.powf((optimal_c - 20.0) / 10.0);
        q10_at_opt * (1.0 - excess / max_range)
    }
}

/// 根系生长（深度和广度扩展）
///
/// 基于温度、水分和时间的简化生长模型。
/// - `temp_c`：土壤温度（°C）
/// - `moisture`：土壤含水量（0..1，田间持水量为 1.0）
/// - `days`：生长天数
///
/// 生长量服从温度×水分双因子乘积模型，且存在最适响应曲线。
pub fn root_growth(system: &mut RootSystem, temp_c: f32, moisture: f32, days: f32) {
    if days <= 0.0 || system.segments.is_empty() {
        return;
    }

    // 温度因子（最适 22°C，Q10=2.0）
    let t_factor = temperature_response(temp_c, 22.0, 2.0);
    // 水分因子：过低胁迫，过高缺氧
    let m_factor = {
        let m = moisture.clamp(0.0, 1.0);
        // 最适水分约 0.6-0.8
        if m < 0.6 {
            m / 0.6 // 干旱胁迫
        } else if m <= 0.85 {
            1.0
        } else {
            // 渍水缺氧
            (1.0 - (m - 0.85) * 3.0).max(0.1)
        }
    };

    // 综合生长速率系数
    let growth_rate = t_factor * m_factor; // 0..2.5
    let daily_extension = 0.5 * growth_rate; // cm/day 基础速率

    for seg in &mut system.segments {
        // 根尖段（无 lateral_count 上限）继续延伸
        let extension = daily_extension * days;
        seg.length_cm += extension;
        seg.age_days += days;
        // 生物量积累（mg/cm 假定 1.0）
        seg.biomass_mg += extension * 1.0;
        // 深度推进（主根/支柱根向下，侧根/不定根向外）
        match seg.root_type {
            RootType::Taproot | RootType::Prop | RootType::Storage => {
                seg.depth_cm += extension;
            }
            RootType::Lateral | RootType::Adventitious | RootType::RootHair => {
                seg.horizontal_cm += extension;
            }
            RootType::Aerial => {
                // 气生根向下生长但不入土
                seg.depth_cm += extension * 0.5;
            }
        }
    }

    system.recompute_stats();
}

/// 水分吸收（Feddes 模型简化版）
///
/// 返回单位根长的水分吸收速率（ml/cm/day）。
///
/// - `depth_cm`：根段深度（cm）
/// - `root_density`：根长密度（cm/cm³）
/// - `soil_water_potential_mpa`：土壤水势（MPa，负值）
///
/// Feddes 模型：在水势 -0.1 至 -0.5 MPa 区间为最适，
/// 低于 -1.5 MPa（萎蔫点）吸收停止。
pub fn water_uptake(depth_cm: f32, root_density: f32, soil_water_potential_mpa: f32) -> f32 {
    if root_density <= 0.0 {
        return 0.0;
    }
    // 深度衰减因子：表层根多吸收多
    let depth_factor = (-depth_cm / 50.0).exp().max(0.05);

    // Feddes 水势响应函数 S(h)
    let potential = soil_water_potential_mpa; // 负值
    let h_response = if potential > -0.1 {
        // 接近饱和，缺氧抑制
        let over = (-potential - 0.1).max(0.0); // 偏离 -0.1 的程度
        (1.0 - over * 5.0).max(0.0)
    } else if potential >= -0.5 {
        // 最适区间
        1.0
    } else if potential > -1.5 {
        // 下降区间
        let frac = (-0.5 - potential) / 1.0; // 0..1
        1.0 - frac
    } else {
        // 低于萎蔫点
        0.0
    };

    // 基础吸收速率 0.05 ml/cm/day
    0.05 * root_density * depth_factor * h_response
}

/// 营养吸收（Michaelis-Menten 动力学）
///
/// 返回吸收速率（nmol/min），按根表面积折算。
///
/// 公式：v = Vmax · (C - Cmin) / (Km + (C - Cmin))
///
/// - `kinetics`：吸收动力学参数
/// - `soil_concentration`：土壤溶液浓度（μM）
/// - `root_surface`：参与吸收的根表面积（cm²）
pub fn nutrient_uptake(kinetics: &UptakeKinetics, soil_concentration: f32, root_surface: f32) -> f32 {
    if root_surface <= 0.0 || soil_concentration <= kinetics.cmin {
        return 0.0;
    }
    let c_eff = soil_concentration - kinetics.cmin;
    let rate = kinetics.vmax * c_eff / (kinetics.km + c_eff);
    // 按表面积缩放（假定 1 cm² 对应 0.1 g 根鲜重）
    rate * root_surface * 0.1
}

/// 菌根增强吸收
///
/// 在基础吸收量上叠加菌根菌丝网络的额外吸收。
pub fn mycorrhiza_enhanced_uptake(base_uptake: f32, association: &MycorrhizalAssociation) -> f32 {
    if base_uptake <= 0.0 {
        return 0.0;
    }
    let factor = association.enhancement_factor();
    // 菌丝长度额外贡献（每 m/g 增 5% 吸收）
    let hyphal_bonus = 1.0 + (association.hyphal_length_m_g * 0.05).min(0.5);
    base_uptake * factor * hyphal_bonus / 1.0
}

/// 固氮量估算
///
/// 综合根瘤活性、温度与光照（光合产物供给）估算日固氮量。
///
/// - `nodule`：根瘤结构
/// - `temp_c`：温度（°C）
/// - `light_intensity`：光强（0..1，相对值）
pub fn nitrogen_fixation(nodule: &RootNodule, temp_c: f32, light_intensity: f32) -> f32 {
    if nodule.count == 0 || nodule.mass_mg <= 0.0 {
        return 0.0;
    }
    // 固氮酶最适温度 25°C
    let t_factor = temperature_response(temp_c, 25.0, 2.0);
    // 光强驱动光合产物供应
    let light_factor = light_intensity.clamp(0.0, 1.0).powi(2);

    // 基础固氮速率（mg N/mg nodule/day）
    let base_rate = nodule.nitrogenase_activity * 0.001; // 简化换算
    let mass_g = nodule.mass_mg * 0.001;
    let daily = base_rate * mass_g * t_factor * light_factor * nodule.efficiency * 1000.0; // 转回 mg
    daily.max(0.0)
}

/// 根系生物量分配（根冠比 R/S）
///
/// 水分与营养胁迫下，植物倾向于将更多光合产物分配给根系，
/// 以扩大吸收表面。这是经典的"功能平衡"模型。
///
/// - `water_stress`：水分胁迫（0..1，1 为严重胁迫）
/// - `nutrient_stress`：营养胁迫（0..1，1 为严重胁迫）
///
/// 返回根冠比（g/g）。典型范围 0.1-0.6。
pub fn root_shoot_ratio(water_stress: f32, nutrient_stress: f32) -> f32 {
    let w = water_stress.clamp(0.0, 1.0);
    let n = nutrient_stress.clamp(0.0, 1.0);
    // 基础根冠比 0.2，每单位胁迫增加 0.4
    let base = 0.2;
    let stress_increase = (w + n) * 0.4;
    // 上限 0.8（极端胁迫）
    (base + stress_increase).min(0.8)
}

/// 根分泌物量
///
/// 根系向土壤分泌有机物（糖、氨基酸、有机酸等），
/// 胁迫条件下分泌增加，以招募有益微生物或活化难溶养分。
///
/// - `root_biomass_g`：根系生物量（g）
/// - `stress_level`：胁迫水平（0..1）
///
/// 返回日分泌量（mg C/day）。
pub fn root_exudation(root_biomass_g: f32, stress_level: f32) -> f32 {
    if root_biomass_g <= 0.0 {
        return 0.0;
    }
    let stress = stress_level.clamp(0.0, 1.0);
    // 正常条件下分泌约 5% 生物量，胁迫下可增至 15%
    let rate = 0.05 + 0.10 * stress;
    root_biomass_g * rate * 1000.0 // g -> mg
}

/// 构建简单根系（主根 + 侧根）
///
/// 根据构型类型生成基础拓扑结构。
pub fn build_simple_root(architecture: RootArchitecture, max_depth_cm: f32) -> RootSystem {
    let mut system = RootSystem::new(architecture);
    let mut next_id: u32 = 0;

    match architecture {
        RootArchitecture::TaprootSystem => {
            // 主根延伸至 max_depth，每 20cm 分生一级侧根
            let taproot_id = next_id;
            next_id += 1;
            let taproot = RootSegment {
                segment_id: taproot_id,
                parent_id: None,
                root_type: RootType::Taproot,
                length_cm: max_depth_cm,
                radius_mm: 2.5,
                depth_cm: max_depth_cm,
                horizontal_cm: 0.0,
                age_days: 30.0,
                biomass_mg: max_depth_cm * 8.0,
                lateral_count: (max_depth_cm / 20.0).floor() as u32,
            };
            system.segments.push(taproot);

            // 沿主根生成侧根
            let layer_count = (max_depth_cm / 20.0).floor() as u32;
            for i in 0..layer_count {
                let depth = (i as f32 + 1.0) * 20.0;
                if depth > max_depth_cm {
                    break;
                }
                // 每层 4 条侧根，呈十字分布
                for j in 0..4u32 {
                    let lateral_id = next_id;
                    next_id += 1;
                    let lateral_len = 15.0 * (1.0 - depth / (max_depth_cm + 1.0)).max(0.2);
                    let angle = (j as f32) * std::f32::consts::FRAC_PI_2;
                    let _ = angle;
                    let lateral = RootSegment {
                        segment_id: lateral_id,
                        parent_id: Some(taproot_id),
                        root_type: RootType::Lateral,
                        length_cm: lateral_len,
                        radius_mm: 0.6,
                        depth_cm: depth,
                        horizontal_cm: lateral_len,
                        age_days: 20.0 - i as f32 * 0.5,
                        biomass_mg: lateral_len * 1.2,
                        lateral_count: 0,
                    };
                    system.segments.push(lateral);
                }
            }
        }
        RootArchitecture::FibrousSystem => {
            // 须根系：多条不定根从根基部辐射，深度较浅
            let root_count = 8u32;
            for i in 0..root_count {
                let root_id = next_id;
                next_id += 1;
                let depth = max_depth_cm * 0.6 * (1.0 - (i as f32) * 0.05).max(0.4);
                let horiz = max_depth_cm * 0.4 * ((i as f32 + 1.0) / root_count as f32);
                let seg = RootSegment {
                    segment_id: root_id,
                    parent_id: None,
                    root_type: RootType::Adventitious,
                    length_cm: (depth * depth + horiz * horiz).sqrt(),
                    radius_mm: 0.8,
                    depth_cm: depth,
                    horizontal_cm: horiz,
                    age_days: 25.0,
                    biomass_mg: depth * 3.0,
                    lateral_count: 3,
                };
                system.segments.push(seg);
            }
        }
        RootArchitecture::Dimorphic => {
            // 二型根系：一条深主根 + 浅层须根网络
            let tap_id = next_id;
            next_id += 1;
            let tap = RootSegment {
                segment_id: tap_id,
                parent_id: None,
                root_type: RootType::Taproot,
                length_cm: max_depth_cm,
                radius_mm: 3.0,
                depth_cm: max_depth_cm,
                horizontal_cm: 0.0,
                age_days: 40.0,
                biomass_mg: max_depth_cm * 10.0,
                lateral_count: 6,
            };
            system.segments.push(tap);

            // 浅层须根（深度 0-30cm）
            for i in 0..6u32 {
                let id = next_id;
                next_id += 1;
                let depth = 5.0 + (i as f32) * 4.0;
                let horiz = 10.0 + (i as f32) * 3.0;
                let seg = RootSegment {
                    segment_id: id,
                    parent_id: Some(tap_id),
                    root_type: RootType::Adventitious,
                    length_cm: (depth * depth + horiz * horiz).sqrt(),
                    radius_mm: 0.7,
                    depth_cm: depth,
                    horizontal_cm: horiz,
                    age_days: 20.0,
                    biomass_mg: 25.0,
                    lateral_count: 2,
                };
                system.segments.push(seg);
            }
        }
    }

    system.recompute_stats();
    system
}

// ============================================================================
// 七、单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 水分吸收：水分充足时（最适水势 -0.3 MPa）吸收显著高于萎蔫点附近
    #[test]
    fn test_water_uptake() {
        let optimal = water_uptake(20.0, 1.0, -0.3);
        let wilting = water_uptake(20.0, 1.0, -1.6);
        assert!(
            optimal > wilting,
            "最适水势下吸收应高于萎蔫点，got optimal={} wilting={}",
            optimal,
            wilting
        );
        assert!(optimal > 0.0, "最适水势下吸收应大于 0");
        assert!(wilting <= 0.0, "萎蔫点以下应停止吸收");
    }

    /// 营养吸收：浓度越高吸收越多，最终饱和于 Vmax
    #[test]
    fn test_nutrient_uptake_saturation() {
        let kin = UptakeKinetics::nitrogen();
        let surface = 10.0;
        let low = nutrient_uptake(&kin, 10.0, surface);
        let mid = nutrient_uptake(&kin, 100.0, surface);
        let high = nutrient_uptake(&kin, 1000.0, surface);
        let very_high = nutrient_uptake(&kin, 10000.0, surface);

        assert!(low < mid, "低浓度应吸收少：low={} mid={}", low, mid);
        assert!(mid < high, "中浓度应少于高浓度：mid={} high={}", mid, high);
        // 饱和：极高浓度间差距应小于高浓度翻倍
        let increase_high_to_vhigh = (very_high - high).abs();
        let increase_mid_to_high = high - mid;
        assert!(
            increase_high_to_vhigh < increase_mid_to_high,
            "高浓度区间应趋于饱和"
        );
    }

    /// 固氮：适宜温度（25°C）下固氮量高于低温或高温
    #[test]
    fn test_nitrogen_fixation() {
        let nodule = RootNodule::typical_legume();
        let optimal = nitrogen_fixation(&nodule, 25.0, 0.8);
        let cold = nitrogen_fixation(&nodule, 5.0, 0.8);
        let hot = nitrogen_fixation(&nodule, 40.0, 0.8);
        assert!(optimal > cold, "最适温度固氮应高于低温：opt={} cold={}", optimal, cold);
        assert!(optimal > hot, "最适温度固氮应高于高温：opt={} hot={}", optimal, hot);
        assert!(optimal > 0.0, "最适条件下应固氮 > 0");
    }

    /// 根冠比：水分胁迫时根冠比应高于无胁迫
    #[test]
    fn test_root_shoot_ratio() {
        let no_stress = root_shoot_ratio(0.0, 0.0);
        let water_stress = root_shoot_ratio(0.8, 0.0);
        let full_stress = root_shoot_ratio(1.0, 1.0);
        assert!(
            water_stress > no_stress,
            "水分胁迫应提高根冠比：no={} stress={}",
            no_stress,
            water_stress
        );
        assert!(full_stress > water_stress, "双重胁迫应进一步增加根冠比");
        assert!(full_stress <= 0.8 + 1e-5, "根冠比不应超过上限 0.8");
    }

    /// 构建简单根系：返回有效结构（至少 1 段，统计量非零）
    #[test]
    fn test_build_simple_root() {
        let sys = build_simple_root(RootArchitecture::TaprootSystem, 100.0);
        assert!(!sys.segments.is_empty(), "直根系应至少包含主根段");
        assert!(sys.max_depth_cm > 0.0, "最大深度应大于 0");
        assert!(sys.total_length_cm > 0.0, "总长度应大于 0");
        assert!(sys.biomass_g > 0.0, "生物量应大于 0");
        // 直根系应包含至少一个 Taproot 类型段
        let has_tap = sys
            .segments
            .iter()
            .any(|s| s.root_type == RootType::Taproot);
        assert!(has_tap, "直根系应包含主根段");

        // 须根系
        let fib = build_simple_root(RootArchitecture::FibrousSystem, 60.0);
        assert!(!fib.segments.is_empty(), "须根系应包含不定根段");
        let has_adv = fib
            .segments
            .iter()
            .any(|s| s.root_type == RootType::Adventitious);
        assert!(has_adv, "须根系应包含不定根段");

        // 二型根系
        let dim = build_simple_root(RootArchitecture::Dimorphic, 120.0);
        assert!(dim.segments.len() >= 2, "二型根系应同时包含深根与浅根");
        assert!(dim.max_depth_cm >= 100.0, "二型根系最大深度应接近设定值");
    }

    /// 菌根增强吸收：丛枝菌根应显著放大磷吸收
    #[test]
    fn test_mycorrhiza_enhancement() {
        let base = 1.0_f32;
        let none = MycorrhizalAssociation::none();
        let am = MycorrhizalAssociation::typical_arbuscular();
        let with_none = mycorrhiza_enhanced_uptake(base, &none);
        let with_am = mycorrhiza_enhanced_uptake(base, &am);
        assert!(
            (with_none - base).abs() < 1e-5,
            "无菌根时不应增强：got={}",
            with_none
        );
        assert!(
            with_am > base * 2.0,
            "丛枝菌根应至少翻倍吸收：got={}",
            with_am
        );
    }

    /// 根段表面积与体积计算
    #[test]
    fn test_segment_geometry() {
        let seg = RootSegment {
            segment_id: 0,
            parent_id: None,
            root_type: RootType::Taproot,
            length_cm: 10.0,
            radius_mm: 1.0, // 0.1 cm
            depth_cm: 10.0,
            horizontal_cm: 0.0,
            age_days: 1.0,
            biomass_mg: 1.0,
            lateral_count: 0,
        };
        let surf = seg.surface_area_cm2();
        let vol = seg.volume_cm3();
        // 期望表面 ≈ 2π·0.1·10 = 6.283
        assert!((surf - 6.2832).abs() < 0.01, "表面积计算错误：{}", surf);
        // 期望体积 ≈ π·0.01·10 = 0.314
        assert!((vol - 0.3142).abs() < 0.01, "体积计算错误：{}", vol);
    }

    /// 根分泌物：胁迫增加分泌量
    #[test]
    fn test_root_exudation() {
        let biomass = 10.0;
        let low_stress = root_exudation(biomass, 0.0);
        let high_stress = root_exudation(biomass, 1.0);
        assert!(high_stress > low_stress, "胁迫应增加分泌物量");
        assert!(low_stress > 0.0, "正常条件下也应分泌");
        // 5% of 10g = 500 mg
        assert!((low_stress - 500.0).abs() < 1.0, "正常分泌约 500mg：{}", low_stress);
    }

    /// 根系生长函数：温度与水分应驱动根段长度增加
    #[test]
    fn test_root_growth_extends_segments() {
        let mut sys = build_simple_root(RootArchitecture::TaprootSystem, 50.0);
        let len_before = sys.total_length_cm;
        root_growth(&mut sys, 22.0, 0.7, 10.0);
        assert!(
            sys.total_length_cm > len_before,
            "生长后总长度应增加：before={} after={}",
            len_before,
            sys.total_length_cm
        );
    }

    /// 根际微生物健康度评分
    #[test]
    fn test_microbiome_health() {
        let mut mb = RhizosphereMicrobiome::default_microbiome();
        let h1 = mb.health_score();
        mb.beneficial_ratio = 0.9;
        mb.pathogen_load = 0.05;
        let h2 = mb.health_score();
        assert!(h2 > h1, "提高有益菌比例应提升健康度");
        assert!(h2 <= 1.0 && h2 >= 0.0, "健康度应在 [0,1]");
    }
}
