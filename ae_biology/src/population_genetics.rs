//! 群体遗传学模块 — Hardy-Weinberg 平衡、遗传漂变、基因流与 F 统计
//!
//! 生物学背景:
//!   群体遗传学研究等位基因频率在群体中的分布与代际变化。Hardy-Weinberg 平衡
//!   描述在无演化压力下的基因型频率 p²+2pq+q²=1。遗传漂变（Wright-Fisher 模型）
//!   描述小群体中频率的随机波动。奠基者效应是遗传漂变的特例，由少数个体建立
//!   新群体导致频率漂移。基因流通过迁移混合群体。F 统计（F_ST 等）量化群体分化。
//!
//! 论文来源:
//! - Hardy, G. H. (1908). "Mendelian proportions in a mixed population." Science 28: 49-50.
//! - Weinberg, W. (1908). "Über den Nachweis der Vererbung beim Menschen."
//!   Jahreshefte des Vereins für vaterländische Naturkunde in Württemberg 64: 368-382.
//! - Wright, S. (1931). "Evolution in Mendelian populations." Genetics 16(2): 97-159.
//!   (Wright-Fisher 漂变模型)
//! - Wright, S. (1951). "The genetical structure of populations." Ann. Eugen. 15: 323-354.
//!   (F 统计 F_ST, F_IS, F_IT)
//! - Nei, M. (1973). "Analysis of gene diversity in subdivided populations."
//!   PNAS 70(12): 3321-3323. (G_ST 等价于 F_ST)
//! - Mayr, E. (1942). "Systematics and the Origin of Species." Columbia Univ. Press.
//!   (奠基者效应)
//!
//! 物理量单位:
//!   - 频率: 无量纲 (0..1)
//!   - 群体大小: 个体数 (整数)
//!   - F 统计: 无量纲 (0..1)

use serde::{Deserialize, Serialize};

/// 等位基因频率
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AlleleFrequency {
    /// 等位基因 A 频率 p (0..1)
    pub p: f32,
    /// 等位基因 a 频率 q = 1 - p (0..1)
    pub q: f32,
}

impl AlleleFrequency {
    /// 由 p 构造, q 自动 = 1 - p
    pub fn from_p(p: f32) -> Self {
        let p_c = p.clamp(0.0, 1.0);
        Self {
            p: p_c,
            q: 1.0 - p_c,
        }
    }

    /// p + q 之和 (理论 = 1.0)
    pub fn sum(&self) -> f32 {
        self.p + self.q
    }

    /// 是否合法 (p, q ∈ [0,1], 和 ≈ 1)
    pub fn is_valid(&self) -> bool {
        (self.sum() - 1.0).abs() < 1e-4
            && self.p >= 0.0
            && self.p <= 1.0
            && self.q >= 0.0
            && self.q <= 1.0
    }

    /// 次要等位基因频率 MAF = min(p, q)
    pub fn maf(&self) -> f32 {
        self.p.min(self.q)
    }
}

impl Default for AlleleFrequency {
    fn default() -> Self {
        // 中性平衡默认 p = q = 0.5
        Self { p: 0.5, q: 0.5 }
    }
}

/// Hardy-Weinberg 平衡基因型频率
/// 来源: Hardy 1908, Weinberg 1908
/// p² + 2pq + q² = 1
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HardyWeinberg {
    /// AA 基因型频率 (= p²)
    pub aa_freq: f32,
    /// Aa 杂合基因型频率 (= 2pq)
    pub hetero_freq: f32,
    /// aa 基因型频率 (= q²)
    pub homo_recessive_freq: f32,
}

impl Default for HardyWeinberg {
    fn default() -> Self {
        // 默认 p = q = 0.5: 0.25 + 0.5 + 0.25 = 1
        Self::from_allele_frequency(AlleleFrequency::default())
    }
}

impl HardyWeinberg {
    /// 由等位基因频率计算 HW 平衡基因型频率
    /// 来源: Hardy 1908 Eq. 1
    pub fn from_allele_frequency(af: AlleleFrequency) -> Self {
        Self {
            aa_freq: af.p * af.p,
            hetero_freq: 2.0 * af.p * af.q,
            homo_recessive_freq: af.q * af.q,
        }
    }

    /// 基因型频率之和 (理论 = 1)
    pub fn sum(&self) -> f32 {
        self.aa_freq + self.hetero_freq + self.homo_recessive_freq
    }

    /// 是否处于 HW 平衡 (和 ≈ 1)
    pub fn is_in_equilibrium(&self) -> bool {
        (self.sum() - 1.0).abs() < 1e-3
    }

    /// 杂合度 H = 2pq
    pub fn heterozygosity(&self) -> f32 {
        self.hetero_freq
    }
}

/// 群体
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Population {
    /// 个体数
    pub size: u32,
    /// 等位基因频率
    pub allele_freq: AlleleFrequency,
    /// 代数
    pub generation: u32,
}

impl Default for Population {
    fn default() -> Self {
        Self {
            size: 1000,
            allele_freq: AlleleFrequency::default(),
            generation: 0,
        }
    }
}

/// 遗传漂变 — Wright-Fisher 模型
/// 来源: Wright 1931
/// 每代从父代等位基因池中以 2N 次二项抽样确定子代频率
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeneticDrift {
    /// 群体有效大小 N_e
    pub effective_size: u32,
    /// 当前代数
    pub generation: u32,
}

impl Default for GeneticDrift {
    fn default() -> Self {
        Self {
            effective_size: 100,
            generation: 0,
        }
    }
}

impl GeneticDrift {
    /// 漂变一步 — 给定当前 p, 用伪随机种子确定性扰动后返回新 p
    /// 真实 Wright-Fisher 需要二项抽样, 此处用确定性近似以避免外部 RNG
    /// 来源: Wright 1931, 简化确定性扰动
    /// Var(Δp) = p(1-p) / (2N_e)
    pub fn drift_step(&mut self, p: f32, perturbation: f32) -> f32 {
        self.generation = self.generation.saturating_add(1);
        let n = self.effective_size.max(1) as f32;
        // 用扰动量级模拟 ± sqrt(p*q/(2N)) 的标准差
        let std_dev = (p * (1.0 - p) / (2.0 * n)).max(0.0).sqrt();
        let new_p = p + perturbation * std_dev;
        new_p.clamp(0.0, 1.0)
    }

    /// 漂变速率 (每代标准差) — 衡量漂变强度
    pub fn drift_rate(&self, p: f32) -> f32 {
        let n = self.effective_size.max(1) as f32;
        (p * (1.0 - p) / (2.0 * n)).max(0.0).sqrt()
    }

    /// 是否已固定 (p 接近 0 或 1)
    pub fn is_fixed(&self, p: f32, tol: f32) -> bool {
        p < tol || p > (1.0 - tol)
    }
}

/// 基因流 — 群体间迁移
/// 来源: 标准群体遗传学教材
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeneFlow {
    /// 迁移率 m (每代迁入比例, 0..1)
    pub migration_rate: f32,
    /// 源群体等位基因频率 p_source
    pub source_p: f32,
}

impl Default for GeneFlow {
    fn default() -> Self {
        Self {
            migration_rate: 0.01,
            source_p: 0.5,
        }
    }
}

impl GeneFlow {
    /// 基因流一步 — 更新本地等位基因频率
    /// p_new = (1 - m) * p_local + m * p_source
    /// 来源: Wright 岛屿模型
    pub fn flow_step(&self, p_local: f32) -> f32 {
        let m = self.migration_rate.clamp(0.0, 1.0);
        (1.0 - m) * p_local + m * self.source_p
    }

    /// 平衡频率 (当 p_local == p_source 时达到)
    pub fn equilibrium_p(&self) -> f32 {
        self.source_p
    }
}

/// F 统计 — 群体分化指数
/// 来源: Wright 1951
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FStatistics {
    /// F_ST — 群体间分化 (0..1)
    pub f_st: f32,
    /// F_IS — 个体内杂合缺失
    pub f_is: f32,
    /// F_IT — 个体总杂合缺失
    pub f_it: f32,
}

impl Default for FStatistics {
    fn default() -> Self {
        Self {
            f_st: 0.0,
            f_is: 0.0,
            f_it: 0.0,
        }
    }
}

impl FStatistics {
    /// 由 H_T (总群体杂合度) 和 H_S (亚群体平均杂合度) 计算 F_ST
    /// F_ST = (H_T - H_S) / H_T
    /// 来源: Wright 1951, Nei 1973
    pub fn f_st_from_heterozygosity(h_t: f32, h_s: f32) -> f32 {
        if h_t > 0.0 {
            ((h_t - h_s) / h_t).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// 由 F_IS 和 F_ST 计算 F_IT
    /// 1 - F_IT = (1 - F_IS) * (1 - F_ST)
    /// 来源: Wright 1951
    pub fn f_it_from_components(f_is: f32, f_st: f32) -> f32 {
        1.0 - (1.0 - f_is) * (1.0 - f_st)
    }

    /// F_ST 解读 — 弱/中/强分化
    /// 来源: Wright 1978 经验阈值
    pub fn f_st_interpretation(f_st: f32) -> FstLevel {
        if f_st < 0.05 {
            FstLevel::Little
        } else if f_st < 0.15 {
            FstLevel::Moderate
        } else if f_st < 0.25 {
            FstLevel::Great
        } else {
            FstLevel::VeryGreat
        }
    }
}

/// F_ST 分化等级
/// 来源: Wright 1978
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FstLevel {
    /// 几乎无分化 (< 0.05)
    Little,
    /// 中度 (0.05 - 0.15)
    Moderate,
    /// 显著 (0.15 - 0.25)
    Great,
    /// 极度 (>= 0.25)
    VeryGreat,
}

/// 奠基者效应 — 少数个体建立新群体导致频率漂移
/// 来源: Mayr 1942
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FounderEffect {
    /// 奠基者数量
    pub founder_count: u32,
    /// 奠基者群体中的等位基因频率
    pub founder_p: f32,
}

impl FounderEffect {
    /// 模拟奠基者效应 — 给定源频率和奠基者数, 返回新群体的预期频率方差
    /// Var(p_founder) = p_source * (1 - p_source) / (2 * founder_count)
    /// 来源: Mayr 1942, 标准群体遗传学
    pub fn expected_variance(&self, source_p: f32) -> f32 {
        let n = self.founder_count.max(1) as f32;
        source_p * (1.0 - source_p) / (2.0 * n)
    }

    /// 预期漂移大小 (标准差)
    pub fn expected_drift(&self, source_p: f32) -> f32 {
        self.expected_variance(source_p).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allele_frequency_default_balanced() {
        let af = AlleleFrequency::default();
        assert!((af.p - 0.5).abs() < 1e-5);
        assert!((af.q - 0.5).abs() < 1e-5);
        assert!((af.sum() - 1.0).abs() < 1e-5);
        assert!(af.is_valid());
    }

    #[test]
    fn test_allele_frequency_from_p_computes_q() {
        let af = AlleleFrequency::from_p(0.3);
        assert!((af.p - 0.3).abs() < 1e-5);
        assert!((af.q - 0.7).abs() < 1e-5);
        assert!(af.is_valid());
    }

    #[test]
    fn test_allele_frequency_from_p_clamps_out_of_range() {
        let af = AlleleFrequency::from_p(1.5);
        assert!((af.p - 1.0).abs() < 1e-5);
        assert!((af.q - 0.0).abs() < 1e-5);
        assert!(af.is_valid());
    }

    #[test]
    fn test_allele_frequency_maf() {
        let af = AlleleFrequency::from_p(0.1);
        assert!((af.maf() - 0.1).abs() < 1e-5);
        let af2 = AlleleFrequency::from_p(0.9);
        assert!((af2.maf() - 0.1).abs() < 1e-5);
    }

    #[test]
    fn test_hardy_weinberg_default_balanced() {
        let hw = HardyWeinberg::default();
        // p=q=0.5: 0.25 + 0.5 + 0.25 = 1
        assert!((hw.aa_freq - 0.25).abs() < 1e-5);
        assert!((hw.hetero_freq - 0.5).abs() < 1e-5);
        assert!((hw.homo_recessive_freq - 0.25).abs() < 1e-5);
        assert!(hw.is_in_equilibrium());
    }

    #[test]
    fn test_hardy_weinberg_p_plus_q_equals_one() {
        let hw = HardyWeinberg::from_allele_frequency(AlleleFrequency::from_p(0.7));
        assert!((hw.sum() - 1.0).abs() < 1e-5);
        assert!(hw.is_in_equilibrium());
    }

    #[test]
    fn test_hardy_weinberg_genotype_frequencies_p_07() {
        // p=0.7, q=0.3
        let hw = HardyWeinberg::from_allele_frequency(AlleleFrequency::from_p(0.7));
        assert!((hw.aa_freq - 0.49).abs() < 1e-5); // 0.7^2
        assert!((hw.hetero_freq - 0.42).abs() < 1e-5); // 2*0.7*0.3
        assert!((hw.homo_recessive_freq - 0.09).abs() < 1e-5); // 0.3^2
    }

    #[test]
    fn test_hardy_weinberg_heterozygosity_max_at_p_05() {
        let hw_low = HardyWeinberg::from_allele_frequency(AlleleFrequency::from_p(0.1));
        let hw_mid = HardyWeinberg::from_allele_frequency(AlleleFrequency::from_p(0.5));
        let hw_high = HardyWeinberg::from_allele_frequency(AlleleFrequency::from_p(0.9));
        assert!(hw_mid.heterozygosity() > hw_low.heterozygosity());
        assert!(hw_mid.heterozygosity() > hw_high.heterozygosity());
        // 最大杂合度 = 2*0.5*0.5 = 0.5
        assert!((hw_mid.heterozygosity() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_hardy_weinberg_extreme_p_1_homogeneous() {
        // p=1: 全部 AA
        let hw = HardyWeinberg::from_allele_frequency(AlleleFrequency::from_p(1.0));
        assert!((hw.aa_freq - 1.0).abs() < 1e-5);
        assert!((hw.hetero_freq - 0.0).abs() < 1e-5);
        assert!((hw.homo_recessive_freq - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_population_default() {
        let pop = Population::default();
        assert_eq!(pop.size, 1000);
        assert_eq!(pop.generation, 0);
        assert!(pop.allele_freq.is_valid());
    }

    #[test]
    fn test_genetic_drift_default() {
        let gd = GeneticDrift::default();
        assert_eq!(gd.effective_size, 100);
        assert_eq!(gd.generation, 0);
    }

    #[test]
    fn test_genetic_drift_step_increments_generation() {
        let mut gd = GeneticDrift::default();
        let _ = gd.drift_step(0.5, 1.0);
        assert_eq!(gd.generation, 1);
        let _ = gd.drift_step(0.5, 1.0);
        assert_eq!(gd.generation, 2);
    }

    #[test]
    fn test_genetic_drift_step_clamps_to_01() {
        let mut gd = GeneticDrift::default();
        let new_p = gd.drift_step(0.99, 10.0);
        assert!(new_p >= 0.0 && new_p <= 1.0);
    }

    #[test]
    fn test_genetic_drift_rate_decreases_with_population_size() {
        let gd_small = GeneticDrift { effective_size: 10, generation: 0 };
        let gd_large = GeneticDrift { effective_size: 10000, generation: 0 };
        let rate_small = gd_small.drift_rate(0.5);
        let rate_large = gd_large.drift_rate(0.5);
        assert!(rate_small > rate_large);
    }

    #[test]
    fn test_genetic_drift_rate_zero_at_fixation() {
        let gd = GeneticDrift::default();
        // p=0 或 p=1 时漂变速率为 0 (已固定)
        assert!((gd.drift_rate(0.0) - 0.0).abs() < 1e-7);
        assert!((gd.drift_rate(1.0) - 0.0).abs() < 1e-7);
    }

    #[test]
    fn test_genetic_drift_is_fixed_detection() {
        let gd = GeneticDrift::default();
        assert!(gd.is_fixed(0.0, 0.01));
        assert!(gd.is_fixed(1.0, 0.01));
        assert!(gd.is_fixed(0.005, 0.01));
        assert!(!gd.is_fixed(0.5, 0.01));
    }

    #[test]
    fn test_gene_flow_default() {
        let gf = GeneFlow::default();
        assert!((gf.migration_rate - 0.01).abs() < 1e-5);
        assert!((gf.source_p - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_gene_flow_step_moves_local_toward_source() {
        let gf = GeneFlow {
            migration_rate: 0.1,
            source_p: 0.9,
        };
        let new_p = gf.flow_step(0.1);
        // (1-0.1)*0.1 + 0.1*0.9 = 0.09 + 0.09 = 0.18
        assert!((new_p - 0.18).abs() < 1e-5);
        // 应向源群体频率方向移动
        assert!(new_p > 0.1);
    }

    #[test]
    fn test_gene_flow_equilibrium_at_source_p() {
        let gf = GeneFlow {
            migration_rate: 0.2,
            source_p: 0.7,
        };
        // 当 p_local == source_p 时频率不变
        let new_p = gf.flow_step(0.7);
        assert!((new_p - 0.7).abs() < 1e-5);
        assert!((gf.equilibrium_p() - 0.7).abs() < 1e-5);
    }

    #[test]
    fn test_gene_flow_zero_migration_no_change() {
        let gf = GeneFlow {
            migration_rate: 0.0,
            source_p: 0.9,
        };
        let new_p = gf.flow_step(0.3);
        assert!((new_p - 0.3).abs() < 1e-5);
    }

    #[test]
    fn test_f_statistics_default_zero() {
        let fs = FStatistics::default();
        assert!((fs.f_st - 0.0).abs() < 1e-5);
        assert!((fs.f_is - 0.0).abs() < 1e-5);
        assert!((fs.f_it - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_f_st_from_heterozygosity_zero_when_equal() {
        // H_T == H_S → F_ST = 0 (无分化)
        let f_st = FStatistics::f_st_from_heterozygosity(0.5, 0.5);
        assert!((f_st - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_f_st_from_heterozygosity_one_when_no_subdivision() {
        // H_S == 0 → F_ST = 1 (完全分化)
        let f_st = FStatistics::f_st_from_heterozygosity(0.5, 0.0);
        assert!((f_st - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_f_st_from_heterozygosity_zero_ht_returns_zero() {
        let f_st = FStatistics::f_st_from_heterozygosity(0.0, 0.0);
        assert!((f_st - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_f_st_range_zero_to_one() {
        // 任意 H_T >= H_S >= 0, F_ST ∈ [0, 1]
        for h_t in [0.1, 0.3, 0.5, 0.7, 0.9].iter() {
            for h_s in [0.0, 0.05, 0.1, 0.2, h_t * 0.5].iter() {
                let f = FStatistics::f_st_from_heterozygosity(*h_t, *h_s);
                assert!(f >= 0.0 && f <= 1.0);
            }
        }
    }

    #[test]
    fn test_f_it_from_components_relationship() {
        // F_IT = 1 - (1-F_IS)(1-F_ST)
        let f_is = 0.2;
        let f_st = 0.3;
        let f_it = FStatistics::f_it_from_components(f_is, f_st);
        // 1 - (0.8)(0.7) = 1 - 0.56 = 0.44
        assert!((f_it - 0.44).abs() < 1e-5);
    }

    #[test]
    fn test_f_it_zero_when_both_zero() {
        let f_it = FStatistics::f_it_from_components(0.0, 0.0);
        assert!((f_it - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_fst_level_interpretation_thresholds() {
        assert_eq!(FStatistics::f_st_interpretation(0.01), FstLevel::Little);
        assert_eq!(FStatistics::f_st_interpretation(0.10), FstLevel::Moderate);
        assert_eq!(FStatistics::f_st_interpretation(0.20), FstLevel::Great);
        assert_eq!(FStatistics::f_st_interpretation(0.40), FstLevel::VeryGreat);
    }

    #[test]
    fn test_founder_effect_variance_decreases_with_founders() {
        let fe_small = FounderEffect { founder_count: 5, founder_p: 0.5 };
        let fe_large = FounderEffect { founder_count: 500, founder_p: 0.5 };
        let var_small = fe_small.expected_variance(0.5);
        let var_large = fe_large.expected_variance(0.5);
        assert!(var_small > var_large);
    }

    #[test]
    fn test_founder_effect_drift_zero_at_extreme_p() {
        let fe = FounderEffect { founder_count: 10, founder_p: 0.5 };
        // p=0 或 p=1 时方差 = 0 (已固定)
        assert!((fe.expected_variance(0.0) - 0.0).abs() < 1e-7);
        assert!((fe.expected_variance(1.0) - 0.0).abs() < 1e-7);
    }

    #[test]
    fn test_founder_effect_drift_max_at_p_05() {
        let fe = FounderEffect { founder_count: 100, founder_p: 0.5 };
        let drift_mid = fe.expected_drift(0.5);
        let drift_low = fe.expected_drift(0.1);
        // p=0.5 时方差最大
        assert!(drift_mid > drift_low);
    }
}
