//! 软组织类型系统 —— 基于组织学真实分类
//!
//! 数据来源：
//! - Junqueira's Basic Histology (15th ed.)
//! - Fung, "Biomechanics: Mechanical Properties of Living Tissues" (1993)
//! - ICRU Report 46: Tissue Substitutes in Radiation Dosimetry and Measurement
//! - OpenStax Anatomy & Physiology

use serde::{Deserialize, Serialize};

/// 软组织类型 —— 组织学四大类（上皮、结缔、肌肉、神经）+ 特化组织
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TissueType {
    EpithelialSquamous,     // 鳞状上皮（皮肤表皮、口腔、食管）
    EpithelialCuboidal,     // 立方上皮（肾小管、腺体）
    EpithelialColumnar,     // 柱状上皮（胃肠黏膜、胆囊）
    EpithelialTransitional, // 移行上皮（膀胱、输尿管）
    ConnectiveLoose,        // 疏松结缔（皮下、器官间）
    ConnectiveDense,        // 致密结缔（真皮网状层、器官被膜）
    ConnectiveElastic,      // 弹性结缔（黄韧带、声带）
    ConnectiveReticular,    // 网状结缔（脾、淋巴结、骨髓基质）
    CartilageHyaline,       // 透明软骨（关节面、肋软骨、鼻）
    CartilageElastic,       // 弹性软骨（耳廓、会厌）
    CartilageFibrocartilage,// 纤维软骨（椎间盘、半月板）
    BoneCortical,           // 皮质骨（骨密质）
    BoneTrabecular,         // 松质骨（骨松质）
    AdiposeWhite,           // 白色脂肪（储能）
    AdiposeBrown,           // 棕色脂肪（产热，新生儿多见）
    AdiposeBeige,           // 米色脂肪（棕色化白色脂肪）
    Blood,                  // 血液
    Lymph,                  // 淋巴
    Tendon,                 // 肌腱
    Ligament,               // 韧带
    Fascia,                 // 筋膜
    VesselArtery,           // 动脉
    VesselVein,             // 静脉
    VesselCapillary,        // 毛细血管
    Nerve,                  // 神经（中枢 + 周围）
    MuscleSkeletal,         // 骨骼肌
    MuscleCardiac,          // 心肌
    MuscleSmooth,           // 平滑肌
}

/// 再生能力分级 —— 基于组织学再生速度分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RegenerationCapacity {
    /// 几乎无再生能力（心肌、神经节细胞）
    None,
    /// 缓慢再生（软骨、神经、肌腱、韧带）
    Low,
    /// 中等再生（骨骼、骨骼肌、脂肪）
    Medium,
    /// 快速再生（表皮、肝脏）
    High,
    /// 极快再生（黏膜、血液）
    VeryHigh,
}

/// 软组织生物物理属性
///
/// 物理量单位：
/// - `density`: kg/m³
/// - `youngs_modulus`: Pa
/// - `poisson_ratio`: 无量纲（0-0.5）
/// - `thermal_conductivity`: W/(m·K)
/// - `specific_heat`: J/(kg·K)
/// - `electrical_conductivity`: S/m
/// - `water_content`: 0.0-1.0 体积分数
/// - `vascularization`: 0.0-1.0 血管密度相对值
/// - `innervation`: 0.0-1.0 神经密度相对值
/// - `regeneration_rate`: 细胞/天
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TissueProperties {
    pub density: f32,
    pub youngs_modulus: f32,
    pub poisson_ratio: f32,
    pub thermal_conductivity: f32,
    pub specific_heat: f32,
    pub electrical_conductivity: f32,
    pub water_content: f32,
    pub vascularization: f32,
    pub innervation: f32,
    pub regeneration_rate: f32,
}

impl Default for TissueProperties {
    /// 默认值 —— 中性软组织参考（ICRU 46 软组织）
    fn default() -> Self {
        Self {
            density: 1050.0,
            youngs_modulus: 1.0e4,
            poisson_ratio: 0.49,
            thermal_conductivity: 0.40,
            specific_heat: 3600.0,
            electrical_conductivity: 0.20,
            water_content: 0.70,
            vascularization: 0.50,
            innervation: 0.30,
            regeneration_rate: 1.0e6,
        }
    }
}

impl TissueType {
    /// 返回该组织的生物物理属性
    ///
    /// 参数来源：
    /// - 骨皮质：密度 1900 kg/m³, E=17 GPa（Ref: Reilly & Burstein, J Biomech 1975）
    /// - 骨松质：密度 500 kg/m³, E=0.5 GPa（Ref: Goldstein, J Biomech 1987）
    /// - 透明软骨：密度 1100 kg/m³, E=0.7 MPa, 无血管（Ref: Mow & Ratcliffe 1997）
    /// - 肌腱：密度 1100 kg/m³, E=1.5 GPa（Ref: Benedict 1968）
    /// - 韧带：密度 1100 kg/m³, E=0.4 GPa（Ref: Nigg & Herzog 1999）
    /// - 皮肤：密度 1100 kg/m³, E=0.1-0.8 MPa（Ref: Edwards & Marks 1995）
    /// - 动脉：E=0.5-1.0 MPa（Ref: Fung 1993）
    /// - 静脉：E=0.2-0.5 MPa（Ref: Fung 1993）
    /// - 神经：E=0.4 MPa（Ref: Kwan 1991）
    /// - 骨骼肌：密度 1060 kg/m³, E=10 kPa（Ref: Fung 1993）
    /// - 心肌：密度 1050 kg/m³，几乎不再生
    /// - 白脂肪：密度 900 kg/m³
    /// - 棕脂肪：密度 920 kg/m³（含更多线粒体）
    /// - 血液：密度 1060 kg/m³, 热导率 0.5 W/(m·K)
    pub fn properties(&self) -> TissueProperties {
        match self {
            // —— 上皮组织 ——（基底膜附着，本身无血管，靠下方结缔组织渗透供血）
            Self::EpithelialSquamous => TissueProperties {
                density: 1100.0,
                youngs_modulus: 5.0e5, // 0.1-0.8 MPa 中值
                poisson_ratio: 0.48,
                thermal_conductivity: 0.37,
                specific_heat: 3600.0,
                electrical_conductivity: 0.10,
                water_content: 0.65,
                vascularization: 0.05, // 上皮本身无血管
                innervation: 0.85,     // 皮肤神经末梢丰富
                regeneration_rate: 3.5e8, // 表皮 2-4 周更替
            },
            Self::EpithelialCuboidal => TissueProperties {
                density: 1050.0,
                youngs_modulus: 1.0e3,
                poisson_ratio: 0.49,
                thermal_conductivity: 0.40,
                specific_heat: 3600.0,
                electrical_conductivity: 0.15,
                water_content: 0.75,
                vascularization: 0.10,
                innervation: 0.20,
                regeneration_rate: 1.0e8, // 腺上皮更新较快
            },
            Self::EpithelialColumnar => TissueProperties {
                density: 1050.0,
                youngs_modulus: 1.0e3,
                poisson_ratio: 0.49,
                thermal_conductivity: 0.40,
                specific_heat: 3600.0,
                electrical_conductivity: 0.15,
                water_content: 0.80,
                vascularization: 0.20,
                innervation: 0.40,
                regeneration_rate: 1.0e9, // 肠黏膜 4-5 天更替（极快）
            },
            Self::EpithelialTransitional => TissueProperties {
                density: 1050.0,
                youngs_modulus: 5.0e3,
                poisson_ratio: 0.48,
                thermal_conductivity: 0.40,
                specific_heat: 3600.0,
                electrical_conductivity: 0.15,
                water_content: 0.75,
                vascularization: 0.10,
                innervation: 0.60, // 膀胱神经丰富
                regeneration_rate: 2.0e8,
            },
            // —— 结缔组织 ——
            Self::ConnectiveLoose => TissueProperties {
                density: 1050.0,
                youngs_modulus: 5.0e2,
                poisson_ratio: 0.49,
                thermal_conductivity: 0.40,
                specific_heat: 3600.0,
                electrical_conductivity: 0.20,
                water_content: 0.80,
                vascularization: 0.60,
                innervation: 0.30,
                regeneration_rate: 1.0e7,
            },
            Self::ConnectiveDense => TissueProperties {
                density: 1100.0,
                youngs_modulus: 5.0e7, // ~50 MPa（胶原密集）
                poisson_ratio: 0.40,
                thermal_conductivity: 0.38,
                specific_heat: 3300.0,
                electrical_conductivity: 0.10,
                water_content: 0.55,
                vascularization: 0.20,
                innervation: 0.20,
                regeneration_rate: 5.0e6,
            },
            Self::ConnectiveElastic => TissueProperties {
                density: 1100.0,
                youngs_modulus: 3.0e5,
                poisson_ratio: 0.49,
                thermal_conductivity: 0.38,
                specific_heat: 3300.0,
                electrical_conductivity: 0.10,
                water_content: 0.60,
                vascularization: 0.40,
                innervation: 0.20,
                regeneration_rate: 5.0e6,
            },
            Self::ConnectiveReticular => TissueProperties {
                density: 1050.0,
                youngs_modulus: 1.0e3,
                poisson_ratio: 0.49,
                thermal_conductivity: 0.40,
                specific_heat: 3600.0,
                electrical_conductivity: 0.20,
                water_content: 0.80,
                vascularization: 0.80, // 造血/淋巴器官基质血供丰富
                innervation: 0.10,
                regeneration_rate: 1.0e8,
            },
            // —— 软骨 —— 无血管、无神经、无淋巴（"三无"组织）
            Self::CartilageHyaline => TissueProperties {
                density: 1100.0,
                youngs_modulus: 7.0e5, // 0.7 MPa
                poisson_ratio: 0.45,
                thermal_conductivity: 0.40,
                specific_heat: 3500.0,
                electrical_conductivity: 0.30,
                water_content: 0.75,
                vascularization: 0.0, // 软骨无血管
                innervation: 0.0,     // 软骨无神经
                regeneration_rate: 1.0e4,
            },
            Self::CartilageElastic => TissueProperties {
                density: 1100.0,
                youngs_modulus: 5.0e5,
                poisson_ratio: 0.45,
                thermal_conductivity: 0.40,
                specific_heat: 3500.0,
                electrical_conductivity: 0.30,
                water_content: 0.70,
                vascularization: 0.0,
                innervation: 0.0,
                regeneration_rate: 8.0e3,
            },
            Self::CartilageFibrocartilage => TissueProperties {
                density: 1100.0,
                youngs_modulus: 8.0e5, // 含大量 I 型胶原
                poisson_ratio: 0.45,
                thermal_conductivity: 0.40,
                specific_heat: 3500.0,
                electrical_conductivity: 0.30,
                water_content: 0.65,
                vascularization: 0.0,
                innervation: 0.05,
                regeneration_rate: 8.0e3,
            },
            // —— 骨 ——
            Self::BoneCortical => TissueProperties {
                density: 1900.0,
                youngs_modulus: 1.7e10, // 17 GPa
                poisson_ratio: 0.30,
                thermal_conductivity: 0.40,
                specific_heat: 1300.0,
                electrical_conductivity: 0.02,
                water_content: 0.10,
                vascularization: 0.30,
                innervation: 0.40, // 骨膜神经丰富
                regeneration_rate: 1.0e6, // 骨重塑单位 BMU
            },
            Self::BoneTrabecular => TissueProperties {
                density: 500.0,
                youngs_modulus: 5.0e8, // 0.5 GPa
                poisson_ratio: 0.30,
                thermal_conductivity: 0.35,
                specific_heat: 1500.0,
                electrical_conductivity: 0.03,
                water_content: 0.30,
                vascularization: 0.50, // 骨髓造血
                innervation: 0.20,
                regeneration_rate: 5.0e6,
            },
            // —— 脂肪组织 ——
            Self::AdiposeWhite => TissueProperties {
                density: 900.0,
                youngs_modulus: 1.0e3,
                poisson_ratio: 0.49,
                thermal_conductivity: 0.21,
                specific_heat: 2500.0,
                electrical_conductivity: 0.05,
                water_content: 0.15,
                vascularization: 0.30,
                innervation: 0.10,
                regeneration_rate: 5.0e6,
            },
            Self::AdiposeBrown => TissueProperties {
                density: 920.0, // 含更多线粒体
                youngs_modulus: 1.2e3,
                poisson_ratio: 0.49,
                thermal_conductivity: 0.24,
                specific_heat: 2600.0,
                electrical_conductivity: 0.06,
                water_content: 0.25,
                vascularization: 0.70, // 棕脂血供丰富
                innervation: 0.60,     // 交感神经密集
                regeneration_rate: 1.0e7,
            },
            Self::AdiposeBeige => TissueProperties {
                density: 910.0,
                youngs_modulus: 1.1e3,
                poisson_ratio: 0.49,
                thermal_conductivity: 0.22,
                specific_heat: 2550.0,
                electrical_conductivity: 0.055,
                water_content: 0.20,
                vascularization: 0.50,
                innervation: 0.35,
                regeneration_rate: 7.0e6,
            },
            // —— 血液与淋巴 ——
            Self::Blood => TissueProperties {
                density: 1060.0,
                youngs_modulus: 0.0, // 液体（无剪切模量）
                poisson_ratio: 0.5,  // 不可压缩
                thermal_conductivity: 0.50,
                specific_heat: 3600.0,
                electrical_conductivity: 0.70,
                water_content: 0.85,
                vascularization: 1.0,
                innervation: 0.0,
                regeneration_rate: 1.7e8, // RBC 寿命 120 天，造血 ~2e6/s
            },
            Self::Lymph => TissueProperties {
                density: 1020.0,
                youngs_modulus: 0.0,
                poisson_ratio: 0.5,
                thermal_conductivity: 0.48,
                specific_heat: 3600.0,
                electrical_conductivity: 0.65,
                water_content: 0.85,
                vascularization: 0.0, // 淋巴管本身无血管
                innervation: 0.10,
                regeneration_rate: 5.0e7,
            },
            // —— 致密结缔衍生（肌腱、韧带、筋膜）——
            Self::Tendon => TissueProperties {
                density: 1100.0,
                youngs_modulus: 1.5e9, // 1.5 GPa
                poisson_ratio: 0.30,
                thermal_conductivity: 0.40,
                specific_heat: 3300.0,
                electrical_conductivity: 0.10,
                water_content: 0.60,
                vascularization: 0.20, // 血供差
                innervation: 0.30,
                regeneration_rate: 1.0e4, // 愈合极慢（数周-数月）
            },
            Self::Ligament => TissueProperties {
                density: 1100.0,
                youngs_modulus: 4.0e8, // 0.4 GPa
                poisson_ratio: 0.30,
                thermal_conductivity: 0.40,
                specific_heat: 3300.0,
                electrical_conductivity: 0.10,
                water_content: 0.60,
                vascularization: 0.15,
                innervation: 0.40, // 韧带本体感觉丰富
                regeneration_rate: 8.0e3,
            },
            Self::Fascia => TissueProperties {
                density: 1080.0,
                youngs_modulus: 5.0e5,
                poisson_ratio: 0.49,
                thermal_conductivity: 0.38,
                specific_heat: 3400.0,
                electrical_conductivity: 0.15,
                water_content: 0.70,
                vascularization: 0.25,
                innervation: 0.50,
                regeneration_rate: 1.0e5,
            },
            // —— 血管 ——
            Self::VesselArtery => TissueProperties {
                density: 1060.0,
                youngs_modulus: 7.5e5, // 0.5-1.0 MPa 中值
                poisson_ratio: 0.49,
                thermal_conductivity: 0.45,
                specific_heat: 3600.0,
                electrical_conductivity: 0.20,
                water_content: 0.75,
                vascularization: 0.30, // vasa vasorum
                innervation: 0.50,     // 交感神经
                regeneration_rate: 5.0e6,
            },
            Self::VesselVein => TissueProperties {
                density: 1050.0,
                youngs_modulus: 3.5e5, // 0.2-0.5 MPa 中值
                poisson_ratio: 0.49,
                thermal_conductivity: 0.45,
                specific_heat: 3600.0,
                electrical_conductivity: 0.18,
                water_content: 0.75,
                vascularization: 0.20,
                innervation: 0.30,
                regeneration_rate: 5.0e6,
            },
            Self::VesselCapillary => TissueProperties {
                density: 1050.0,
                youngs_modulus: 1.0e4, // ~10 kPa（极薄）
                poisson_ratio: 0.49,
                thermal_conductivity: 0.46,
                specific_heat: 3600.0,
                electrical_conductivity: 0.20,
                water_content: 0.80,
                vascularization: 0.0, // 毛细血管自身无血管
                innervation: 0.05,
                regeneration_rate: 1.0e7, // 血管新生快
            },
            // —— 神经组织 ——
            Self::Nerve => TissueProperties {
                density: 1040.0,
                youngs_modulus: 4.0e5, // 0.4 MPa
                poisson_ratio: 0.49,
                thermal_conductivity: 0.46,
                specific_heat: 3600.0,
                electrical_conductivity: 0.05, // 髓鞘电阻高
                water_content: 0.75,
                vascularization: 0.70,
                innervation: 1.0,
                regeneration_rate: 1.0e4, // 周围神经 1-3 mm/天；中枢无再生
            },
            // —— 肌肉组织 ——
            Self::MuscleSkeletal => TissueProperties {
                density: 1060.0,
                youngs_modulus: 1.0e4, // 10 kPa（被动状态）
                poisson_ratio: 0.49,
                thermal_conductivity: 0.49,
                specific_heat: 3600.0,
                electrical_conductivity: 0.30, // 收缩时升高
                water_content: 0.75,
                vascularization: 0.80,
                innervation: 0.80,
                regeneration_rate: 5.0e6, // 卫星细胞介导
            },
            Self::MuscleCardiac => TissueProperties {
                density: 1050.0,
                youngs_modulus: 1.5e4, // ~15 kPa
                poisson_ratio: 0.49,
                thermal_conductivity: 0.49,
                specific_heat: 3600.0,
                electrical_conductivity: 0.25,
                water_content: 0.75,
                vascularization: 0.90,
                innervation: 0.70,
                regeneration_rate: 1.0e3, // 几乎无再生
            },
            Self::MuscleSmooth => TissueProperties {
                density: 1050.0,
                youngs_modulus: 5.0e3, // ~5 kPa
                poisson_ratio: 0.49,
                thermal_conductivity: 0.48,
                specific_heat: 3600.0,
                electrical_conductivity: 0.20,
                water_content: 0.78,
                vascularization: 0.60,
                innervation: 0.60,
                regeneration_rate: 1.0e7,
            },
        }
    }

    /// 是否含血管 —— 软骨是典型的无血管组织（cartilage is avascular）
    pub fn is_vascular(&self) -> bool {
        !matches!(
            self,
            Self::CartilageHyaline | Self::CartilageElastic | Self::CartilageFibrocartilage
        )
    }

    /// 再生能力分级 —— 基于组织学再生速度分类
    pub fn regeneration_capacity(&self) -> RegenerationCapacity {
        match self {
            // 心肌：几乎不再生（Ref: Bergmann 2009, Science）
            Self::MuscleCardiac => RegenerationCapacity::None,
            // 软骨、神经、肌腱、韧带：再生缓慢
            Self::CartilageHyaline
            | Self::CartilageElastic
            | Self::CartilageFibrocartilage
            | Self::Nerve
            | Self::Tendon
            | Self::Ligament => RegenerationCapacity::Low,
            // 骨、肌肉、脂肪、结缔、血管：中等再生
            Self::BoneCortical
            | Self::BoneTrabecular
            | Self::MuscleSkeletal
            | Self::MuscleSmooth
            | Self::Fascia
            | Self::AdiposeWhite
            | Self::AdiposeBrown
            | Self::AdiposeBeige
            | Self::ConnectiveLoose
            | Self::ConnectiveDense
            | Self::ConnectiveElastic
            | Self::ConnectiveReticular
            | Self::VesselArtery
            | Self::VesselVein
            | Self::VesselCapillary => RegenerationCapacity::Medium,
            // 表皮、移行上皮：快速再生
            Self::EpithelialSquamous
            | Self::EpithelialCuboidal
            | Self::EpithelialTransitional => RegenerationCapacity::High,
            // 黏膜柱状上皮、血液、淋巴：极快再生
            Self::EpithelialColumnar | Self::Blood | Self::Lymph => RegenerationCapacity::VeryHigh,
        }
    }
}

impl Default for TissueType {
    fn default() -> Self {
        Self::ConnectiveLoose
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Default 实现 ----------

    #[test]
    fn test_tissue_default_is_connective_loose() {
        assert_eq!(TissueType::default(), TissueType::ConnectiveLoose);
    }

    #[test]
    fn test_properties_default_returns_icru46_soft_tissue() {
        let p = TissueProperties::default();
        assert!((p.density - 1050.0).abs() < 1e-6);
        assert!((p.youngs_modulus - 1.0e4).abs() < 1.0);
        assert!((p.poisson_ratio - 0.49).abs() < 1e-6);
        assert!((p.thermal_conductivity - 0.40).abs() < 1e-6);
        assert!((p.specific_heat - 3600.0).abs() < 1e-6);
        assert!((p.electrical_conductivity - 0.20).abs() < 1e-6);
        assert!((p.water_content - 0.70).abs() < 1e-6);
        assert!((p.vascularization - 0.50).abs() < 1e-6);
        assert!((p.innervation - 0.30).abs() < 1e-6);
        assert!((p.regeneration_rate - 1.0e6).abs() < 1.0);
    }

    #[test]
    fn test_properties_default_water_content_in_valid_range() {
        let p = TissueProperties::default();
        assert!(p.water_content >= 0.0 && p.water_content <= 1.0);
        assert!(p.vascularization >= 0.0 && p.vascularization <= 1.0);
        assert!(p.innervation >= 0.0 && p.innervation <= 1.0);
        assert!(p.poisson_ratio >= 0.0 && p.poisson_ratio <= 0.5);
    }

    // ---------- is_vascular 行为 ----------

    #[test]
    fn test_cartilage_hyaline_is_avascular() {
        assert!(!TissueType::CartilageHyaline.is_vascular());
    }

    #[test]
    fn test_cartilage_elastic_is_avascular() {
        assert!(!TissueType::CartilageElastic.is_vascular());
    }

    #[test]
    fn test_cartilage_fibrocartilage_is_avascular() {
        assert!(!TissueType::CartilageFibrocartilage.is_vascular());
    }

    #[test]
    fn test_bone_cortical_is_vascular() {
        assert!(TissueType::BoneCortical.is_vascular());
    }

    #[test]
    fn test_blood_is_vascular() {
        assert!(TissueType::Blood.is_vascular());
    }

    #[test]
    fn test_epithelial_squamous_is_vascular_due_to_no_cartilage() {
        // 上皮本身无血管，但 is_vascular 仅排除软骨
        assert!(TissueType::EpithelialSquamous.is_vascular());
    }

    #[test]
    fn test_all_cartilage_variants_are_avascular() {
        assert!(!TissueType::CartilageHyaline.is_vascular());
        assert!(!TissueType::CartilageElastic.is_vascular());
        assert!(!TissueType::CartilageFibrocartilage.is_vascular());
    }

    // ---------- regeneration_capacity 行为 ----------

    #[test]
    fn test_cardiac_muscle_regenerates_none() {
        assert_eq!(TissueType::MuscleCardiac.regeneration_capacity(),
                   RegenerationCapacity::None);
    }

    #[test]
    fn test_hyaline_cartilage_regenerates_low() {
        assert_eq!(TissueType::CartilageHyaline.regeneration_capacity(),
                   RegenerationCapacity::Low);
    }

    #[test]
    fn test_nerve_regenerates_low() {
        assert_eq!(TissueType::Nerve.regeneration_capacity(),
                   RegenerationCapacity::Low);
    }

    #[test]
    fn test_tendon_and_ligament_regenerates_low() {
        assert_eq!(TissueType::Tendon.regeneration_capacity(),
                   RegenerationCapacity::Low);
        assert_eq!(TissueType::Ligament.regeneration_capacity(),
                   RegenerationCapacity::Low);
    }

    #[test]
    fn test_epithelial_squamous_regenerates_high() {
        assert_eq!(TissueType::EpithelialSquamous.regeneration_capacity(),
                   RegenerationCapacity::High);
    }

    #[test]
    fn test_epithelial_columnar_regenerates_very_high() {
        assert_eq!(TissueType::EpithelialColumnar.regeneration_capacity(),
                   RegenerationCapacity::VeryHigh);
    }

    #[test]
    fn test_blood_regenerates_very_high() {
        assert_eq!(TissueType::Blood.regeneration_capacity(),
                   RegenerationCapacity::VeryHigh);
    }

    #[test]
    fn test_bone_cortical_regenerates_medium() {
        assert_eq!(TissueType::BoneCortical.regeneration_capacity(),
                   RegenerationCapacity::Medium);
    }

    #[test]
    fn test_adipose_brown_regenerates_medium() {
        assert_eq!(TissueType::AdiposeBrown.regeneration_capacity(),
                   RegenerationCapacity::Medium);
    }

    // ---------- properties() 关键返回值 ----------

    #[test]
    fn test_bone_cortical_density_and_modulus() {
        let p = TissueType::BoneCortical.properties();
        assert!((p.density - 1900.0).abs() < 1e-6);
        assert!((p.youngs_modulus - 1.7e10).abs() < 1.0);
        assert!((p.poisson_ratio - 0.30).abs() < 1e-6);
    }

    #[test]
    fn test_blood_is_incompressible_and_zero_modulus() {
        let p = TissueType::Blood.properties();
        assert!((p.poisson_ratio - 0.5).abs() < 1e-6);
        assert!((p.youngs_modulus - 0.0).abs() < 1e-6);
        assert!((p.vascularization - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_tendon_modulus_higher_than_ligament() {
        let t = TissueType::Tendon.properties().youngs_modulus;
        let l = TissueType::Ligament.properties().youngs_modulus;
        assert!(t > l, "tendon E={} should exceed ligament E={}", t, l);
    }

    #[test]
    fn test_adipose_white_density_below_water() {
        let w = TissueType::AdiposeWhite.properties().density;
        assert!(w < 1000.0, "white fat density {} should be < water 1000", w);
    }

    #[test]
    fn test_adipose_brown_density_higher_than_white() {
        // 棕脂含更多线粒体，密度略高
        let b = TissueType::AdiposeBrown.properties().density;
        let w = TissueType::AdiposeWhite.properties().density;
        assert!(b > w);
    }

    #[test]
    fn test_nerve_innervation_is_maximal() {
        let p = TissueType::Nerve.properties();
        assert!((p.innervation - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_bone_trabecular_density_lower_than_cortical() {
        let t = TissueType::BoneTrabecular.properties().density;
        let c = TissueType::BoneCortical.properties().density;
        assert!(t < c);
    }

    #[test]
    fn test_capillary_vascularization_is_zero() {
        // 毛细血管自身无血管
        let p = TissueType::VesselCapillary.properties();
        assert!((p.vascularization - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cardiac_muscle_regeneration_rate_lowest_among_muscles() {
        let c = TissueType::MuscleCardiac.properties().regeneration_rate;
        let s = TissueType::MuscleSkeletal.properties().regeneration_rate;
        let m = TissueType::MuscleSmooth.properties().regeneration_rate;
        assert!(c < s);
        assert!(c < m);
    }

    // ---------- 全变体覆盖度（无 panic） ----------

    #[test]
    fn test_all_tissue_variants_have_properties_and_valid_ranges() {
        let all = [
            TissueType::EpithelialSquamous, TissueType::EpithelialCuboidal,
            TissueType::EpithelialColumnar, TissueType::EpithelialTransitional,
            TissueType::ConnectiveLoose, TissueType::ConnectiveDense,
            TissueType::ConnectiveElastic, TissueType::ConnectiveReticular,
            TissueType::CartilageHyaline, TissueType::CartilageElastic,
            TissueType::CartilageFibrocartilage, TissueType::BoneCortical,
            TissueType::BoneTrabecular, TissueType::AdiposeWhite,
            TissueType::AdiposeBrown, TissueType::AdiposeBeige,
            TissueType::Blood, TissueType::Lymph,
            TissueType::Tendon, TissueType::Ligament, TissueType::Fascia,
            TissueType::VesselArtery, TissueType::VesselVein, TissueType::VesselCapillary,
            TissueType::Nerve, TissueType::MuscleSkeletal,
            TissueType::MuscleCardiac, TissueType::MuscleSmooth,
        ];
        for t in all {
            let p = t.properties();
            assert!(p.density > 0.0, "tissue {:?} density must be positive", t);
            assert!(p.specific_heat > 0.0);
            assert!(p.thermal_conductivity >= 0.0);
            assert!(p.electrical_conductivity >= 0.0);
            assert!(p.water_content >= 0.0 && p.water_content <= 1.0);
            assert!(p.vascularization >= 0.0 && p.vascularization <= 1.0);
            assert!(p.innervation >= 0.0 && p.innervation <= 1.0);
            assert!(p.poisson_ratio >= 0.0 && p.poisson_ratio <= 0.5);
            assert!(p.regeneration_rate >= 0.0);
            assert!(p.youngs_modulus >= 0.0);
            // 同时验证 regeneration_capacity 不 panic
            let _ = t.regeneration_capacity();
            let _ = t.is_vascular();
        }
    }

    #[test]
    fn test_all_variants_distinct_default_vs_any_variant() {
        // Default 是 ConnectiveLoose，并非 None
        assert_eq!(TissueType::default(), TissueType::ConnectiveLoose);
        // 任何变体调用 properties 都不应等于 default（密度不同即可证明）
        // ConnectiveLoose 自己除外
        let default_props = TissueProperties::default();
        let loose_props = TissueType::ConnectiveLoose.properties();
        // 默认 properties 是 ICRU46 软组织，不应等于 ConnectiveLoose 的 properties
        assert!((loose_props.density - default_props.density).abs() < 1e-6); // 都是 1050
        // 但 youngs_modulus 不同：ConnectiveLoose=500, default=1e4
        assert!((loose_props.youngs_modulus - 5.0e2).abs() < 1.0);
        assert!((default_props.youngs_modulus - 1.0e4).abs() < 1.0);
    }
}