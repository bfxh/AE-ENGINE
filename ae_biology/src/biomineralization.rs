use serde::{Deserialize, Serialize};

// ============ 1. 矿物类型 ============

/// 生物矿化主要矿物种类
/// 来源: Lowenstam & Weiner 1989, Mann 2001
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Biomineral {
    /// 碳酸钙 CaCO3（最普遍，贝壳/珊瑚/骨骼前驱）
    CalciumCarbonate,
    /// 磷酸钙（羟基磷灰石 Ca10(PO4)6(OH)2，骨/牙）
    CalciumPhosphate,
    /// 二氧化硅 SiO2·nH2O（硅藻/放射虫/海绵骨针/植物硅体）
    Silica,
    /// 磁铁矿 Fe3O4（磁小体）
    Magnetite,
    /// 针铁矿 α-FeOOH（帽贝齿舌）
    Goethite,
    /// 硫酸钡 BaSO4（重晶石沉积，某些绿藻）
    BariumSulfate,
    /// 硫酸锶 SrSO4（天青石，棘皮类变形器官）
    StrontiumSulfate,
    /// 草酸钙（植物防御针晶，龙舌兰/海芋）
    CalciumOxalate,
    /// 石膏 CaSO4（硫酸盐沉积）
    Gypsum,
    /// 石盐 NaCl（盐杆菌沉积）
    Halite,
}

/// CaCO3 多型相，稳定性: 方解石 > 文石 > 球霰石 > 无定形 ACC
/// 来源: Weiner 2003; Addadi 2003
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CaCO3Polymorph {
    /// 方解石（最稳定三方晶系，珊瑚外层/有孔虫/海胆刺）
    Calcite,
    /// 文石（正交晶系，珍珠层/珊瑚骨架/乌贼骨）
    Aragonite,
    /// 球霰石（六方晶系，最不稳定，过渡相）
    Vaterite,
    /// 无定形碳酸钙 ACC（瞬时前驱相，约 30 nm 颗粒）
    Amorphous,
}

/// 矿化策略分类
/// 来源: Lowenstam 1981
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MineralizationStrategy {
    /// 生物控制矿化（骨骼/贝壳/牙，细胞内精确调控）
    BiologicallyControlled,
    /// 生物诱导矿化（细菌沉积/蓝藻叠层石，环境依赖）
    BiologicallyInduced,
    /// 边界组织矿化（细胞外基质模板诱导）
    BoundaryOrganized,
}

// ============ 2. 生物矿化结构 ============

/// 单个生物矿化结构的力学与生物学描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomineralStructure {
    pub name: String,
    pub mineral: Biomineral,
    pub polymorph: Option<CaCO3Polymorph>,
    pub organism: String,
    pub location: String,
    pub function: String,
    /// 莫氏硬度
    pub hardness_mohs: f32,
    /// 密度 kg/m³
    pub density_kg_m3: f32,
    /// 杨氏模量 GPa
    pub youngs_modulus_gpa: f32,
    /// 断裂韧性 MPa·m^0.5
    pub fracture_toughness_mpa_m05: f32,
    pub strategy: MineralizationStrategy,
    /// 层次结构级数（骨 7 级最高）
    pub hierarchical_levels: u32,
}

// ============ 3. 真实生物矿化数据库 ============

/// 真实生物矿化数据库（18 条典型条目）
/// 数据来源: Weiner 2003; Meyers 2008; Knoll 2003; Faivre 2008
pub fn biomineral_database() -> Vec<BiomineralStructure> {
    vec![
        // 1. 珍珠层 Nacre —— 最强天然复合材料
        BiomineralStructure {
            name: "珍珠层 Nacre".to_string(),
            mineral: Biomineral::CalciumCarbonate,
            polymorph: Some(CaCO3Polymorph::Aragonite),
            organism: "珠母贝 Pinctada".to_string(),
            location: "贝壳内侧珍珠层".to_string(),
            function: "结构增强/防御/珍珠生成".to_string(),
            // 95% 文石 + 5% 有机质(蛋白+几丁质)
            // "砖-泥"结构: 10-20 μm 文石板 + 30 nm 有机层
            // 文石自身 Kc≈0.2，nacre 4-7，增韧 3000× (Weiner 2003)
            hardness_mohs: 3.5,
            density_kg_m3: 2700.0,
            youngs_modulus_gpa: 70.0,
            fracture_toughness_mpa_m05: 5.5, // 4-7 中位
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 4,
        },
        // 2. 珊瑚骨架 —— 受海洋酸化威胁
        BiomineralStructure {
            name: "珊瑚骨架 Coral skeleton".to_string(),
            mineral: Biomineral::CalciumCarbonate,
            polymorph: Some(CaCO3Polymorph::Aragonite),
            organism: "石珊瑚 Scleractinia".to_string(),
            location: "珊瑚虫分泌骨架".to_string(),
            function: "结构支撑/礁体建造".to_string(),
            // 关键蛋白: SOM 酸性蛋白 CARP (Mass 2007)
            // 受海洋酸化威胁: CO2 + CO3^2- → 2HCO3^-
            hardness_mohs: 3.5,
            density_kg_m3: 2940.0,
            youngs_modulus_gpa: 60.0,
            fracture_toughness_mpa_m05: 0.5,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 3,
        },
        // 3. 骨骼 Bone —— 7 级层次结构
        BiomineralStructure {
            name: "皮质骨 Cortical bone".to_string(),
            mineral: Biomineral::CalciumPhosphate,
            polymorph: None,
            organism: "脊椎动物 Vertebrata".to_string(),
            location: "骨架".to_string(),
            function: "支撑/造血/钙库".to_string(),
            // 65% 羟基磷灰石 + 25% I 型胶原 + 10% 水
            // 7 级: 胶原-羟基磷灰石-纤维-束-骨板-骨单位-骨
            // 皮质骨 E=17 GPa, σ_yield 100-150 MPa
            // 骨陷窝 25 μm，骨细胞感应应力 (Weiner 2003)
            hardness_mohs: 3.5,
            density_kg_m3: 2000.0,
            youngs_modulus_gpa: 17.0,
            fracture_toughness_mpa_m05: 6.0, // 2-12 中位
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 7,
        },
        // 4. 牙釉质 Enamel —— 最硬生物组织
        BiomineralStructure {
            name: "牙釉质 Enamel".to_string(),
            mineral: Biomineral::CalciumPhosphate,
            polymorph: None,
            organism: "脊椎动物 Vertebrata".to_string(),
            location: "牙冠外层".to_string(),
            function: "咀嚼研磨".to_string(),
            // 96% 羟基磷灰石(晶体 >100nm 宽, μm 级长)
            // 3% 水 + 1% 有机质; 莫氏硬度 5
            // 不再生(成釉细胞死亡)
            hardness_mohs: 5.0,
            density_kg_m3: 2900.0,
            youngs_modulus_gpa: 80.0,
            fracture_toughness_mpa_m05: 1.0, // 0.6-1.5
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 5,
        },
        // 5. 鲨鱼软骨
        BiomineralStructure {
            name: "鲨鱼软骨 Shark cartilage".to_string(),
            mineral: Biomineral::CalciumPhosphate,
            polymorph: None,
            organism: "软骨鱼 Chondrichthyes".to_string(),
            location: "全软骨骨架".to_string(),
            function: "结构支撑(钙盐强化)".to_string(),
            // TMAO 氧化三甲胺稳定蛋白质
            hardness_mohs: 2.0,
            density_kg_m3: 1100.0,
            youngs_modulus_gpa: 0.5,
            fracture_toughness_mpa_m05: 1.5,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 2,
        },
        // 6. 硅藻硅壳 Frustule
        BiomineralStructure {
            name: "硅藻壳 Frustule".to_string(),
            mineral: Biomineral::Silica,
            polymorph: None,
            organism: "硅藻门 Bacillariophyceae".to_string(),
            location: "细胞壁".to_string(),
            function: "防护/光学/浮力".to_string(),
            // 10^5 物种, 100 nm 级精细孔洞
            // 海洋 20% 初级生产力 (Mann 2001)
            hardness_mohs: 6.0,
            density_kg_m3: 2200.0,
            youngs_modulus_gpa: 30.0,
            fracture_toughness_mpa_m05: 0.8,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 3,
        },
        // 7. 放射虫硅骨架
        BiomineralStructure {
            name: "放射虫骨架 Radiolarian".to_string(),
            mineral: Biomineral::Silica,
            polymorph: None,
            organism: "放射虫 Radiolaria".to_string(),
            location: "细胞外骨架".to_string(),
            function: "结构/浮力".to_string(),
            hardness_mohs: 6.0,
            density_kg_m3: 2200.0,
            youngs_modulus_gpa: 30.0,
            fracture_toughness_mpa_m05: 0.5,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 2,
        },
        // 8. 玻璃海绵骨针
        BiomineralStructure {
            name: "玻璃海绵骨针 Glass sponge spicule".to_string(),
            mineral: Biomineral::Silica,
            polymorph: None,
            organism: "六放海绵 Hexactinellida".to_string(),
            location: "骨架骨针".to_string(),
            function: "结构/光纤传光".to_string(),
            // Euplectella aspergillum 偕老同穴，复杂玻璃结构
            hardness_mohs: 6.0,
            density_kg_m3: 2200.0,
            youngs_modulus_gpa: 35.0,
            fracture_toughness_mpa_m05: 1.0,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 3,
        },
        // 9. 有孔虫钙壳
        BiomineralStructure {
            name: "有孔虫壳 Foraminifera test".to_string(),
            mineral: Biomineral::CalciumCarbonate,
            polymorph: Some(CaCO3Polymorph::Calcite),
            organism: "有孔虫 Foraminifera".to_string(),
            location: "测试壳".to_string(),
            function: "结构/古气候指标".to_string(),
            // 5 万种化石, 浮游有孔虫 0.1-1 mm
            hardness_mohs: 3.0,
            density_kg_m3: 2710.0,
            youngs_modulus_gpa: 70.0,
            fracture_toughness_mpa_m05: 0.3,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 2,
        },
        // 10. 颗石藻颗石板
        BiomineralStructure {
            name: "颗石 Coccolith".to_string(),
            mineral: Biomineral::CalciumCarbonate,
            polymorph: Some(CaCO3Polymorph::Calcite),
            organism: "颗石藻 Emiliania huxleyi".to_string(),
            location: "细胞表面颗石板".to_string(),
            function: "光学/防御/钙化".to_string(),
            // 30 nm 颗石板, 球霰石→方解石 (Young 2003)
            // 大规模藻华, 卫星可见
            hardness_mohs: 3.0,
            density_kg_m3: 2710.0,
            youngs_modulus_gpa: 70.0,
            fracture_toughness_mpa_m05: 0.2,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 2,
        },
        // 11. 磁小体 Magnetosome
        BiomineralStructure {
            name: "磁小体 Magnetosome".to_string(),
            mineral: Biomineral::Magnetite,
            polymorph: None,
            organism: "趋磁细菌 Magnetospirillum magneticum".to_string(),
            location: "细胞内链状".to_string(),
            function: "地磁场定向游动".to_string(),
            // 50 nm 磁铁矿单畴颗粒链
            // 0.1-0.5% 干重, Mam 系列蛋白 (Faivre 2008)
            hardness_mohs: 6.0,
            density_kg_m3: 5180.0,
            youngs_modulus_gpa: 230.0,
            fracture_toughness_mpa_m05: 1.5,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 2,
        },
        // 12. 蓝细菌叠层石
        BiomineralStructure {
            name: "叠层石 Stromatolite".to_string(),
            mineral: Biomineral::CalciumCarbonate,
            polymorph: Some(CaCO3Polymorph::Calcite),
            organism: "蓝细菌 Cyanobacteria".to_string(),
            location: "微生物席沉积".to_string(),
            function: "沉积建造/最古老化石".to_string(),
            // 35 亿年化石, Shark Bay 现存
            // 诱导: 光合 CO2↓ → CaCO3 沉淀
            hardness_mohs: 3.0,
            density_kg_m3: 2710.0,
            youngs_modulus_gpa: 70.0,
            fracture_toughness_mpa_m05: 0.5,
            strategy: MineralizationStrategy::BiologicallyInduced,
            hierarchical_levels: 2,
        },
        // 13. 乌贼骨 Cuttlebone
        BiomineralStructure {
            name: "乌贼骨 Cuttlebone".to_string(),
            mineral: Biomineral::CalciumCarbonate,
            polymorph: Some(CaCO3Polymorph::Aragonite),
            organism: "乌贼 Sepia".to_string(),
            location: "内部壳".to_string(),
            function: "浮力调节".to_string(),
            // 文石气体腔，多孔结构
            hardness_mohs: 3.0,
            density_kg_m3: 600.0, // 多孔低密度
            youngs_modulus_gpa: 5.0,
            fracture_toughness_mpa_m05: 0.5,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 2,
        },
        // 14. 海胆刺
        BiomineralStructure {
            name: "海胆刺 Sea urchin spine".to_string(),
            mineral: Biomineral::CalciumCarbonate,
            polymorph: Some(CaCO3Polymorph::Calcite),
            organism: "海胆 Echinoidea".to_string(),
            location: "体表棘刺".to_string(),
            function: "防御/运动".to_string(),
            // 单晶方解石 + 0.1% 有机质
            // 断裂为原子级平滑面
            // "无定形前驱体" ACC → 方解石 (Weiner 2003)
            hardness_mohs: 3.5,
            density_kg_m3: 2710.0,
            youngs_modulus_gpa: 75.0,
            fracture_toughness_mpa_m05: 0.5,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 2,
        },
        // 15. 帽贝齿舌 Radula
        BiomineralStructure {
            name: "帽贝齿舌 Radula".to_string(),
            mineral: Biomineral::Goethite,
            polymorph: None,
            organism: "帽贝 Patella".to_string(),
            location: "齿舌齿尖".to_string(),
            function: "刮食岩石".to_string(),
            // 针铁矿 α-FeOOH + 几丁质
            // 齿尖 13% Fe，最硬生物结构
            // 莫氏硬度 5.5 (磁铁矿 6)
            hardness_mohs: 5.5,
            density_kg_m3: 4270.0,
            youngs_modulus_gpa: 100.0,
            fracture_toughness_mpa_m05: 1.5,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 3,
        },
        // 16. 鱼耳石 Otolith
        BiomineralStructure {
            name: "鱼耳石 Otolith".to_string(),
            mineral: Biomineral::CalciumCarbonate,
            polymorph: Some(CaCO3Polymorph::Aragonite),
            organism: "硬骨鱼 Osteichthyes".to_string(),
            location: "内耳".to_string(),
            function: "听觉/平衡/年龄鉴定".to_string(),
            // 文石日轮，每日 1 层
            hardness_mohs: 3.5,
            density_kg_m3: 2930.0,
            youngs_modulus_gpa: 70.0,
            fracture_toughness_mpa_m05: 0.5,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 2,
        },
        // 17. 草酸钙针晶 Raphide
        BiomineralStructure {
            name: "草酸钙针晶 Raphide".to_string(),
            mineral: Biomineral::CalciumOxalate,
            polymorph: None,
            organism: "天南星科 Araceae".to_string(),
            location: "叶/茎细胞".to_string(),
            function: "草食防御/钙调节".to_string(),
            // 龙舌兰、海芋毒性防御
            hardness_mohs: 2.5,
            density_kg_m3: 2120.0, // 一水草酸钙
            youngs_modulus_gpa: 30.0,
            fracture_toughness_mpa_m05: 0.3,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 1,
        },
        // 18. 植物硅体 Phytolith
        BiomineralStructure {
            name: "植物硅体 Phytolith".to_string(),
            mineral: Biomineral::Silica,
            polymorph: None,
            organism: "禾本科 Poaceae".to_string(),
            location: "叶/茎细胞壁".to_string(),
            function: "结构/防御/硅循环".to_string(),
            // 草原硅循环, 竹类累积
            hardness_mohs: 6.0,
            density_kg_m3: 2200.0,
            youngs_modulus_gpa: 30.0,
            fracture_toughness_mpa_m05: 0.5,
            strategy: MineralizationStrategy::BiologicallyControlled,
            hierarchical_levels: 1,
        },
    ]
}

// ============ 4. 矿化过程建模 ============

/// 成核类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Nucleation {
    /// 均相成核（罕见，需高过饱和度）
    Homogeneous,
    /// 异相成核（有机基质表面，最常见）
    Heterogeneous,
    /// 外延生长（晶体结构匹配）
    Epitaxial,
}

/// 有机基质组成
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganicMatrix {
    /// 酸性蛋白 Asp/Glu 富集（CARP/SOM）
    pub acidic_proteins_pct: f32,
    /// 多糖（硫酸化）
    pub polysaccharides_pct: f32,
    /// 几丁质
    pub chitin_pct: f32,
    /// 胶原蛋白
    pub collagen_pct: f32,
    /// 脂质
    pub lipid_pct: f32,
}

/// 矿化过程动力学模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineralizationProcess {
    pub mineral: Biomineral,
    /// 过饱和度 S = IAP/Ksp
    pub supersaturation: f32,
    pub nucleation_type: Nucleation,
    /// 晶体生长速率 nm/s
    pub growth_rate_nm_per_s: f32,
    pub temperature_c: f32,
    pub ph: f32,
    pub inhibitors: Vec<String>,
    pub promoters: Vec<String>,
    pub organic_matrix: Option<OrganicMatrix>,
}

impl MineralizationProcess {
    pub fn new(mineral: Biomineral) -> Self {
        // 典型生理/海水条件默认值
        let (supersaturation, growth_rate, temperature_c, ph) = match mineral {
            // 海水 CaCO3 钙化条件 (Gattuso 1999)
            Biomineral::CalciumCarbonate => (3.0, 5.0, 25.0, 8.2),
            // 生理磷酸钙, 骨沉积 1-2 μm/day ≈ 0.01-0.02 nm/s
            Biomineral::CalciumPhosphate => (2.0, 0.05, 37.0, 7.4),
            // 硅藻硅化
            Biomineral::Silica => (2.5, 1.0, 20.0, 8.0),
            Biomineral::Magnetite => (5.0, 0.1, 25.0, 7.0),
            Biomineral::Goethite => (3.0, 0.5, 25.0, 7.0),
            Biomineral::BariumSulfate => (10.0, 2.0, 25.0, 7.0),
            Biomineral::StrontiumSulfate => (8.0, 1.5, 25.0, 7.0),
            Biomineral::CalciumOxalate => (2.0, 0.5, 25.0, 6.5),
            Biomineral::Gypsum => (3.0, 2.0, 25.0, 7.0),
            Biomineral::Halite => (5.0, 10.0, 25.0, 7.0),
        };
        Self {
            mineral,
            supersaturation,
            nucleation_type: Nucleation::Heterogeneous,
            growth_rate_nm_per_s: growth_rate,
            temperature_c,
            ph,
            inhibitors: Vec::new(),
            promoters: Vec::new(),
            organic_matrix: None,
        }
    }

    /// 推进矿化一步，返回沉积质量 mg（每 cm² 截面积）
    /// 模型: 厚度增量 = growth_rate × dt，质量 = 厚度 × 密度 × 面积
    pub fn step(&mut self, dt_s: f32) -> f32 {
        let density_g_cm3 = mineral_density_g_cm3(self.mineral);
        // nm × s × 1e-7 cm/nm = 厚度增量 cm
        let thickness_cm = self.growth_rate_nm_per_s * dt_s * 1e-7;
        // 过饱和度驱动因子 (S-1)，无过饱和则无沉积
        let saturation_factor = (self.supersaturation - 1.0).max(0.0);
        // 抑制剂阻尼（每个 -20%）
        let inhibitor_factor = 1.0 / (1.0 + self.inhibitors.len() as f32 * 0.2);
        // 有机基质促进异相成核
        let promoter_factor = if self.organic_matrix.is_some() { 1.2 } else { 1.0 };
        // 单位面积质量 g → mg
        let mass_mg = thickness_cm
            * density_g_cm3
            * 1000.0
            * saturation_factor
            * inhibitor_factor
            * promoter_factor;
        // 过饱和度逐步消耗（沉积消耗离子）
        self.supersaturation =
            (self.supersaturation - saturation_factor * 1e-4 * dt_s).max(1.0);
        // 晶体随时间缓慢粗化
        self.growth_rate_nm_per_s *= 1.0 + saturation_factor * 1e-3 * dt_s;
        mass_mg.max(0.0)
    }

    /// 晶体尺寸 nm（线性生长近似）
    pub fn crystal_size(&self, t_s: f32) -> f32 {
        self.growth_rate_nm_per_s * t_s
    }

    /// 依据温度与过饱和度判定主导多型（仅 CaCO3）
    /// 高温(>30℃) → 文石; 高过饱和 → ACC 前驱; 低温中性 → 球霰石过渡; 默认方解石
    pub fn polymorph_predominant(&self) -> Option<CaCO3Polymorph> {
        if self.mineral != Biomineral::CalciumCarbonate {
            return None;
        }
        if self.supersaturation > 10.0 {
            return Some(CaCO3Polymorph::Amorphous);
        }
        if self.temperature_c > 30.0 {
            return Some(CaCO3Polymorph::Aragonite);
        }
        if self.temperature_c < 10.0 && self.ph > 8.0 {
            return Some(CaCO3Polymorph::Vaterite);
        }
        Some(CaCO3Polymorph::Calcite)
    }
}

/// 矿物密度 g/cm³（用于质量换算）
/// 来源:矿物学手册
fn mineral_density_g_cm3(mineral: Biomineral) -> f32 {
    match mineral {
        // 文石 2.93, 方解石 2.71; 折中
        Biomineral::CalciumCarbonate => 2.93,
        // 羟基磷灰石
        Biomineral::CalciumPhosphate => 3.18,
        // 蛋白石含水
        Biomineral::Silica => 2.20,
        Biomineral::Magnetite => 5.18,
        Biomineral::Goethite => 4.27,
        Biomineral::BariumSulfate => 4.49,
        Biomineral::StrontiumSulfate => 3.96,
        // 一水草酸钙
        Biomineral::CalciumOxalate => 2.12,
        Biomineral::Gypsum => 2.32,
        Biomineral::Halite => 2.17,
    }
}

// ============ 5. 矿化速度与生物控制 ============

/// 各矿物典型矿化速率 g/m²/day
/// 来源: Gattuso 1999 (珊瑚); Marshall 2003 (颗石藻); Dean 2000 (牡蛎)
pub fn mineralization_rates() -> Vec<(Biomineral, f32)> {
    vec![
        // 珊瑚: 5-20 g CaCO3/m²/day (Gattuso 1999)
        (Biomineral::CalciumCarbonate, 12.5),
        // 骨骼沉积 1-2 μm/day → 折算 ~0.5 g/m²/day
        (Biomineral::CalciumPhosphate, 0.5),
        // 硅藻 0.5-2 g/m²/day
        (Biomineral::Silica, 1.0),
        // 磁小体（小量沉积）
        (Biomineral::Magnetite, 0.01),
        // 帽贝齿舌
        (Biomineral::Goethite, 0.05),
        (Biomineral::BariumSulfate, 0.1),
        (Biomineral::StrontiumSulfate, 0.05),
        // 植物草酸钙
        (Biomineral::CalciumOxalate, 0.2),
        (Biomineral::Gypsum, 0.5),
        (Biomineral::Halite, 1.0),
    ]
}

// ============ 6. 海洋酸化对矿化影响 ============
// 来源: Orr 2005 Nature; Doney 2009 Annu Rev Mar Sci; Zeebe & Wolf-Gladrow 2001

/// 海洋酸化对钙化生物影响
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OceanAcidificationImpact {
    pub co2_ppm: f32,
    /// 工业前 8.17，预测 2100 7.7-7.9
    pub ph: f32,
    /// CO3²⁻ 浓度 μmol/kg（饱和度下降核心指标）
    pub carbonate_ion_umol_kg: f32,
    /// 文石饱和度 Ω_arag（工业前 3.3）
    pub aragonite_saturation: f32,
    /// 方解石饱和度 Ω_calc（工业前 5.0）
    pub calcite_saturation: f32,
    /// (生物, 影响系数 0-1)
    pub organism_impact: Vec<(String, f32)>,
}

impl OceanAcidificationImpact {
    /// 根据大气 CO2 浓度计算海洋酸化参数
    /// 工业前 CO2=280 ppm, pH=8.17; 2100 预测 700-1000 ppm
    pub fn new(co2_ppm: f32) -> Self {
        // 简化关系: ΔpH ≈ -0.85 × log10(CO2/280) (Zeebe & Wolf-Gladrow 2001)
        let ph = 8.17 - 0.85 * (co2_ppm / 280.0).log10();
        // Ω 与 [CO3²⁻] 正比, 工业前 CO3²⁻≈200 μmol/kg
        // CO3²⁻ ∝ 10^(pH - 8.17) (简化)
        let factor = 10.0_f32.powf(ph - 8.17);
        let carbonate_ion_umol_kg = 200.0 * factor;
        let aragonite_saturation = 3.3 * factor;
        let calcite_saturation = 5.0 * factor;
        // 影响系数: pH 越低，影响越大
        let impact = ((8.2 - ph) / 0.6).clamp(0.0, 1.0);
        let organism_impact = vec![
            ("珊瑚 Scleractinia".to_string(), impact.clamp(0.0, 1.0)),
            ("颗石藻 Coccolithophore".to_string(), (impact * 0.85).clamp(0.0, 1.0)),
            ("有孔虫 Foraminifera".to_string(), (impact * 0.7).clamp(0.0, 1.0)),
            ("牡蛎 Oyster".to_string(), (impact * 0.6).clamp(0.0, 1.0)),
            ("海胆 Sea urchin".to_string(), (impact * 0.5).clamp(0.0, 1.0)),
        ];
        Self {
            co2_ppm,
            ph,
            carbonate_ion_umol_kg,
            aragonite_saturation,
            calcite_saturation,
            organism_impact,
        }
    }

    /// 钙化速率因子 0.0-1.0
    /// 基于 Ω_arag: Ω<1 溶解; Ω>4 饱和 (Orr 2005)
    pub fn calcification_rate_factor(&self) -> f32 {
        let omega = self.aragonite_saturation;
        if omega <= 1.0 {
            return 0.0;
        }
        ((omega - 1.0) / 3.0).clamp(0.0, 1.0)
    }

    /// 文石溶解临界 pH（Ω_arag=1）
    /// 8.17 - log10(3.3) ≈ 7.65
    pub fn dissolution_threshold_ph(&self) -> f32 {
        8.17 - 3.3_f32.log10()
    }
}

// ============ 7. 辅助查询方法 ============

impl Biomineral {
    /// 是否为含钙矿物
    pub fn is_calcium_based(&self) -> bool {
        matches!(
            self,
            Biomineral::CalciumCarbonate | Biomineral::CalciumPhosphate | Biomineral::CalciumOxalate
        )
    }

    /// 典型莫氏硬度（矿物学手册）
    pub fn typical_hardness_mohs(&self) -> f32 {
        match self {
            Biomineral::CalciumCarbonate => 3.5,
            Biomineral::CalciumPhosphate => 5.0,
            Biomineral::Silica => 6.0,
            Biomineral::Magnetite => 6.0,
            Biomineral::Goethite => 5.5,
            Biomineral::BariumSulfate => 3.5,
            Biomineral::StrontiumSulfate => 3.5,
            Biomineral::CalciumOxalate => 2.5,
            Biomineral::Gypsum => 2.0,
            Biomineral::Halite => 2.5,
        }
    }

    /// 化学式
    pub fn chemical_formula(&self) -> &'static str {
        match self {
            Biomineral::CalciumCarbonate => "CaCO3",
            Biomineral::CalciumPhosphate => "Ca10(PO4)6(OH)2",
            Biomineral::Silica => "SiO2·nH2O",
            Biomineral::Magnetite => "Fe3O4",
            Biomineral::Goethite => "α-FeOOH",
            Biomineral::BariumSulfate => "BaSO4",
            Biomineral::StrontiumSulfate => "SrSO4",
            Biomineral::CalciumOxalate => "CaC2O4·H2O",
            Biomineral::Gypsum => "CaSO4·2H2O",
            Biomineral::Halite => "NaCl",
        }
    }

    /// 晶系
    pub fn crystal_system(&self) -> &'static str {
        match self {
            Biomineral::CalciumCarbonate => "trigonal/orthorhombic",
            Biomineral::CalciumPhosphate => "hexagonal",
            Biomineral::Silica => "amorphous",
            Biomineral::Magnetite => "isometric",
            Biomineral::Goethite => "orthorhombic",
            Biomineral::BariumSulfate => "orthorhombic",
            Biomineral::StrontiumSulfate => "orthorhombic",
            Biomineral::CalciumOxalate => "monoclinic",
            Biomineral::Gypsum => "monoclinic",
            Biomineral::Halite => "isometric",
        }
    }
}

impl CaCO3Polymorph {
    /// 稳定性排序：0=ACC, 1=球霰石, 2=文石, 3=方解石
    pub fn stability_rank(&self) -> u8 {
        match self {
            CaCO3Polymorph::Amorphous => 0,
            CaCO3Polymorph::Vaterite => 1,
            CaCO3Polymorph::Aragonite => 2,
            CaCO3Polymorph::Calcite => 3,
        }
    }

    /// 是否为稳定相（文石或方解石）
    pub fn is_stable(&self) -> bool {
        self.stability_rank() >= 2
    }

    /// 典型莫氏硬度
    pub fn typical_hardness_mohs(&self) -> f32 {
        match self {
            CaCO3Polymorph::Calcite => 3.0,
            CaCO3Polymorph::Aragonite => 4.0,
            CaCO3Polymorph::Vaterite => 2.0,
            CaCO3Polymorph::Amorphous => 2.0,
        }
    }
}

impl BiomineralStructure {
    /// 矿物类型
    pub fn mineral_type(&self) -> Biomineral {
        self.mineral
    }

    /// 莫氏硬度（getter 封装）
    pub fn hardness_mohs(&self) -> f32 {
        self.hardness_mohs
    }

    /// 硬度分类
    pub fn hardness_class(&self) -> &'static str {
        if self.hardness_mohs < 3.0 {
            "soft"
        } else if self.hardness_mohs < 5.0 {
            "medium"
        } else {
            "hard"
        }
    }

    /// 是否为钙化结构（CaCO3 或 磷酸钙）
    pub fn is_calcified(&self) -> bool {
        matches!(
            self.mineral,
            Biomineral::CalciumCarbonate | Biomineral::CalciumPhosphate
        )
    }

    /// 是否含钙
    pub fn is_calcium_based(&self) -> bool {
        self.mineral.is_calcium_based()
    }

    /// 力学品质指数 = 断裂韧性 × 硬度
    pub fn mechanical_quality(&self) -> f32 {
        self.fracture_toughness_mpa_m05 * self.hardness_mohs
    }

    /// 典型矿化速率 g/m²/day（查表）
    pub fn calcification_rate(&self) -> f32 {
        for (mineral, rate) in mineralization_rates() {
            if mineral == self.mineral {
                return rate;
            }
        }
        0.0
    }

    /// 饱和指数 SI（基于矿物典型海水/生理条件，近似值）
    pub fn saturation_index(&self) -> f32 {
        match self.mineral {
            Biomineral::CalciumCarbonate => 0.52,
            Biomineral::CalciumPhosphate => 0.30,
            Biomineral::Silica => 0.40,
            Biomineral::Magnetite => 0.70,
            _ => 0.50,
        }
    }
}

impl MineralizationProcess {
    /// 矿物类型
    pub fn mineral_type(&self) -> Biomineral {
        self.mineral
    }

    /// 饱和指数 SI = log10(S) = log10(IAP/Ksp)
    pub fn saturation_index(&self) -> f32 {
        if self.supersaturation > 0.0 {
            self.supersaturation.log10()
        } else {
            0.0
        }
    }

    /// 是否过饱和（SI > 0，趋向沉淀）
    pub fn is_supersaturated(&self) -> bool {
        self.supersaturation > 1.0
    }

    /// 是否未饱和（SI < 0，趋向溶解）
    pub fn is_undersaturated(&self) -> bool {
        self.supersaturation < 1.0
    }

    /// 矿化速率 g/m²/day（查表）
    pub fn calcification_rate(&self) -> f32 {
        for (mineral, rate) in mineralization_rates() {
            if mineral == self.mineral {
                return rate;
            }
        }
        0.0
    }

    /// 生长效率 0.0-1.0：基于过饱和度
    pub fn growth_efficiency(&self) -> f32 {
        if self.supersaturation <= 0.0 {
            return 0.0;
        }
        (1.0 - 1.0 / self.supersaturation).clamp(0.0, 1.0)
    }
}

impl OceanAcidificationImpact {
    /// 涉及的矿物类型（钙化生物主要为 CaCO3）
    pub fn mineral_type(&self) -> Biomineral {
        Biomineral::CalciumCarbonate
    }

    /// 文石饱和指数 SI = log10(Ω_arag)
    pub fn saturation_index(&self) -> f32 {
        if self.aragonite_saturation > 0.0 {
            self.aragonite_saturation.log10()
        } else {
            0.0
        }
    }

    /// 是否腐蚀性（Ω_arag < 1，文石溶解）
    pub fn is_corrosive(&self) -> bool {
        self.aragonite_saturation < 1.0
    }

    /// 钙化速率因子 0.0-1.0
    pub fn calcification_rate(&self) -> f32 {
        self.calcification_rate_factor()
    }

    /// 酸化严重度
    pub fn acidification_severity(&self) -> &'static str {
        if self.ph >= 8.0 {
            "minimal"
        } else if self.ph >= 7.8 {
            "moderate"
        } else if self.ph >= 7.65 {
            "severe"
        } else {
            "extreme"
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biomineral_database_count() {
        let db = biomineral_database();
        assert_eq!(db.len(), 18, "should have 18 biomineral structures");
    }

    #[test]
    fn test_biomineral_is_calcium_based() {
        assert!(Biomineral::CalciumCarbonate.is_calcium_based());
        assert!(Biomineral::CalciumPhosphate.is_calcium_based());
        assert!(Biomineral::CalciumOxalate.is_calcium_based());
        assert!(!Biomineral::Silica.is_calcium_based());
        assert!(!Biomineral::Magnetite.is_calcium_based());
        assert!(!Biomineral::Halite.is_calcium_based());
    }

    #[test]
    fn test_biomineral_chemical_formula() {
        assert_eq!(Biomineral::CalciumCarbonate.chemical_formula(), "CaCO3");
        assert_eq!(Biomineral::Magnetite.chemical_formula(), "Fe3O4");
        assert_eq!(Biomineral::Silica.chemical_formula(), "SiO2·nH2O");
        assert_eq!(Biomineral::Halite.chemical_formula(), "NaCl");
    }

    #[test]
    fn test_biomineral_typical_hardness() {
        assert!((Biomineral::CalciumPhosphate.typical_hardness_mohs() - 5.0).abs() < 1e-3);
        assert!(Biomineral::Silica.typical_hardness_mohs() > Biomineral::Gypsum.typical_hardness_mohs());
        assert!((Biomineral::Gypsum.typical_hardness_mohs() - 2.0).abs() < 1e-3);
    }

    #[test]
    fn test_biomineral_crystal_system() {
        assert_eq!(Biomineral::Magnetite.crystal_system(), "isometric");
        assert_eq!(Biomineral::Silica.crystal_system(), "amorphous");
        assert_eq!(Biomineral::CalciumPhosphate.crystal_system(), "hexagonal");
    }

    #[test]
    fn test_caco3_polymorph_stability_rank() {
        assert!(CaCO3Polymorph::Calcite.stability_rank() > CaCO3Polymorph::Aragonite.stability_rank());
        assert!(CaCO3Polymorph::Aragonite.stability_rank() > CaCO3Polymorph::Vaterite.stability_rank());
        assert!(CaCO3Polymorph::Vaterite.stability_rank() > CaCO3Polymorph::Amorphous.stability_rank());
    }

    #[test]
    fn test_caco3_polymorph_is_stable() {
        assert!(CaCO3Polymorph::Calcite.is_stable());
        assert!(CaCO3Polymorph::Aragonite.is_stable());
        assert!(!CaCO3Polymorph::Vaterite.is_stable());
        assert!(!CaCO3Polymorph::Amorphous.is_stable());
    }

    #[test]
    fn test_caco3_polymorph_hardness() {
        assert!((CaCO3Polymorph::Calcite.typical_hardness_mohs() - 3.0).abs() < 1e-3);
        assert!(CaCO3Polymorph::Aragonite.typical_hardness_mohs() > CaCO3Polymorph::Calcite.typical_hardness_mohs());
    }

    #[test]
    fn test_biomineral_structure_hardness_class() {
        let db = biomineral_database();
        let nacre = db.iter().find(|s| s.name.contains("Nacre")).unwrap();
        assert_eq!(nacre.hardness_class(), "medium");
        let enamel = db.iter().find(|s| s.name.contains("Enamel")).unwrap();
        assert_eq!(enamel.hardness_class(), "hard");
        let cartilage = db.iter().find(|s| s.name.contains("cartilage")).unwrap();
        assert_eq!(cartilage.hardness_class(), "soft");
    }

    #[test]
    fn test_biomineral_structure_is_calcified() {
        let db = biomineral_database();
        let nacre = db.iter().find(|s| s.name.contains("Nacre")).unwrap();
        assert!(nacre.is_calcified());
        let bone = db.iter().find(|s| s.name.contains("bone")).unwrap();
        assert!(bone.is_calcified());
        let magnetosome = db.iter().find(|s| s.name.contains("Magnetosome")).unwrap();
        assert!(!magnetosome.is_calcified());
    }

    #[test]
    fn test_biomineral_structure_calcification_rate() {
        let db = biomineral_database();
        let nacre = db.iter().find(|s| s.name.contains("Nacre")).unwrap();
        assert!((nacre.calcification_rate() - 12.5).abs() < 1e-3, "CaCO3 rate {}", nacre.calcification_rate());
        let bone = db.iter().find(|s| s.name.contains("bone")).unwrap();
        assert!((bone.calcification_rate() - 0.5).abs() < 1e-3, "CaPhosphate rate {}", bone.calcification_rate());
    }

    #[test]
    fn test_biomineral_structure_mechanical_quality() {
        let db = biomineral_database();
        let bone = db.iter().find(|s| s.name.contains("bone")).unwrap();
        let mq = bone.mechanical_quality();
        let expected = bone.fracture_toughness_mpa_m05 * bone.hardness_mohs;
        assert!((mq - expected).abs() < 1e-3);
        assert!(mq > 0.0);
    }

    #[test]
    fn test_biomineral_structure_mineral_type() {
        let db = biomineral_database();
        let nacre = db.iter().find(|s| s.name.contains("Nacre")).unwrap();
        assert_eq!(nacre.mineral_type(), Biomineral::CalciumCarbonate);
        assert!(nacre.is_calcium_based());
    }
    #[test]
    fn test_mineralization_process_new_caco3() {
        let p = MineralizationProcess::new(Biomineral::CalciumCarbonate);
        assert_eq!(p.mineral_type(), Biomineral::CalciumCarbonate);
        assert!((p.supersaturation - 3.0).abs() < 1e-3);
        assert!((p.temperature_c - 25.0).abs() < 1e-3);
        assert!((p.ph - 8.2).abs() < 1e-3);
        assert!(p.is_supersaturated());
        assert!(!p.is_undersaturated());
    }

    #[test]
    fn test_mineralization_saturation_index() {
        let p = MineralizationProcess::new(Biomineral::CalciumCarbonate);
        let si = p.saturation_index();
        assert!((si - 3.0_f32.log10()).abs() < 1e-3, "SI {}", si);
        assert!(si > 0.0, "supersaturated should have positive SI");
    }

    #[test]
    fn test_mineralization_growth_efficiency() {
        let p = MineralizationProcess::new(Biomineral::CalciumCarbonate);
        let eff = p.growth_efficiency();
        assert!((eff - (1.0 - 1.0 / 3.0)).abs() < 1e-3, "efficiency {}", eff);
        assert!(eff > 0.0 && eff < 1.0);
    }

    #[test]
    fn test_mineralization_polymorph_predominant() {
        let mut p = MineralizationProcess::new(Biomineral::CalciumCarbonate);
        assert_eq!(p.polymorph_predominant(), Some(CaCO3Polymorph::Calcite));
        p.temperature_c = 35.0;
        assert_eq!(p.polymorph_predominant(), Some(CaCO3Polymorph::Aragonite));
        p.supersaturation = 15.0;
        assert_eq!(p.polymorph_predominant(), Some(CaCO3Polymorph::Amorphous));
    }

    #[test]
    fn test_mineralization_polymorph_non_caco3() {
        let p = MineralizationProcess::new(Biomineral::Silica);
        assert_eq!(p.polymorph_predominant(), None);
    }

    #[test]
    fn test_mineralization_step_positive() {
        let mut p = MineralizationProcess::new(Biomineral::CalciumCarbonate);
        let mass = p.step(1.0);
        assert!(mass > 0.0, "step should deposit positive mass, got {}", mass);
        assert!(p.supersaturation < 3.0, "supersaturation should decrease");
    }

    #[test]
    fn test_mineralization_step_undersaturated_zero() {
        let mut p = MineralizationProcess::new(Biomineral::CalciumCarbonate);
        p.supersaturation = 0.5;
        let mass = p.step(1.0);
        assert_eq!(mass, 0.0, "undersaturated should deposit nothing");
    }

    #[test]
    fn test_mineralization_rates_nonempty() {
        let rates = mineralization_rates();
        assert_eq!(rates.len(), 10);
        let caco3_rate = rates.iter().find(|(m, _)| *m == Biomineral::CalciumCarbonate).unwrap();
        assert!((caco3_rate.1 - 12.5).abs() < 1e-3);
    }

    #[test]
    fn test_ocean_acidification_pre_industrial() {
        let oai = OceanAcidificationImpact::new(280.0);
        assert!((oai.ph - 8.17).abs() < 1e-3, "pre-industrial pH {}", oai.ph);
        assert!((oai.aragonite_saturation - 3.3).abs() < 1e-2);
        assert!(!oai.is_corrosive());
        assert_eq!(oai.acidification_severity(), "minimal");
        assert!(oai.calcification_rate() > 0.7);
        assert_eq!(oai.mineral_type(), Biomineral::CalciumCarbonate);
    }

    #[test]
    fn test_ocean_acidification_high_co2() {
        let oai = OceanAcidificationImpact::new(1000.0);
        assert!(oai.ph < 8.0 && oai.ph > 7.5, "pH {}", oai.ph);
        assert!(oai.aragonite_saturation < 2.0);
        assert!(!oai.is_corrosive());
        assert_eq!(oai.acidification_severity(), "severe");
        assert!(oai.calcification_rate() < 0.1, "calcification should be low, got {}", oai.calcification_rate());
    }

    #[test]
    fn test_ocean_acidification_corrosive() {
        let oai = OceanAcidificationImpact::new(2000.0);
        assert!(oai.ph < 7.65, "pH {}", oai.ph);
        assert!(oai.is_corrosive(), "should be corrosive at 2000 ppm");
        assert_eq!(oai.acidification_severity(), "extreme");
        assert_eq!(oai.calcification_rate(), 0.0);
        assert!(oai.saturation_index() < 0.0, "corrosive SI should be negative, got {}", oai.saturation_index());
    }

    #[test]
    fn test_ocean_acidification_dissolution_threshold() {
        let oai = OceanAcidificationImpact::new(280.0);
        let threshold = oai.dissolution_threshold_ph();
        assert!((threshold - 7.65).abs() < 0.05, "threshold pH {}", threshold);
    }
}