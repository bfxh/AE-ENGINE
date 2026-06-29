//! 植物生态学模块
//!
//! 涵盖种群动态、群落生态、生态位理论、生物多样性指数、生态系统功能与物种互作网络。
//! 该模块为 wasteland_botany crate 提供植物群落建模与生态过程模拟的核心抽象。

use serde::{Deserialize, Serialize};

// ============================================================================
// 种群动态
// ============================================================================

/// 种群结构（按年龄/大小阶级划分）
///
/// 采用 Leslie 矩阵模型描述阶段化种群，结合 Logistic 增长的环境容纳量约束。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    /// 物种名
    pub species_name: String,
    /// 各大小/年龄阶级个体数（顺序：幼龄 → 成龄 → 老龄）
    pub size_classes: Vec<u32>,
    /// 各阶级存活率（0..=1）
    pub survival_rates: Vec<f32>,
    /// 各阶级繁殖力（每个体产生的后代数）
    pub fecundity_rates: Vec<f32>,
    /// 环境容纳量 K
    pub carrying_capacity: f32,
    /// 内禀增长率 r
    pub growth_rate: f32,
}

impl Population {
    /// 总个体数
    pub fn total(&self) -> u32 {
        self.size_classes.iter().sum()
    }

    /// 当前种群规模占容纳量比例（0..1+）
    pub fn carrying_ratio(&self) -> f32 {
        if self.carrying_capacity <= 0.0 {
            return 0.0;
        }
        self.total() as f32 / self.carrying_capacity
    }
}

/// Leslie 矩阵种群投影（一步）
///
/// 构造 L = [f0 f1 ... fn; s0 0 ... 0; 0 s1 ... 0; ...] 并左乘当前阶级向量。
/// 对繁殖力施加 Logistic 阻尼以反映密度依赖。
pub fn leslie_projection(population: &Population) -> Population {
    let n = population.size_classes.len();
    if n == 0 {
        return population.clone();
    }

    // 密度依赖阻尼：N/K 越接近 1，繁殖力越低
    let ratio = population.carrying_ratio();
    let dampening = 1.0 / (1.0 + ratio.max(0.0));

    let mut next_classes = vec![0u32; n];

    // 第一行：新生幼体 = Σ (fecundity_i * size_i) * dampening
    let mut newborns = 0.0f32;
    for i in 0..n {
        let f = population.fecundity_rates.get(i).copied().unwrap_or(0.0);
        newborns += f * population.size_classes[i] as f32 * dampening;
    }
    next_classes[0] = newborns.round().max(0.0) as u32;

    // 存活转移：next[i+1] = size[i] * survival[i]
    for i in 0..n.saturating_sub(1) {
        let s = population.survival_rates.get(i).copied().unwrap_or(0.0);
        let survived = population.size_classes[i] as f32 * s;
        next_classes[i + 1] = survived.round().max(0.0) as u32;
    }

    // 末阶级存活个体保留在原阶级（避免被丢弃）
    if let Some(last_s) = population.survival_rates.get(n - 1).copied() {
        let retained = population.size_classes[n - 1] as f32 * last_s;
        next_classes[n - 1] = next_classes[n - 1].saturating_add(retained.round() as u32);
    }

    Population {
        species_name: population.species_name.clone(),
        size_classes: next_classes,
        survival_rates: population.survival_rates.clone(),
        fecundity_rates: population.fecundity_rates.clone(),
        carrying_capacity: population.carrying_capacity,
        growth_rate: population.growth_rate,
    }
}

/// Logistic 种群增长
///
/// dN/dt = r * N * (1 - N/K)
/// 采用显式欧拉积分：N(t+dt) = N + r*N*(1 - N/K)*dt
pub fn logistic_growth(n: f32, r: f32, k: f32, dt: f32) -> f32 {
    if k <= 0.0 {
        return (n + r * n * dt).max(0.0);
    }
    let dn = r * n * (1.0 - n / k) * dt;
    (n + dn).max(0.0)
}

/// Lotka-Volterra 竞争模型（物种 1）
///
/// dN1/dt = r1 * N1 * (1 - (N1 + alpha12 * N2) / K1)
pub fn lotka_volterra_competition(
    n1: f32,
    n2: f32,
    r1: f32,
    k1: f32,
    alpha12: f32,
    dt: f32,
) -> f32 {
    if k1 <= 0.0 {
        return n1.max(0.0);
    }
    let dn = r1 * n1 * (1.0 - (n1 + alpha12 * n2) / k1) * dt;
    (n1 + dn).max(0.0)
}

/// Lotka-Volterra 互利共生模型（物种 1）
///
/// dN1/dt = r1 * N1 * (1 - N1 / (K1 + beta12 * N2))
/// 互利者存在时等效容纳量提升。
pub fn lotka_volterra_mutualism(
    n1: f32,
    n2: f32,
    r1: f32,
    k1: f32,
    beta12: f32,
    dt: f32,
) -> f32 {
    let effective_k = k1 + beta12 * n2.max(0.0);
    if effective_k <= 0.0 {
        return n1.max(0.0);
    }
    let dn = r1 * n1 * (1.0 - n1 / effective_k) * dt;
    (n1 + dn).max(0.0)
}

// ============================================================================
// 物种互作
// ============================================================================

/// 物种互作类型（按作用方向分类）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InteractionType {
    /// 竞争 (-,-)
    Competition,
    /// 互利共生 (+,+)
    Mutualism,
    /// 偏利共生 (+,0)
    Commensalism,
    /// 寄生 (+,-)
    Parasitism,
    /// 偏害共生 (-,0)
    Amensalism,
    /// 中性 (0,0)
    Neutralism,
}

impl InteractionType {
    /// 返回 (对 A 的影响符号, 对 B 的影响符号)，正为有利，负为有害
    pub fn effect_signs(self) -> (i8, i8) {
        match self {
            InteractionType::Competition => (-1, -1),
            InteractionType::Mutualism => (1, 1),
            InteractionType::Commensalism => (1, 0),
            InteractionType::Parasitism => (-1, 1),
            InteractionType::Amensalism => (-1, 0),
            InteractionType::Neutralism => (0, 0),
        }
    }

    /// 是否为不对称关系（一方获利一方受损）
    pub fn is_asymmetric(self) -> bool {
        matches!(
            self,
            InteractionType::Commensalism | InteractionType::Parasitism | InteractionType::Amensalism
        )
    }
}

// ============================================================================
// 群落
// ============================================================================

/// 物种丰度记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesAbundance {
    /// 物种名
    pub name: String,
    /// 个体数
    pub count: u32,
    /// 生物量（克）
    pub biomass_g: f32,
    /// 相对丰度（0..1）
    pub relative_abundance: f32,
}

/// 物种互作记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesInteraction {
    /// 物种 A 索引（指向 community.species）
    pub species_a: usize,
    /// 物种 B 索引
    pub species_b: usize,
    /// 互作类型
    pub interaction: InteractionType,
    /// 互作强度（0..1）
    pub strength: f32,
}

/// 群落
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    /// 物种丰度列表
    pub species: Vec<SpeciesAbundance>,
    /// 物种间互作关系
    pub interactions: Vec<SpeciesInteraction>,
    /// 总生物量（克）
    pub total_biomass: f32,
    /// 生产力（g/m²/day）
    pub productivity: f32,
}

impl Community {
    /// 重新归一化各物种相对丰度
    pub fn recompute_relative_abundances(&mut self) {
        let total: u32 = self.species.iter().map(|s| s.count).sum();
        if total == 0 {
            for s in &mut self.species {
                s.relative_abundance = 0.0;
            }
            return;
        }
        for s in &mut self.species {
            s.relative_abundance = s.count as f32 / total as f32;
        }
    }

    /// 物种数（物种丰富度 S）
    pub fn species_richness(&self) -> usize {
        self.species.iter().filter(|s| s.count > 0).count()
    }
}

/// 群落生物量（汇总各物种生物量）
pub fn community_biomass(community: &Community) -> f32 {
    community.species.iter().map(|s| s.biomass_g).sum()
}

// ============================================================================
// 生态位理论
// ============================================================================

/// 生态位（多维资源/环境耐受空间）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcologicalNiche {
    /// 温度耐受范围 °C
    pub temp_range: [f32; 2],
    /// pH 耐受范围
    pub ph_range: [f32; 2],
    /// 土壤水分范围 0..1
    pub moisture_range: [f32; 2],
    /// 光照范围 μmol/m²/s
    pub light_range: [f32; 2],
    /// 氮含量范围 mg/kg
    pub nutrient_range: [f32; 2],
    /// true=基础生态位（生理耐受），false=实际生态位（受生物互作限制）
    pub fundamental_niche: bool,
}

impl EcologicalNiche {
    /// 计算单一维度重叠比例（区间交集 / 较短区间长度）
    fn interval_overlap(a: [f32; 2], b: [f32; 2]) -> f32 {
        let a_lo = a[0].min(a[1]);
        let a_hi = a[0].max(a[1]);
        let b_lo = b[0].min(b[1]);
        let b_hi = b[0].max(b[1]);

        let lo = a_lo.max(b_lo);
        let hi = a_hi.min(b_hi);
        let overlap = (hi - lo).max(0.0);

        let min_len = (a_hi - a_lo).min(b_hi - b_lo);
        if min_len <= 0.0 {
            return 0.0;
        }
        (overlap / min_len).clamp(0.0, 1.0)
    }
}

/// Pianka 生态位重叠指数
///
/// 对各维度重叠取几何平均，O ∈ [0,1]。1 表示生态位完全重合。
pub fn niche_overlap(niche_a: &EcologicalNiche, niche_b: &EcologicalNiche) -> f32 {
    let dims = [
        EcologicalNiche::interval_overlap(niche_a.temp_range, niche_b.temp_range),
        EcologicalNiche::interval_overlap(niche_a.ph_range, niche_b.ph_range),
        EcologicalNiche::interval_overlap(niche_a.moisture_range, niche_b.moisture_range),
        EcologicalNiche::interval_overlap(niche_a.light_range, niche_b.light_range),
        EcologicalNiche::interval_overlap(niche_a.nutrient_range, niche_b.nutrient_range),
    ];

    // 几何平均（避免零值导致整体归零，对零做平滑）
    let eps = 1e-6;
    let mut log_sum = 0.0f32;
    let mut valid = 0u32;
    for d in &dims {
        if *d > eps {
            log_sum += d.ln();
            valid += 1;
        }
    }
    if valid == 0 {
        return 0.0;
    }
    (log_sum / valid as f32).exp().clamp(0.0, 1.0)
}

// ============================================================================
// 演替
// ============================================================================

/// 演替类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuccessionType {
    /// 初生演替（裸地起始）
    Primary,
    /// 次生演替（干扰后已有土壤种子库）
    Secondary,
}

/// 演替阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuccessionStage {
    /// 先锋阶段
    Pioneer,
    /// 早期
    Early,
    /// 中期
    Mid,
    /// 晚期
    Late,
    /// 顶极
    Climax,
}

impl SuccessionStage {
    /// 阶段顺序索引
    fn order(self) -> u8 {
        match self {
            SuccessionStage::Pioneer => 0,
            SuccessionStage::Early => 1,
            SuccessionStage::Mid => 2,
            SuccessionStage::Late => 3,
            SuccessionStage::Climax => 4,
        }
    }

    /// 推进到下一阶段（若已 Climax 则保持）
    fn advance(self) -> SuccessionStage {
        match self {
            SuccessionStage::Pioneer => SuccessionStage::Early,
            SuccessionStage::Early => SuccessionStage::Mid,
            SuccessionStage::Mid => SuccessionStage::Late,
            SuccessionStage::Late => SuccessionStage::Climax,
            SuccessionStage::Climax => SuccessionStage::Climax,
        }
    }
}

/// 演替模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessionModel {
    /// 演替类型
    pub succession_type: SuccessionType,
    /// 当前阶段
    pub current_stage: SuccessionStage,
    /// 干扰后年数
    pub years_since_disturbance: f32,
    /// 物种周转率（每年替代比例）
    pub species_turnover_rate: f32,
}

impl SuccessionModel {
    /// 各阶段阈值（年）。初生演替阈值更高，次生演替更快。
    fn stage_thresholds(&self) -> [f32; 4] {
        match self.succession_type {
            SuccessionType::Primary => [5.0, 25.0, 75.0, 150.0],
            SuccessionType::Secondary => [2.0, 10.0, 30.0, 80.0],
        }
    }
}

/// 推进演替进程
///
/// 累加年数并按阈值判定是否进入下一阶段，顶极阶段后停止推进。
pub fn advance_succession(model: &mut SuccessionModel, years: f32) {
    model.years_since_disturbance += years.max(0.0);

    let thresholds = model.stage_thresholds();
    // 依次检查可跨越的阈值
    let target_stage = match model.years_since_disturbance {
        y if y >= thresholds[3] => SuccessionStage::Climax,
        y if y >= thresholds[2] => SuccessionStage::Late,
        y if y >= thresholds[1] => SuccessionStage::Mid,
        y if y >= thresholds[0] => SuccessionStage::Early,
        _ => SuccessionStage::Pioneer,
    };

    if target_stage.order() > model.current_stage.order() {
        model.current_stage = target_stage;
    }
}

// ============================================================================
// 生物多样性指数
// ============================================================================

/// Shannon 多样性指数 H = -Σ pi * ln(pi)
///
/// 输入为相对丰度切片（自动归一化）。
pub fn shannon_diversity(abundances: &[f32]) -> f32 {
    let total: f32 = abundances.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let mut h = 0.0f32;
    for &a in abundances {
        if a <= 0.0 {
            continue;
        }
        let p = a / total;
        h -= p * p.ln();
    }
    h.max(0.0)
}

/// Simpson 多样性指数 D = 1 - Σ pi²
///
/// 值越大多样性越高。
pub fn simpson_diversity(abundances: &[f32]) -> f32 {
    let total: f32 = abundances.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let mut sum_sq = 0.0f32;
    for &a in abundances {
        let p = a / total;
        sum_sq += p * p;
    }
    (1.0 - sum_sq).max(0.0)
}

/// Pielou 均匀度指数 J = H / ln(S)
///
/// S 为物种数（>0 个体）。完全均匀分布时 J=1。
pub fn pielou_evenness(abundances: &[f32]) -> f32 {
    let s = abundances.iter().filter(|&&a| a > 0.0).count();
    if s <= 1 {
        return 0.0;
    }
    let h = shannon_diversity(abundances);
    let denom = (s as f32).ln();
    if denom <= 0.0 {
        return 0.0;
    }
    (h / denom).clamp(0.0, 1.0)
}

/// Margalef 丰富度指数 R = (S - 1) / ln(N)
pub fn margalef_richness(abundances: &[f32]) -> f32 {
    let s = abundances.iter().filter(|&&a| a > 0.0).count();
    let n: f32 = abundances.iter().sum();
    if n <= 0.0 {
        return 0.0;
    }
    let ln_n = n.ln();
    if ln_n <= 0.0 {
        return 0.0;
    }
    ((s as f32) - 1.0) / ln_n
}

/// Berger-Parker 优势度指数 = N_max / N
///
/// 值越大表示优势种越突出，多样性越低。
pub fn berger_parker_dominance(abundances: &[f32]) -> f32 {
    let total: f32 = abundances.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let max = abundances.iter().cloned().fold(0.0f32, f32::max);
    max / total
}

// ============================================================================
// 生态系统功能
// ============================================================================

/// 生态系统功能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemFunction {
    /// 净初级生产力 g/m²/day
    pub npp: f32,
    /// 凋落物分解率 1/day
    pub decomposition_rate: f32,
    /// 氮矿化速率 mg/kg/day
    pub nitrogen_mineralization: f32,
    /// 碳储量 kg/m²
    pub carbon_storage: f32,
    /// 蒸散速率 mm/day
    pub evapotranspiration: f32,
}

impl EcosystemFunction {
    /// 由生物量与生产力粗略估算
    ///
    /// NPP 直接由生产力给定；其他指标基于经验关系。
    pub fn from_productivity(biomass_g_per_m2: f32, productivity: f32) -> Self {
        let carbon_storage = biomass_g_per_m2 * 0.5 / 1000.0; // 假设 50% 干物质为碳
        let decomposition_rate = 0.01 + 0.0002 * biomass_g_per_m2.max(0.0) / 100.0;
        let nitrogen_mineralization = 0.5 + 0.05 * productivity.max(0.0);
        let evapotranspiration = 1.0 + 0.02 * productivity.max(0.0);
        EcosystemFunction {
            npp: productivity.max(0.0),
            decomposition_rate: decomposition_rate.min(0.1),
            nitrogen_mineralization,
            carbon_storage: carbon_storage.max(0.0),
            evapotranspiration: evapotranspiration.min(10.0),
        }
    }
}

// ============================================================================
// 物种互作网络
// ============================================================================

/// 互作网络节点度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDegree {
    /// 节点索引（对应物种索引）
    pub node: usize,
    /// 出度（作为物种 A 的连接数）
    pub out_degree: usize,
    /// 入度（作为物种 B 的连接数）
    pub in_degree: usize,
    /// 加权度（按互作强度求和）
    pub weighted_degree: f32,
}

/// 计算互作网络各节点的度统计
pub fn network_degrees(community: &Community) -> Vec<NetworkDegree> {
    let n = community.species.len();
    let mut degrees = vec![
        NetworkDegree {
            node: 0,
            out_degree: 0,
            in_degree: 0,
            weighted_degree: 0.0,
        };
        n
    ];
    for (i, d) in degrees.iter_mut().enumerate() {
        d.node = i;
    }
    for inter in &community.interactions {
        let a = inter.species_a.min(n.saturating_sub(1));
        let b = inter.species_b.min(n.saturating_sub(1));
        if a < n {
            degrees[a].out_degree += 1;
            degrees[a].weighted_degree += inter.strength;
        }
        if b < n {
            degrees[b].in_degree += 1;
            degrees[b].weighted_degree += inter.strength;
        }
    }
    degrees
}

/// 网络连接度（connectance = 实际连接数 / 可能连接数）
pub fn network_connectance(community: &Community) -> f32 {
    let n = community.species.len();
    if n < 2 {
        return 0.0;
    }
    let possible = (n * (n - 1)) as f32;
    community.interactions.len() as f32 / possible
}

/// 网络平均互作强度
pub fn network_mean_strength(community: &Community) -> f32 {
    if community.interactions.is_empty() {
        return 0.0;
    }
    community.interactions.iter().map(|i| i.strength).sum::<f32>()
        / community.interactions.len() as f32
}

// ============================================================================
// 化感作用
// ============================================================================

/// 化感作用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allelopathy {
    /// 释放化感物质的物种索引
    pub donor: usize,
    /// 受影响物种索引
    pub receiver: usize,
    /// 化感物质浓度（任意单位）
    pub concentration: f32,
    /// 抑制（负）或促进（正）效应系数
    pub effect_coefficient: f32,
}

/// 计算化感作用对目标物种的等效种群抑制因子
///
/// 返回 (0..1+）的乘性因子。1 = 无影响；<1 抑制；>1 促进。
pub fn allelopathic_factor(allelopathies: &[Allelopathy], receiver: usize) -> f32 {
    let mut factor = 1.0f32;
    for a in allelopathies {
        if a.receiver != receiver {
            continue;
        }
        // 简化的 Michaelis-Menten 型响应
        let response = a.effect_coefficient * a.concentration / (1.0 + a.concentration);
        factor *= (1.0 + response).max(0.0);
    }
    factor
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logistic_growth() {
        // 当 N 接近 K 时增长趋缓，超过 K 时为负
        let k = 100.0;
        let r = 0.5;
        let dt = 1.0;

        // N=10 增长较快
        let n_low = logistic_growth(10.0, r, k, dt);
        assert!(n_low > 10.0, "N<K 时应增长");

        // N=99 增长几乎停滞
        let n_near_k = logistic_growth(99.0, r, k, dt);
        let delta_near = (n_near_k - 99.0).abs();
        assert!(delta_near < 1.0, "N 接近 K 时增长趋缓，delta={}", delta_near);

        // N=K 时无增长
        let n_at_k = logistic_growth(k, r, k, dt);
        assert!((n_at_k - k).abs() < 1e-4, "N=K 时应稳定");

        // N>K 时下降
        let n_over = logistic_growth(150.0, r, k, dt);
        assert!(n_over < 150.0, "N>K 时应下降");
    }

    #[test]
    fn test_lotka_volterra() {
        // 竞争抑制种群增长：相比无竞争，加入 N2 后 N1 增量更小
        let r1 = 0.4;
        let k1 = 100.0;
        let dt = 1.0;
        let n1 = 30.0;

        let alone = logistic_growth(n1, r1, k1, dt);
        let with_competitor = lotka_volterra_competition(n1, 50.0, r1, k1, 0.5, dt);

        assert!(
            with_competitor < alone,
            "竞争应抑制增长: 独立={} 有竞争={}",
            alone,
            with_competitor
        );

        // alpha12 越大抑制越强
        let weak = lotka_volterra_competition(n1, 50.0, r1, k1, 0.2, dt);
        let strong = lotka_volterra_competition(n1, 50.0, r1, k1, 1.5, dt);
        assert!(strong < weak, "更强竞争系数应导致更小种群");
    }

    #[test]
    fn test_shannon_diversity() {
        // 均匀分布时多样性最高
        let even = vec![10.0, 10.0, 10.0, 10.0];
        let skewed = vec![100.0, 1.0, 1.0, 1.0];

        let h_even = shannon_diversity(&even);
        let h_skewed = shannon_diversity(&skewed);

        assert!(h_even > h_skewed, "均匀分布多样性应更高");
        // 均匀分布 4 物种：H = ln(4) ≈ 1.386
        assert!(
            (h_even - 4.0f32.ln()).abs() < 1e-3,
            "均匀分布 H 应等于 ln(S), got {}",
            h_even
        );
    }

    #[test]
    fn test_niche_overlap() {
        let niche = EcologicalNiche {
            temp_range: [10.0, 30.0],
            ph_range: [5.0, 7.0],
            moisture_range: [0.2, 0.8],
            light_range: [200.0, 800.0],
            nutrient_range: [50.0, 200.0],
            fundamental_niche: true,
        };

        // 完全相同生态位 → 重叠为 1
        let overlap_same = niche_overlap(&niche, &niche);
        assert!(
            (overlap_same - 1.0).abs() < 1e-3,
            "相同生态位重叠应为 1, got {}",
            overlap_same
        );

        // 完全不重叠生态位 → 重叠为 0
        let disjoint = EcologicalNiche {
            temp_range: [-10.0, 0.0],
            ph_range: [2.0, 3.0],
            moisture_range: [0.0, 0.1],
            light_range: [10.0, 50.0],
            nutrient_range: [1.0, 5.0],
            fundamental_niche: true,
        };
        let overlap_disjoint = niche_overlap(&niche, &disjoint);
        assert!(
            overlap_disjoint < 1e-3,
            "完全不重叠应为 0, got {}",
            overlap_disjoint
        );
    }

    #[test]
    fn test_pielou_evenness() {
        // 完全均匀分布时 J=1
        let even = vec![5.0, 5.0, 5.0, 5.0];
        let j = pielou_evenness(&even);
        assert!(
            (j - 1.0).abs() < 1e-3,
            "完全均匀时 J 应为 1, got {}",
            j
        );

        // 不均匀分布 J<1
        let skewed = vec![100.0, 1.0, 1.0, 1.0];
        let j_skewed = pielou_evenness(&skewed);
        assert!(j_skewed < 1.0, "不均匀分布 J 应小于 1");
        assert!(j_skewed > 0.0, "J 应为正");
    }

    #[test]
    fn test_leslie_projection_stability() {
        // 简单 3 阶级种群投影
        let pop = Population {
            species_name: "TestPlant".to_string(),
            size_classes: vec![100, 50, 20],
            survival_rates: vec![0.5, 0.7, 0.3],
            fecundity_rates: vec![0.0, 2.0, 5.0],
            carrying_capacity: 1000.0,
            growth_rate: 0.3,
        };
        let next = leslie_projection(&pop);
        // 幼体 = 50*2 + 20*5 = 200，乘以阻尼
        assert!(next.size_classes[0] > 0, "应有新生幼体");
        assert!(next.size_classes[1] > 0, "应有个体转移到第二阶级");
        assert!(next.total() > 0, "投影后种群非零");
    }

    #[test]
    fn test_succession_advance() {
        let mut model = SuccessionModel {
            succession_type: SuccessionType::Secondary,
            current_stage: SuccessionStage::Pioneer,
            years_since_disturbance: 0.0,
            species_turnover_rate: 0.1,
        };
        advance_succession(&mut model, 3.0);
        assert_eq!(model.current_stage, SuccessionStage::Early);
        advance_succession(&mut model, 10.0);
        assert_eq!(model.current_stage, SuccessionStage::Mid);
        advance_succession(&mut model, 100.0);
        assert_eq!(model.current_stage, SuccessionStage::Climax);
        // 顶极后再推进不变
        advance_succession(&mut model, 50.0);
        assert_eq!(model.current_stage, SuccessionStage::Climax);
    }

    #[test]
    fn test_simpson_and_berger_parker() {
        let even = vec![10.0, 10.0, 10.0, 10.0];
        let skewed = vec![100.0, 1.0, 1.0, 1.0];

        let d_even = simpson_diversity(&even);
        let d_skewed = simpson_diversity(&skewed);
        assert!(d_even > d_skewed, "均匀分布 Simpson 多样性更高");

        let bp_even = berger_parker_dominance(&even);
        let bp_skewed = berger_parker_dominance(&skewed);
        assert!(bp_skewed > bp_even, " skewed 应有更高优势度");
        assert!(
            (bp_even - 0.25).abs() < 1e-3,
            "4 物种均匀分布 Berger-Parker = 0.25, got {}",
            bp_even
        );
    }
}
