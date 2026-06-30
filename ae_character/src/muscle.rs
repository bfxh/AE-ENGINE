use serde::{Deserialize, Serialize};

// ============================================================================
// 肌纤维三型分类 (Brooke & Kaiser 1970)
// ============================================================================

/// 肌纤维类型：基于 Brooke & Kaiser 1970 的组织化学分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MuscleFiberType {
    /// I 型 - 慢肌氧化纤维 (SO, Slow Oxidative)
    /// 线粒体/肌红蛋白高，抗疲劳，适合姿势维持
    SlowOxidative,
    /// IIa 型 - 快肌氧化糖酵解纤维 (FOG, Fast Oxidative Glycolytic)
    /// 中等特性，混合代谢
    FastOxidativeGlycolytic,
    /// IIx 型 - 快肌糖酵解纤维 (FG, Fast Glycolytic)
    /// 爆发力，易疲劳，肌红蛋白低（白色）
    FastGlycolytic,
}

impl MuscleFiberType {
    /// 最大收缩速度因子 (l_opt/s)
    /// I=1.0, IIa=4.0, IIx=8.0 (Brooke & Kaiser 1970)
    pub fn v_max_factor(&self) -> f32 {
        match self {
            MuscleFiberType::SlowOxidative => 1.0,
            MuscleFiberType::FastOxidativeGlycolytic => 4.0,
            MuscleFiberType::FastGlycolytic => 8.0,
        }
    }

    /// 最大等长收缩力系数
    /// I=1.0, IIa=1.2, IIx=1.5
    pub fn f_max_factor(&self) -> f32 {
        match self {
            MuscleFiberType::SlowOxidative => 1.0,
            MuscleFiberType::FastOxidativeGlycolytic => 1.2,
            MuscleFiberType::FastGlycolytic => 1.5,
        }
    }

    /// 抗疲劳性 (0.0-1.0)
    /// I=0.9 高抗疲劳, IIa=0.5 中等, IIx=0.2 易疲劳
    pub fn fatigue_resistance(&self) -> f32 {
        match self {
            MuscleFiberType::SlowOxidative => 0.9,
            MuscleFiberType::FastOxidativeGlycolytic => 0.5,
            MuscleFiberType::FastGlycolytic => 0.2,
        }
    }

    /// 线粒体密度 (相对值)
    /// 决定有氧代谢能力
    pub fn mitochondria_density(&self) -> f32 {
        match self {
            MuscleFiberType::SlowOxidative => 0.95,
            MuscleFiberType::FastOxidativeGlycolytic => 0.55,
            MuscleFiberType::FastGlycolytic => 0.20,
        }
    }

    /// 肌红蛋白含量 (相对值)
    /// 决定肌肉红色程度（I型红肌，IIx型白肌）
    pub fn myoglobin(&self) -> f32 {
        match self {
            MuscleFiberType::SlowOxidative => 0.90,
            MuscleFiberType::FastOxidativeGlycolytic => 0.45,
            MuscleFiberType::FastGlycolytic => 0.10,
        }
    }
}

// ============================================================================
// 基因映射 (ACTN3 R577X + MSTN Myostatin)
// ============================================================================

/// ACTN3 R577X 多态性基因型
/// 影响 α-actinin-3 表达，与爆发力/耐力表现相关
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Actn3Genotype {
    /// RR 纯合子 - α-actinin-3 正常表达，爆发力优势
    RR,
    /// RX 杂合子 - 混合表现型
    RX,
    /// XX 纯合子 - α-actinin-3 缺失，耐力优势
    XX,
}

/// MSTN (Myostatin) 基因型
/// Myostatin 抑制肌肉生长，缺失导致"双肌"表型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MyostatinGenotype {
    /// 正常 - Myostatin 表达，肌肉量正常
    Normal,
    /// Knockout - Myostatin 缺失，肌肉量显著增加
    Knockout,
}

/// 基因档案
/// 综合 ACTN3 与 MSTN 多态性对肌肉表型的影响
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneProfile {
    pub actn3: Actn3Genotype,
    pub myostatin: MyostatinGenotype,
}

impl GeneProfile {
    /// 最大收缩力乘数
    /// ACTN3: RR=+15%, RX=+5%, XX=基线
    /// MSTN KO: +15%
    /// 组合：RR+MSTN KO = 1.15*1.15 = 1.32
    pub fn f_max_multiplier(&self) -> f32 {
        let actn3_factor = match self.actn3 {
            Actn3Genotype::RR => 1.15,
            Actn3Genotype::RX => 1.05,
            Actn3Genotype::XX => 1.00,
        };
        let mstn_factor = match self.myostatin {
            MyostatinGenotype::Normal => 1.00,
            MyostatinGenotype::Knockout => 1.15,
        };
        actn3_factor * mstn_factor
    }

    /// 肌肉量乘数
    /// MSTN KO 导致肌肉量 +20-30% ("双肌"表型)
    pub fn muscle_mass_multiplier(&self) -> f32 {
        match self.myostatin {
            MyostatinGenotype::Normal => 1.00,
            MyostatinGenotype::Knockout => 1.25,
        }
    }

    /// 根据基因型调整肌纤维分布
    /// ACTN3 RR: IIx +10%, RX: IIx +5%, XX: IIx -5%
    /// MSTN KO: IIx +20%
    pub fn fiber_distribution_adjust(&self, base: FiberDistribution) -> FiberDistribution {
        let mut d = base;
        match self.actn3 {
            Actn3Genotype::RR => {
                let take = d.type_i.min(0.10);
                d.type_i -= take;
                d.type_ii_x += take;
            }
            Actn3Genotype::RX => {
                let take = d.type_i.min(0.05);
                d.type_i -= take;
                d.type_ii_x += take;
            }
            Actn3Genotype::XX => {
                let take = d.type_ii_x.min(0.05);
                d.type_ii_x -= take;
                d.type_i += take;
            }
        }
        if self.myostatin == MyostatinGenotype::Knockout {
            let take = d.type_i.min(0.20);
            d.type_i -= take;
            d.type_ii_x += take;
        }
        let sum = d.type_i + d.type_ii_a + d.type_ii_x;
        if sum > 0.0 {
            d.type_i /= sum;
            d.type_ii_a /= sum;
            d.type_ii_x /= sum;
        }
        d
    }
}

impl Default for GeneProfile {
    fn default() -> Self {
        Self {
            actn3: Actn3Genotype::RX,
            myostatin: MyostatinGenotype::Normal,
        }
    }
}

// ============================================================================
// 肌纤维分布
// ============================================================================

/// 肌纤维分布比例
/// 三型比例之和应为 1.0
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FiberDistribution {
    /// I 型比例 (0.0-1.0)
    pub type_i: f32,
    /// IIa 型比例 (0.0-1.0)
    pub type_ii_a: f32,
    /// IIx 型比例 (0.0-1.0)
    pub type_ii_x: f32,
}

impl FiberDistribution {
    /// 人体典型分布：I 50%, IIa 35%, IIx 15%
    pub fn default_human() -> Self {
        Self {
            type_i: 0.50,
            type_ii_a: 0.35,
            type_ii_x: 0.15,
        }
    }

    /// 耐力运动员：I 75%, IIa 20%, IIx 5%
    pub fn endurance_athlete() -> Self {
        Self {
            type_i: 0.75,
            type_ii_a: 0.20,
            type_ii_x: 0.05,
        }
    }

    /// 爆发力运动员：I 30%, IIa 40%, IIx 30%
    pub fn power_athlete() -> Self {
        Self {
            type_i: 0.30,
            type_ii_a: 0.40,
            type_ii_x: 0.30,
        }
    }

    /// 验证三项之和 ≈ 1.0 (容差 0.05)
    pub fn validate(&self) -> bool {
        let sum = self.type_i + self.type_ii_a + self.type_ii_x;
        (sum - 1.0).abs() < 0.05
    }
}

impl Default for FiberDistribution {
    fn default() -> Self {
        Self::default_human()
    }
}

// ============================================================================
// Hill 肌肉力学模型 (Hill 1938, Zajac 1989, Millard 2012)
// ============================================================================

/// Hill 三元件肌肉力学模型
/// 由收缩元件 (CE)、并联弹性元件 (PE)、串联弹性元件 (SE) 组成
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HillMuscle {
    /// 最优长度 l_opt (m)，力-长度曲线峰值处
    pub l_opt: f32,
    /// 静息长度 l_slack (m)，弹性元件 slack 长度
    pub l_slack: f32,
    /// 最大等长收缩力 F_max (N)
    pub f_max: f32,
    /// 最大收缩速度 v_max (l_opt/s)
    pub v_max: f32,
    /// Hill 常数 a = 0.25·F_max
    pub a_hill: f32,
    /// 肌纤维类型
    pub fiber_type: MuscleFiberType,
    /// 羽状角 (rad)，肌纤维与肌腱方向的夹角
    pub pennation_angle: f32,
}

impl HillMuscle {
    /// 创建新肌肉
    /// v_max 由肌纤维类型决定：v_max = v_max_factor · l_opt
    /// a_hill = 0.25 · F_max (Hill 1938)
    /// l_slack = 0.5 · l_opt（典型腱松弛长度）
    /// pennation_angle = 0（默认无羽状角）
    pub fn new(l_opt: f32, f_max: f32, fiber_type: MuscleFiberType) -> Self {
        let v_max = fiber_type.v_max_factor() * l_opt;
        Self {
            l_opt,
            l_slack: 0.5 * l_opt,
            f_max,
            v_max,
            a_hill: 0.25 * f_max,
            fiber_type,
            pennation_angle: 0.0,
        }
    }

    /// 力-长度曲线 f_l(l) (Gordon 1966 高斯近似)
    /// f_l(l) = exp(-((l/l_opt - 1)/0.3)²)
    /// 等距收缩平台宽度 ±0.3·l_opt
    pub fn force_length(&self, length: f32) -> f32 {
        if self.l_opt <= 0.0 {
            return 0.0;
        }
        let norm = length / self.l_opt - 1.0;
        (-((norm / 0.3).powi(2))).exp()
    }

    /// 力-速度曲线 g_v(v) (Hill 1938 双曲方程)
    /// 向心收缩 (v >= 0): g_v = (v_max - v) / (v_max + v·a/F_max)
    /// 离心收缩 (v < 0): g_v = 1.8 (Millard 2012 离心平台)
    pub fn force_velocity(&self, velocity: f32) -> f32 {
        if velocity < 0.0 {
            // 离心收缩 - 力增强平台
            return 1.8;
        }
        // 向心收缩 - Hill 双曲
        if self.v_max <= 0.0 {
            return 0.0;
        }
        let a_over_f = if self.f_max > 0.0 {
            self.a_hill / self.f_max
        } else {
            0.25
        };
        let denom = self.v_max + velocity * a_over_f;
        if denom <= 0.0 {
            return 0.0;
        }
        (self.v_max - velocity) / denom
    }

    /// 并联弹性元件力 F_PE (Millard 2012)
    /// F_PE = F_pe_max · exp(-(l_opt - l)/l_slack · k_pe)
    /// F_pe_max = 0.05·F_max, k_pe = 5.0
    pub fn parallel_elastic(&self, length: f32) -> f32 {
        if self.l_slack <= 0.0 {
            return 0.0;
        }
        let f_pe_max = 0.05 * self.f_max;
        let k_pe = 5.0;
        let arg = -(self.l_opt - length) / self.l_slack * k_pe;
        f_pe_max * arg.exp()
    }

    /// 串联弹性元件力 F_SE (Millard 2012)
    /// F_SE = F_se_max · (exp((l - l_slack)/l_slack · k_se) - 1)
    /// F_se_max = F_max（串联元件可承受最大主动力）, k_se = 5.0
    /// 当 l < l_slack 时返回 0（无压力）
    pub fn series_elastic(&self, length: f32) -> f32 {
        if self.l_slack <= 0.0 || length <= self.l_slack {
            return 0.0;
        }
        let f_se_max = self.f_max;
        let k_se = 5.0;
        let arg = (length - self.l_slack) / self.l_slack * k_se;
        let force = f_se_max * (arg.exp() - 1.0);
        force.max(0.0)
    }

    /// 总主动力 (收缩元件输出)
    /// F_CE = activation · f_l(l) · g_v(v) · F_max · cos(pennation)
    /// cos(pennation) 为羽状角对肌腱方向力的投影
    pub fn active_force(&self, length: f32, velocity: f32, activation: f32) -> f32 {
        let a = activation.clamp(0.0, 1.0);
        if a <= 0.0 {
            return 0.0;
        }
        let f_l = self.force_length(length);
        let g_v = self.force_velocity(velocity);
        let cos_penn = self.pennation_angle.cos();
        a * f_l * g_v * self.f_max * cos_penn
    }

    /// 总力 (主动 + 被动)
    /// F_total = F_CE + F_PE
    pub fn total_force(&self, length: f32, velocity: f32, activation: f32) -> f32 {
        let active = self.active_force(length, velocity, activation);
        let passive = self.parallel_elastic(length);
        active + passive
    }

    /// 应用疲劳 - 降低 F_max
    /// 疲劳速率 = (1 - fatigue_resistance) · intensity
    /// 反映肌纤维类型对疲劳的敏感性
    pub fn apply_fatigue(&mut self, duration: f32, intensity: f32) {
        let intensity = intensity.clamp(0.0, 1.0);
        if intensity <= 0.0 || duration <= 0.0 {
            return;
        }
        let fatigue_rate = (1.0 - self.fiber_type.fatigue_resistance()) * intensity * 0.1;
        let fatigue_factor = (1.0 - fatigue_rate * duration).max(0.1);
        self.f_max *= fatigue_factor;
    }
}

// ============================================================================
// 训练适应类型
// ============================================================================

/// 训练类型 - 决定肌纤维适应方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainingType {
    /// 耐力训练 - I 型比例增加
    Endurance,
    /// 力量训练 - IIx→IIa 转化，F_max 增加
    Strength,
    /// 增肌训练 - 肌纤维横截面积增加
    Hypertrophy,
    /// 爆发力训练 - IIx 型比例增加
    Power,
}

impl TrainingType {
    /// 应用训练适应到肌纤维分布
    /// weeks: 训练周数
    pub fn apply_to_distribution(&self, dist: &mut FiberDistribution, weeks: f32) {
        let shift = (weeks * 0.01).min(0.15);
        match self {
            TrainingType::Endurance => {
                // IIx → I, IIa → I
                let take_x = dist.type_ii_x.min(shift);
                dist.type_ii_x -= take_x;
                dist.type_i += take_x;
                let take_a = dist.type_ii_a.min(shift * 0.5);
                dist.type_ii_a -= take_a;
                dist.type_i += take_a;
            }
            TrainingType::Strength => {
                // IIx → IIa 转化 (典型力量训练适应)
                let take = dist.type_ii_x.min(shift);
                dist.type_ii_x -= take;
                dist.type_ii_a += take;
            }
            TrainingType::Hypertrophy => {
                // 横截面积增加为主，少量 IIx → IIa
                let take = dist.type_ii_x.min(shift * 0.3);
                dist.type_ii_x -= take;
                dist.type_ii_a += take;
            }
            TrainingType::Power => {
                // IIa → IIx, I → IIx (爆发力适应)
                let take_a = dist.type_ii_a.min(shift);
                dist.type_ii_a -= take_a;
                dist.type_ii_x += take_a;
                let take_i = dist.type_i.min(shift * 0.5);
                dist.type_i -= take_i;
                dist.type_ii_x += take_i;
            }
        }
        let sum = dist.type_i + dist.type_ii_a + dist.type_ii_x;
        if sum > 0.0 {
            dist.type_i /= sum;
            dist.type_ii_a /= sum;
            dist.type_ii_x /= sum;
        }
    }

    /// 肌肥大因子 - 肌纤维横截面积乘数
    /// Wolff 定律：长期训练也增强附着点骨密度 (+5-15%) 与肌腱强度 (+10-20%)
    pub fn hypertrophy_factor(&self, weeks: f32) -> f32 {
        match self {
            TrainingType::Hypertrophy => 1.0 + (weeks * 0.02).min(0.50),
            TrainingType::Strength => 1.0 + (weeks * 0.01).min(0.25),
            TrainingType::Power => 1.0 + (weeks * 0.015).min(0.30),
            TrainingType::Endurance => 1.0 + (weeks * 0.005).min(0.10),
        }
    }
}

// ============================================================================
// 肌肉系统
// ============================================================================

/// 肌肉系统 - 整合基因、肌纤维分布、疲劳与训练适应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuscularSystem {
    /// 肌肉集合
    pub muscles: Vec<HillMuscle>,
    /// 基因档案
    pub gene_profile: GeneProfile,
    /// 肌纤维分布
    pub fiber_distribution: FiberDistribution,
    /// 系统疲劳度 (0.0-1.0)
    pub fatigue: f32,
}

impl MuscularSystem {
    /// 创建新肌肉系统
    /// 根据基因档案调整默认肌纤维分布
    pub fn new(gene: GeneProfile) -> Self {
        let fiber_distribution = gene.fiber_distribution_adjust(FiberDistribution::default_human());
        Self {
            muscles: Vec::new(),
            gene_profile: gene,
            fiber_distribution,
            fatigue: 0.0,
        }
    }

    /// 步进更新
    /// dt: 时间步长 (s)
    /// activations: 各肌肉激活度 (0.0-1.0)，长度需与 muscles 匹配
    pub fn step(&mut self, dt: f32, activations: &[f32]) {
        let n = self.muscles.len();
        if n == 0 {
            return;
        }
        let mut avg_act = 0.0f32;
        for (i, muscle) in self.muscles.iter_mut().enumerate() {
            let act = activations.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            avg_act += act;
            if act > 0.0 {
                muscle.apply_fatigue(dt, act);
            }
        }
        avg_act /= n as f32;
        // 系统疲劳累积 (激活时) 与恢复 (静息时)
        let fatigue_rate = avg_act * 0.05;
        let recovery_rate = 0.02;
        self.fatigue = (self.fatigue + fatigue_rate * dt - recovery_rate * dt).clamp(0.0, 1.0);
    }

    /// 总力输出 - 所有肌肉在等长收缩 (l=l_opt, v=0) 下的最大力之和
    /// 考虑基因乘数与系统疲劳
    pub fn total_force_output(&self) -> f32 {
        let gene_mult = self.gene_profile.f_max_multiplier();
        let fatigue_mult = 1.0 - self.fatigue * 0.5;
        self.muscles
            .iter()
            .map(|m| m.f_max * gene_mult * fatigue_mult)
            .sum()
    }

    /// 应用长期训练适应
    /// weeks: 训练周数
    /// intensity: 训练强度 (0.0-1.0)，决定训练类型
    pub fn apply_training(&mut self, weeks: f32, intensity: f32) {
        let intensity = intensity.clamp(0.0, 1.0);
        let training_type = if intensity > 0.8 {
            TrainingType::Strength
        } else if intensity > 0.5 {
            TrainingType::Hypertrophy
        } else {
            TrainingType::Endurance
        };
        training_type.apply_to_distribution(&mut self.fiber_distribution, weeks);
        let hyper = training_type.hypertrophy_factor(weeks);
        let mass_mult = self.gene_profile.muscle_mass_multiplier();
        for m in &mut self.muscles {
            m.f_max *= hyper * mass_mult;
        }
        // 长期训练提升抗疲劳能力
        self.fatigue *= 0.95;
    }
}

impl Default for MuscularSystem {
    fn default() -> Self {
        Self::new(GeneProfile::default())
    }
}
