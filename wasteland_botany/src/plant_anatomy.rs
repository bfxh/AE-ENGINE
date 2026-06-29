//! 植物解剖模块
//!
//! 覆盖：
//! - 组织类型 (分生/保护/基本/输导/分泌)
//! - 维管系统 (木质部/韧皮部/维管束排列)
//! - 木材结构 (早材/晚材/年轮/心材/边材/射线)
//! - 树皮 (内皮/木栓形成层/外皮)

use serde::{Deserialize, Serialize};

// ============================================================================
// 组织类型 Tissue Types
// ============================================================================

/// 组织大类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TissueCategory {
    /// 分生组织 (meristem)
    Meristem,
    /// 保护组织 (dermal)
    Dermal,
    /// 基本组织 (ground)
    Ground,
    /// 维管组织 (vascular)
    Vascular,
    /// 分泌组织 (secretory)
    Secretory,
}

/// 分生组织按位置分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeristemPosition {
    /// 顶端分生组织 SAM/RAM
    Apical,
    /// 侧生分生组织 (维管形成层/木栓形成层)
    Lateral,
    /// 居间分生组织 (节间基部)
    Intercalary,
}

/// 保护组织
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DermalTissue {
    /// 表皮 (初生保护)
    Epidermis,
    /// 周皮 (次生保护，含木栓/木栓形成层/栓内层)
    Periderm,
}

/// 基本组织
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroundTissue {
    /// 薄壁组织 (光合/储藏/愈合)
    Parenchyma,
    /// 厚角组织 (机械支持，活细胞)
    Collenchyma,
    /// 厚壁组织 (机械支持，死细胞，含纤维/石细胞)
    Sclerenchyma,
}

/// 输导组织细分
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConductingTissue {
    /// 导管分子 vessel member (大口径输水)
    VesselMember,
    /// 管胞 tracheid (小口径，原始)
    Tracheid,
    /// 木纤维 xylem fiber
    XylemFiber,
    /// 木薄壁细胞
    XylemParenchyma,
    /// 筛管分子 sieve tube member
    SieveTubeMember,
    /// 伴胞 companion cell
    CompanionCell,
    /// 韧皮纤维
    PhloemFiber,
}

/// 分泌结构
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretoryStructure {
    /// 腺毛
    GlandularTrichome,
    /// 蜜腺
    Nectary,
    /// 树脂道
    ResinDuct,
    /// 乳汁管
    Laticifer,
    /// 分泌腔
    OilCavity,
}

/// 组织描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tissue {
    pub category: TissueCategory,
    pub cell_count: u32,
    /// 细胞平均直径 (μm)
    pub avg_cell_diameter_um: f32,
    /// 细胞壁厚度 (μm)
    pub cell_wall_thickness_um: f32,
    /// 是否木质化
    pub lignified: bool,
    /// 是否生活细胞
    pub living: bool,
}

impl Tissue {
    pub fn parenchyma_default() -> Self {
        Self {
            category: TissueCategory::Ground,
            cell_count: 1000,
            avg_cell_diameter_um: 50.0,
            cell_wall_thickness_um: 0.5,
            lignified: false,
            living: true,
        }
    }
    pub fn sclerenchyma_default() -> Self {
        Self {
            category: TissueCategory::Ground,
            cell_count: 500,
            avg_cell_diameter_um: 20.0,
            cell_wall_thickness_um: 5.0,
            lignified: true,
            living: false,
        }
    }
}

// ============================================================================
// 维管系统 Vascular System
// ============================================================================

/// 维管束排列方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BundleArrangement {
    /// 外韧型 (韧皮部外，木质部内) - 双子叶茎常见
    Collateral,
    /// 双韧型 (木质部内外均有韧皮部) - 葫芦科/茄科
    Bicollateral,
    /// 同心型 - 周木型/周韧型
    Concentric,
    /// 辐射型 (根的初生结构)
    Radial,
}

/// 木质部组分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Xylem {
    /// 导管分子列表 (孔径 μm)
    pub vessel_diameters_um: Vec<f32>,
    /// 管胞数
    pub tracheid_count: u32,
    /// 木纤维数
    pub fiber_count: u32,
    /// 导水率 K (kg·m·MPa^-1·s^-1)
    pub hydraulic_conductivity: f32,
    /// 木质部水势 (MPa)
    pub water_potential: f32,
}

impl Default for Xylem {
    fn default() -> Self {
        Self {
            vessel_diameters_um: vec![40.0, 60.0, 80.0],
            tracheid_count: 2000,
            fiber_count: 5000,
            hydraulic_conductivity: 1.5,
            water_potential: -1.0,
        }
    }
}

impl Xylem {
    /// 根据 Hagen-Poiseuille：导水率与导管半径 4 次方和成正比
    pub fn update_conductivity(&mut self) {
        let sum_r4: f32 = self
            .vessel_diameters_um
            .iter()
            .map(|&d| {
                let r = d * 0.5e-6;
                r * r * r * r
            })
            .sum();
        self.hydraulic_conductivity = sum_r4 * 1.0e6 * (self.vessel_diameters_um.len() as f32);
    }

    /// 空穴化风险 (embolism)：水势越负风险越高
    pub fn cavitation_risk(&self) -> f32 {
        // 经验 PLC 曲线：P < -1 MPa 开始空穴，P < -3 MPa 接近完全堵塞
        let p = self.water_potential;
        if p > -1.0 {
            0.0
        } else {
            (1.0 - ((p + 1.0) / 1.0).exp()).clamp(0.0, 1.0)
        }
    }
}

/// 韧皮部组分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phloem {
    /// 筛管分子数
    pub sieve_tube_count: u32,
    /// 伴胞数
    pub companion_cell_count: u32,
    /// 韧皮纤维数
    pub fiber_count: u32,
    /// 同化物卸载速率 (μmol/m^2/s)
    pub unloading_rate: f32,
    /// 集流速率 (cm/h)
    pub mass_flow_rate: f32,
}

impl Default for Phloem {
    fn default() -> Self {
        Self {
            sieve_tube_count: 800,
            companion_cell_count: 800,
            fiber_count: 1500,
            unloading_rate: 5.0,
            mass_flow_rate: 0.5,
        }
    }
}

/// 一个维管束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VascularBundle {
    pub arrangement: BundleArrangement,
    pub xylem: Xylem,
    pub phloem: Phloem,
    /// 是否具次生生长能力 (形成层)
    pub has_cambium: bool,
}

impl VascularBundle {
    pub fn collateral_default() -> Self {
        Self {
            arrangement: BundleArrangement::Collateral,
            xylem: Xylem::default(),
            phloem: Phloem::default(),
            has_cambium: true,
        }
    }
}
// ============================================================================
// 木材结构 Wood Anatomy
// ============================================================================

/// 年轮单个生长环
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthRing {
    /// 早材：腔大壁薄，导管多
    pub earlywood_width_mm: f32,
    /// 晚材：腔小壁厚，纤维多
    pub latewood_width_mm: f32,
    /// 形成年份 (年)
    pub year: u32,
    /// 平均密度 (g/cm^3)
    pub density_g_cm3: f32,
}

impl GrowthRing {
    pub fn total_width(&self) -> f32 {
        self.earlywood_width_mm + self.latewood_width_mm
    }
    /// 晚材比例 (0..1)
    pub fn latewood_fraction(&self) -> f32 {
        if self.total_width() > 0.0 {
            self.latewood_width_mm / self.total_width()
        } else {
            0.0
        }
    }
}

/// 木材切片状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wood {
    /// 年轮序列 (从内到外)
    pub rings: Vec<GrowthRing>,
    /// 心材半径 (cm，死细胞，色素沉积)
    pub heartwood_radius_cm: f32,
    /// 边材半径 (cm，活细胞，输水)
    pub sapwood_radius_cm: f32,
    /// 木射线密度 (条/mm²)
    pub ray_density: f32,
}

impl Default for Wood {
    fn default() -> Self {
        Self {
            rings: Vec::new(),
            heartwood_radius_cm: 0.0,
            sapwood_radius_cm: 1.0,
            ray_density: 5.0,
        }
    }
}

impl Wood {
    /// 添加新一年的年轮
    pub fn add_annual_ring(&mut self, year: u32, climate_index: f32) {
        // climate_index: 0 干旱 → 1 湿润
        let ew = 0.5 + climate_index * 1.5; // 0.5-2.0 mm
        let lw = 0.3 + (1.0 - climate_index) * 0.8; // 干旱年晚材更多
        let density = 0.4 + lw * 0.3;
        self.rings.push(GrowthRing {
            earlywood_width_mm: ew,
            latewood_width_mm: lw,
            year,
            density_g_cm3: density,
        });
        // 边材逐步转为心材
        if self.rings.len() > 10 {
            self.heartwood_radius_cm += 0.05;
            self.sapwood_radius_cm = (self.sapwood_radius_cm - 0.05).max(0.5);
        }
    }

    /// 树轮年代学：推断年龄与平均生长率
    pub fn dendrochronology_stats(&self) -> (u32, f32) {
        let n = self.rings.len() as u32;
        if n == 0 {
            return (0, 0.0);
        }
        let avg_width: f32 = self.rings.iter().map(|r| r.total_width()).sum::<f32>() / n as f32;
        (n, avg_width)
    }
}

// ============================================================================
// 树皮 Bark
// ============================================================================

/// 树皮层
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarkLayer {
    /// 内皮 (次生韧皮部，活细胞，输糖)
    InnerBark,
    /// 木栓形成层 (cork cambium，侧生分生)
    CorkCambium,
    /// 外皮 (木栓层，防水防腐，死细胞)
    OuterBark,
}

/// 树皮描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bark {
    /// 内皮厚度 (mm)
    pub inner_thickness_mm: f32,
    /// 木栓形成层活跃度 (0..1)
    pub cork_cambium_activity: f32,
    /// 外皮厚度 (mm)
    pub outer_thickness_mm: f32,
    /// 是否含树皮纤维强
    pub fibrous: bool,
    /// 树脂含量 (0..1)
    pub resin_content: f32,
}

impl Default for Bark {
    fn default() -> Self {
        Self {
            inner_thickness_mm: 2.0,
            cork_cambium_activity: 0.6,
            outer_thickness_mm: 5.0,
            fibrous: true,
            resin_content: 0.1,
        }
    }
}

impl Bark {
    pub fn total_thickness(&self) -> f32 {
        self.inner_thickness_mm + self.outer_thickness_mm
    }
    /// 防御评分 (0..1)，外皮厚 + 树脂 → 高
    pub fn defense_score(&self) -> f32 {
        let thickness_score = (self.outer_thickness_mm / 20.0).min(1.0);
        (thickness_score * 0.6 + self.resin_content * 0.4).clamp(0.0, 1.0)
    }
    /// 推进：每年形成层产生新外皮
    pub fn annual_growth(&mut self) {
        if self.cork_cambium_activity > 0.3 {
            self.outer_thickness_mm += self.cork_cambium_activity * 1.5;
            // 旧内皮逐步脱落转外皮
            self.inner_thickness_mm = (self.inner_thickness_mm * 0.9).max(1.0);
        }
    }
}

// ============================================================================
// 茎/根横截面汇总
// ============================================================================

/// 茎横截面 (双子叶次生结构)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StemCrossSection {
    pub radius_cm: f32,
    pub bark: Bark,
    pub wood: Wood,
    /// 中央髓 (parenchyma)
    pub pith_radius_cm: f32,
    /// 维管束数 (初生)
    pub primary_bundle_count: u32,
}

impl Default for StemCrossSection {
    fn default() -> Self {
        Self {
            radius_cm: 2.0,
            bark: Bark::default(),
            wood: Wood::default(),
            pith_radius_cm: 0.3,
            primary_bundle_count: 8,
        }
    }
}

impl StemCrossSection {
    /// 次生生长一年：增加一个年轮，树皮增长
    pub fn annual_secondary_growth(&mut self, year: u32, climate_index: f32) {
        self.wood.add_annual_ring(year, climate_index);
        self.bark.annual_growth();
        // 半径增长 = 当年年轮宽度
        if let Some(ring) = self.wood.rings.last() {
            self.radius_cm += ring.total_width() * 0.1; // mm → cm
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tissue_parenchyma_vs_sclerenchyma() {
        let p = Tissue::parenchyma_default();
        let s = Tissue::sclerenchyma_default();
        assert!(p.living);
        assert!(!s.living);
        assert!(p.cell_wall_thickness_um < s.cell_wall_thickness_um);
        assert!(s.lignified);
        assert!(!p.lignified);
    }

    #[test]
    fn test_vascular_bundle_collateral_default() {
        let b = VascularBundle::collateral_default();
        assert_eq!(b.arrangement, BundleArrangement::Collateral);
        assert!(b.has_cambium);
        assert!(b.xylem.hydraulic_conductivity > 0.0);
    }

    #[test]
    fn test_xylem_cavitation_risk() {
        let mut x = Xylem::default();
        x.water_potential = -0.5;
        assert_eq!(x.cavitation_risk(), 0.0);
        x.water_potential = -2.0;
        let mid = x.cavitation_risk();
        assert!(mid > 0.0 && mid < 1.0);
        x.water_potential = -5.0;
        assert!((x.cavitation_risk() - 1.0).abs() < 0.05);
    }

    #[test]
    fn test_xylem_conductivity_scales_with_diameter() {
        let mut x1 = Xylem::default();
        x1.vessel_diameters_um = vec![40.0];
        x1.update_conductivity();
        let k1 = x1.hydraulic_conductivity;
        let mut x2 = Xylem::default();
        x2.vessel_diameters_um = vec![80.0];
        x2.update_conductivity();
        let k2 = x2.hydraulic_conductivity;
        // 直径翻倍，半径^4 应增加 16 倍
        assert!(k2 > k1 * 10.0, "Larger vessels should have much higher K");
    }

    #[test]
    fn test_growth_ring_latewood_fraction() {
        let r = GrowthRing {
            earlywood_width_mm: 1.0,
            latewood_width_mm: 1.0,
            year: 2024,
            density_g_cm3: 0.5,
        };
        assert!((r.latewood_fraction() - 0.5).abs() < 1e-6);
        assert_eq!(r.total_width(), 2.0);
    }

    #[test]
    fn test_wood_dendrochronology() {
        let mut w = Wood::default();
        for y in 2020..=2024 {
            w.add_annual_ring(y, 0.5);
        }
        let (n, avg) = w.dendrochronology_stats();
        assert_eq!(n, 5);
        assert!(avg > 0.0);
        // 5 个年轮后心材应开始累积 (n > 10 时才会)
        assert_eq!(w.heartwood_radius_cm, 0.0);
    }

    #[test]
    fn test_wood_heartwood_formation_after_10_years() {
        let mut w = Wood::default();
        for y in 2000..=2020 {
            w.add_annual_ring(y, 0.6);
        }
        assert!(w.heartwood_radius_cm > 0.0, "Heartwood should form after 10 years");
    }

    #[test]
    fn test_bark_defense_score() {
        let b = Bark::default();
        let score = b.defense_score();
        assert!(score > 0.0 && score <= 1.0);
        let mut thick = b.clone();
        thick.outer_thickness_mm = 50.0;
        thick.resin_content = 1.0;
        assert!(thick.defense_score() > score);
    }

    #[test]
    fn test_stem_cross_section_growth() {
        let mut s = StemCrossSection::default();
        let r0 = s.radius_cm;
        for y in 2020..=2024 {
            s.annual_secondary_growth(y, 0.7);
        }
        assert!(s.radius_cm > r0, "Stem must grow");
        assert_eq!(s.wood.rings.len(), 5);
    }
}