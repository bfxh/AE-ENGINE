//! 植物繁殖模块
//!
//! 涵盖有性繁殖（花器官、传粉、受精、种子发育）、无性繁殖（营养繁殖与克隆生长）、
//! 传粉综合征（pollination syndromes）、种子扩散（seed dispersal）、生活史策略
//! （r/K 选择）、繁殖分配（reproductive allocation）与交配系统（mating system）。
//!
//! 所有数值计算采用 f32，便于与渲染/物理系统对接。

use serde::{Deserialize, Serialize};

// ============================================================================
// 繁殖方式
// ============================================================================

/// 繁殖方式大类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReproductionMode {
    /// 有性繁殖（经减数分裂与配子融合）
    Sexual,
    /// 无性繁殖（营养体或无融合生殖）
    Asexual,
    /// 混合繁殖（同时具备有性与无性途径）
    Mixed,
}

/// 无性繁殖具体模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AsexualMode {
    /// 匍匐茎（stolon）：地表横向蔓延，如草莓
    Stolon,
    /// 根状茎（rhizome）：地下横向茎，如竹类
    Rhizome,
    /// 块茎（tuber）：地下茎膨大储藏，如马铃薯
    Tuber,
    /// 鳞茎（bulb）：肉质鳞片叶包裹，如洋葱
    Bulb,
    /// 球茎（corm）：实心地下茎，如番红花
    Corm,
    /// 根蘖（sucker）：从侧根不定芽萌发，如杨树
    Sucker,
    /// 无融合生殖（apomixis）：未经受精产生种子，如蒲公英
    Apomixis,
    /// 断裂繁殖（fragmentation）：植物体片段再生，如落地生根
    Fragmentation,
}

impl AsexualMode {
    /// 返回该无性模式的典型年扩散半径基数（米）
    pub fn base_spread_radius_m(self) -> f32 {
        match self {
            AsexualMode::Stolon => 1.2,
            AsexualMode::Rhizome => 0.8,
            AsexualMode::Tuber => 0.3,
            AsexualMode::Bulb => 0.15,
            AsexualMode::Corm => 0.2,
            AsexualMode::Sucker => 2.5,
            AsexualMode::Apomixis => 0.0,  // 无融合靠种子扩散，不在此处计算
            AsexualMode::Fragmentation => 0.5,
        }
    }

    /// 该模式对温度的响应曲线最适值（摄氏度）
    pub fn optimal_temp_c(self) -> f32 {
        match self {
            AsexualMode::Stolon => 22.0,
            AsexualMode::Rhizome => 18.0,
            AsexualMode::Tuber => 16.0,
            AsexualMode::Bulb => 20.0,
            AsexualMode::Corm => 21.0,
            AsexualMode::Sucker => 24.0,
            AsexualMode::Apomixis => 23.0,
            AsexualMode::Fragmentation => 25.0,
        }
    }
}

// ============================================================================
// 传粉综合征
// ============================================================================

/// 传粉综合征（pollination syndrome）
///
/// 花部特征与传粉媒介协同进化形成的形态/生理组合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PollinationSyndrome {
    /// 风媒花：花小、无花被、花粉量大、柱头羽毛状
    Wind,
    /// 虫媒花（蜂/蝶/蛾/蝇）：花被鲜艳、有蜜腺
    Insect,
    /// 鸟媒花：红色管状花、花蜜稀薄量大
    Bird,
    /// 蝙蝠媒花：夜间开放、白色大型、霉味
    Bat,
    /// 水媒花：沉水或浮水传粉
    Water,
    /// 自花传粉：闭花或同株异花
    SelfPollination,
}

impl PollinationSyndrome {
    /// 基础传粉成功率（无外界影响下的理论值）
    pub fn baseline_success(self) -> f32 {
        match self {
            PollinationSyndrome::Wind => 0.25,
            PollinationSyndrome::Insect => 0.55,
            PollinationSyndrome::Bird => 0.6,
            PollinationSyndrome::Bat => 0.5,
            PollinationSyndrome::Water => 0.35,
            PollinationSyndrome::SelfPollination => 0.85,
        }
    }

    /// 该综合征对传粉者活动的依赖权重（0=完全自交，1=绝对依赖）
    pub fn pollinator_dependency(self) -> f32 {
        match self {
            PollinationSyndrome::Wind => 0.2,
            PollinationSyndrome::Insect => 0.85,
            PollinationSyndrome::Bird => 0.9,
            PollinationSyndrome::Bat => 0.9,
            PollinationSyndrome::Water => 0.1,
            PollinationSyndrome::SelfPollination => 0.0,
        }
    }

    /// Allee 效应中花密度对该综合征的影响系数
    pub fn density_sensitivity(self) -> f32 {
        match self {
            PollinationSyndrome::Wind => 0.3,
            PollinationSyndrome::Insect => 0.7,
            PollinationSyndrome::Bird => 0.6,
            PollinationSyndrome::Bat => 0.5,
            PollinationSyndrome::Water => 0.4,
            PollinationSyndrome::SelfPollination => 0.05,
        }
    }
}

// ============================================================================
// 交配系统
// ============================================================================

/// 交配系统（mating system）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatingSystem {
    /// 异交（outcrossing）：主要通过不同个体间交配
    Outcrossing,
    /// 自交（selfing）：同花或同株授粉
    Selfing,
    /// 混合交配：自交与异交并存
    Mixed,
    /// 雌雄异株（dioecious）：雌花雄花在不同植株
    Dioecious,
    /// 雌雄同株（monoecious）：雌花雄花同株
    Monoecious,
    /// 雌全异株（gynodioecious）：雌株与两性株共存
    Gynodioecious,
}

impl MatingSystem {
    /// 默认自交率（outcrossing rate 的补数）
    pub fn default_selfing_rate(self) -> f32 {
        match self {
            MatingSystem::Outcrossing => 0.1,
            MatingSystem::Selfing => 0.9,
            MatingSystem::Mixed => 0.4,
            MatingSystem::Dioecious => 0.0,
            MatingSystem::Monoecious => 0.3,
            MatingSystem::Gynodioecious => 0.2,
        }
    }
}

// ============================================================================
// 种子扩散
// ============================================================================

/// 种子扩散模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DispersalMode {
    /// 风力传播（anemochory）
    Wind,
    /// 水力传播（hydrochory）
    Water,
    /// 动物传播（zoochory）：外附或吞食
    Animal,
    /// 重力传播（barochory）
    Gravity,
    /// 弹射传播（autochory/ballochory）
    Explosive,
    /// 人为传播（anthropochory）
    Human,
}

/// 扩散参数集
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispersalParams {
    pub mode: DispersalMode,
    /// 种子质量（毫克）
    pub seed_mass_mg: f32,
    /// 终端速度（m/s），重力下落平衡速度
    pub terminal_velocity_ms: f32,
    /// 最大扩散距离（米）
    pub max_distance_m: f32,
    /// 扩散核参数（控制尾部分布形状）
    pub dispersal_kernels: f32,
}

impl Default for DispersalParams {
    fn default() -> Self {
        Self {
            mode: DispersalMode::Wind,
            seed_mass_mg: 0.5,
            terminal_velocity_ms: 0.6,
            max_distance_m: 100.0,
            dispersal_kernels: 2.0,
        }
    }
}

// ============================================================================
// 花与种子
// ============================================================================

/// 花器官描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flower {
    /// 花瓣数量
    pub petal_count: u32,
    /// 花瓣颜色 RGB 0..1
    pub petal_color: [f32; 3],
    /// 花蜜量（微升 μL）
    pub nectar_volume_ul: f32,
    /// 花蜜糖浓度（百分比 %）
    pub nectar_sugar_pct: f32,
    /// 花粉粒数
    pub pollen_grains: u32,
    /// 花期持续天数
    pub flowering_duration_d: f32,
    /// 传粉综合征
    pub syndrome: PollinationSyndrome,
}

impl Default for Flower {
    fn default() -> Self {
        Self {
            petal_count: 5,
            petal_color: [1.0, 0.8, 0.2],
            nectar_volume_ul: 2.0,
            nectar_sugar_pct: 30.0,
            pollen_grains: 10000,
            flowering_duration_d: 7.0,
            syndrome: PollinationSyndrome::Insect,
        }
    }
}

/// 种子休眠类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DormancyType {
    /// 无休眠
    None,
    /// 物理休眠（硬种皮不透水）
    Physical,
    /// 生理休眠（胚需后熟）
    Physiological,
    /// 形态休眠（胚未发育完全）
    Morphological,
    /// 形态生理休眠（兼具以上两种）
    Morphophysiological,
}

/// 种子描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seed {
    /// 种子质量（毫克）
    pub mass_mg: f32,
    /// 活力（0..1）
    pub viability: f32,
    /// 休眠类型
    pub dormancy_type: DormancyType,
    /// 在适宜条件下的萌发率（0..1）
    pub germination_rate: f32,
}

impl Default for Seed {
    fn default() -> Self {
        Self {
            mass_mg: 1.0,
            viability: 0.9,
            dormancy_type: DormancyType::None,
            germination_rate: 0.8,
        }
    }
}

// ============================================================================
// 生活史策略与繁殖分配
// ============================================================================

/// 生活史策略（r/K 选择理论）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifeHistoryStrategy {
    /// r-策略：后代数量多、种子小、繁殖早、扩散强
    RSelected,
    /// K-策略：后代数量少、种子大、繁殖晚、竞争强
    KSelected,
    /// 中间型
    Intermediate,
}

impl LifeHistoryStrategy {
    /// 繁殖成熟年龄基数（天）
    pub fn maturation_age_d(self) -> f32 {
        match self {
            LifeHistoryStrategy::RSelected => 30.0,
            LifeHistoryStrategy::KSelected => 365.0 * 3.0,
            LifeHistoryStrategy::Intermediate => 180.0,
        }
    }

    /// 单株最大繁殖分配比例（盛花期）
    pub fn max_reproductive_fraction(self) -> f32 {
        match self {
            LifeHistoryStrategy::RSelected => 0.55,
            LifeHistoryStrategy::KSelected => 0.25,
            LifeHistoryStrategy::Intermediate => 0.40,
        }
    }
}

/// 繁殖分配结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproductiveAllocation {
    /// 营养生长分配比例（0..1）
    pub vegetative_fraction: f32,
    /// 繁殖总分配比例（0..1）
    pub reproductive_fraction: f32,
    /// 花器官分配（占总繁殖分配的一部分）
    pub flower_fraction: f32,
    /// 果实分配
    pub fruit_fraction: f32,
    /// 种子分配
    pub seed_fraction: f32,
}

impl Default for ReproductiveAllocation {
    fn default() -> Self {
        Self {
            vegetative_fraction: 0.7,
            reproductive_fraction: 0.3,
            flower_fraction: 0.1,
            fruit_fraction: 0.1,
            seed_fraction: 0.1,
        }
    }
}

// ============================================================================
// 关键函数实现
// ============================================================================

/// 传粉成功率
///
/// 综合考虑：
/// - 传粉综合征基础成功率
/// - 传粉者活动强度（0..1，1 表示最活跃）
/// - 花密度（每平方米花数）通过 Allee 效应影响
///
/// 返回值 0..1
pub fn pollination_success(
    syndrome: PollinationSyndrome,
    pollinator_activity: f32,
    flower_density: f32,
) -> f32 {
    let baseline = syndrome.baseline_success();
    let dependency = syndrome.pollinator_dependency();
    let density_sens = syndrome.density_sensitivity();

    // 传粉者活动贡献（与依赖度成正比）
    let activity_term = dependency * pollinator_activity.clamp(0.0, 1.0);

    // Allee 效应：低密度时传粉受阻，使用 Michaelis-Menten 形式
    // 当 flower_density = k_half 时贡献为 0.5
    let k_half = 5.0_f32; // 半饱和花密度（花/m²）
    let density_term = density_sens * (flower_density / (flower_density + k_half));

    // 自交部分不依赖传粉者
    let self_term = (1.0 - dependency) * 0.9;

    let raw = baseline * (activity_term + density_term + self_term) / 1.5;
    raw.clamp(0.0, 1.0)
}

/// 种子扩散距离分布（幂律衰减核）
///
/// 模型：distance = max_distance * (1 - exp(-k * wind)) * kernel_factor
/// 其中 kernel_factor 反映种子质量与终端速度的相对关系：
///   - 质量小、终端速度低 → 风传更远
///   - 模式非风力时衰减距离
///
/// 返回单次扩散事件的预期距离（米）
pub fn seed_dispersal_distance(params: &DispersalParams, wind_speed: f32) -> f32 {
    // 质量与终端速度共同决定"易飘浮度"
    let buoyancy = if params.terminal_velocity_ms > 0.0 {
        1.0 / (1.0 + params.terminal_velocity_ms)
    } else {
        1.0
    };

    // 种子质量影响（mg 越小飘得越远），用对数尺度
    let mass_factor = if params.seed_mass_mg > 0.0 {
        1.0 / (1.0 + params.seed_mass_mg.ln().max(0.0) * 0.1)
    } else {
        1.0
    };

    // 风速指数衰减
    let wind_factor = 1.0 - (-wind_speed * 0.3).exp();

    // 模式修正：非风传模式扩散半径显著降低
    let mode_factor = match params.mode {
        DispersalMode::Wind => 1.0,
        DispersalMode::Water => 0.7,
        DispersalMode::Animal => 0.5,
        DispersalMode::Gravity => 0.1,
        DispersalMode::Explosive => 0.15,
        DispersalMode::Human => 0.4,
    };

    // 扩散核参数调节尾部
    let kernel = params.dispersal_kernels.max(0.1);

    let distance = params.max_distance_m
        * wind_factor
        * buoyancy
        * mass_factor
        * mode_factor
        * (1.0 / kernel);

    distance.clamp(0.0, params.max_distance_m)
}

/// 种子萌发概率
///
/// 受温度（高斯响应）、水分（饱和响应）、光照（阈值响应）、
/// 休眠类型与种子自身活力共同影响。
///
/// - temp_c: 环境温度（摄氏度）
/// - moisture: 0..1 土壤含水量
/// - light: 0..1 光照强度
pub fn germination_probability(seed: &Seed, temp_c: f32, moisture: f32, light: f32) -> f32 {
    // 种子活力上限
    let viability = seed.viability.clamp(0.0, 1.0);

    // 温度响应：最适 20°C，标准差 8°C 的高斯曲线
    let optimal_temp = 20.0_f32;
    let temp_sd = 8.0_f32;
    let temp_resp = (-((temp_c - optimal_temp).powi(2)) / (2.0 * temp_sd * temp_sd)).exp();

    // 水分响应：Michaelis-Menten 饱和
    let moisture_k = 0.3;
    let moisture_resp = moisture / (moisture + moisture_k);

    // 光照响应：阈值型，>0.2 后趋近 1
    let light_resp = if light < 0.2 {
        light / 0.2 * 0.5
    } else {
        0.5 + (light - 0.2) * 0.625
    };

    // 休眠抑制因子
    let dormancy_inhibition = match seed.dormancy_type {
        DormancyType::None => 1.0,
        DormancyType::Physical => 0.4,       // 需机械破皮
        DormancyType::Physiological => 0.5,  // 需低温层积
        DormancyType::Morphological => 0.6,  // 需胚后熟
        DormancyType::Morphophysiological => 0.25,
    };

    let raw = seed.germination_rate
        * viability
        * temp_resp
        * moisture_resp
        * light_resp
        * dormancy_inhibition;

    raw.clamp(0.0, 1.0)
}

/// 繁殖分配
///
/// 根据生活史策略、植物年龄（天）与资源可利用性（0..1）计算分配比例。
/// 幼年期几乎全部分配给营养生长；成熟后逐步过渡到繁殖分配。
pub fn reproductive_allocation(
    strategy: LifeHistoryStrategy,
    age_d: f32,
    resource_availability: f32,
) -> ReproductiveAllocation {
    let maturation = strategy.maturation_age_d();
    let max_repro = strategy.max_reproductive_fraction();

    // 成熟度曲线：logistic 平滑过渡
    // age << maturation 时 → 0；age >> maturation 时 → 1
    let maturity = 1.0 / (1.0 + (-(age_d - maturation) / (maturation * 0.25 + 1.0)).exp());

    // 资源胁迫下繁殖分配减少（资源保存策略）
    let resource_factor = resource_availability.clamp(0.0, 1.0).powf(0.7);

    let repro = max_repro * maturity * resource_factor;
    let vegetative = 1.0 - repro;

    // 繁殖分配内部再分配（花/果/种子）
    // r-策略偏向花和种子（多而小）；K-策略偏向果实（少而精）
    let (flower_f, fruit_f, seed_f) = match strategy {
        LifeHistoryStrategy::RSelected => (0.35, 0.20, 0.45),
        LifeHistoryStrategy::KSelected => (0.20, 0.45, 0.35),
        LifeHistoryStrategy::Intermediate => (0.30, 0.30, 0.40),
    };

    ReproductiveAllocation {
        vegetative_fraction: vegetative,
        reproductive_fraction: repro,
        flower_fraction: repro * flower_f,
        fruit_fraction: repro * fruit_f,
        seed_fraction: repro * seed_f,
    }
}

/// 克隆生长速率（无性繁殖扩散）
///
/// 返回单株每日扩散半径增量（米/天）。
/// 受资源（0..1）、温度（摄氏度）与无性模式共同影响。
pub fn clonal_growth_rate(mode: AsexualMode, resource: f32, temp_c: f32) -> f32 {
    let base = mode.base_spread_radius_m();

    // 资源响应：低资源时几乎停止
    let resource_resp = resource.clamp(0.0, 1.0).powf(1.5);

    // 温度响应：在最适温度附近高斯峰
    let opt = mode.optimal_temp_c();
    let temp_sd = 6.0_f32;
    let temp_resp = (-((temp_c - opt).powi(2)) / (2.0 * temp_sd * temp_sd)).exp();

    // 无融合生殖不产生营养体扩散
    if mode == AsexualMode::Apomixis {
        return 0.0;
    }

    let daily = base * 0.05 * resource_resp * temp_resp;
    daily.max(0.0)
}

/// 种群增长率（Logistic 模型）
///
/// dN/dt = r * N * (1 - N/K)
///
/// 返回当前种群单位时间增长量（个体数）。
/// 当 N 接近 K 时，增长率趋近 0；当 N > K 时，增长率为负（负增长）。
pub fn population_growth_rate(r_max: f32, n: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return 0.0;
    }
    let n_safe = n.max(0.0);
    r_max * n_safe * (1.0 - n_safe / k)
}

/// 近交衰退系数
///
/// 经典模型：δ = 1 - exp(-a * F * S)
/// 其中：
/// - selfing_rate：自交率（0..1）
/// - inbreeding_coefficient：近交系数 F（0..1）
///
/// 返回 0..1，0 表示无近交衰退，1 表示完全衰退。
pub fn inbreeding_depression(selfing_rate: f32, inbreeding_coefficient: f32) -> f32 {
    let s = selfing_rate.clamp(0.0, 1.0);
    let f = inbreeding_coefficient.clamp(0.0, 1.0);
    // 经验系数 a：典型自交衰退载荷
    let a = 2.5;
    let delta = 1.0 - (-a * f * s).exp();
    delta.clamp(0.0, 1.0)
}

// ============================================================================
// 高层组合接口
// ============================================================================

/// 单株繁殖产出（种子数）
///
/// 综合花数、传粉成功率、繁殖分配与资源情况，估算单株年产种子数。
pub fn estimate_annual_seed_yield(
    flower: &Flower,
    pollinator_activity: f32,
    flower_density: f32,
    allocation: &ReproductiveAllocation,
    resource: f32,
) -> u32 {
    let success = pollination_success(flower.syndrome, pollinator_activity, flower_density);
    // 每花平均可产生的种子数（与花粉量和资源相关）
    let base_seeds_per_flower = (flower.pollen_grains as f32 / 1000.0).min(20.0);
    // 花期持续天数折算的有效花数
    let effective_flowers = (flower.flowering_duration_d * 0.5).max(1.0);
    // 资源限制总产出
    let resource_cap = resource.clamp(0.0, 1.0) * 2000.0;

    let raw_yield = base_seeds_per_flower
        * effective_flowers
        * success
        * (allocation.seed_fraction * 10.0)
        * resource_cap;

    raw_yield.max(0.0) as u32
}

/// 估算种群世代时间（天）
///
/// 受生活史策略和资源可利用性影响。
pub fn estimate_generation_time(strategy: LifeHistoryStrategy, resource: f32) -> f32 {
    let base = strategy.maturation_age_d();
    let r = resource.clamp(0.0, 1.0);
    // 资源充足时缩短世代时间
    base * (1.5 - 0.5 * r)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pollination_success() {
        // 高传粉者活动时成功率应显著高于低活动
        let low = pollination_success(PollinationSyndrome::Insect, 0.1, 5.0);
        let high = pollination_success(PollinationSyndrome::Insect, 0.9, 5.0);
        assert!(
            high > low,
            "高传粉者活动应带来更高传粉成功率: high={} low={}",
            high,
            low
        );
        assert!(high <= 1.0 && low >= 0.0, "传粉成功率应在 [0,1] 区间");

        // 自花传粉对传粉者活动不敏感
        let self_low = pollination_success(PollinationSyndrome::SelfPollination, 0.1, 5.0);
        let self_high = pollination_success(PollinationSyndrome::SelfPollination, 0.9, 5.0);
        let diff = (self_high - self_low).abs();
        assert!(
            diff < 0.15,
            "自花传粉对传粉者活动应不敏感: diff={}",
            diff
        );
    }

    #[test]
    fn test_seed_dispersal() {
        let params = DispersalParams {
            mode: DispersalMode::Wind,
            seed_mass_mg: 0.3,
            terminal_velocity_ms: 0.4,
            max_distance_m: 200.0,
            dispersal_kernels: 2.0,
        };
        let low_wind = seed_dispersal_distance(&params, 1.0);
        let high_wind = seed_dispersal_distance(&params, 10.0);
        assert!(
            high_wind > low_wind,
            "风速越大传播应越远: high={} low={}",
            high_wind,
            low_wind
        );
        assert!(high_wind <= params.max_distance_m, "不应超过最大距离");
        assert!(low_wind >= 0.0, "距离非负");

        // 重力模式扩散距离应远小于风传
        let gravity_params = DispersalParams {
            mode: DispersalMode::Gravity,
            ..params.clone()
        };
        let gravity_dist = seed_dispersal_distance(&gravity_params, 10.0);
        assert!(
            gravity_dist < high_wind,
            "重力传播应短于风传: gravity={} wind={}",
            gravity_dist,
            high_wind
        );
    }

    #[test]
    fn test_germination() {
        let seed = Seed {
            mass_mg: 1.0,
            viability: 0.95,
            dormancy_type: DormancyType::None,
            germination_rate: 0.9,
        };
        // 适宜条件：20°C、湿润、光照充足
        let good = germination_probability(&seed, 20.0, 0.6, 0.8);
        // 恶劣条件：极端高温、干燥、黑暗
        let bad = germination_probability(&seed, 45.0, 0.05, 0.05);
        assert!(
            good > bad,
            "适宜条件下萌发率应更高: good={} bad={}",
            good,
            bad
        );
        assert!(good > 0.3, "适宜条件下萌发率应较高: {}", good);
        assert!(bad < good, "恶劣条件下萌发率应较低");
        assert!(good <= 1.0 && bad >= 0.0, "萌发率应在 [0,1]");

        // 休眠显著抑制萌发
        let dormant_seed = Seed {
            dormancy_type: DormancyType::Morphophysiological,
            ..seed.clone()
        };
        let dormant = germination_probability(&dormant_seed, 20.0, 0.6, 0.8);
        assert!(
            dormant < good,
            "休眠种子萌发率应低于无休眠种子: dormant={} good={}",
            dormant,
            good
        );
    }

    #[test]
    fn test_reproductive_allocation() {
        // K-策略繁殖分配应少于 r-策略（成年个体）
        let r_alloc = reproductive_allocation(
            LifeHistoryStrategy::RSelected,
            60.0,  // 已成熟
            0.8,
        );
        let k_alloc = reproductive_allocation(
            LifeHistoryStrategy::KSelected,
            1500.0,  // K 策略成熟更晚，给充足年龄
            0.8,
        );
        assert!(
            r_alloc.reproductive_fraction > k_alloc.reproductive_fraction,
            "r-策略繁殖分配应高于 K-策略: r={} k={}",
            r_alloc.reproductive_fraction,
            k_alloc.reproductive_fraction
        );

        // 幼苗期几乎不分配给繁殖
        let juvenile = reproductive_allocation(
            LifeHistoryStrategy::RSelected,
            5.0,  // 幼苗
            0.8,
        );
        assert!(
            juvenile.reproductive_fraction < 0.05,
            "幼苗期繁殖分配应极低: {}",
            juvenile.reproductive_fraction
        );

        // 分配总和合理：营养 + 繁殖 ≈ 1
        let total = r_alloc.vegetative_fraction + r_alloc.reproductive_fraction;
        assert!(
            (total - 1.0).abs() < 1e-5,
            "营养与繁殖分配总和应为 1: {}",
            total
        );

        // 花/果/种子分配之和应等于繁殖分配
        let repro_sum = r_alloc.flower_fraction + r_alloc.fruit_fraction + r_alloc.seed_fraction;
        assert!(
            (repro_sum - r_alloc.reproductive_fraction).abs() < 1e-5,
            "花果种子分配之和应等于繁殖总分配: {} vs {}",
            repro_sum,
            r_alloc.reproductive_fraction
        );
    }

    #[test]
    fn test_population_growth() {
        // N 远小于 K 时接近指数增长
        let r = 0.5;
        let k = 1000.0;
        let low_n = population_growth_rate(r, 10.0, k);
        let mid_n = population_growth_rate(r, 500.0, k);
        let near_k = population_growth_rate(r, 990.0, k);
        let at_k = population_growth_rate(r, 1000.0, k);
        let over_k = population_growth_rate(r, 1200.0, k);

        // N 接近 K 时增长率趋近 0
        assert!(
            at_k.abs() < 1e-3,
            "N=K 时增长率应趋近 0: {}",
            at_k
        );
        assert!(
            near_k < mid_n,
            "N 接近 K 时增长率应小于中段: near={} mid={}",
            near_k,
            mid_n
        );
        // 超过 K 时负增长
        assert!(
            over_k < 0.0,
            "N>K 时应为负增长: {}",
            over_k
        );
        // 远小于 K 时增长率正且较大
        assert!(low_n > 0.0 && low_n < mid_n, "低密度增长率应大于0且小于中段");
    }

    #[test]
    fn test_clonal_growth() {
        // 资源充足时长势更好
        let poor = clonal_growth_rate(AsexualMode::Stolon, 0.1, 22.0);
        let rich = clonal_growth_rate(AsexualMode::Stolon, 0.9, 22.0);
        assert!(rich > poor, "资源充足时长势更好: rich={} poor={}", rich, poor);

        // 最适温度时长势最佳
        let cold = clonal_growth_rate(AsexualMode::Rhizome, 0.8, 5.0);
        let optimal = clonal_growth_rate(AsexualMode::Rhizome, 0.8, 18.0);
        assert!(optimal > cold, "最适温度时长势最佳: optimal={} cold={}", optimal, cold);

        // 无融合生殖不产生营养扩散
        let apo = clonal_growth_rate(AsexualMode::Apomixis, 1.0, 25.0);
        assert!(apo == 0.0, "无融合生殖不应有营养扩散: {}", apo);
    }

    #[test]
    fn test_inbreeding_depression() {
        // 高自交率 + 高近交系数 → 高衰退
        let low = inbreeding_depression(0.1, 0.1);
        let high = inbreeding_depression(0.9, 0.5);
        assert!(high > low, "高自交应导致更高衰退: high={} low={}", high, low);
        assert!(high <= 1.0 && low >= 0.0, "衰退系数应在 [0,1]");
        // 完全异交（自交率=0）应无近交衰退
        let none = inbreeding_depression(0.0, 0.5);
        assert!(none.abs() < 1e-6, "完全异交应无近交衰退: {}", none);
    }
}
