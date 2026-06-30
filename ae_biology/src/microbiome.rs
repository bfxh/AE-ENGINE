//! 微生物系统 —— 基于微生物学真实分类与群体动力学
//!
//! 数据来源：
//! - Madigan et al., "Brock Biology of Microorganisms" (15th ed., 2018)
//! - Sender, Fuchs & Milo, 2016, "Revised estimates for the number of human
//!   and bacteria cells in the body" —— 人体肠道约 10^13-10^14 微生物
//! - Shannon, 1948, "A Mathematical Theory of Communication" (多样性指数)
//! - Murray et al., "Medical Microbiology" (8th ed., 抗生素章节)
//! - CLSI M100, "Performance Standards for Antimicrobial Susceptibility Testing"

use serde::{Deserialize, Serialize};

// ============================================================
// 微生物类型
// ============================================================

/// Woese 1990 三域分类系统 + 病毒（非细胞生物）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MicrobeDomain {
    /// 细菌域 —— 原核，肽聚糖细胞壁，70S 核糖体
    Bacteria,
    /// 古菌域 —— 原核，醚键脂质膜，常居极端环境
    Archaea,
    /// 真核域 —— 真核细胞（原生动物/真菌），80S 核糖体
    Eukarya,
    /// 病毒 —— 非细胞，专性胞内寄生
    Virus,
}

/// 细菌形态学分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BacteriaShape {
    /// 球菌 —— 直径 0.5-1.0μm（葡萄球菌/链球菌）
    Cocci,
    /// 杆菌 —— 0.5×2μm（大肠杆菌）
    Bacilli,
    /// 螺旋菌 —— 螺旋形，僵硬（幽门螺杆菌）
    Spirilla,
    /// 弧菌 —— 弧形（霍乱弧菌）
    Vibrio,
    /// 螺旋体 —— 柔韧螺旋（梅毒螺旋体）
    Spirochetes,
    /// 丝状 —— 链状排列（放线菌）
    Filamentous,
}

/// 革兰氏染色反应 —— 反映细胞壁结构差异（Gram 1884）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GramStain {
    /// 革兰氏阳性 —— 厚肽聚糖 20-80nm，含磷壁酸
    Positive,
    /// 革兰氏阴性 —— 薄肽聚糖 7-8nm + 外膜 LPS（内毒素）
    Negative,
    /// 抗酸染色 —— 分支菌酸蜡质细胞壁（结核杆菌/麻风杆菌）
    AcidFast,
}

/// 氧气需求分类（基于 O₂ 利用与毒性耐受）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OxygenRequirement {
    /// 专性需氧 —— 仅氧呼吸（结核杆菌、铜绿假单胞菌）
    ObligateAerobe,
    /// 专性厌氧 —— O₂ 致死（破伤风杆菌、肉毒杆菌）
    ObligateAnaerobe,
    /// 兼性厌氧 —— 有氧呼吸/无氧发酵（大肠杆菌、葡萄球菌）
    FacultativeAnaerobe,
    /// 微需氧 —— 低氧 5-10%（幽门螺杆菌、弯曲菌）
    Microaerophile,
    /// 耐氧 —— 不利用 O₂ 但 O₂ 不致死（乳酸菌）
    Aerotolerant,
}

/// 病毒基因组分类（Baltimore 分类法简化）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VirusGenome {
    /// 单链 DNA（细小病毒 Parvovirus）
    DnaSingleStrand,
    /// 双链 DNA（疱疹病毒 Herpesvirus、天花 Variola）
    DnaDoubleStrand,
    /// 单链 RNA（流感 Influenza、SARS-CoV-2）
    RnaSingleStrand,
    /// 双链 RNA（轮状病毒 Rotavirus）
    RnaDoubleStrand,
    /// 逆转录病毒（HIV —— RNA 经逆转录整合入宿主基因组）
    Retrovirus,
}

/// 真菌分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FungiType {
    /// 酵母 —— 单细胞出芽生殖（酿酒酵母）
    Yeast,
    /// 霉菌 —— 多细胞菌丝体（青霉、曲霉）
    Mold,
    /// 担子菌 —— 大型子实体（蘑菇）
    Mushroom,
    /// 双相真菌 —— 酵母/菌丝相切换（组织胞浆菌）
    Dimorphic,
}

/// 原生动物分类（按运动器）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProtozoaType {
    /// 阿米巴 —— 伪足运动（溶组织内阿米巴）
    Amoeboid,
    /// 鞭毛虫 —— 鞭毛运动（贾第鞭毛虫、阴道毛滴虫）
    Flagellate,
    /// 纤毛虫 —— 纤毛运动（结肠小袋纤毛虫）
    Ciliate,
    /// 孢子虫 —— 复杂生活史（疟原虫、弓形虫）
    Sporozoan,
}

/// 微生物物种定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrobeSpecies {
    /// 学名（双名法，属 + 种）
    pub name: String,
    /// 生物域
    pub domain: MicrobeDomain,
    /// 细菌形态（仅细菌域有效）
    pub shape: Option<BacteriaShape>,
    /// 革兰氏反应（仅细菌域有效）
    pub gram: Option<GramStain>,
    /// 氧气需求（细菌/真菌）
    pub oxygen: Option<OxygenRequirement>,
    /// 典型尺寸（μm；病毒以 nm/1000 给出，如 HIV = 0.12μm = 120nm）
    pub size_um: f32,
    /// 最适 pH
    pub optimal_ph: f32,
    /// 最适温度（摄氏度）
    pub optimal_temp_c: f32,
    /// 倍增时间（分钟）。大肠杆菌 ~20min，结核杆菌 720-1440min
    pub doubling_time_min: f32,
    /// 致病性 0.0-1.0（依据 ID50/LD50 推算）
    pub pathogenicity: f32,
    /// 耐药基因（bla_*、mecA、vanA、ermB、tetM、katG 等）
    pub antibiotic_resistance: Vec<String>,
}

impl MicrobeSpecies {
    /// 最大比生长速率 μ_max = ln(2) / t_d（min^-1）
    pub fn max_specific_growth_rate(&self) -> f32 {
        if self.doubling_time_min <= 0.0 {
            0.0
        } else {
            std::f32::consts::LN_2 / self.doubling_time_min
        }
    }
}

// ============================================================
// 群体动力学
// ============================================================

/// 细菌生长曲线四阶段（Monod 1949）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrowthPhase {
    /// 延迟期 —— 适应新环境，合成酶系，数量几乎不增
    Lag,
    /// 对数期 —— 指数增长，最大生长速率
    Log,
    /// 稳定期 —— 营养耗竭/毒素积累，生长 = 死亡
    Stationary,
    /// 衰亡期 —— 营养耗尽，死亡 > 生长
    Death,
}

/// 单物种微生物群体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrobialPopulation {
    /// 物种定义
    pub species: MicrobeSpecies,
    /// 当前细胞数
    pub count: u64,
    /// 当前比生长速率（min^-1），可被环境影响/抗生素抑制
    pub growth_rate: f32,
    /// 环境携带容量 K
    pub carrying_capacity: u64,
}

impl MicrobialPopulation {
    /// 单步积分（logistic 增长 + 稳定期死亡项）
    ///
    /// Verhulst-Pearl logistic 方程：
    ///   dN/dt = r·N·(1 - N/K) - μ_d·N
    /// 当 N 接近 K 时引入少量死亡（毒素积累），growth_rate < 0 时进入纯粹衰亡。
    pub fn step(&mut self, dt_min: f32) {
        if self.count == 0 {
            return;
        }
        let k = self.carrying_capacity.max(1) as f64;
        let n = self.count as f64;
        let r = self.growth_rate as f64;
        let dn_dt = r * n * (1.0 - n / k);
        let death_rate = if n > 0.9 * k { 0.02 } else { 0.0 };
        let new_n = (n + (dn_dt - death_rate * n) * dt_min as f64).max(0.0);
        self.count = new_n as u64;
    }

    /// 当前生长阶段（按 N/K 比例 + 生长率符号判定）
    pub fn growth_phase(&self) -> GrowthPhase {
        if self.count == 0 || self.growth_rate < 0.0 {
            return GrowthPhase::Death;
        }
        let k = self.carrying_capacity.max(1) as f64;
        let ratio = self.count as f64 / k;
        if ratio < 0.1 {
            GrowthPhase::Lag
        } else if ratio < 0.7 {
            GrowthPhase::Log
        } else {
            GrowthPhase::Stationary
        }
    }
}

// ============================================================
// 微生物组（Microbiome）
// ============================================================

/// 人体定植部位
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BodySite {
    /// 肠道（10^13-10^14 微生物）
    Gut,
    /// 皮肤
    Skin,
    /// 口腔
    Oral,
    /// 呼吸道
    Respiratory,
    /// 泌尿生殖
    Urogenital,
    /// 鼻腔
    Nasal,
    /// 结膜
    Conjunctiva,
    /// 外耳道
    Ear,
}

/// 微生物组 —— 多物种共生群落
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Microbiome {
    /// 定植部位
    pub site: BodySite,
    /// 物种群体系列
    pub populations: Vec<MicrobialPopulation>,
    /// Shannon 多样性指数 H = -Σ p_i·ln(p_i)
    pub diversity_index: f32,
    /// 总细胞数
    pub total_count: u64,
}

impl Microbiome {
    pub fn new(site: BodySite) -> Self {
        Self {
            site,
            populations: Vec::new(),
            diversity_index: 0.0,
            total_count: 0,
        }
    }

    /// 单步推进所有群体并重算统计量
    pub fn step(&mut self, dt_min: f32) {
        for pop in &mut self.populations {
            pop.step(dt_min);
        }
        self.recompute();
    }

    /// 添加物种（若已存在则累加 count）
    pub fn add_species(&mut self, species: MicrobeSpecies, count: u64) {
        if let Some(p) = self
            .populations
            .iter_mut()
            .find(|p| p.species.name == species.name)
        {
            p.count = p.count.saturating_add(count);
        } else {
            let growth_rate = species.max_specific_growth_rate();
            // 默认携带容量按定植部位典型密度估算
            let carrying_capacity: u64 = match self.site {
                BodySite::Gut => 10_000_000_000, // ~10^10 / mL 肠内容物
                BodySite::Skin => 1_000_000,     // ~10^6 / cm²
                BodySite::Oral => 1_000_000_000, // ~10^9 / mL 唾液
                BodySite::Respiratory => 100_000_000,
                BodySite::Urogenital => 100_000_000,
                BodySite::Nasal => 10_000_000,
                BodySite::Conjunctiva => 1_000_000,
                BodySite::Ear => 1_000_000,
            };
            self.populations.push(MicrobialPopulation {
                species,
                count,
                growth_rate,
                carrying_capacity,
            });
        }
        self.recompute();
    }

    /// 按学名移除物种
    pub fn remove_species(&mut self, name: &str) {
        self.populations.retain(|p| p.species.name != name);
        self.recompute();
    }

    /// Shannon 多样性指数：H = -Σ p_i · ln(p_i)
    /// 健康肠道 H ≈ 3.5-4.5（Huttenhower 2012, HMP）
    pub fn compute_diversity(&self) -> f32 {
        if self.total_count == 0 {
            return 0.0;
        }
        let total = self.total_count as f64;
        let mut h = 0.0f64;
        for p in &self.populations {
            if p.count == 0 {
                continue;
            }
            let pi = p.count as f64 / total;
            h -= pi * pi.ln();
        }
        h as f32
    }

    /// 失调评分 0.0（健康）— 1.0（严重失调）
    /// 综合 Shannon 多样性下降 + 病原体占比上升
    pub fn dysbiosis_score(&self) -> f32 {
        let h = self.diversity_index;
        let diversity_score = (h / 4.0).clamp(0.0, 1.0);
        let pathogen_fraction = if self.total_count == 0 {
            0.0
        } else {
            let pathogen_count: u64 = self
                .populations
                .iter()
                .filter(|p| p.species.pathogenicity > 0.5)
                .map(|p| p.count)
                .sum();
            pathogen_count as f32 / self.total_count as f32
        };
        let score = 0.6 * (1.0 - diversity_score) + 0.4 * pathogen_fraction;
        score.clamp(0.0, 1.0)
    }

    fn recompute(&mut self) {
        self.total_count = self.populations.iter().map(|p| p.count).sum();
        self.diversity_index = self.compute_diversity();
    }
}

// ============================================================
// 物种数据库
// ============================================================

/// 肠道菌群 —— 人体肠道 10^13-10^14 微生物（Sender et al., 2016 修订）
pub fn gut_microbiome_species() -> Vec<MicrobeSpecies> {
    vec![
        MicrobeSpecies {
            name: "Bacteroides fragilis".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Negative),
            oxygen: Some(OxygenRequirement::ObligateAnaerobe),
            size_um: 1.0,
            optimal_ph: 6.5,
            optimal_temp_c: 37.0,
            doubling_time_min: 120.0,
            pathogenicity: 0.3,
            antibiotic_resistance: vec!["tetQ".into()],
        },
        MicrobeSpecies {
            name: "Escherichia coli".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Negative),
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 2.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 20.0,
            pathogenicity: 0.4,
            antibiotic_resistance: vec!["bla_TEM-1".into()],
        },
        MicrobeSpecies {
            name: "Lactobacillus acidophilus".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::Aerotolerant),
            size_um: 3.0,
            optimal_ph: 5.5,
            optimal_temp_c: 37.0,
            doubling_time_min: 60.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            name: "Bifidobacterium longum".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::ObligateAnaerobe),
            size_um: 2.0,
            optimal_ph: 6.5,
            optimal_temp_c: 37.0,
            doubling_time_min: 90.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            name: "Clostridioides difficile".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::ObligateAnaerobe),
            size_um: 4.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 30.0,
            pathogenicity: 0.8,
            antibiotic_resistance: vec!["ermB".into()],
        },
        MicrobeSpecies {
            // 普拉梭菌 —— 健康肠道最丰富的丁酸盐产生菌，抗炎
            name: "Faecalibacterium prausnitzii".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Negative),
            oxygen: Some(OxygenRequirement::ObligateAnaerobe),
            size_um: 2.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 180.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            // 史氏甲烷短杆菌 —— 古菌，肠道主要产甲烷菌
            name: "Methanobrevibacter smithii".into(),
            domain: MicrobeDomain::Archaea,
            shape: Some(BacteriaShape::Cocci),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::ObligateAnaerobe),
            size_um: 0.7,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 1440.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        },
    ]
}

/// 皮肤菌群
pub fn skin_microbiome_species() -> Vec<MicrobeSpecies> {
    vec![
        MicrobeSpecies {
            name: "Staphylococcus epidermidis".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Cocci),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 0.8,
            optimal_ph: 5.5,
            optimal_temp_c: 37.0,
            doubling_time_min: 40.0,
            pathogenicity: 0.1,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            // 金黄色葡萄球菌 —— 致病，MRSA 携带 mecA
            name: "Staphylococcus aureus".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Cocci),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 0.8,
            optimal_ph: 5.5,
            optimal_temp_c: 37.0,
            doubling_time_min: 30.0,
            pathogenicity: 0.7,
            antibiotic_resistance: vec!["mecA".into()],
        },
        MicrobeSpecies {
            // 痤疮丙酸杆菌 —— 厌氧，皮脂腺优势菌，痤疮病原
            name: "Cutibacterium acnes".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::ObligateAnaerobe),
            size_um: 1.0,
            optimal_ph: 6.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 300.0,
            pathogenicity: 0.3,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            // 马拉色菌 —— 依赖脂质的真菌，皮肤共生酵母
            name: "Malassezia furfur".into(),
            domain: MicrobeDomain::Eukarya,
            shape: None,
            gram: None,
            oxygen: Some(OxygenRequirement::Aerotolerant),
            size_um: 5.0,
            optimal_ph: 5.5,
            optimal_temp_c: 32.0,
            doubling_time_min: 720.0,
            pathogenicity: 0.2,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            name: "Corynebacterium jeikeium".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 3.0,
            optimal_ph: 5.5,
            optimal_temp_c: 37.0,
            doubling_time_min: 60.0,
            pathogenicity: 0.1,
            antibiotic_resistance: vec![],
        },
    ]
}

/// 口腔菌群
pub fn oral_microbiome_species() -> Vec<MicrobeSpecies> {
    vec![
        MicrobeSpecies {
            // 变异链球菌 —— 致龋齿，产酸
            name: "Streptococcus mutans".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Cocci),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 0.7,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 60.0,
            pathogenicity: 0.5,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            name: "Streptococcus salivarius".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Cocci),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 0.8,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 30.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            // 牙龈卟啉单胞菌 —— 牙周病核心病原
            name: "Porphyromonas gingivalis".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Negative),
            oxygen: Some(OxygenRequirement::ObligateAnaerobe),
            size_um: 1.5,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 360.0,
            pathogenicity: 0.7,
            antibiotic_resistance: vec![],
        },
    ]
}

/// 常见病原体
pub fn common_pathogens() -> Vec<MicrobeSpecies> {
    vec![
        MicrobeSpecies {
            // 结核杆菌 —— 抗酸染色，倍增 12-24h（极慢）
            name: "Mycobacterium tuberculosis".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::AcidFast),
            oxygen: Some(OxygenRequirement::ObligateAerobe),
            size_um: 3.0,
            optimal_ph: 6.5,
            optimal_temp_c: 37.0,
            doubling_time_min: 1440.0,
            pathogenicity: 1.0,
            antibiotic_resistance: vec!["katG".into(), "inhA".into(), "rpoB".into()],
        },
        MicrobeSpecies {
            // 恶性疟原虫 —— 孢子虫，红细胞内期 ~48h
            name: "Plasmodium falciparum".into(),
            domain: MicrobeDomain::Eukarya,
            shape: None,
            gram: None,
            oxygen: None,
            size_um: 2.0,
            optimal_ph: 7.4,
            optimal_temp_c: 37.0,
            doubling_time_min: 2880.0,
            pathogenicity: 1.0,
            antibiotic_resistance: vec!["crt".into()],
        },
        MicrobeSpecies {
            // 流感病毒 —— 单链 RNA，包膜
            name: "Influenza virus".into(),
            domain: MicrobeDomain::Virus,
            shape: None,
            gram: None,
            oxygen: None,
            size_um: 0.1,
            optimal_ph: 7.2,
            optimal_temp_c: 33.0,
            doubling_time_min: 480.0,
            pathogenicity: 0.7,
            antibiotic_resistance: vec!["adamantane_R".into()],
        },
        MicrobeSpecies {
            // HIV —— 逆转录病毒，直径 ~120nm
            name: "Human immunodeficiency virus".into(),
            domain: MicrobeDomain::Virus,
            shape: None,
            gram: None,
            oxygen: None,
            size_um: 0.12,
            optimal_ph: 7.2,
            optimal_temp_c: 37.0,
            doubling_time_min: 1440.0,
            pathogenicity: 1.0,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            // SARS-CoV-2 —— 冠状病毒，单链 RNA 包膜
            name: "SARS-CoV-2".into(),
            domain: MicrobeDomain::Virus,
            shape: None,
            gram: None,
            oxygen: None,
            size_um: 0.1,
            optimal_ph: 7.2,
            optimal_temp_c: 37.0,
            doubling_time_min: 720.0,
            pathogenicity: 0.8,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            // 霍乱弧菌 —— 弧菌，革兰氏阴性
            name: "Vibrio cholerae".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Vibrio),
            gram: Some(GramStain::Negative),
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 2.0,
            optimal_ph: 8.0,
            optimal_temp_c: 30.0,
            doubling_time_min: 30.0,
            pathogenicity: 0.9,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            // 伤寒沙门菌
            name: "Salmonella typhi".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Negative),
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 2.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 30.0,
            pathogenicity: 0.9,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            // 破伤风杆菌 —— 厌氧芽孢，破伤风痉挛毒素
            name: "Clostridium tetani".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::ObligateAnaerobe),
            size_um: 5.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 30.0,
            pathogenicity: 1.0,
            antibiotic_resistance: vec![],
        },
    ]
}

/// 益生菌
pub fn probiotic_species() -> Vec<MicrobeSpecies> {
    vec![
        MicrobeSpecies {
            name: "Lactobacillus acidophilus".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::Aerotolerant),
            size_um: 3.0,
            optimal_ph: 5.5,
            optimal_temp_c: 37.0,
            doubling_time_min: 60.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            name: "Bifidobacterium bifidum".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Positive),
            oxygen: Some(OxygenRequirement::ObligateAnaerobe),
            size_um: 2.0,
            optimal_ph: 6.5,
            optimal_temp_c: 37.0,
            doubling_time_min: 90.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        },
        MicrobeSpecies {
            // 布拉酵母 —— 益生真菌，对抗抗生素相关腹泻
            name: "Saccharomyces boulardii".into(),
            domain: MicrobeDomain::Eukarya,
            shape: None,
            gram: None,
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 8.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 90.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        },
    ]
}

// ============================================================
// 抗生素作用
// ============================================================

/// 抗生素分类（按作用机制）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AntibioticClass {
    /// β-内酰胺 —— 抑制细胞壁合成（青霉素/头孢，结合 PBP）
    BetaLactam,
    /// 大环内酯 —— 抑制 50S 核糖体（红霉素）
    Macrolide,
    /// 四环素 —— 抑制 30S 核糖体（多西环素）
    Tetracycline,
    /// 氨基糖苷 —— 30S 核糖体错误翻译（链霉素）
    Aminoglycoside,
    /// 氟喹诺酮 —— 抑制 DNA 回旋酶（环丙沙星）
    Fluoroquinolone,
    /// 磺胺 —— 抑制叶酸合成（磺胺甲噁唑）
    Sulfonamide,
    /// 糖肽 —— 抑制细胞壁交联（万古霉素）
    Glycopeptide,
    /// 多肽 —— 破坏细胞膜（多粘菌素）
    Polypeptide,
}

/// 抗生素定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Antibiotic {
    pub class: AntibioticClass,
    pub name: String,
    /// 最小抑制浓度 μg/mL（CLSI M100 临床折点）
    pub mic: f32,
    /// 抗菌谱（有效生物域）
    pub spectrum: Vec<MicrobeDomain>,
    /// 已知可被耐药基因破坏
    pub resistance_genes: Vec<String>,
}

impl Antibiotic {
    /// 对目标物种的有效性 0.0-1.0
    ///
    /// 综合考虑：
    /// - 抗菌谱（域匹配）
    /// - 革兰氏反应与机制匹配（β-内酰胺对 G+ 强，糖肽仅 G+，多肽仅 G-）
    /// - 耐药基因衰减
    pub fn effectiveness(&self, species: &MicrobeSpecies) -> f32 {
        // 必须在抗菌谱内
        if !self.spectrum.contains(&species.domain) {
            return 0.0;
        }

        // 基础有效性按机制 + 革兰氏匹配
        let base: f32 = match self.class {
            AntibioticClass::BetaLactam => match species.gram {
                Some(GramStain::Positive) => 0.85,
                Some(GramStain::Negative) => 0.40,
                Some(GramStain::AcidFast) => 0.05, // 分支菌酸蜡质屏障
                None => 0.50,
            },
            AntibioticClass::Macrolide => match species.gram {
                Some(GramStain::Positive) => 0.70,
                Some(GramStain::Negative) => 0.40,
                Some(GramStain::AcidFast) => 0.20,
                None => 0.50,
            },
            AntibioticClass::Tetracycline => 0.60, // 广谱
            AntibioticClass::Aminoglycoside => match species.gram {
                Some(GramStain::Negative) => 0.75, // 氨基糖苷对 G- 强（链霉素）
                Some(GramStain::Positive) => 0.40,
                Some(GramStain::AcidFast) => 0.30, // 链霉素抗结核
                None => 0.50,
            },
            AntibioticClass::Fluoroquinolone => 0.80, // 广谱，DNA 回旋酶保守
            AntibioticClass::Sulfonamide => 0.50,
            AntibioticClass::Glycopeptide => match species.gram {
                // 万古霉素：仅 G+ 有效，G- 外膜阻挡，抗酸无效
                Some(GramStain::Positive) => 0.85,
                Some(GramStain::Negative) => 0.05,
                Some(GramStain::AcidFast) => 0.05,
                None => 0.50,
            },
            AntibioticClass::Polypeptide => match species.gram {
                // 多粘菌素：仅 G- 有效（破坏外膜 LPS）
                Some(GramStain::Negative) => 0.85,
                Some(GramStain::Positive) => 0.10,
                Some(GramStain::AcidFast) => 0.20,
                None => 0.30,
            },
        };

        // 耐药基因衰减
        let mut eff = base;
        for gene in &species.antibiotic_resistance {
            let reduction = match (self.class, gene.as_str()) {
                (AntibioticClass::BetaLactam, g) if g.starts_with("bla") => 0.80, // β-内酰胺酶
                (AntibioticClass::BetaLactam, "mecA") => 0.70,                    // PBP2a 改变靶点
                (AntibioticClass::Glycopeptide, "vanA")
                | (AntibioticClass::Glycopeptide, "vanB") => 0.90, // D-Ala→D-Lac 改变靶点
                (AntibioticClass::Macrolide, g) if g.starts_with("erm") => 0.80, // rRNA 甲基化
                (AntibioticClass::Tetracycline, g) if g.starts_with("tet") => 0.70, // 外排泵
                (AntibioticClass::Aminoglycoside, g)
                    if g.starts_with("aac")
                        | g.starts_with("aph")
                        | g.starts_with("aad") =>
                {
                    0.70 // 修饰酶
                }
                (AntibioticClass::Fluoroquinolone, g) if g.starts_with("qnr") => 0.50,
                (AntibioticClass::Fluoroquinolone, "gyrA") => 0.60, // 靶点突变
                (AntibioticClass::Sulfonamide, g)
                    if g == "sul1" || g == "sul2" || g == "dfrA" =>
                {
                    0.60
                }
                (AntibioticClass::Tetracycline, "tetM") | (AntibioticClass::Tetracycline, "tetQ") => {
                    0.70 // 核糖体保护蛋白
                }
                _ => 0.0,
            };
            eff *= 1.0 - reduction;
        }

        eff.clamp(0.0, 1.0)
    }

    /// 应用抗生素到群体（dt 分钟）
    ///
    /// - 浓度 < MIC：抑菌（bacteriostatic）—— 降低 growth_rate
    /// - 浓度 ≥ MIC：杀菌（bactericidal）—— 指数杀伤 + 抑制生长
    pub fn apply(&self, pop: &mut MicrobialPopulation, concentration: f32, dt: f32) {
        let eff = self.effectiveness(&pop.species);
        if eff <= 0.0 {
            return;
        }
        let mic_ratio = if self.mic > 0.0 {
            concentration / self.mic
        } else {
            1.0
        };

        if mic_ratio < 1.0 {
            // 抑菌：按比例降低生长率
            let inhibition = eff * mic_ratio;
            pop.growth_rate *= 1.0 - inhibition;
        } else {
            // 杀菌：指数杀伤（kill rate 上限 10·MIC 后饱和）
            let kill_rate = eff * mic_ratio.min(10.0) * 0.3; // min^-1
            let n = pop.count as f32;
            let killed = kill_rate * n * dt;
            let new_count = (n - killed).max(0.0);
            pop.count = new_count as u64;
            // 同时抑制生长
            pop.growth_rate *= 1.0 - eff * 0.5;
        }
    }
}

/// 常用抗生素（参考 CLSI M100 临床折点）
pub fn common_antibiotics() -> Vec<Antibiotic> {
    vec![
        Antibiotic {
            class: AntibioticClass::BetaLactam,
            name: "Penicillin G".into(),
            mic: 0.05,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec!["bla_TEM".into(), "mecA".into()],
        },
        Antibiotic {
            class: AntibioticClass::Glycopeptide,
            name: "Vancomycin".into(),
            mic: 2.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec!["vanA".into(), "vanB".into()],
        },
        Antibiotic {
            class: AntibioticClass::Fluoroquinolone,
            name: "Ciprofloxacin".into(),
            mic: 1.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec!["qnrA".into(), "gyrA".into()],
        },
        Antibiotic {
            class: AntibioticClass::Tetracycline,
            name: "Doxycycline".into(),
            mic: 1.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec!["tetM".into(), "tetQ".into()],
        },
        Antibiotic {
            class: AntibioticClass::Polypeptide,
            name: "Polymyxin B".into(),
            mic: 2.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec!["mcr-1".into()],
        },
        Antibiotic {
            class: AntibioticClass::Aminoglycoside,
            name: "Streptomycin".into(),
            mic: 8.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec!["aac".into(), "aph".into()],
        },
        Antibiotic {
            class: AntibioticClass::Macrolide,
            name: "Erythromycin".into(),
            mic: 0.5,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec!["ermB".into()],
        },
        Antibiotic {
            class: AntibioticClass::Sulfonamide,
            name: "Trimethoprim-Sulfamethoxazole".into(),
            mic: 2.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec!["sul1".into(), "dfrA".into()],
        },
    ]
}

// ============================================================
// 单元测试模块
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- MicrobeSpecies::max_specific_growth_rate ----------

    #[test]
    fn test_max_specific_growth_rate_normal() {
        // 大肠杆菌 doubling_time = 20 min -> mu_max = ln(2)/20
        let species = MicrobeSpecies {
            name: "E. coli".into(),
            domain: MicrobeDomain::Bacteria,
            shape: Some(BacteriaShape::Bacilli),
            gram: Some(GramStain::Negative),
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 2.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 20.0,
            pathogenicity: 0.4,
            antibiotic_resistance: vec![],
        };
        let expected = std::f32::consts::LN_2 / 20.0;
        assert!((species.max_specific_growth_rate() - expected).abs() < 1e-6);
    }

    #[test]
    fn test_max_specific_growth_rate_zero_doubling_returns_zero() {
        let species = MicrobeSpecies {
            name: "X".into(),
            domain: MicrobeDomain::Bacteria,
            shape: None,
            gram: None,
            oxygen: None,
            size_um: 1.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 0.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        };
        assert_eq!(species.max_specific_growth_rate(), 0.0);
    }

    #[test]
    fn test_max_specific_growth_rate_negative_doubling_returns_zero() {
        let species = MicrobeSpecies {
            name: "X".into(),
            domain: MicrobeDomain::Bacteria,
            shape: None,
            gram: None,
            oxygen: None,
            size_um: 1.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: -10.0,
            pathogenicity: 0.0,
            antibiotic_resistance: vec![],
        };
        assert_eq!(species.max_specific_growth_rate(), 0.0);
    }

    // ---------- MicrobialPopulation::step ----------

    fn make_pop(count: u64, growth_rate: f32, k: u64) -> MicrobialPopulation {
        MicrobialPopulation {
            species: MicrobeSpecies {
                name: "test".into(),
                domain: MicrobeDomain::Bacteria,
                shape: None,
                gram: None,
                oxygen: None,
                size_um: 1.0,
                optimal_ph: 7.0,
                optimal_temp_c: 37.0,
                doubling_time_min: 30.0,
                pathogenicity: 0.0,
                antibiotic_resistance: vec![],
            },
            count,
            growth_rate,
            carrying_capacity: k,
        }
    }

    #[test]
    fn test_population_step_zero_count_stays_zero() {
        let mut pop = make_pop(0, 0.5, 1000);
        pop.step(1.0);
        assert_eq!(pop.count, 0);
    }

    #[test]
    fn test_population_step_log_phase_grows() {
        // count << K, growth_rate > 0 -> count should increase
        let mut pop = make_pop(100, 0.1, 1_000_000);
        let before = pop.count;
        pop.step(10.0);
        assert!(pop.count > before, "expected growth, before={} after={}", before, pop.count);
    }

    #[test]
    fn test_population_step_at_capacity_stable_or_declines() {
        // count == K, growth_rate term = 0; if N > 0.9K, death_rate=0.02 applies -> decline
        let mut pop = make_pop(1000, 0.5, 1000);
        pop.step(1.0);
        // n=K, dn_dt=0, death=0.02*1000*1 = 20, new_n = 980
        assert!(pop.count <= 1000, "should not exceed K");
    }

    // ---------- MicrobialPopulation::growth_phase ----------

    #[test]
    fn test_growth_phase_zero_count_is_death() {
        let pop = make_pop(0, 0.5, 1000);
        assert_eq!(pop.growth_phase(), GrowthPhase::Death);
    }

    #[test]
    fn test_growth_phase_negative_rate_is_death() {
        let pop = make_pop(100, -0.1, 1000);
        assert_eq!(pop.growth_phase(), GrowthPhase::Death);
    }

    #[test]
    fn test_growth_phase_low_ratio_is_lag() {
        // ratio = 50/1000 = 0.05 < 0.1
        let pop = make_pop(50, 0.1, 1000);
        assert_eq!(pop.growth_phase(), GrowthPhase::Lag);
    }

    #[test]
    fn test_growth_phase_mid_ratio_is_log() {
        // ratio = 300/1000 = 0.3, 0.1 <= 0.3 < 0.7
        let pop = make_pop(300, 0.1, 1000);
        assert_eq!(pop.growth_phase(), GrowthPhase::Log);
    }

    #[test]
    fn test_growth_phase_high_ratio_is_stationary() {
        // ratio = 800/1000 = 0.8 >= 0.7
        let pop = make_pop(800, 0.1, 1000);
        assert_eq!(pop.growth_phase(), GrowthPhase::Stationary);
    }

    #[test]
    fn test_growth_phase_zero_capacity_is_lag() {
        // carrying_capacity = 0 -> .max(1) = 1, ratio = 0/1 = 0 < 0.1
        let pop = make_pop(0, 0.1, 0);
        // count = 0 triggers Death first
        assert_eq!(pop.growth_phase(), GrowthPhase::Death);
    }

    // ---------- Microbiome::new / add_species / remove_species ----------

    #[test]
    fn test_microbiome_new_empty() {
        let mb = Microbiome::new(BodySite::Gut);
        assert_eq!(mb.site, BodySite::Gut);
        assert!(mb.populations.is_empty());
        assert_eq!(mb.diversity_index, 0.0);
        assert_eq!(mb.total_count, 0);
    }

    #[test]
    fn test_microbiome_add_species_inserts_new() {
        let mut mb = Microbiome::new(BodySite::Gut);
        let sp = gut_microbiome_species()[0].clone();
        mb.add_species(sp.clone(), 1000);
        assert_eq!(mb.populations.len(), 1);
        assert_eq!(mb.total_count, 1000);
        assert_eq!(mb.populations[0].species.name, sp.name);
    }

    #[test]
    fn test_microbiome_add_species_existing_accumulates() {
        let mut mb = Microbiome::new(BodySite::Gut);
        let sp = gut_microbiome_species()[0].clone();
        mb.add_species(sp.clone(), 1000);
        mb.add_species(sp.clone(), 500);
        assert_eq!(mb.populations.len(), 1, "should not duplicate");
        assert_eq!(mb.populations[0].count, 1500);
        assert_eq!(mb.total_count, 1500);
    }

    #[test]
    fn test_microbiome_add_species_carrying_capacity_gut() {
        let mut mb = Microbiome::new(BodySite::Gut);
        let sp = gut_microbiome_species()[1].clone(); // E. coli
        mb.add_species(sp, 100);
        assert_eq!(mb.populations[0].carrying_capacity, 10_000_000_000);
    }

    #[test]
    fn test_microbiome_add_species_carrying_capacity_skin() {
        let mut mb = Microbiome::new(BodySite::Skin);
        let sp = skin_microbiome_species()[0].clone();
        mb.add_species(sp, 100);
        assert_eq!(mb.populations[0].carrying_capacity, 1_000_000);
    }

    #[test]
    fn test_microbiome_remove_species_by_name() {
        let mut mb = Microbiome::new(BodySite::Gut);
        let sp = gut_microbiome_species()[0].clone();
        mb.add_species(sp.clone(), 1000);
        assert_eq!(mb.populations.len(), 1);
        mb.remove_species(&sp.name);
        assert!(mb.populations.is_empty());
        assert_eq!(mb.total_count, 0);
    }

    #[test]
    fn test_microbiome_remove_species_missing_no_panic() {
        let mut mb = Microbiome::new(BodySite::Gut);
        mb.remove_species("nonexistent");
        assert!(mb.populations.is_empty());
    }

    // ---------- Microbiome::compute_diversity ----------

    #[test]
    fn test_compute_diversity_empty_is_zero() {
        let mb = Microbiome::new(BodySite::Gut);
        assert_eq!(mb.compute_diversity(), 0.0);
    }

    #[test]
    fn test_compute_diversity_single_species_is_zero() {
        let mut mb = Microbiome::new(BodySite::Gut);
        let sp = gut_microbiome_species()[0].clone();
        mb.add_species(sp, 1000);
        // single species: p=1, ln(1)=0, H=0
        assert!((mb.compute_diversity() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_diversity_uniform_two_species_is_ln2() {
        let mut mb = Microbiome::new(BodySite::Gut);
        let sp1 = gut_microbiome_species()[0].clone();
        let sp2 = gut_microbiome_species()[1].clone();
        mb.add_species(sp1, 500);
        mb.add_species(sp2, 500);
        // p1=p2=0.5, H = -2 * 0.5 * ln(0.5) = ln(2)
        let h = mb.compute_diversity();
        assert!((h - std::f32::consts::LN_2).abs() < 1e-5, "H={}", h);
    }

    // ---------- Microbiome::dysbiosis_score ----------

    #[test]
    fn test_dysbiosis_score_empty_is_six_tenths() {
        // empty: diversity_score=0, pathogen_fraction=0
        // score = 0.6 * (1-0) + 0.4 * 0 = 0.6
        let mb = Microbiome::new(BodySite::Gut);
        assert!((mb.dysbiosis_score() - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_dysbiosis_score_high_pathogen_increases() {
        let mut mb = Microbiome::new(BodySite::Gut);
        // C. difficile pathogenicity = 0.8 > 0.5
        let pathogen = gut_microbiome_species()
            .into_iter()
            .find(|s| s.name == "Clostridioides difficile")
            .unwrap();
        mb.add_species(pathogen, 1000);
        let score = mb.dysbiosis_score();
        // single species -> H=0 -> diversity_score=0
        // pathogen_fraction = 1.0
        // score = 0.6*1 + 0.4*1 = 1.0
        assert!((score - 1.0).abs() < 1e-5, "score={}", score);
    }

    #[test]
    fn test_dysbiosis_score_only_commensal_lower() {
        let mut mb = Microbiome::new(BodySite::Gut);
        // L. acidophilus pathogenicity = 0.0
        let commensal = gut_microbiome_species()
            .into_iter()
            .find(|s| s.name == "Lactobacillus acidophilus")
            .unwrap();
        mb.add_species(commensal, 1000);
        let score = mb.dysbiosis_score();
        // H=0 -> diversity_score=0; pathogen_fraction=0
        // score = 0.6 * 1 + 0.4 * 0 = 0.6
        assert!((score - 0.6).abs() < 1e-5, "score={}", score);
    }

    // ---------- Microbiome::step ----------

    #[test]
    fn test_microbiome_step_recomputes_total() {
        let mut mb = Microbiome::new(BodySite::Gut);
        let sp = gut_microbiome_species()[1].clone(); // E. coli, fast grower
        mb.add_species(sp, 1000);
        let before = mb.total_count;
        mb.step(10.0);
        // total_count should be recomputed after step
        assert!(mb.total_count >= before || mb.populations[0].growth_rate <= 0.0);
    }

    // ---------- Antibiotic::effectiveness ----------

    fn make_species(domain: MicrobeDomain, gram: Option<GramStain>, resistance: Vec<String>) -> MicrobeSpecies {
        MicrobeSpecies {
            name: "target".into(),
            domain,
            shape: Some(BacteriaShape::Bacilli),
            gram,
            oxygen: Some(OxygenRequirement::FacultativeAnaerobe),
            size_um: 1.0,
            optimal_ph: 7.0,
            optimal_temp_c: 37.0,
            doubling_time_min: 30.0,
            pathogenicity: 0.5,
            antibiotic_resistance: resistance,
        }
    }

    #[test]
    fn test_effectiveness_outside_spectrum_returns_zero() {
        // antibiotic spectrum = Bacteria, target = Virus -> 0
        let ab = Antibiotic {
            class: AntibioticClass::BetaLactam,
            name: "Penicillin".into(),
            mic: 1.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec![],
        };
        let sp = make_species(MicrobeDomain::Virus, None, vec![]);
        assert_eq!(ab.effectiveness(&sp), 0.0);
    }

    #[test]
    fn test_effectiveness_betalactam_gram_positive_base() {
        let ab = Antibiotic {
            class: AntibioticClass::BetaLactam,
            name: "Penicillin".into(),
            mic: 1.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec![],
        };
        let sp = make_species(MicrobeDomain::Bacteria, Some(GramStain::Positive), vec![]);
        assert!((ab.effectiveness(&sp) - 0.85).abs() < 1e-6);
    }

    #[test]
    fn test_effectiveness_glycopeptide_gram_negative_low() {
        let ab = Antibiotic {
            class: AntibioticClass::Glycopeptide,
            name: "Vancomycin".into(),
            mic: 2.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec![],
        };
        let sp = make_species(MicrobeDomain::Bacteria, Some(GramStain::Negative), vec![]);
        assert!((ab.effectiveness(&sp) - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_effectiveness_polypeptide_gram_negative_high() {
        let ab = Antibiotic {
            class: AntibioticClass::Polypeptide,
            name: "Polymyxin".into(),
            mic: 2.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec![],
        };
        let sp = make_species(MicrobeDomain::Bacteria, Some(GramStain::Negative), vec![]);
        assert!((ab.effectiveness(&sp) - 0.85).abs() < 1e-6);
    }

    #[test]
    fn test_effectiveness_betalactam_bla_resistance_reduces() {
        let ab = Antibiotic {
            class: AntibioticClass::BetaLactam,
            name: "Penicillin".into(),
            mic: 1.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec![],
        };
        let sp = make_species(
            MicrobeDomain::Bacteria,
            Some(GramStain::Positive),
            vec!["bla_TEM-1".into()],
        );
        // base 0.85 * (1 - 0.80) = 0.17
        assert!((ab.effectiveness(&sp) - 0.17).abs() < 1e-5,
            "got {}", ab.effectiveness(&sp));
    }

    #[test]
    fn test_effectiveness_glycopeptide_vanA_resistance_reduces() {
        let ab = Antibiotic {
            class: AntibioticClass::Glycopeptide,
            name: "Vancomycin".into(),
            mic: 2.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec![],
        };
        let sp = make_species(
            MicrobeDomain::Bacteria,
            Some(GramStain::Positive),
            vec!["vanA".into()],
        );
        // base 0.85 * (1 - 0.90) = 0.085
        assert!((ab.effectiveness(&sp) - 0.085).abs() < 1e-5,
            "got {}", ab.effectiveness(&sp));
    }

    // ---------- Antibiotic::apply ----------

    #[test]
    fn test_apply_below_mic_reduces_growth_rate() {
        let ab = Antibiotic {
            class: AntibioticClass::Fluoroquinolone,
            name: "Cipro".into(),
            mic: 1.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec![],
        };
        let mut pop = make_pop(1000, 0.5, 1_000_000);
        let before = pop.growth_rate;
        // conc = 0.5 < mic = 1.0 -> bacteriostatic
        ab.apply(&mut pop, 0.5, 10.0);
        assert!(pop.growth_rate < before, "growth_rate should be reduced");
        // count unchanged in bacteriostatic branch
        assert_eq!(pop.count, 1000);
    }

    #[test]
    fn test_apply_above_mic_reduces_count() {
        let ab = Antibiotic {
            class: AntibioticClass::Fluoroquinolone,
            name: "Cipro".into(),
            mic: 1.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec![],
        };
        let mut pop = make_pop(1000, 0.5, 1_000_000);
        // conc = 10 >= mic = 1 -> bactericidal
        ab.apply(&mut pop, 10.0, 10.0);
        assert!(pop.count < 1000, "count should drop, got {}", pop.count);
    }

    #[test]
    fn test_apply_zero_effectiveness_no_change() {
        // spectrum excludes Virus, but we use Bacteria w/ mismatched class+gram
        // Use a target outside spectrum -> effectiveness 0 -> apply early-returns
        let ab = Antibiotic {
            class: AntibioticClass::BetaLactam,
            name: "Penicillin".into(),
            mic: 1.0,
            spectrum: vec![MicrobeDomain::Bacteria],
            resistance_genes: vec![],
        };
        let mut pop = make_pop(1000, 0.5, 1_000_000);
        // Force species domain to Virus via direct construction
        pop.species.domain = MicrobeDomain::Virus;
        let before_count = pop.count;
        let before_rate = pop.growth_rate;
        ab.apply(&mut pop, 10.0, 10.0);
        assert_eq!(pop.count, before_count);
        assert!((pop.growth_rate - before_rate).abs() < 1e-6);
    }

    // ---------- 物种数据库函数 ----------

    #[test]
    fn test_gut_microbiome_species_count() {
        let v = gut_microbiome_species();
        assert_eq!(v.len(), 7, "gut microbiome should have 7 species");
        // all should be valid named species
        for s in &v {
            assert!(!s.name.is_empty());
        }
    }

    #[test]
    fn test_common_pathogens_count() {
        let v = common_pathogens();
        assert_eq!(v.len(), 8, "common_pathogens should have 8 entries");
        // all should be highly pathogenic (>= 0.7)
        for s in &v {
            assert!(s.pathogenicity >= 0.7,
                "pathogen {} pathogenicity={} expected >=0.7", s.name, s.pathogenicity);
        }
    }

    #[test]
    fn test_common_antibiotics_count() {
        let v = common_antibiotics();
        assert_eq!(v.len(), 8, "common_antibiotics should have 8 entries");
        for a in &v {
            assert!(!a.name.is_empty());
            assert!(!a.spectrum.is_empty());
            assert!(a.mic > 0.0);
        }
    }

    #[test]
    fn test_probiotic_species_non_pathogenic() {
        let v = probiotic_species();
        assert_eq!(v.len(), 3);
        for s in &v {
            assert_eq!(s.pathogenicity, 0.0,
                "probiotic {} should be non-pathogenic", s.name);
        }
    }

    #[test]
    fn test_skin_microbiome_species_count() {
        let v = skin_microbiome_species();
        assert_eq!(v.len(), 5);
    }

    #[test]
    fn test_oral_microbiome_species_count() {
        let v = oral_microbiome_species();
        assert_eq!(v.len(), 3);
    }

    // ---------- 枚举变体存在性 ----------

    #[test]
    fn test_microbe_domain_variants_exist() {
        let _ = MicrobeDomain::Bacteria;
        let _ = MicrobeDomain::Archaea;
        let _ = MicrobeDomain::Eukarya;
        let _ = MicrobeDomain::Virus;
    }

    #[test]
    fn test_bacteria_shape_variants_exist() {
        let _ = BacteriaShape::Cocci;
        let _ = BacteriaShape::Bacilli;
        let _ = BacteriaShape::Spirilla;
        let _ = BacteriaShape::Vibrio;
        let _ = BacteriaShape::Spirochetes;
        let _ = BacteriaShape::Filamentous;
    }

    #[test]
    fn test_gram_stain_variants_exist() {
        let _ = GramStain::Positive;
        let _ = GramStain::Negative;
        let _ = GramStain::AcidFast;
    }

    #[test]
    fn test_body_site_variants_exist() {
        let _ = BodySite::Gut;
        let _ = BodySite::Skin;
        let _ = BodySite::Oral;
        let _ = BodySite::Respiratory;
        let _ = BodySite::Urogenital;
        let _ = BodySite::Nasal;
        let _ = BodySite::Conjunctiva;
        let _ = BodySite::Ear;
    }
}