//! 嗜极生物与极端环境模块
//!
//! 基于真实嗜极生物学研究实现，涵盖嗜热/嗜冷/嗜酸/嗜碱/嗜盐/嗜压/
//! 耐辐射/嗜旱等极端生命形式，深海热泉生态系统，太空生存能力，
//! 以及已知生命边界参数。
//!
//! 主要参考：
//! - Van Dover 2000, "The Ecology of Deep-Sea Hydrothermal Vents"
//! - Jönsson 2008 (太空生存)
//! - Horneck 2008 (太空微生物学)
//! - Madigan & Bender 2018, "Brock Biology of Microorganisms"
//! - Pikuta et al. 2007, "Microbial Extremophiles from the Last Frontiers"

use serde::{Deserialize, Serialize};

// ============ 1. 嗜极生物分类 ============

/// 嗜极生物类型分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtremophileType {
    /// 嗜热（>45°C）
    Thermophile,
    /// 超嗜热（>80°C，最适>65°C）
    Hyperthermophile,
    /// 嗜冷（<15°C）
    Psychrophile,
    /// 嗜酸（pH<3）
    Acidophile,
    /// 嗜碱（pH>9）
    Alkaliphile,
    /// 嗜盐（>2M NaCl）
    Halophile,
    /// 嗜压（>40 MPa）
    Barophile,
    /// 寡营养（极低营养）
    Oligotroph,
    /// 耐毒
    Toxitolerant,
    /// 耐辐射
    Radiotolerant,
    /// 嗜旱（低水活度）
    Xerophile,
    /// 耐金属
    Metalotolerant,
    /// 嗜冷（同 Psychrophile）
    Cryophile,
    /// 岩石内（endolithic）
    Endolith,
}

/// 极端环境代谢类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtremophileMetabolism {
    /// 化能自养（热泉硫化氢氧化）
    Chemoautotroph,
    /// 光能自养
    Photoautotroph,
    /// 异养
    Heterotroph,
    /// 产甲烷（古菌）
    Methanogen,
    /// 硫酸还原
    SulfateReducer,
    /// 铁氧化
    IronOxidizer,
    /// 氢氧化
    HydrogenOxidizer,
}

/// 单个嗜极生物描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extremophile {
    pub organism_type: ExtremophileType,
    pub name: String,
    pub optimal_temp_c: f32,
    pub min_temp_c: f32,
    pub max_temp_c: f32,
    pub optimal_ph: f32,
    pub min_ph: f32,
    pub max_ph: f32,
    pub optimal_pressure_mpa: f32,
    pub max_pressure_mpa: f32,
    /// NaCl 百分比
    pub optimal_salinity_pct: f32,
    pub min_water_activity: f32,
    pub radiation_resistance_gy: f32,
    pub metabolism_type: ExtremophileMetabolism,
    pub discovered_at: String,
}

// ============ 2. 真实嗜极生物数据库 ============

/// 嗜热生物数据库
/// 来源：Brock 1978, Stetter 2006, Madigan 2018
pub fn thermophile_database() -> Vec<Extremophile> {
    vec![
        // Thermus aquaticus — 黄石公园 70°C，Taq DNA 聚合酶来源（PCR 基础）
        // Brock & Freeze 1969
        Extremophile {
            organism_type: ExtremophileType::Thermophile,
            name: "Thermus aquaticus".into(),
            optimal_temp_c: 70.0,
            min_temp_c: 40.0,
            max_temp_c: 79.0,
            optimal_ph: 8.0,
            min_ph: 6.0,
            max_ph: 10.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 200.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Yellowstone, 1969".into(),
        },
        // Pyrococcus furiosus — 猛烈火球菌，100°C 最适
        // Fiala & Stetter 1986
        Extremophile {
            organism_type: ExtremophileType::Hyperthermophile,
            name: "Pyrococcus furiosus".into(),
            optimal_temp_c: 100.0,
            min_temp_c: 70.0,
            max_temp_c: 103.0,
            optimal_ph: 7.0,
            min_ph: 5.0,
            max_ph: 9.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 30.0,
            optimal_salinity_pct: 2.0,
            min_water_activity: 0.95,
            radiation_resistance_gy: 1500.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Vulcano Island, Italy, 1986".into(),
        },
        // Pyrolobus fumarii — 烟孔火叶菌，113°C 最高温度记录
        // Blöchl et al. 1997
        Extremophile {
            organism_type: ExtremophileType::Hyperthermophile,
            name: "Pyrolobus fumarii".into(),
            optimal_temp_c: 106.0,
            min_temp_c: 90.0,
            max_temp_c: 113.0,
            optimal_ph: 5.5,
            min_ph: 4.0,
            max_ph: 7.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 30.0,
            optimal_salinity_pct: 1.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 1500.0,
            metabolism_type: ExtremophileMetabolism::Chemoautotroph,
            discovered_at: "Mid-Atlantic Ridge, 1997".into(),
        },
        // Strain 121 — 121°C 高压灭菌器中存活
        // Kashefi & Lovley 2003
        Extremophile {
            organism_type: ExtremophileType::Hyperthermophile,
            name: "Strain 121 (Geogemma)".into(),
            optimal_temp_c: 106.0,
            min_temp_c: 85.0,
            max_temp_c: 121.0,
            optimal_ph: 6.5,
            min_ph: 5.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 0.2,
            max_pressure_mpa: 50.0,
            optimal_salinity_pct: 1.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 1500.0,
            metabolism_type: ExtremophileMetabolism::IronOxidizer,
            discovered_at: "Juan de Fuca Ridge, 2003".into(),
        },
        // Methanopyrus kandleri Strain 116 — 122°C 最高生存温度记录
        // Takai et al. 2008
        Extremophile {
            organism_type: ExtremophileType::Hyperthermophile,
            name: "Methanopyrus kandleri Strain 116".into(),
            optimal_temp_c: 98.0,
            min_temp_c: 84.0,
            max_temp_c: 122.0,
            optimal_ph: 6.5,
            min_ph: 5.5,
            max_ph: 8.0,
            optimal_pressure_mpa: 20.0,
            max_pressure_mpa: 60.0,
            optimal_salinity_pct: 1.0,
            min_water_activity: 0.95,
            radiation_resistance_gy: 1500.0,
            metabolism_type: ExtremophileMetabolism::Methanogen,
            discovered_at: "Central Indian Ridge, 2008".into(),
        },
        // Geogemma barossii — 同 Strain 121，122°C 复制
        Extremophile {
            organism_type: ExtremophileType::Hyperthermophile,
            name: "Geogemma barossii".into(),
            optimal_temp_c: 106.0,
            min_temp_c: 85.0,
            max_temp_c: 122.0,
            optimal_ph: 6.5,
            min_ph: 5.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 0.2,
            max_pressure_mpa: 50.0,
            optimal_salinity_pct: 1.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 1500.0,
            metabolism_type: ExtremophileMetabolism::IronOxidizer,
            discovered_at: "Pacific Ridge, 2003".into(),
        },
    ]
}

/// 嗜冷生物数据库
/// 来源：Morita 1975, Cavicchioli 2006
pub fn psychrophile_database() -> Vec<Extremophile> {
    vec![
        // Psychrobacter arcticus — 北极嗜冷杆菌，-10°C 仍可生长
        Extremophile {
            organism_type: ExtremophileType::Psychrophile,
            name: "Psychrobacter arcticus".into(),
            optimal_temp_c: 4.0,
            min_temp_c: -10.0,
            max_temp_c: 22.0,
            optimal_ph: 7.0,
            min_ph: 6.0,
            max_ph: 9.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 3.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Siberian permafrost, 2005".into(),
        },
        // Polaromonas vacuolata — 极区单胞菌，4°C 最适
        Extremophile {
            organism_type: ExtremophileType::Psychrophile,
            name: "Polaromonas vacuolata".into(),
            optimal_temp_c: 4.0,
            min_temp_c: 0.0,
            max_temp_c: 12.0,
            optimal_ph: 7.0,
            min_ph: 6.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 3.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Antarctic sea ice, 1996".into(),
        },
        // Flavobacterium frigidimaris — 冷栖黄杆菌
        Extremophile {
            organism_type: ExtremophileType::Psychrophile,
            name: "Flavobacterium frigidimaris".into(),
            optimal_temp_c: 5.0,
            min_temp_c: -5.0,
            max_temp_c: 18.0,
            optimal_ph: 7.0,
            min_ph: 6.0,
            max_ph: 9.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Antarctica, 2001".into(),
        },
        // Mesenchytraeus solifugus — 冰虫，0°C 最适，温度升高即死亡
        // Hartzell 1998
        Extremophile {
            organism_type: ExtremophileType::Cryophile,
            name: "Mesenchytraeus solifugus (ice worm)".into(),
            optimal_temp_c: 0.0,
            min_temp_c: -20.0,
            max_temp_c: 10.0,
            optimal_ph: 7.0,
            min_ph: 6.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.0,
            min_water_activity: 0.99,
            radiation_resistance_gy: 30.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "North American glaciers, 1998".into(),
        },
    ]
}

/// 嗜酸生物数据库
/// 来源：Schleper et al. 1995, Edwards 1999
pub fn acidophile_database() -> Vec<Extremophile> {
    vec![
        // Picrophilus torridus — pH 0.7 最适，最低 pH 记录
        // Schleper et al. 1995
        Extremophile {
            organism_type: ExtremophileType::Acidophile,
            name: "Picrophilus torridus".into(),
            optimal_temp_c: 60.0,
            min_temp_c: 45.0,
            max_temp_c: 65.0,
            optimal_ph: 0.7,
            min_ph: 0.0,
            max_ph: 4.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.1,
            min_water_activity: 0.95,
            radiation_resistance_gy: 100.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Japanese solfatara, 1995".into(),
        },
        // Ferroplasma acidarmanus — pH 0，铁氧化古菌，酸性矿排水
        // Edwards et al. 2000
        Extremophile {
            organism_type: ExtremophileType::Acidophile,
            name: "Ferroplasma acidarmanus".into(),
            optimal_temp_c: 42.0,
            min_temp_c: 30.0,
            max_temp_c: 50.0,
            optimal_ph: 1.0,
            min_ph: 0.0,
            max_ph: 3.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 100.0,
            metabolism_type: ExtremophileMetabolism::IronOxidizer,
            discovered_at: "Iron Mountain, California, 2000".into(),
        },
        // Acetobacter aceti — 醋酸菌，pH 4
        Extremophile {
            organism_type: ExtremophileType::Acidophile,
            name: "Acetobacter aceti".into(),
            optimal_temp_c: 30.0,
            min_temp_c: 10.0,
            max_temp_c: 40.0,
            optimal_ph: 4.0,
            min_ph: 3.0,
            max_ph: 6.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Beer/wine fermentation, classical".into(),
        },
        // Dunaliella acidophila — 绿藻，pH 0-1
        Extremophile {
            organism_type: ExtremophileType::Acidophile,
            name: "Dunaliella acidophila".into(),
            optimal_temp_c: 25.0,
            min_temp_c: 5.0,
            max_temp_c: 35.0,
            optimal_ph: 1.0,
            min_ph: 0.0,
            max_ph: 3.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Photoautotroph,
            discovered_at: "Acidic volcanic lake, 1970s".into(),
        },
    ]
}

/// 嗜碱生物数据库
/// 来源：Horikoshi 1999
pub fn alkaliphile_database() -> Vec<Extremophile> {
    vec![
        // Natronobacterium gregoryi — 苏打湖，pH 10-12
        Extremophile {
            organism_type: ExtremophileType::Alkaliphile,
            name: "Natronobacterium gregoryi".into(),
            optimal_temp_c: 40.0,
            min_temp_c: 20.0,
            max_temp_c: 50.0,
            optimal_ph: 11.0,
            min_ph: 9.0,
            max_ph: 12.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 20.0,
            min_water_activity: 0.90,
            radiation_resistance_gy: 100.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Lake Magadi, Kenya, 1980s".into(),
        },
        // Bacillus alcalophilus — pH 10
        Extremophile {
            organism_type: ExtremophileType::Alkaliphile,
            name: "Bacillus alcalophilus".into(),
            optimal_temp_c: 30.0,
            min_temp_c: 10.0,
            max_temp_c: 40.0,
            optimal_ph: 10.0,
            min_ph: 8.0,
            max_ph: 11.5,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 1.0,
            min_water_activity: 0.95,
            radiation_resistance_gy: 100.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Alkaline soil, 1934 (Vedder)".into(),
        },
        // Spirulina — 螺旋藻，pH 11，碱湖
        Extremophile {
            organism_type: ExtremophileType::Alkaliphile,
            name: "Spirulina (Arthrospira)".into(),
            optimal_temp_c: 35.0,
            min_temp_c: 20.0,
            max_temp_c: 45.0,
            optimal_ph: 11.0,
            min_ph: 8.0,
            max_ph: 12.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 5.0,
            min_water_activity: 0.95,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Photoautotroph,
            discovered_at: "Lake Chad, Lake Texcoco, classical".into(),
        },
    ]
}

/// 嗜盐生物数据库
/// 来源：Oren 2002, "Halophilic Microorganisms and their Environments"
pub fn halophile_database() -> Vec<Extremophile> {
    vec![
        // Halobacterium salinarum — 4M NaCl，紫膜质光驱动质子泵
        // Oesterhelt & Stoeckenius 1971
        Extremophile {
            organism_type: ExtremophileType::Halophile,
            name: "Halobacterium salinarum".into(),
            optimal_temp_c: 40.0,
            min_temp_c: 25.0,
            max_temp_c: 50.0,
            optimal_ph: 7.0,
            min_ph: 5.5,
            max_ph: 8.5,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 25.0,
            min_water_activity: 0.75,
            radiation_resistance_gy: 5000.0,
            metabolism_type: ExtremophileMetabolism::Photoautotroph,
            discovered_at: "Salted fish, 1920s".into(),
        },
        // Dunaliella salina — 盐藻，产 β-胡萝卜素
        Extremophile {
            organism_type: ExtremophileType::Halophile,
            name: "Dunaliella salina".into(),
            optimal_temp_c: 30.0,
            min_temp_c: 10.0,
            max_temp_c: 40.0,
            optimal_ph: 8.0,
            min_ph: 6.0,
            max_ph: 10.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 25.0,
            min_water_activity: 0.75,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Photoautotroph,
            discovered_at: "Salt lakes worldwide, classical".into(),
        },
        // Haloquadratum walsbyi — 方形嗜盐菌
        // Burns et al. 2007
        Extremophile {
            organism_type: ExtremophileType::Halophile,
            name: "Haloquadratum walsbyi".into(),
            optimal_temp_c: 40.0,
            min_temp_c: 25.0,
            max_temp_c: 50.0,
            optimal_ph: 7.5,
            min_ph: 6.0,
            max_ph: 9.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 30.0,
            min_water_activity: 0.70,
            radiation_resistance_gy: 100.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Salt lakes, 1980 (Walsby)".into(),
        },
    ]
}

/// 嗜压生物数据库
/// 来源：Yayanos 1995, Bartlett 2002
pub fn barophile_database() -> Vec<Extremophile> {
    vec![
        // Shewanella benthica — 深海，100 MPa
        Extremophile {
            organism_type: ExtremophileType::Barophile,
            name: "Shewanella benthica".into(),
            optimal_temp_c: 4.0,
            min_temp_c: -2.0,
            max_temp_c: 15.0,
            optimal_ph: 7.0,
            min_ph: 6.0,
            max_ph: 8.5,
            optimal_pressure_mpa: 100.0,
            max_pressure_mpa: 120.0,
            optimal_salinity_pct: 3.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Mariana Trench, 1980s".into(),
        },
        // Colwellia hadaliensis — 100 MPa，海底 10km
        Extremophile {
            organism_type: ExtremophileType::Barophile,
            name: "Colwellia hadaliensis".into(),
            optimal_temp_c: 2.0,
            min_temp_c: -2.0,
            max_temp_c: 10.0,
            optimal_ph: 7.0,
            min_ph: 6.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 100.0,
            max_pressure_mpa: 120.0,
            optimal_salinity_pct: 3.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Puerto Rico Trench, 8000m, 1987".into(),
        },
        // Pyrococcus yayanosii — 52 MPa 最适，深海热泉
        // Birrien et al. 2011
        Extremophile {
            organism_type: ExtremophileType::Barophile,
            name: "Pyrococcus yayanosii".into(),
            optimal_temp_c: 95.0,
            min_temp_c: 70.0,
            max_temp_c: 105.0,
            optimal_ph: 6.5,
            min_ph: 5.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 52.0,
            max_pressure_mpa: 80.0,
            optimal_salinity_pct: 2.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 1500.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Ashadze hydrothermal field, 4100m, 2011".into(),
        },
    ]
}

/// 耐辐射生物数据库
/// 来源：Cox & Battista 2005, Slade & Radman 2011
pub fn radiotolerant_database() -> Vec<Extremophile> {
    vec![
        // Deinococcus radiodurans — 5000 Gy，Guinness 最韧细菌
        // 机制：Manganese antioxidant complex + efficient DNA repair
        // Anderson et al. 1956, Minton 1994
        Extremophile {
            organism_type: ExtremophileType::Radiotolerant,
            name: "Deinococcus radiodurans".into(),
            optimal_temp_c: 30.0,
            min_temp_c: 4.0,
            max_temp_c: 45.0,
            optimal_ph: 7.0,
            min_ph: 5.0,
            max_ph: 9.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 10.0,
            optimal_salinity_pct: 0.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 5000.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Canned meat, 1956".into(),
        },
        // Thermococcus gammatolerans — 古菌，30000 Gy
        // Jolivet et al. 2003
        Extremophile {
            organism_type: ExtremophileType::Radiotolerant,
            name: "Thermococcus gammatolerans".into(),
            optimal_temp_c: 88.0,
            min_temp_c: 60.0,
            max_temp_c: 95.0,
            optimal_ph: 6.5,
            min_ph: 5.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 30.0,
            optimal_salinity_pct: 2.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 30000.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Guaymas Basin hydrothermal vent, 2003".into(),
        },
        // Cryptococcus neoformans — 真菌，耐辐射，可在切尔诺贝利反应堆辐射下生长
        Extremophile {
            organism_type: ExtremophileType::Radiotolerant,
            name: "Cryptococcus neoformans".into(),
            optimal_temp_c: 30.0,
            min_temp_c: 4.0,
            max_temp_c: 40.0,
            optimal_ph: 7.0,
            min_ph: 5.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.5,
            min_water_activity: 0.95,
            radiation_resistance_gy: 1000.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Chernobyl reactor, 1990s".into(),
        },
    ]
}

/// 嗜旱生物数据库
/// 来源：Grant 2004, "Life at low water activity"
pub fn xerophile_database() -> Vec<Extremophile> {
    vec![
        // Aspergillus penicillioides — xerophilic mold, aw 0.65
        Extremophile {
            organism_type: ExtremophileType::Xerophile,
            name: "Aspergillus penicillioides".into(),
            optimal_temp_c: 25.0,
            min_temp_c: 10.0,
            max_temp_c: 35.0,
            optimal_ph: 6.0,
            min_ph: 4.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.5,
            min_water_activity: 0.65,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Dried food, indoor dust, classical".into(),
        },
        // Xeromyces bisporus — aw 0.61，最低水活度记录
        // Pitt & Christian 1968
        Extremophile {
            organism_type: ExtremophileType::Xerophile,
            name: "Xeromyces bisporus".into(),
            optimal_temp_c: 25.0,
            min_temp_c: 10.0,
            max_temp_c: 35.0,
            optimal_ph: 5.5,
            min_ph: 4.0,
            max_ph: 8.0,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 0.5,
            min_water_activity: 0.61,
            radiation_resistance_gy: 50.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Dried prunes, 1968".into(),
        },
        // Haloarcula — 高盐低水活度
        Extremophile {
            organism_type: ExtremophileType::Xerophile,
            name: "Haloarcula spp.".into(),
            optimal_temp_c: 40.0,
            min_temp_c: 25.0,
            max_temp_c: 50.0,
            optimal_ph: 7.0,
            min_ph: 6.0,
            max_ph: 8.5,
            optimal_pressure_mpa: 0.1,
            max_pressure_mpa: 1.0,
            optimal_salinity_pct: 25.0,
            min_water_activity: 0.75,
            radiation_resistance_gy: 200.0,
            metabolism_type: ExtremophileMetabolism::Heterotroph,
            discovered_at: "Salt flats, 1980s".into(),
        },
    ]
}

// ============ 3. 深海热泉生态系统（Hydrothermal Vent）============
// 来源：Van Dover 2000, "The Ecology of Deep-Sea Hydrothermal Vents"

/// 深海热泉类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HydrothermalVentType {
    /// 黑烟囱（350-400°C，硫化物）
    BlackSmoker,
    /// 白烟囱（100-300°C，钡钙硅）
    WhiteSmoker,
    /// 扩散流（<30°C）
    DiffuseFlow,
    /// 失落之城（碱性，90°C，蛇纹石化）
    LostCity,
}

/// 深海热泉环境参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydrothermalVent {
    pub vent_type: HydrothermalVentType,
    pub temperature_c: f32,
    /// 深度，典型 1500-4000 m
    pub depth_m: f32,
    pub pressure_mpa: f32,
    pub ph: f32,
    /// 硫化氢浓度 (mM)
    pub h2s_conc_mm: f32,
    /// 甲烷浓度 (mM)
    pub ch4_conc_mm: f32,
    /// 铁浓度 (mM)
    pub fe_conc_mm: f32,
    /// CO2 浓度 (mM)
    pub co2_conc_mm: f32,
    /// 生物量密度 (kg/m²)，热泉可达 10-50，远超深海背景 0.01
    pub biomass_density_kg_m2: f32,
}

impl HydrothermalVent {
    pub fn new(vtype: HydrothermalVentType, depth_m: f32) -> Self {
        // 静水压力：每 10m 增加 1 MPa
        let pressure_mpa = depth_m / 10.0;
        let (temperature_c, ph, h2s, ch4, fe, co2, biomass) = match vtype {
            HydrothermalVentType::BlackSmoker => {
                // 350-400°C 黑烟囱，富含金属硫化物
                (370.0_f32, 3.5, 5.0, 1.5, 8.0, 25.0, 30.0)
            }
            HydrothermalVentType::WhiteSmoker => {
                // 100-300°C 白烟囱，钡钙硅
                (250.0, 6.0, 1.5, 0.8, 1.0, 12.0, 20.0)
            }
            HydrothermalVentType::DiffuseFlow => {
                // <30°C 扩散流，管虫与贻贝主要栖息地
                (15.0, 6.5, 0.3, 0.2, 0.1, 5.0, 15.0)
            }
            HydrothermalVentType::LostCity => {
                // 90°C 碱性蛇纹石化，pH 9-11
                (90.0, 10.0, 0.05, 2.0, 0.01, 0.5, 8.0)
            }
        };
        Self {
            vent_type: vtype,
            temperature_c,
            depth_m,
            pressure_mpa,
            ph,
            h2s_conc_mm: h2s,
            ch4_conc_mm: ch4,
            fe_conc_mm: fe,
            co2_conc_mm: co2,
            biomass_density_kg_m2: biomass,
        }
    }

    /// 化能合成初级生产速率 (g C/m²/day)
    /// 黑烟囱可达 100-1000 g C/m²/day，远超光合成表层海洋 1-3 g C/m²/day
    /// 来源：McCollom 2000, Jannasch 1995
    pub fn primary_production_rate(&self) -> f32 {
        // 与 H2S、CH4、Fe 浓度成正比，与温度偏离最适 30°C 成反比
        let chemo_energy =
            self.h2s_conc_mm * 5.0 + self.ch4_conc_mm * 3.0 + self.fe_conc_mm * 1.5;
        // 温度因子：嗜热化能合成最适 30-60°C
        let temp_factor = if self.temperature_c < 200.0 {
            (self.temperature_c / 60.0).min(1.5)
        } else {
            0.5 // 高温区只能由古菌利用，速率降低
        };
        chemo_energy * temp_factor * 2.0
    }

    /// 热泉群落组成
    /// 来源：Van Dover 2000
    pub fn vent_community(&self) -> Vec<VentOrganism> {
        match self.vent_type {
            HydrothermalVentType::BlackSmoker => {
                vec![
                    VentOrganism {
                        name: "Alvinella pompejana".into(), // 庞贝虫，80°C 最耐热多细胞动物
                        role: VentTrophicRole::SymbiontHost,
                        biomass_pct: 35.0,
                        temp_tolerance_c: (20.0, 80.0),
                    },
                    VentOrganism {
                        name: "Rimicaris exoculata".into(), // 盲虾，背眼感热
                        role: VentTrophicRole::Grazer,
                        biomass_pct: 25.0,
                        temp_tolerance_c: (2.0, 40.0),
                    },
                    VentOrganism {
                        name: "Bythograea thermydron".into(), // 热泉蟹
                        role: VentTrophicRole::Predator,
                        biomass_pct: 15.0,
                        temp_tolerance_c: (2.0, 35.0),
                    },
                    VentOrganism {
                        name: "Sulfur-oxidizing bacteria".into(),
                        role: VentTrophicRole::PrimaryProducer,
                        biomass_pct: 20.0,
                        temp_tolerance_c: (10.0, 100.0),
                    },
                    VentOrganism {
                        name: "Archaeal biofilm".into(),
                        role: VentTrophicRole::PrimaryProducer,
                        biomass_pct: 5.0,
                        temp_tolerance_c: (60.0, 113.0),
                    },
                ]
            }
            HydrothermalVentType::WhiteSmoker => {
                vec![
                    VentOrganism {
                        name: "Bathymodiolus thermophilus".into(), // 热泉贻贝，共生菌
                        role: VentTrophicRole::SymbiontHost,
                        biomass_pct: 40.0,
                        temp_tolerance_c: (2.0, 30.0),
                    },
                    VentOrganism {
                        name: "Tevnia jerichonana".into(), // 管虫
                        role: VentTrophicRole::SymbiontHost,
                        biomass_pct: 25.0,
                        temp_tolerance_c: (5.0, 35.0),
                    },
                    VentOrganism {
                        name: "Limpet species".into(),
                        role: VentTrophicRole::Grazer,
                        biomass_pct: 15.0,
                        temp_tolerance_c: (2.0, 25.0),
                    },
                    VentOrganism {
                        name: "Sulfur-oxidizing bacteria".into(),
                        role: VentTrophicRole::PrimaryProducer,
                        biomass_pct: 20.0,
                        temp_tolerance_c: (10.0, 80.0),
                    },
                ]
            }
            HydrothermalVentType::DiffuseFlow => {
                vec![
                    VentOrganism {
                        name: "Riftia pachyptila".into(),
                        // 巨型管虫，2-3m 长，无口无肠，靠共生硫氧化菌
                        role: VentTrophicRole::SymbiontHost,
                        biomass_pct: 50.0,
                        temp_tolerance_c: (2.0, 25.0),
                    },
                    VentOrganism {
                        name: "Calyptogena magnifica".into(), // 巨型白蚌，共生菌
                        role: VentTrophicRole::SymbiontHost,
                        biomass_pct: 20.0,
                        temp_tolerance_c: (2.0, 20.0),
                    },
                    VentOrganism {
                        name: "Bathymodiolus thermophilus".into(),
                        role: VentTrophicRole::SymbiontHost,
                        biomass_pct: 15.0,
                        temp_tolerance_c: (2.0, 30.0),
                    },
                    VentOrganism {
                        name: "Sulfur-oxidizing bacteria".into(),
                        role: VentTrophicRole::PrimaryProducer,
                        biomass_pct: 10.0,
                        temp_tolerance_c: (2.0, 50.0),
                    },
                    VentOrganism {
                        name: "Bythograea thermydron".into(),
                        role: VentTrophicRole::Scavenger,
                        biomass_pct: 5.0,
                        temp_tolerance_c: (2.0, 35.0),
                    },
                ]
            }
            HydrothermalVentType::LostCity => {
                vec![
                    VentOrganism {
                        name: "Lost City Methanosarcinales".into(), // 甲烷八叠球菌目
                        role: VentTrophicRole::PrimaryProducer,
                        biomass_pct: 60.0,
                        temp_tolerance_c: (10.0, 90.0),
                    },
                    VentOrganism {
                        name: "Serpentinization-associated biofilm".into(),
                        role: VentTrophicRole::PrimaryProducer,
                        biomass_pct: 25.0,
                        temp_tolerance_c: (10.0, 90.0),
                    },
                    VentOrganism {
                        name: "Snails (Provannidae)".into(),
                        role: VentTrophicRole::Grazer,
                        biomass_pct: 10.0,
                        temp_tolerance_c: (2.0, 25.0),
                    },
                    VentOrganism {
                        name: "Polychaete worms".into(),
                        role: VentTrophicRole::Scavenger,
                        biomass_pct: 5.0,
                        temp_tolerance_c: (2.0, 25.0),
                    },
                ]
            }
        }
    }
}

/// 热泉生物群落成员
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VentOrganism {
    pub name: String,
    pub role: VentTrophicRole,
    /// 占群落总生物量百分比
    pub biomass_pct: f32,
    /// 温度耐受范围 (°C)
    pub temp_tolerance_c: (f32, f32),
}

/// 热泉营养级角色
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VentTrophicRole {
    /// 化能自养菌（硫氧化细菌）
    PrimaryProducer,
    /// 共生宿主（管虫、贻贝、蚌）
    SymbiontHost,
    /// 食菌者（螺、虾）
    Grazer,
    /// 捕食者（蟹、鱼）
    Predator,
    /// 食腐者
    Scavenger,
}

// ============ 4. 太空生存 ============
// 来源：Jönsson 2008, Horneck 2008

/// 太空生存能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceSurvivalCapability {
    pub organism: String,
    pub vacuum_survival_days: u32,
    /// 紫外线抗性 0.0-1.0
    pub uv_resistance: f32,
    pub cosmic_ray_resistance_gy: f32,
    pub temp_range_c: (f32, f32),
    /// 太空中估算存活年限
    pub estimated_viability_years: u32,
}

/// 已知太空生存生物
/// 来源：BIOPAN、EXPOSE-E、Photon-M、Tanpopo 实验任务
pub fn space_survivors() -> Vec<SpaceSurvivalCapability> {
    vec![
        // Tardigrades — 水熊虫，真空 10 天+，太空 2007 TARDIS 实验
        // Jönsson et al. 2008
        SpaceSurvivalCapability {
            organism: "Tardigrades (R. coronifer)".into(),
            vacuum_survival_days: 10,
            uv_resistance: 0.95,
            cosmic_ray_resistance_gy: 5000.0,
            temp_range_c: (-272.0, 150.0),
            estimated_viability_years: 100,
        },
        // Bacillus subtilis spores — 6 年太空，Spall 1999
        // Horneck 2008
        SpaceSurvivalCapability {
            organism: "Bacillus subtilis spores".into(),
            vacuum_survival_days: 2190,
            uv_resistance: 0.7,
            cosmic_ray_resistance_gy: 4000.0,
            temp_range_c: (-50.0, 100.0),
            estimated_viability_years: 6,
        },
        // Deinococcus radiodurans — 3 年太空实验 Tanpopo
        // Kawaguchi et al. 2020
        SpaceSurvivalCapability {
            organism: "Deinococcus radiodurans".into(),
            vacuum_survival_days: 1095,
            uv_resistance: 0.85,
            cosmic_ray_resistance_gy: 5000.0,
            temp_range_c: (-40.0, 60.0),
            estimated_viability_years: 3,
        },
        // Lichens — 15 天太空暴露，BIOPAN
        // Sancho et al. 2007
        SpaceSurvivalCapability {
            organism: "Lichens (Rhizocarpon)".into(),
            vacuum_survival_days: 15,
            uv_resistance: 0.6,
            cosmic_ray_resistance_gy: 1000.0,
            temp_range_c: (-50.0, 80.0),
            estimated_viability_years: 1,
        },
        // Bdelloid rotifers — 轮虫
        SpaceSurvivalCapability {
            organism: "Bdelloid rotifers (Adineta)".into(),
            vacuum_survival_days: 7,
            uv_resistance: 0.5,
            cosmic_ray_resistance_gy: 1000.0,
            temp_range_c: (-30.0, 60.0),
            estimated_viability_years: 1,
        },
    ]
}

// ============ 5. 极端环境参数边界 ============

/// 生命存在的环境参数边界
/// 来源：Merino et al. 2019, "Life at the Extremes"
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LifeBoundary {
    pub temp_c: (f32, f32),
    pub ph: (f32, f32),
    pub pressure_mpa: (f32, f32),
    pub salinity_pct: (f32, f32),
    pub water_activity: (f32, f32),
    pub radiation_gy: (f32, f32),
    pub depth_m: (f32, f32),
}

impl LifeBoundary {
    /// 已知生命存在的边界（实测）
    /// [补充] 改为关联函数以便无实例调用，原 spec 为 &self -> Self
    pub fn known_life() -> Self {
        Self {
            temp_c: (-20.0, 122.0), // -20 Psychrobacter，122 Methanopyrus
            ph: (0.0, 13.0),         // 0 Picrophilus，13 碱湖
            pressure_mpa: (0.0, 120.0), // 120 MPa Mariana Trench
            salinity_pct: (0.0, 35.0),  // 35% 饱和盐湖
            water_activity: (0.6, 1.0), // 0.61 Xeromyces
            radiation_gy: (0.0, 30000.0), // 30000 Gy Thermococcus gammatolerans
            depth_m: (0.0, 11000.0),     // 11000 m Mariana
        }
    }

    /// 理论宜居带（含推演空间，标注 [推演]）
    /// [推演] 基于现有记录外推，理论上生命可在 -40°C（冻结态休眠）至 150°C（高压液态水）生存
    pub fn habitable_zone() -> Self {
        Self {
            temp_c: (-40.0, 150.0),
            ph: (-0.5, 14.0),
            pressure_mpa: (0.0, 1500.0), // 推演至地壳深处
            salinity_pct: (0.0, 40.0),
            water_activity: (0.5, 1.0), // 推演外推
            radiation_gy: (0.0, 50000.0),
            depth_m: (0.0, 15000.0), // 推演至地壳深部
        }
    }
}

impl Default for LifeBoundary {
    fn default() -> Self {
        Self::known_life()
    }
}

// ============ 6. 辅助查询方法 ============

impl Extremophile {
    /// 返回温度耐受范围 (min, max) °C
    pub fn temp_tolerance_range(&self) -> (f32, f32) {
        (self.min_temp_c, self.max_temp_c)
    }

    /// 返回 pH 耐受范围
    pub fn ph_tolerance_range(&self) -> (f32, f32) {
        (self.min_ph, self.max_ph)
    }

    /// 温度是否在耐受范围内
    pub fn temp_in_range(&self, temp_c: f32) -> bool {
        temp_c >= self.min_temp_c && temp_c <= self.max_temp_c
    }

    /// pH 是否在耐受范围内
    pub fn ph_in_range(&self, ph: f32) -> bool {
        ph >= self.min_ph && ph <= self.max_ph
    }

    /// 给定环境条件下的存活概率 (0.0-1.0)
    pub fn survival_probability(
        &self,
        temp_c: f32,
        ph: f32,
        pressure_mpa: f32,
        salinity_pct: f32,
        radiation_gy: f32,
    ) -> f32 {
        if !self.temp_in_range(temp_c)
            || !self.ph_in_range(ph)
            || pressure_mpa > self.max_pressure_mpa
            || radiation_gy > self.radiation_resistance_gy
        {
            return 0.0;
        }
        let temp_factor = if temp_c >= self.optimal_temp_c {
            let upper = (self.max_temp_c - self.optimal_temp_c).max(0.1);
            (1.0 - (temp_c - self.optimal_temp_c) / upper).max(0.0)
        } else {
            let lower = (self.optimal_temp_c - self.min_temp_c).max(0.1);
            (1.0 - (self.optimal_temp_c - temp_c) / lower).max(0.0)
        };
        let ph_factor = if ph >= self.optimal_ph {
            let upper = (self.max_ph - self.optimal_ph).max(0.05);
            (1.0 - (ph - self.optimal_ph) / upper).max(0.0)
        } else {
            let lower = (self.optimal_ph - self.min_ph).max(0.05);
            (1.0 - (self.optimal_ph - ph) / lower).max(0.0)
        };
        let p_factor = if pressure_mpa >= self.optimal_pressure_mpa {
            let upper = (self.max_pressure_mpa - self.optimal_pressure_mpa).max(0.1);
            (1.0 - (pressure_mpa - self.optimal_pressure_mpa) / upper).max(0.0)
        } else {
            let lower = self.optimal_pressure_mpa.max(0.1);
            (1.0 - (self.optimal_pressure_mpa - pressure_mpa) / lower).max(0.0)
        };
        let sal_factor = if self.optimal_salinity_pct > 0.0 {
            (1.0 - ((salinity_pct - self.optimal_salinity_pct).abs()
                / (self.optimal_salinity_pct * 2.0)))
                .max(0.0)
        } else {
            1.0
        };
        let r_factor = if self.radiation_resistance_gy > 0.0 {
            1.0 - (radiation_gy / self.radiation_resistance_gy).min(0.95)
        } else {
            0.0
        };
        temp_factor * ph_factor * p_factor * sal_factor * r_factor
    }

    /// 应力指数 0.0-1.0：与最适条件偏离越大，应力越高
    pub fn stress_index(&self, temp_c: f32, ph: f32, pressure_mpa: f32) -> f32 {
        let temp_stress = if temp_c >= self.optimal_temp_c {
            let upper = (self.max_temp_c - self.optimal_temp_c).max(0.1);
            ((temp_c - self.optimal_temp_c) / upper).min(1.0)
        } else {
            let lower = (self.optimal_temp_c - self.min_temp_c).max(0.1);
            ((self.optimal_temp_c - temp_c) / lower).min(1.0)
        };
        let ph_stress = if ph >= self.optimal_ph {
            let upper = (self.max_ph - self.optimal_ph).max(0.05);
            ((ph - self.optimal_ph) / upper).min(1.0)
        } else {
            let lower = (self.optimal_ph - self.min_ph).max(0.05);
            ((self.optimal_ph - ph) / lower).min(1.0)
        };
        let p_stress = if pressure_mpa >= self.optimal_pressure_mpa {
            let upper = (self.max_pressure_mpa - self.optimal_pressure_mpa).max(0.1);
            ((pressure_mpa - self.optimal_pressure_mpa) / upper).min(1.0)
        } else {
            let lower = self.optimal_pressure_mpa.max(0.1);
            ((self.optimal_pressure_mpa - pressure_mpa) / lower).min(1.0)
        };
        (temp_stress + ph_stress + p_stress) / 3.0
    }

    /// 当前环境条件是否对该生物致命
    pub fn is_lethal(&self, temp_c: f32, ph: f32, pressure_mpa: f32) -> bool {
        temp_c < self.min_temp_c
            || temp_c > self.max_temp_c
            || ph < self.min_ph
            || ph > self.max_ph
            || pressure_mpa > self.max_pressure_mpa
    }

    /// 该生物对给定环境的适应收益 (0.0-1.0)
    pub fn adaptation_benefit(&self, env_temp_c: f32) -> f32 {
        let baseline_deviation = (env_temp_c - 25.0).abs();
        (baseline_deviation / 80.0).min(1.0)
    }

    /// 是否为超嗜热生物（max_temp_c >= 80°C）
    pub fn is_hyperthermophile(&self) -> bool {
        self.max_temp_c >= 80.0
    }

    /// 辐射抗性分级
    pub fn radiation_resistance_class(&self) -> &'static str {
        if self.radiation_resistance_gy >= 10000.0 {
            "extreme"
        } else if self.radiation_resistance_gy >= 1000.0 {
            "high"
        } else if self.radiation_resistance_gy >= 100.0 {
            "medium"
        } else {
            "low"
        }
    }
}

impl HydrothermalVent {
    pub fn is_black_smoker(&self) -> bool {
        matches!(self.vent_type, HydrothermalVentType::BlackSmoker)
    }

    pub fn depth_category(&self) -> &'static str {
        if self.depth_m < 200.0 {
            "shallow"
        } else if self.depth_m < 2000.0 {
            "deep"
        } else if self.depth_m < 6000.0 {
            "abyssal"
        } else {
            "hadal"
        }
    }

    pub fn chemosynthesis_energy(&self) -> f32 {
        self.h2s_conc_mm * 5.0 + self.ch4_conc_mm * 3.0 + self.fe_conc_mm * 1.5
    }

    pub fn biomass_category(&self) -> &'static str {
        if self.biomass_density_kg_m2 >= 30.0 {
            "lush"
        } else if self.biomass_density_kg_m2 >= 15.0 {
            "moderate"
        } else if self.biomass_density_kg_m2 >= 5.0 {
            "sparse"
        } else {
            "barren"
        }
    }

    pub fn is_high_temperature(&self) -> bool {
        self.temperature_c > 100.0
    }
}

impl SpaceSurvivalCapability {
    pub fn survival_score(&self) -> f32 {
        let vacuum_score = (self.vacuum_survival_days as f32 / 2190.0).min(1.0);
        let uv_score = self.uv_resistance.clamp(0.0, 1.0);
        let rad_score = (self.cosmic_ray_resistance_gy / 5000.0).min(1.0);
        let temp_span = (self.temp_range_c.1 - self.temp_range_c.0).abs() / 422.0;
        (vacuum_score + uv_score + rad_score + temp_span) / 4.0
    }

    pub fn can_survive_vacuum(&self, days: u32) -> bool {
        self.vacuum_survival_days >= days
    }

    pub fn uv_protection_class(&self) -> &'static str {
        if self.uv_resistance >= 0.85 {
            "high"
        } else if self.uv_resistance >= 0.6 {
            "medium"
        } else {
            "low"
        }
    }

    pub fn temp_tolerance_range(&self) -> (f32, f32) {
        self.temp_range_c
    }
}

impl LifeBoundary {
    pub fn contains_temp(&self, temp_c: f32) -> bool {
        temp_c >= self.temp_c.0 && temp_c <= self.temp_c.1
    }

    pub fn contains_ph(&self, ph: f32) -> bool {
        ph >= self.ph.0 && ph <= self.ph.1
    }

    pub fn temp_span(&self) -> f32 {
        self.temp_c.1 - self.temp_c.0
    }

    pub fn pressure_span(&self) -> f32 {
        self.pressure_mpa.1 - self.pressure_mpa.0
    }

    pub fn radiation_limit_gy(&self) -> f32 {
        self.radiation_gy.1
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermophile_database_nonempty() {
        let db = thermophile_database();
        assert!(db.len() >= 5);
        for org in &db {
            assert!(org.max_temp_c >= 70.0, "thermophile should tolerate >= 70C");
        }
    }

    #[test]
    fn test_psychrophile_database_nonempty() {
        let db = psychrophile_database();
        assert!(!db.is_empty());
        for org in &db {
            assert!(org.optimal_temp_c <= 10.0, "psychrophile optimal <= 10C");
        }
    }

    #[test]
    fn test_extremophile_temp_in_range() {
        let org = &thermophile_database()[0];
        assert!(org.temp_in_range(70.0));
        assert!(org.temp_in_range(40.0));
        assert!(org.temp_in_range(79.0));
        assert!(!org.temp_in_range(39.9));
        assert!(!org.temp_in_range(79.1));
    }

    #[test]
    fn test_extremophile_ph_in_range() {
        let org = &acidophile_database()[0];
        assert!(org.ph_in_range(0.7));
        assert!(org.ph_in_range(0.0));
        assert!(!org.ph_in_range(4.1));
    }

    #[test]
    fn test_survival_probability_optimal_is_max() {
        let org = &thermophile_database()[0];
        let p_opt = org.survival_probability(70.0, 8.0, 0.1, 0.5, 0.0);
        assert!((p_opt - 1.0).abs() < 1e-5, "optimal survival should be 1.0, got {}", p_opt);
        let p_off = org.survival_probability(50.0, 8.0, 0.1, 0.5, 0.0);
        assert!(p_off < p_opt, "non-optimal should be lower");
        assert!(p_off > 0.0, "non-optimal should still be positive within range");
    }

    #[test]
    fn test_survival_probability_zero_outside_range() {
        let org = &thermophile_database()[0];
        let p = org.survival_probability(200.0, 8.0, 0.1, 0.5, 0.0);
        assert_eq!(p, 0.0);
        let p2 = org.survival_probability(70.0, 8.0, 0.1, 0.5, 10000.0);
        assert_eq!(p2, 0.0);
    }

    #[test]
    fn test_stress_index_zero_at_optimal() {
        let org = &thermophile_database()[0];
        let s = org.stress_index(70.0, 8.0, 0.1);
        assert!(s < 1e-5, "stress at optimal should be 0, got {}", s);
    }

    #[test]
    fn test_stress_index_high_at_extreme() {
        let org = &thermophile_database()[0];
        let s = org.stress_index(79.0, 6.0, 1.0);
        assert!(s > 0.5, "stress at boundary should be high, got {}", s);
    }

    #[test]
    fn test_is_lethal_boundaries() {
        let org = &thermophile_database()[0];
        assert!(!org.is_lethal(40.0, 6.0, 1.0));
        assert!(org.is_lethal(39.9, 6.0, 1.0));
        assert!(org.is_lethal(79.1, 6.0, 1.0));
        assert!(org.is_lethal(40.0, 5.9, 1.0));
        assert!(org.is_lethal(40.0, 10.1, 1.0));
        assert!(org.is_lethal(40.0, 6.0, 1.1));
    }

    #[test]
    fn test_adaptation_benefit_increases_with_extremity() {
        let org = &thermophile_database()[0];
        let b_mild = org.adaptation_benefit(25.0);
        let b_extreme = org.adaptation_benefit(90.0);
        assert!(b_extreme > b_mild);
        assert!(b_mild >= 0.0 && b_mild <= 1.0);
        assert!(b_extreme >= 0.0 && b_extreme <= 1.0);
    }

    #[test]
    fn test_is_hyperthermophile() {
        let thermo = &thermophile_database()[0];
        assert!(!thermo.is_hyperthermophile());
        let hyper = &thermophile_database()[1];
        assert!(hyper.is_hyperthermophile());
    }

    #[test]
    fn test_radiation_resistance_class() {
        let psychro = &psychrophile_database()[0];
        assert_eq!(psychro.radiation_resistance_class(), "low");
        let normal = &thermophile_database()[0];
        assert_eq!(normal.radiation_resistance_class(), "medium");
        let radiotolerant = &radiotolerant_database()[0];
        assert_eq!(radiotolerant.radiation_resistance_class(), "high");
        let gammatolerans = &radiotolerant_database()[1];
        assert_eq!(gammatolerans.radiation_resistance_class(), "extreme");
    }

    #[test]
    fn test_hydrothermal_vent_construction() {
        let bs = HydrothermalVent::new(HydrothermalVentType::BlackSmoker, 2500.0);
        assert!(bs.is_black_smoker());
        assert!((bs.pressure_mpa - 250.0).abs() < 1e-3);
        assert_eq!(bs.depth_category(), "abyssal");
        assert!(bs.is_high_temperature());
        assert_eq!(bs.biomass_category(), "lush");
    }

    #[test]
    fn test_vent_diffuse_flow() {
        let df = HydrothermalVent::new(HydrothermalVentType::DiffuseFlow, 100.0);
        assert!(!df.is_black_smoker());
        assert_eq!(df.depth_category(), "shallow");
        assert!(!df.is_high_temperature());
        assert!(df.chemosynthesis_energy() > 0.0);
    }

    #[test]
    fn test_primary_production_rate_positive() {
        for vtype in [
            HydrothermalVentType::BlackSmoker,
            HydrothermalVentType::WhiteSmoker,
            HydrothermalVentType::DiffuseFlow,
            HydrothermalVentType::LostCity,
        ] {
            let v = HydrothermalVent::new(vtype, 2000.0);
            assert!(v.primary_production_rate() > 0.0, "production rate should be positive");
        }
    }

    #[test]
    fn test_space_survival_capability() {
        let survivors = space_survivors();
        assert!(survivors.len() >= 4);
        let tardigrade = &survivors[0];
        assert!(tardigrade.can_survive_vacuum(5));
        assert!(!tardigrade.can_survive_vacuum(100));
        let score = tardigrade.survival_score();
        assert!(score > 0.5 && score <= 1.0, "tardigrade score {}", score);
        assert_eq!(tardigrade.uv_protection_class(), "high");
    }

    #[test]
    fn test_space_survival_temp_range() {
        let s = &space_survivors()[0];
        let (lo, hi) = s.temp_tolerance_range();
        assert!(lo < 0.0 && hi > 100.0);
    }

    #[test]
    fn test_life_boundary_known_life() {
        let lb = LifeBoundary::known_life();
        assert!(lb.contains_temp(37.0));
        assert!(lb.contains_temp(-20.0));
        assert!(!lb.contains_temp(-30.0));
        assert!(lb.contains_ph(7.0));
        assert!(lb.contains_ph(0.0));
        assert!(!lb.contains_ph(-0.1));
        assert!((lb.temp_span() - 142.0).abs() < 1e-3);
        assert!(lb.radiation_limit_gy() >= 30000.0);
    }

    #[test]
    fn test_life_boundary_habitable_zone_wider() {
        let known = LifeBoundary::known_life();
        let habitable = LifeBoundary::habitable_zone();
        assert!(habitable.temp_span() > known.temp_span());
        assert!(habitable.pressure_span() > known.pressure_span());
        assert!(habitable.contains_temp(-30.0));
        assert!(!known.contains_temp(-30.0));
    }

    #[test]
    fn test_life_boundary_default_is_known_life() {
        let default_lb = LifeBoundary::default();
        let known_lb = LifeBoundary::known_life();
        assert_eq!(default_lb.temp_c, known_lb.temp_c);
        assert_eq!(default_lb.ph, known_lb.ph);
    }

    #[test]
    fn test_halophile_database() {
        let db = halophile_database();
        assert!(db.len() >= 3);
        for org in &db {
            assert!(org.optimal_salinity_pct >= 20.0, "halophile should need high salinity");
        }
    }

    #[test]
    fn test_barophile_database_pressure() {
        let db = barophile_database();
        assert!(db.len() >= 3);
        for org in &db {
            assert!(org.optimal_pressure_mpa >= 50.0, "barophile optimal pressure >= 50 MPa");
        }
    }

    #[test]
    fn test_vent_community_nonempty() {
        let v = HydrothermalVent::new(HydrothermalVentType::BlackSmoker, 2500.0);
        let community = v.vent_community();
        assert!(community.len() >= 4);
        let total_biomass: f32 = community.iter().map(|o| o.biomass_pct).sum();
        assert!((total_biomass - 100.0).abs() < 1.0, "biomass should sum to 100, got {}", total_biomass);
    }

    #[test]
    fn test_xerophile_low_water_activity() {
        let db = xerophile_database();
        assert!(db.len() >= 2);
        for org in &db {
            assert!(org.min_water_activity <= 0.75, "xerophile should tolerate low aw");
        }
    }
}