//! 科幻生物模拟系统 sci_fic_biology
//!
//! 涵盖六大科幻/前沿生物系统：
//! 1. 辐射变异系统 RadiationMutagenesis —— LQ 模型、DSB/SSB、ARS
//! 2. CRISPR 基因改造 CrisprEdit —— Cas9 切割、NHEJ/HDR/Base/Prime、脱靶、基因驱动
//! 3. 外星生物学 AlienBiology —— 硅基/氨基/甲烷基、非水溶剂、放射合成
//! 4. 合成生物学 SyntheticBiology —— 基因回路、生物制造、最小基因组
//! 5. 共生体系统 Symbiosis —— 互利/偏利/寄生、HGT、群体感应
//! 6. 赛博格系统 Cybernetics —— BCI、感官增强、神经可塑性
//!
//! 物理常数来源标注于各模块注释中。

use serde::{Deserialize, Serialize};

// ========================================================
// 公共 trait：所有科幻生物子系统统一接口
// ========================================================

/// 科幻生物子系统统一接口（对象安全）
pub trait SciBioSubsystem {
    /// 子系统名称
    fn subsystem_name(&self) -> &'static str;
    /// 当前是否激活
    fn is_active(&self) -> bool;
    /// 单步演化（dt 秒）
    fn step(&mut self, dt: f32);
}

// ========================================================
// 1. 辐射变异系统 RadiationMutagenesis
// ========================================================
// 参考：
// - ICRP 103 (2007): 人类典型 α=0.3/Gy, β=0.1/Gy²（低 LET）
// - Hall & Giaccia, "Radiobiology for the Radiologist"
// - CDC Acute Radiation Syndrome (ARS) 分期

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RadiationType {
    /// α 粒子，高 LET (~100 keV/μm)，短程 (<100 μm)
    Alpha,
    /// β 粒子，低 LET (<1 keV/μm)，长程 (mm-cm)
    Beta,
    /// γ 射线，穿透力强，低 LET
    Gamma,
    /// 中子，活化物质，高 RBE
    Neutron,
}

impl RadiationType {
    /// 线性能量传递 (keV/μm)，典型值
    pub fn let_kev_per_um(&self) -> f32 {
        match self {
            RadiationType::Alpha => 100.0,
            RadiationType::Beta => 0.2,
            RadiationType::Gamma => 0.3,
            RadiationType::Neutron => 50.0,
        }
    }
    /// 相对生物效应 RBE（参考 γ=1）
    pub fn rbe(&self) -> f32 {
        match self {
            RadiationType::Alpha => 20.0,
            RadiationType::Beta => 1.0,
            RadiationType::Gamma => 1.0,
            RadiationType::Neutron => 10.0,
        }
    }
}

/// DNA 损伤类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DnaDamageType {
    /// 双链断裂 —— 最危险，难修复
    Dsb,
    /// 单链断裂
    Ssb,
    /// 碱基损伤（氧化、烷基化）
    BaseDamage,
    /// 链间交联
    InterstrandCrosslink,
}

/// DNA 损伤计数
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct DnaDamage {
    pub dsb: u32,
    pub ssb: u32,
    pub base_damage: u32,
    pub crosslink: u32,
}

impl DnaDamage {
    pub fn total(&self) -> u32 {
        self.dsb + self.ssb + self.base_damage + self.crosslink
    }
    /// 修复难度（DSB 权重最大）
    pub fn repair_difficulty(&self) -> f32 {
        (self.dsb as f32) * 1.0
            + (self.ssb as f32) * 0.1
            + (self.base_damage as f32) * 0.05
            + (self.crosslink as f32) * 0.8
    }
}

/// 辐射剂量学
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RadiationDose {
    /// 吸收剂量 (Gy)
    pub dose_gy: f32,
    /// 辐射类型
    pub radiation: RadiationType,
    /// 暴露时长 (s)
    pub duration_s: f32,
}

impl RadiationDose {
    /// 当量剂量 H = D × Q (Sv)，Q = RBE
    pub fn equivalent_dose_sv(&self) -> f32 {
        self.dose_gy * self.radiation.rbe()
    }
}
/// LQ 模型效应 E = α·D + β·D²
/// 人类典型值 α=0.3/Gy, β=0.1/Gy² (ICRP 103, 低 LET)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LinearQuadraticModel {
    pub alpha_per_gy: f32,
    pub beta_per_gy2: f32,
}

impl Default for LinearQuadraticModel {
    fn default() -> Self {
        Self { alpha_per_gy: 0.3, beta_per_gy2: 0.1 }
    }
}

impl LinearQuadraticModel {
    pub fn effect(&self, dose_gy: f32) -> f32 {
        self.alpha_per_gy * dose_gy + self.beta_per_gy2 * dose_gy * dose_gy
    }
    /// α/β 比（区分早反应/晚反应组织，人类早反应 ~3 Gy）
    pub fn alpha_beta_ratio(&self) -> f32 {
        self.alpha_per_gy / self.beta_per_gy2
    }
}

/// 急性辐射综合征 ARS 分期
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArsStage {
    /// 前驱期（0-2 天，1-2 Gy 起）
    Prodromal,
    /// 潜伏期（2 天 - 3 周）
    Latent,
    /// 临床期（>3 周）
    Manifest,
    /// 恢复或死亡
    RecoveryOrDeath,
}

/// ARS 综合征类型（按剂量阈值）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArsSyndrome {
    /// 骨髓综合征（>1 Gy，1-6 Gy 可恢复，>6 Gy 多致死）
    Hematopoietic,
    /// 胃肠综合征（>6 Gy，几乎全部致死）
    Gastrointestinal,
    /// 神经血管综合征（>20 Gy，必死）
    Neurovascular,
}

impl ArsSyndrome {
    /// 根据吸收剂量判定主导综合征
    pub fn from_dose(dose_gy: f32) -> Option<Self> {
        if dose_gy >= 20.0 {
            Some(ArsSyndrome::Neurovascular)
        } else if dose_gy >= 6.0 {
            Some(ArsSyndrome::Gastrointestinal)
        } else if dose_gy >= 1.0 {
            Some(ArsSyndrome::Hematopoietic)
        } else {
            None
        }
    }
    /// LD50/60（60 天内 50% 死亡剂量，人类 ~4.5 Gy 无治疗）
    pub fn ld50_human_gy() -> f32 {
        4.5
    }
}

/// 突变表型库
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MutationPhenotype {
    SkinPigmentation,
    BodySizeChange,
    EnhancedSense,
    MetabolicShift,
    LimbMalformation,
    TumorGrowth,
    Sterility,
    ResistanceGain,
}

/// 辐射变异系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiationMutagenesis {
    /// LQ 模型参数
    pub lq: LinearQuadraticModel,
    /// 暴露累计剂量 (Gy)
    pub cumulative_dose_gy: f32,
    /// DNA 损伤累积
    pub damage: DnaDamage,
    /// 累计突变数
    pub mutation_count: u32,
    /// 已显现表型
    pub phenotypes: Vec<MutationPhenotype>,
    /// 当前 ARS 期
    pub ars_stage: ArsStage,
    /// ARS 起病后经过时间（天）
    pub ars_days: f32,
    /// 暴露后总时长（天）
    pub elapsed_days: f32,
}

impl Default for RadiationMutagenesis {
    fn default() -> Self {
        Self::new()
    }
}

impl RadiationMutagenesis {
    pub fn new() -> Self {
        Self {
            lq: LinearQuadraticModel::default(),
            cumulative_dose_gy: 0.0,
            damage: DnaDamage::default(),
            mutation_count: 0,
            phenotypes: Vec::new(),
            ars_stage: ArsStage::Prodromal,
            ars_days: 0.0,
            elapsed_days: 0.0,
        }
    }

    /// 一次急性暴露：返回产生的 DNA 损伤
    /// 单位剂量 DSB 产额 ~30/Gy/cell（人类典型）
    pub fn expose(&mut self, dose: RadiationDose) -> DnaDamage {
        self.cumulative_dose_gy += dose.dose_gy;
        let rbe = dose.radiation.rbe();
        let eff_dose = dose.dose_gy * rbe;
        // 各类损伤产额（每细胞每 Gy）：DSB ~30, SSB ~1000, Base ~2000
        // 高 LET (alpha/neutron) 复杂损伤比例升高
        let let_factor = (dose.radiation.let_kev_per_um() / 0.3).min(50.0);
        let dsb = (30.0 * eff_dose * (1.0 + 0.05 * let_factor)) as u32;
        let ssb = (1000.0 * eff_dose) as u32;
        let base_damage = (2000.0 * eff_dose) as u32;
        let crosslink = (5.0 * eff_dose * (1.0 + 0.1 * let_factor)) as u32;
        let dmg = DnaDamage { dsb, ssb, base_damage, crosslink };
        self.damage.dsb += dsb;
        self.damage.ssb += ssb;
        self.damage.base_damage += base_damage;
        self.damage.crosslink += crosslink;
        dmg
    }

    /// 突变率：每碱基每 Gy ~1e-7
    /// 基因组 3e9 bp，返回期望突变数
    pub fn expected_mutations(&self, genome_size_bp: u64) -> f32 {
        let mu_per_bp_per_gy = 1e-7_f32;
        mu_per_bp_per_gy * (genome_size_bp as f32) * self.cumulative_dose_gy
    }

    /// LQ 模型细胞存活分数 SF = exp(-E)
    pub fn survival_fraction(&self) -> f32 {
        let e = self.lq.effect(self.cumulative_dose_gy);
        (-e).exp()
    }

    /// 抽取一个表型（基于简单哈希权重，确定性）
    pub fn roll_phenotype(&self, seed: u64) -> MutationPhenotype {
        let table = [
            (MutationPhenotype::SkinPigmentation, 30_u32),
            (MutationPhenotype::BodySizeChange, 15),
            (MutationPhenotype::EnhancedSense, 5),
            (MutationPhenotype::MetabolicShift, 10),
            (MutationPhenotype::LimbMalformation, 10),
            (MutationPhenotype::TumorGrowth, 15),
            (MutationPhenotype::Sterility, 5),
            (MutationPhenotype::ResistanceGain, 3),
        ];
        let total: u32 = table.iter().map(|(_, w)| *w).sum();
        let mut x = (seed.wrapping_mul(2654435761) >> 32) % (total as u64);
        for (ph, w) in table.iter() {
            if x < *w as u64 {
                return *ph;
            }
            x -= *w as u64;
        }
        table[0].0
    }

    /// ARS 推进
    pub fn update_ars(&mut self, dt_days: f32) {
        self.ars_days += dt_days;
        self.elapsed_days += dt_days;
        let dose = self.cumulative_dose_gy;
        if dose < 1.0 {
            self.ars_stage = ArsStage::RecoveryOrDeath;
            return;
        }
        // 三段分期：0-2 天前驱；2 天-3 周潜伏；>3 周临床
        if self.ars_days < 2.0 {
            self.ars_stage = ArsStage::Prodromal;
        } else if self.ars_days < 21.0 {
            self.ars_stage = ArsStage::Latent;
        } else {
            self.ars_stage = ArsStage::Manifest;
        }
    }

    /// 当前 ARS 综合征（按累计剂量阈值）
    pub fn current_syndrome(&self) -> Option<ArsSyndrome> {
        ArsSyndrome::from_dose(self.cumulative_dose_gy)
    }
}

impl SciBioSubsystem for RadiationMutagenesis {
    fn subsystem_name(&self) -> &'static str { "RadiationMutagenesis" }
    fn is_active(&self) -> bool { self.cumulative_dose_gy > 0.0 }
    fn step(&mut self, dt: f32) {
        self.update_ars(dt / 86400.0);
    }
}
// ========================================================
// 2. CRISPR 基因改造 CrisprEdit
// ========================================================
// 参考：
// - Jinek 2012, Science (Cas9 切割机制)
// - Anzalone 2019, Nature (Prime editing)
// - Komor 2016, Nature (Base editing)
// - Esvelt 2014, eLife (Gene drive)

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrisprRepairPath {
    /// 非同源末端连接 —— indel，30% 效率，易错
    Nhej,
    /// 同源定向修复 —— 模板精确，5% 效率
    Hdr,
    /// 碱基编辑 C→T/A→G，不切割
    BaseEditing,
    /// Prime editing —— 任意替换
    PrimeEditing,
}

impl CrisprRepairPath {
    pub fn efficiency(&self) -> f32 {
        match self {
            CrisprRepairPath::Nhej => 0.30,
            CrisprRepairPath::Hdr => 0.05,
            CrisprRepairPath::BaseEditing => 0.70,
            CrisprRepairPath::PrimeEditing => 0.20,
        }
    }
    pub fn error_rate(&self) -> f32 {
        match self {
            CrisprRepairPath::Nhej => 0.50,
            CrisprRepairPath::Hdr => 0.01,
            CrisprRepairPath::BaseEditing => 0.05,
            CrisprRepairPath::PrimeEditing => 0.03,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VectorSystem {
    /// AAV —— 4.7 kb 容量限制
    Aav,
    /// 慢病毒 —— 大容量 ~8 kb
    Lentivirus,
    /// 脂质纳米颗粒 —— mRNA
    LipidNanoparticle,
    /// 电穿孔 —— 裸 DNA
    Electroporation,
}

impl VectorSystem {
    pub fn capacity_kb(&self) -> f32 {
        match self {
            VectorSystem::Aav => 4.7,
            VectorSystem::Lentivirus => 8.0,
            VectorSystem::LipidNanoparticle => 100.0,
            VectorSystem::Electroporation => 200.0,
        }
    }
}

/// CRISPR 靶点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisprTarget {
    /// 20 nt 向导 RNA 序列
    pub guide_rna: String,
    /// PAM 序列（Sp-Cas9 = NGG）
    pub pam: String,
    /// 切割位点相对 PAM 3' 端的位置（bp，典型 -3）
    pub cut_offset_bp: i32,
    /// 载体
    pub vector: VectorSystem,
}

impl CrisprTarget {
    /// 检查 PAM 是否符合 NGG（Sp-Cas9）
    pub fn pam_valid_sp_cas9(&self) -> bool {
        let p = self.pam.as_bytes();
        // SpCas9 PAM = 5'-NGG-3', N = 任意碱基 (A/T/G/C/N)
        p.len() == 3
            && matches!(p[0], b'A' | b'T' | b'G' | b'C' | b'N')
            && p[1] == b'G'
            && p[2] == b'G'
    }
    /// seed 区域（PAM 近端 12 bp）严格匹配，远端 8 bp 宽松
    pub fn seed_mismatch_tolerance(&self) -> (usize, usize) {
        (12, 8)
    }
}

/// 脱靶位点分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffTargetSite {
    pub chromosome: String,
    pub position: u64,
    pub mismatches: u32,
    /// seed 区域错配数
    pub seed_mismatches: u32,
    /// 结合得分（0-1，越高越易脱靶）
    pub score: f32,
}

impl OffTargetSite {
    /// 错配评分：seed 区错配权重 1.0，远端 0.3
    pub fn compute_score(seed_mm: u32, dist_mm: u32) -> f32 {
        let penalty = (seed_mm as f32) * 1.0 + (dist_mm as f32) * 0.3;
        (1.0 - penalty / 12.0).max(0.0)
    }
}

/// 基因驱动模型（显性纯合转化）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeneDrive {
    /// 同源驱动拷贝（亲子传递率，野生 0.5 → 驱动 0.95+）
    pub inheritance_bias: f32,
    /// 适合度代价（0=无，1=致死）
    pub fitness_cost: f32,
    /// 隔离碎裂参数（0=自由混合，1=完全隔离）
    pub fragmentation: f32,
}

impl Default for GeneDrive {
    fn default() -> Self {
        Self { inheritance_bias: 0.95, fitness_cost: 0.1, fragmentation: 0.0 }
    }
}

impl GeneDrive {
    /// 单代等位基因频率扩散
    /// p_next = p · (1 + s) / (1 + p·s)，s = 2·bias - 1 - cost
    pub fn advance_frequency(&self, p: f32) -> f32 {
        let s = (2.0 * self.inheritance_bias - 1.0) - self.fitness_cost;
        let p_next = p * (1.0 + s) / (1.0 + p * s);
        // 隔离降低扩散速度
        p_next * (1.0 - self.fragmentation) + p * self.fragmentation
    }
    /// 达到 99% 频率所需代数（迭代估算）
    pub fn generations_to_fix(&self, p0: f32) -> u32 {
        let mut p = p0;
        let mut n = 0;
        while p < 0.99 && n < 1000 {
            p = self.advance_frequency(p);
            n += 1;
        }
        n
    }
}

/// CRISPR 编辑结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisprEditResult {
    pub repair_path: CrisprRepairPath,
    pub on_target_efficiency: f32,
    pub off_target_count: u32,
    pub indel_size_bp: i32,
    pub success: bool,
}

/// CRISPR 系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisprEdit {
    pub target: CrisprTarget,
    pub repair_path: CrisprRepairPath,
    pub off_targets: Vec<OffTargetSite>,
    pub gene_drive: Option<GeneDrive>,
    /// 已编辑细胞数
    pub edited_cells: u64,
    /// 总细胞数
    pub total_cells: u64,
}

impl CrisprEdit {
    pub fn new(target: CrisprTarget, repair: CrisprRepairPath) -> Self {
        Self {
            target,
            repair_path: repair,
            off_targets: Vec::new(),
            gene_drive: None,
            edited_cells: 0,
            total_cells: 0,
        }
    }
    /// 模拟一次编辑（细胞数 N）
    pub fn run(&mut self, n_cells: u64) -> CrisprEditResult {
        self.total_cells = self.total_cells.saturating_add(n_cells);
        let eff = self.repair_path.efficiency();
        let edited = (n_cells as f32 * eff) as u64;
        self.edited_cells += edited;
        // NHEJ 平均 indel 约 -5 bp（缺失为主）
        let indel = match self.repair_path {
            CrisprRepairPath::Nhej => -5,
            _ => 0,
        };
        let success = edited as f32 / (n_cells as f32).max(1.0) > 0.01;
        CrisprEditResult {
            repair_path: self.repair_path,
            on_target_efficiency: eff,
            off_target_count: self.off_targets.len() as u32,
            indel_size_bp: indel,
            success,
        }
    }
    /// 总体错误率（脱靶 + 编辑错误）
    pub fn total_error_rate(&self) -> f32 {
        let edit_err = self.repair_path.error_rate();
        let off_target_rate = if self.total_cells > 0 {
            (self.off_targets.len() as f64 / self.total_cells as f64) as f32
        } else {
            0.0
        };
        edit_err + off_target_rate
    }
}

impl SciBioSubsystem for CrisprEdit {
    fn subsystem_name(&self) -> &'static str { "CrisprEdit" }
    fn is_active(&self) -> bool { self.edited_cells > 0 }
    fn step(&mut self, _dt: f32) {}
}
// ========================================================
// 3. 外星生物学 AlienBiology
// ========================================================
// 参考：
// - Bains 2004, Astrobiology (硅基生命)
// - Benner 2010 (非水溶剂生化)
// - Dadachova 2007 (放射合成真菌 Cryptococcus neoformans)
// - DasSarma 2006 (紫膜光合)

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlienBiochemistry {
    /// 碳基水溶（地球）
    CarbonWater,
    /// 硅基 Si-O-Si，耐高温 200-400°C，溶剂液态硫
    Silicate,
    /// 氨基生命 NH₃ 溶剂，-80°C
    Ammonia,
    /// 甲烷基 CH₄ 溶剂，-180°C（Titan 式）
    Methane,
}

impl AlienBiochemistry {
    pub fn optimal_temp_c(&self) -> (f32, f32) {
        match self {
            AlienBiochemistry::CarbonWater => (0.0, 100.0),
            AlienBiochemistry::Silicate => (200.0, 400.0),
            AlienBiochemistry::Ammonia => (-80.0, -33.0),
            AlienBiochemistry::Methane => (-180.0, -150.0),
        }
    }
    pub fn solvent(&self) -> &'static str {
        match self {
            AlienBiochemistry::CarbonWater => "H2O",
            AlienBiochemistry::Silicate => "S (liquid sulfur)",
            AlienBiochemistry::Ammonia => "NH3",
            AlienBiochemistry::Methane => "CH4",
        }
    }
}

/// 非水溶剂属性表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolventProperties {
    pub name: String,
    /// 极性（介电常数 ε）
    pub dielectric_const: f32,
    /// 沸点 (°C)
    pub boiling_point_c: f32,
    /// 粘度 (cP)
    pub viscosity_cp: f32,
    /// 极性指数（0=非极性，1=强极性）
    pub polarity_index: f32,
}

impl SolventProperties {
    pub fn water() -> Self {
        Self { name: "H2O".to_string(), dielectric_const: 80.0, boiling_point_c: 100.0, viscosity_cp: 0.89, polarity_index: 1.0 }
    }
    pub fn ammonia() -> Self {
        Self { name: "NH3".to_string(), dielectric_const: 22.0, boiling_point_c: -33.0, viscosity_cp: 0.25, polarity_index: 0.7 }
    }
    pub fn methane() -> Self {
        Self { name: "CH4".to_string(), dielectric_const: 1.7, boiling_point_c: -161.0, viscosity_cp: 0.01, polarity_index: 0.0 }
    }
    pub fn liquid_sulfur() -> Self {
        Self { name: "S".to_string(), dielectric_const: 3.5, boiling_point_c: 444.0, viscosity_cp: 7.0, polarity_index: 0.2 }
    }
}

/// 电子受体（呼吸链末端）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ElectronAcceptor {
    O2,
    H2s,
    Fe3,
    So4,
    No3,
    Co2,
}

impl ElectronAcceptor {
    /// 还原电位 (V) —— 越正越有利
    pub fn redox_potential_v(&self) -> f32 {
        match self {
            ElectronAcceptor::O2 => 0.82,
            ElectronAcceptor::Fe3 => 0.20,
            ElectronAcceptor::No3 => 0.42,
            ElectronAcceptor::So4 => -0.22,
            ElectronAcceptor::Co2 => -0.24,
            ElectronAcceptor::H2s => -0.28,
        }
    }
}

/// 能量代谢变体
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EnergyMetabolism {
    /// 光合（地球式，叶绿素 400-700nm）
    Photosynthesis,
    /// 紫膜光合（视紫红质 500-650nm，无叶绿素）
    RhodopsinPhotosynthesis,
    /// 红外光合（>700nm）
    InfraredPhotosynthesis,
    /// 化能合成（深海热泉 H₂S + O₂ → S + H₂O）
    Chemosynthesis,
    /// 放射合成（γ → melanin → 化学能，Cryptococcus neoformans）
    Radiosynthesis,
}

impl EnergyMetabolism {
    pub fn wavelength_nm(&self) -> Option<(f32, f32)> {
        match self {
            EnergyMetabolism::Photosynthesis => Some((400.0, 700.0)),
            EnergyMetabolism::RhodopsinPhotosynthesis => Some((500.0, 650.0)),
            EnergyMetabolism::InfraredPhotosynthesis => Some((700.0, 1100.0)),
            _ => None,
        }
    }
    /// 能量产率（相对单位，地球光合=1.0）
    pub fn energy_yield(&self) -> f32 {
        match self {
            EnergyMetabolism::Photosynthesis => 1.0,
            EnergyMetabolism::RhodopsinPhotosynthesis => 0.3,
            EnergyMetabolism::InfraredPhotosynthesis => 0.15,
            EnergyMetabolism::Chemosynthesis => 0.5,
            EnergyMetabolism::Radiosynthesis => 0.05,
        }
    }
}

/// DNA 替代物（XNA）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeneticPolymer {
    DNA,
    RNA,
    /// 肽核酸（中性骨架）
    Pna,
    /// 苏糖核酸
    Tna,
    /// 甘油核酸
    Gna,
    /// 其他 XNA
    Xna,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlienBiology {
    pub biochemistry: AlienBiochemistry,
    pub metabolism: EnergyMetabolism,
    pub electron_acceptor: ElectronAcceptor,
    pub genetic_polymer: GeneticPolymer,
    pub solvent: SolventProperties,
    /// 当前环境温度 (°C)
    pub env_temp_c: f32,
}

impl AlienBiology {
    pub fn earth_like() -> Self {
        Self {
            biochemistry: AlienBiochemistry::CarbonWater,
            metabolism: EnergyMetabolism::Photosynthesis,
            electron_acceptor: ElectronAcceptor::O2,
            genetic_polymer: GeneticPolymer::DNA,
            solvent: SolventProperties::water(),
            env_temp_c: 25.0,
        }
    }
    pub fn titan_methane() -> Self {
        Self {
            biochemistry: AlienBiochemistry::Methane,
            metabolism: EnergyMetabolism::Chemosynthesis,
            electron_acceptor: ElectronAcceptor::H2s,
            genetic_polymer: GeneticPolymer::Pna,
            solvent: SolventProperties::methane(),
            env_temp_c: -179.0,
        }
    }
    pub fn hydrothermal_vent() -> Self {
        Self {
            biochemistry: AlienBiochemistry::CarbonWater,
            metabolism: EnergyMetabolism::Chemosynthesis,
            electron_acceptor: ElectronAcceptor::H2s,
            genetic_polymer: GeneticPolymer::DNA,
            solvent: SolventProperties::water(),
            env_temp_c: 80.0,
        }
    }
    /// 环境适宜性（0-1）
    pub fn habitability(&self) -> f32 {
        let (tmin, tmax) = self.biochemistry.optimal_temp_c();
        let t_factor = if self.env_temp_c >= tmin && self.env_temp_c <= tmax {
            1.0 - (self.env_temp_c - (tmin + tmax) * 0.5).abs() / ((tmax - tmin) * 0.5).max(1.0)
        } else {
            0.0
        };
        let m_factor = self.metabolism.energy_yield();
        let e_factor = (self.electron_acceptor.redox_potential_v() / 0.82).max(0.0);
        t_factor.max(0.0) * m_factor * e_factor
    }
}

impl SciBioSubsystem for AlienBiology {
    fn subsystem_name(&self) -> &'static str { "AlienBiology" }
    fn is_active(&self) -> bool { self.habitability() > 0.0 }
    fn step(&mut self, _dt: f32) {}
}
// ========================================================
// 4. 合成生物学 SyntheticBiology
// ========================================================
// 参考：
// - Elowitz & Leibler 2000, Nature (Repressilator)
// - Gardner 2000, Nature (Toggle switch)
// - Gibson 2010, Science (JCVI-syn1.0)
// - Hutchison 2016, Science (JCVI-syn3.0, 473 基因)

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CircuitGate {
    And,
    Or,
    Not,
    Xor,
    Nand,
    Nor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CircuitType {
    /// 双稳态开关（LacI/IPTG vs TetR/aTc）
    ToggleSwitch,
    /// 振荡器（3 抑制子环）
    Repressilator,
    /// 重组酶计数器
    RecombinaseCounter,
    /// 逻辑门
    LogicGate(CircuitGate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneCircuit {
    pub ctype: CircuitType,
    /// 输入信号（0-1）
    pub inputs: Vec<f32>,
    /// 当前输出（0-1）
    pub output: f32,
    /// 振荡相位（仅 repressilator）
    pub phase: f32,
    /// 计数值（仅 counter）
    pub count: u32,
}

impl GeneCircuit {
    pub fn new(ctype: CircuitType) -> Self {
        Self { ctype, inputs: Vec::new(), output: 0.0, phase: 0.0, count: 0 }
    }
    /// 计算逻辑门
    pub fn evaluate_gate(&mut self) -> f32 {
        let out = match self.ctype {
            CircuitType::LogicGate(CircuitGate::And) => {
                self.inputs.iter().fold(1.0_f32, |a, b| a.min(*b))
            }
            CircuitType::LogicGate(CircuitGate::Or) => {
                self.inputs.iter().fold(0.0_f32, |a, b| a.max(*b))
            }
            CircuitType::LogicGate(CircuitGate::Not) => {
                1.0 - self.inputs.first().copied().unwrap_or(0.0)
            }
            CircuitType::LogicGate(CircuitGate::Xor) => {
                let a = self.inputs.first().copied().unwrap_or(0.0);
                let b = self.inputs.get(1).copied().unwrap_or(0.0);
                (a - b).abs()
            }
            CircuitType::LogicGate(CircuitGate::Nand) => {
                1.0 - self.inputs.iter().fold(1.0_f32, |a, b| a.min(*b))
            }
            CircuitType::LogicGate(CircuitGate::Nor) => {
                1.0 - self.inputs.iter().fold(0.0_f32, |a, b| a.max(*b))
            }
            _ => self.output,
        };
        self.output = out;
        out
    }
    /// repressilator 步进 —— 3 抑制子环振荡，周期 ~6h
    pub fn step_repressilator(&mut self, dt_h: f32) {
        let period_h = 6.0;
        self.phase += dt_h / period_h * 2.0 * std::f32::consts::PI;
        self.phase = self.phase.rem_euclid(2.0 * std::f32::consts::PI);
        self.output = (self.phase.sin() + 1.0) * 0.5;
    }
    /// toggle switch —— 双稳态，带滞后
    pub fn step_toggle(&mut self, input: f32) {
        if input > 0.6 { self.output = 1.0; }
        else if input < 0.4 { self.output = 0.0; }
    }
    /// 计数器
    pub fn increment_counter(&mut self) -> u32 {
        self.count += 1;
        self.count
    }
}

/// 生物制造产物
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomanufacturingProduct {
    pub name: String,
    pub host: String,
    pub yield_g_per_l: f32,
    pub titer_grams: f32,
}

impl BiomanufacturingProduct {
    pub fn insulin() -> Self {
        Self { name: "Insulin".into(), host: "E. coli".into(), yield_g_per_l: 1.5, titer_grams: 0.0 }
    }
    pub fn artemisinin() -> Self {
        Self { name: "Artemisinin".into(), host: "S. cerevisiae".into(), yield_g_per_l: 2.5, titer_grams: 0.0 }
    }
    pub fn spider_silk() -> Self {
        Self { name: "SpiderSilk".into(), host: "E. coli".into(), yield_g_per_l: 0.3, titer_grams: 0.0 }
    }
    /// 在反应器体积 V (L) 中生产
    pub fn produce(&mut self, volume_l: f32) -> f32 {
        self.titer_grams = self.yield_g_per_l * volume_l;
        self.titer_grams
    }
}

/// 最小基因组（JCVI-syn3.0 = 473 基因）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimalGenome {
    pub gene_count: u32,
    pub genome_size_bp: u64,
    pub name: String,
}

impl MinimalGenome {
    /// JCVI-syn3.0 (Hutchison 2016, Science) —— 473 基因 / 531kb
    pub fn jcvi_syn3() -> Self {
        Self { gene_count: 473, genome_size_bp: 531_000, name: "JCVI-syn3.0".to_string() }
    }
    /// JCVI-syn1.0 (Gibson 2010, Science) —— 901 基因 / 1.08Mbp
    pub fn jcvi_syn1() -> Self {
        Self { gene_count: 901, genome_size_bp: 1_080_000, name: "JCVI-syn1.0".to_string() }
    }
}

/// 合成生物学系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticBiology {
    pub circuits: Vec<GeneCircuit>,
    pub products: Vec<BiomanufacturingProduct>,
    pub minimal_genome: MinimalGenome,
    /// 是否使用 DNA 计算
    pub dna_computing: bool,
}

impl Default for SyntheticBiology {
    fn default() -> Self {
        Self {
            circuits: vec![GeneCircuit::new(CircuitType::ToggleSwitch)],
            products: vec![BiomanufacturingProduct::insulin()],
            minimal_genome: MinimalGenome::jcvi_syn3(),
            dna_computing: false,
        }
    }
}

impl SyntheticBiology {
    pub fn new() -> Self { Self::default() }
    /// 批量生产（每产物体积 V 升）
    pub fn batch_produce(&mut self, volume_l: f32) -> f32 {
        let mut total = 0.0;
        for p in &mut self.products {
            total += p.produce(volume_l);
        }
        total
    }
    /// 添加电路
    pub fn add_circuit(&mut self, c: GeneCircuit) {
        self.circuits.push(c);
    }
    /// 总基因数
    pub fn total_genes(&self) -> u32 {
        self.minimal_genome.gene_count
    }
}

impl SciBioSubsystem for SyntheticBiology {
    fn subsystem_name(&self) -> &'static str { "SyntheticBiology" }
    fn is_active(&self) -> bool { !self.circuits.is_empty() }
    fn step(&mut self, dt: f32) {
        let dt_h = dt / 3600.0;
        for c in &mut self.circuits {
            match c.ctype {
                CircuitType::Repressilator => c.step_repressilator(dt_h),
                _ => {
                    if !c.inputs.is_empty() {
                        let inp = c.inputs[0];
                        c.step_toggle(inp);
                        c.evaluate_gate();
                    }
                }
            }
        }
    }
}
// ========================================================
// 5. 共生体系统 Symbiosis
// ========================================================
// 参考：
// - Douglas 2010, "The Symbiotic Habit"
// - Margulis 1970 (内共生理论)
// - Waters & Bassler 2005, Annu Rev Cell Dev Biol (Quorum sensing)

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbiosisType {
    /// 互利共生（双方受益）
    Mutualism,
    /// 偏利共生（一方受益，一方无害）
    Commensalism,
    /// 寄生（一方受益，一方受害）
    Parasitism,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbiosisCase {
    /// 地衣（真菌+藻类/蓝细菌）
    Lichen,
    /// 珊瑚虫+虫黄藻
    CoralZooxanthellae,
    /// 豆科+根瘤菌（固氮）
    LegumeRhizobium,
    /// 深海管虫+硫氧化菌
    TubewormSulfurBacteria,
    /// 人体+线粒体（内共生）
    HumanMitochondria,
}

impl SymbiosisCase {
    pub fn symbiosis_type(&self) -> SymbiosisType {
        match self {
            SymbiosisCase::Lichen
            | SymbiosisCase::CoralZooxanthellae
            | SymbiosisCase::LegumeRhizobium
            | SymbiosisCase::TubewormSulfurBacteria
            | SymbiosisCase::HumanMitochondria => SymbiosisType::Mutualism,
        }
    }
    pub fn description(&self) -> &'static str {
        match self {
            SymbiosisCase::Lichen => "Fungus + algae/cyanobacteria",
            SymbiosisCase::CoralZooxanthellae => "Coral + Symbiodinium (photosynthate)",
            SymbiosisCase::LegumeRhizobium => "Legume + Rhizobium (N2 fixation)",
            SymbiosisCase::TubewormSulfurBacteria => "Riftia + sulfur-oxidizing bacteria",
            SymbiosisCase::HumanMitochondria => "Eukaryote + alpha-proteobacterium (endosymbiosis)",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EndosymbiosisLevel {
    /// 原生共生（原核 → 真核）
    Primary,
    /// 次生共生（真核吞噬真核）
    Secondary,
    /// 三级共生
    Tertiary,
}

/// 水平基因转移方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HgtMechanism {
    /// 转化（裸 DNA 摄入）
    Transformation,
    /// 转导（噬菌体介导）
    Transduction,
    /// 接合（细胞-细胞）
    Conjugation,
}

impl HgtMechanism {
    /// 转移率（每代每细胞）
    pub fn transfer_rate(&self) -> f32 {
        match self {
            HgtMechanism::Transformation => 1e-6,
            HgtMechanism::Transduction => 1e-5,
            HgtMechanism::Conjugation => 1e-3,
        }
    }
}

/// 群体感应信号分子
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuorumSignal {
    /// AHL（革兰阴性菌）
    Ahl,
    /// AIP（革兰阳性菌）
    Aip,
    /// AI-2（跨物种）
    Ai2,
}

impl QuorumSignal {
    pub fn name(&self) -> &'static str {
        match self {
            QuorumSignal::Ahl => "Acyl-Homoserine Lactone",
            QuorumSignal::Aip => "Autoinducing Peptide",
            QuorumSignal::Ai2 => "Autoinducer-2",
        }
    }
    /// 群体感应激活阈值（细胞密度 /mL）
    pub fn activation_density(&self) -> f32 {
        match self {
            QuorumSignal::Ahl => 1e8,
            QuorumSignal::Aip => 1e7,
            QuorumSignal::Ai2 => 1e7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioticRelationship {
    pub case: SymbiosisCase,
    pub stype: SymbiosisType,
    /// 共生体适合度增益
    pub host_fitness_gain: f32,
    pub symbiont_fitness_gain: f32,
    /// 稳定性（0-1）
    pub stability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbiosis {
    pub relationships: Vec<SymbioticRelationship>,
    pub hgt_events: u64,
    pub quorum_signals: Vec<QuorumSignal>,
    /// 当前细胞密度（用于群体感应判定）
    pub cell_density_per_ml: f32,
    /// 群体感应激活状态
    pub quorum_active: bool,
    /// 内共生层级
    pub endosymbiosis_level: EndosymbiosisLevel,
}

impl Default for Symbiosis {
    fn default() -> Self { Self::new() }
}

impl Symbiosis {
    pub fn new() -> Self {
        Self {
            relationships: Vec::new(),
            hgt_events: 0,
            quorum_signals: vec![QuorumSignal::Ahl],
            cell_density_per_ml: 1e5,
            quorum_active: false,
            endosymbiosis_level: EndosymbiosisLevel::Primary,
        }
    }
    /// 添加经典案例
    pub fn add_classic(&mut self, case: SymbiosisCase) {
        let (host_gain, sym_gain, stab) = match case {
            SymbiosisCase::Lichen => (0.3, 0.4, 0.9),
            SymbiosisCase::CoralZooxanthellae => (0.5, 0.5, 0.8),
            SymbiosisCase::LegumeRhizobium => (0.2, 0.3, 0.85),
            SymbiosisCase::TubewormSulfurBacteria => (0.6, 0.4, 0.9),
            SymbiosisCase::HumanMitochondria => (1.0, 0.9, 1.0),
        };
        self.relationships.push(SymbioticRelationship {
            case,
            stype: case.symbiosis_type(),
            host_fitness_gain: host_gain,
            symbiont_fitness_gain: sym_gain,
            stability: stab,
        });
    }
    /// 触发一次 HGT 事件
    pub fn trigger_hgt(&mut self, mech: HgtMechanism) -> f32 {
        self.hgt_events += 1;
        mech.transfer_rate()
    }
    /// 检查群体感应
    pub fn check_quorum(&mut self) -> bool {
        let threshold = self
            .quorum_signals
            .iter()
            .map(|s| s.activation_density())
            .fold(f32::MAX, f32::min);
        self.quorum_active = self.cell_density_per_ml >= threshold;
        self.quorum_active
    }
    /// 累计适合度增益
    pub fn total_host_fitness(&self) -> f32 {
        self.relationships.iter().map(|r| r.host_fitness_gain * r.stability).sum()
    }
}

impl SciBioSubsystem for Symbiosis {
    fn subsystem_name(&self) -> &'static str { "Symbiosis" }
    fn is_active(&self) -> bool { !self.relationships.is_empty() }
    fn step(&mut self, _dt: f32) {
        self.check_quorum();
    }
}
// ========================================================
// 6. 赛博格系统 Cybernetics
// ========================================================
// 参考：
// - Hochberg 2019, Nature (BCI review)
// - Musk 2019, Neuralink white paper (3072 通道)
// - Markram 2011, Nat Rev Neurosci (STDP)
// - Northmore 1996 (Utah array)

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BciModality {
    /// 非侵入式 EEG，μV 级，10-100Hz
    Eeg,
    /// 侵入式 Utah array，100 通道
    UtahArray,
    /// 神经织网（Neuralink 式，柔性 3072 通道）
    NeuralLace,
    /// ECoG 皮层表面
    Ecog,
}

impl BciModality {
    pub fn channel_count(&self) -> u32 {
        match self {
            BciModality::Eeg => 64,
            BciModality::UtahArray => 100,
            BciModality::NeuralLace => 3072,
            BciModality::Ecog => 256,
        }
    }
    pub fn signal_amplitude_uv(&self) -> f32 {
        match self {
            BciModality::Eeg => 50.0,
            BciModality::UtahArray => 500.0,
            BciModality::NeuralLace => 200.0,
            BciModality::Ecog => 1000.0,
        }
    }
    pub fn bandwidth_hz(&self) -> (f32, f32) {
        match self {
            BciModality::Eeg => (0.5, 100.0),
            BciModality::UtahArray => (0.1, 5000.0),
            BciModality::NeuralLace => (0.1, 10000.0),
            BciModality::Ecog => (0.1, 500.0),
        }
    }
    pub fn is_invasive(&self) -> bool {
        match self {
            BciModality::Eeg => false,
            _ => true,
        }
    }
}

/// 感官增强类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SensoryAugmentation {
    /// 磁感应植入（钕磁铁）
    MagneticImplant,
    /// 红外视觉
    InfraredVision,
    /// 超声回声（仿蝙蝠）
    UltrasonicEcholocation,
    /// 内置 GPS
    InternalGps,
}

impl SensoryAugmentation {
    pub fn description(&self) -> &'static str {
        match self {
            SensoryAugmentation::MagneticImplant => "Neodymium magnet, EM field sense",
            SensoryAugmentation::InfraredVision => "IR camera -> neural signal",
            SensoryAugmentation::UltrasonicEcholocation => "Bat-like sonar",
            SensoryAugmentation::InternalGps => "Embedded GPS module",
        }
    }
}

/// 排异反应类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RejectionType {
    /// 神经电极胶质疤痕
    GlialScar,
    /// 外周纤维包裹
    FibrousEncapsulation,
    /// 免疫排斥
    ImmuneRejection,
    /// 感染
    Infection,
}

impl RejectionType {
    pub fn typical_onset_days(&self) -> f32 {
        match self {
            RejectionType::GlialScar => 14.0,
            RejectionType::FibrousEncapsulation => 30.0,
            RejectionType::ImmuneRejection => 7.0,
            RejectionType::Infection => 3.0,
        }
    }
}

/// 神经可塑性模型（STDP —— 脉冲时间依赖可塑性）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StdpSynapse {
    pub weight: f32,
    /// 突触前最近放电时刻 (s)
    pub last_pre_spike: f32,
    /// 突触后最近放电时刻 (s)
    pub last_post_spike: f32,
    /// 学习率
    pub lr: f32,
    /// 时间窗 (s)，典型 20ms
    pub tau: f32,
}

impl Default for StdpSynapse {
    fn default() -> Self {
        Self {
            weight: 0.5,
            last_pre_spike: -1e9,
            last_post_spike: -1e9,
            lr: 0.01,
            tau: 0.02,
        }
    }
}

impl StdpSynapse {
    /// STDP 更新：pre 在 post 之前 50ms 内 → LTP（强化）；
    /// pre 在 post 之后 50ms 内 → LTD（弱化）
    pub fn update(&mut self, t_now: f32, pre_fired: bool, post_fired: bool) {
        if pre_fired { self.last_pre_spike = t_now; }
        if post_fired { self.last_post_spike = t_now; }
        if self.last_pre_spike > 0.0 && self.last_post_spike > 0.0 {
            let delta = self.last_post_spike - self.last_pre_spike;
            if delta > 0.0 && delta < 0.05 {
                self.weight = (self.weight + self.lr).min(1.0);
            } else if delta < 0.0 && delta > -0.05 {
                self.weight = (self.weight - self.lr * 0.5).max(0.0);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cybernetics {
    pub bci_modality: BciModality,
    pub augmentations: Vec<SensoryAugmentation>,
    /// 通道数
    pub channels: u32,
    /// 已植入时长 (天)
    pub implanted_days: f32,
    /// 排异反应等级（0-1）
    pub rejection_level: f32,
    /// 神经可塑性突触库
    pub synapses: Vec<StdpSynapse>,
    /// 运动意图解码准确率
    pub motor_decode_accuracy: f32,
}

impl Cybernetics {
    pub fn new(modality: BciModality) -> Self {
        Self {
            bci_modality: modality,
            augmentations: Vec::new(),
            channels: modality.channel_count(),
            implanted_days: 0.0,
            rejection_level: 0.0,
            synapses: vec![StdpSynapse::default(); 100],
            motor_decode_accuracy: 0.0,
        }
    }
    /// 添加感官增强
    pub fn augment(&mut self, aug: SensoryAugmentation) {
        self.augmentations.push(aug);
    }
    /// 推进时间，更新排异和可塑性
    pub fn update(&mut self, dt_days: f32) {
        self.implanted_days += dt_days;
        // 排异随时间增长，侵入式更快
        let rate = if self.bci_modality.is_invasive() { 0.001 } else { 0.0001 };
        self.rejection_level = (self.rejection_level + rate * dt_days).min(1.0);
        // 运动解码准确率随学习上升，随排异下降
        let target = 0.9 - self.rejection_level * 0.5;
        self.motor_decode_accuracy += (target - self.motor_decode_accuracy) * 0.01 * dt_days;
        self.motor_decode_accuracy = self.motor_decode_accuracy.clamp(0.0, 0.95);
    }
    /// 训练一次（突触可塑性）
    pub fn train(&mut self, t_now: f32, pre_idx: usize, post_idx: usize) {
        for (i, s) in self.synapses.iter_mut().enumerate() {
            if i == pre_idx {
                s.update(t_now, true, false);
            }
            if i == post_idx && post_idx != pre_idx {
                s.update(t_now, false, true);
            }
        }
    }
    /// 平均突触权重
    pub fn avg_synapse_weight(&self) -> f32 {
        if self.synapses.is_empty() { 0.0 }
        else { self.synapses.iter().map(|s| s.weight).sum::<f32>() / self.synapses.len() as f32 }
    }
}

impl SciBioSubsystem for Cybernetics {
    fn subsystem_name(&self) -> &'static str { "Cybernetics" }
    fn is_active(&self) -> bool { self.implanted_days > 0.0 }
    fn step(&mut self, dt: f32) {
        self.update(dt / 86400.0);
    }
}
// ========================================================
// 单元测试
// ========================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 1. 辐射变异系统测试 ----

    #[test]
    fn test_radiation_lq_model() {
        let lq = LinearQuadraticModel::default();
        // 人类典型 α=0.3, β=0.1 (ICRP 103)
        assert_eq!(lq.alpha_per_gy, 0.3);
        assert_eq!(lq.beta_per_gy2, 0.1);
        // α/β = 3（早反应组织典型）
        assert!((lq.alpha_beta_ratio() - 3.0).abs() < 1e-6);
        // 1 Gy 应变效应 = 0.3 + 0.1 = 0.4
        assert!((lq.effect(1.0) - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_radiation_mutagenesis_dose() {
        let mut rm = RadiationMutagenesis::new();
        let dose = RadiationDose { dose_gy: 2.0, radiation: RadiationType::Gamma, duration_s: 60.0 };
        let dmg = rm.expose(dose);
        // 2 Gy γ × RBE(1) × 30 DSB/Gy ≈ 63（含 LET 校正）
        assert!(dmg.dsb >= 60);
        assert!((rm.cumulative_dose_gy - 2.0).abs() < 1e-6);
        // 当量剂量 2 Gy × RBE(1) = 2 Sv
        assert!((dose.equivalent_dose_sv() - 2.0).abs() < 1e-6);
        // ARS 综合征：2 Gy → 骨髓
        assert_eq!(rm.current_syndrome(), Some(ArsSyndrome::Hematopoietic));
    }

    #[test]
    fn test_radiation_alpha_let_and_rbe() {
        let alpha = RadiationType::Alpha;
        assert!((alpha.let_kev_per_um() - 100.0).abs() < 1e-6);
        assert!((alpha.rbe() - 20.0).abs() < 1e-6);
        // 中子 RBE=10
        assert!((RadiationType::Neutron.rbe() - 10.0).abs() < 1e-6);
        // ARS 阈值：6 Gy 胃肠，20 Gy 神经
        assert_eq!(ArsSyndrome::from_dose(7.0), Some(ArsSyndrome::Gastrointestinal));
        assert_eq!(ArsSyndrome::from_dose(25.0), Some(ArsSyndrome::Neurovascular));
        assert_eq!(ArsSyndrome::from_dose(0.5), None);
    }

    #[test]
    fn test_radiation_survival_fraction() {
        let mut rm = RadiationMutagenesis::new();
        rm.expose(RadiationDose { dose_gy: 1.0, radiation: RadiationType::Gamma, duration_s: 1.0 });
        // SF = exp(-0.4) ≈ 0.67
        let sf = rm.survival_fraction();
        assert!(sf > 0.6 && sf < 0.7);
    }

    // ---- 2. CRISPR 测试 ----

    #[test]
    fn test_crispr_pam_validation() {
        let target = CrisprTarget {
            guide_rna: "GATTACAAGCTGCTAACTGG".into(),
            pam: "AGG".into(),
            cut_offset_bp: -3,
            vector: VectorSystem::Aav,
        };
        assert!(target.pam_valid_sp_cas9());
        let (seed, dist) = target.seed_mismatch_tolerance();
        assert_eq!(seed, 12);
        assert_eq!(dist, 8);
        // 载体容量 AAV=4.7kb
        assert!((VectorSystem::Aav.capacity_kb() - 4.7).abs() < 1e-6);
    }

    #[test]
    fn test_crispr_repair_efficiency() {
        // NHEJ 30%, HDR 5%, Base 70%, Prime 20%
        assert!((CrisprRepairPath::Nhej.efficiency() - 0.30).abs() < 1e-6);
        assert!((CrisprRepairPath::Hdr.efficiency() - 0.05).abs() < 1e-6);
        assert!((CrisprRepairPath::BaseEditing.efficiency() - 0.70).abs() < 1e-6);
        assert!((CrisprRepairPath::PrimeEditing.efficiency() - 0.20).abs() < 1e-6);
        // HDR 错误率最低
        assert!(CrisprRepairPath::Hdr.error_rate() < CrisprRepairPath::Nhej.error_rate());
        // 脱靶评分：seed 错配越多越低
        let s0 = OffTargetSite::compute_score(0, 0);
        let s3 = OffTargetSite::compute_score(3, 0);
        assert!(s0 > s3);
    }

    #[test]
    fn test_gene_drive_spread() {
        let gd = GeneDrive::default();
        // bias=0.95, cost=0.1
        let p0 = 0.01;
        let p1 = gd.advance_frequency(p0);
        // 一代后频率应显著上升
        assert!(p1 > p0);
        // 99% 固定所需代数（10-100 之间合理）
        let gens = gd.generations_to_fix(p0);
        assert!(gens > 0 && gens < 200);
    }

    #[test]
    fn test_crispr_edit_run() {
        let target = CrisprTarget {
            guide_rna: "GATTACAAGCTGCTAACTGG".into(),
            pam: "AGG".into(),
            cut_offset_bp: -3,
            vector: VectorSystem::Lentivirus,
        };
        let mut ce = CrisprEdit::new(target, CrisprRepairPath::Nhej);
        let result = ce.run(1000);
        // 30% 效率 → 300 细胞
        assert!((result.on_target_efficiency - 0.30).abs() < 1e-6);
        assert_eq!(ce.edited_cells, 300);
        assert!(result.success);
    }
}
