//! 癌症模块 —— 肿瘤生长、血管新生、转移与 Hallmarks of Cancer
//!
//! 科学来源：
//! - Gompertz, B. (1825). Phil. Trans. R. Soc. 115: 513-585. —— Gompertz 生长曲线
//! - Laird, A. K. (1964). Br. J. Cancer 18: 490-502. —— 肿瘤 Gompertz 生长动力学
//! - Folkman, J. (1971). N. Engl. J. Med. 285: 1182-1186. —— 血管新生与 VEGF
//! - Hanahan, D. & Weinberg, R. A. (2000). Cell 100: 57-70. —— 癌症六大标志
//! - Hanahan, D. & Weinberg, R. A. (2011). Cell 144: 646-674. —— 十大 Hallmarks
//! - Chambers, A. F. et al. (2002). Nat. Rev. Cancer 2: 563-572. —— 转移级联
//!
//! 核心规律：Gompertz 生长 dV/dt = a·V·ln(K/V)，K 为承载量；
//! VEGF 阈值 ~35 pg/mL 触发血管新生；转移概率随肿瘤体积上升；
//! Hanahan & Weinberg 2011 十大 Hallmarks。

use serde::{Deserialize, Serialize};

/// 癌症十大标志（Hanahan & Weinberg 2011, Cell 144: 646-674）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hallmark {
    /// 持续增殖信号
    SustainingProliferativeSignaling,
    /// 逃避生长抑制
    EvadingGrowthSuppressors,
    /// 抵抗细胞死亡
    ResistingCellDeath,
    /// 无限复制能力（端粒维持）
    EnablingReplicativeImmortality,
    /// 诱导血管新生
    InducingAngiogenesis,
    /// 激活侵袭与转移
    ActivatingInvasionAndMetastasis,
    /// 基因组不稳定与突变
    GenomeInstabilityAndMutation,
    /// 肿瘤促进炎症
    TumorPromotingInflammation,
    /// 能量代谢重编程
    DeregulatingCellularEnergetics,
    /// 避免免疫摧毁
    AvoidingImmuneDestruction,
}

impl Hallmark {
    /// 返回全部十大 Hallmarks（Hanahan & Weinberg 2011）
    pub fn all() -> [Hallmark; 10] {
        [
            Hallmark::SustainingProliferativeSignaling,
            Hallmark::EvadingGrowthSuppressors,
            Hallmark::ResistingCellDeath,
            Hallmark::EnablingReplicativeImmortality,
            Hallmark::InducingAngiogenesis,
            Hallmark::ActivatingInvasionAndMetastasis,
            Hallmark::GenomeInstabilityAndMutation,
            Hallmark::TumorPromotingInflammation,
            Hallmark::DeregulatingCellularEnergetics,
            Hallmark::AvoidingImmuneDestruction,
        ]
    }
}

/// Hallmark 评估面板 —— 记录每个标志的激活强度（0..1）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HallmarkPanel {
    /// 各 Hallmark 激活强度，索引对应 Hallmark::all() 顺序
    pub activations: [f32; 10],
}

impl HallmarkPanel {
    pub fn new() -> Self { Self { activations: [0.0; 10] } }
    /// 设置某 Hallmark 的激活强度
    pub fn set(&mut self, hallmark: Hallmark, value: f32) {
        self.activations[hallmark as usize] = value.clamp(0.0, 1.0);
    }
    /// 读取某 Hallmark 的激活强度
    pub fn get(&self, hallmark: Hallmark) -> f32 { self.activations[hallmark as usize] }
    /// 是否为恶性表型（任一 hallmark > 0.5）
    pub fn is_malignant(&self) -> bool { self.activations.iter().any(|&v| v > 0.5) }
    /// 已激活 hallmark 数（强度 > 阈值）
    pub fn active_count(&self, threshold: f32) -> usize {
        self.activations.iter().filter(|&&v| v > threshold).count()
    }
}
impl Default for HallmarkPanel { fn default() -> Self { Self::new() } }

/// 血管新生信号（Folkman 1971）—— VEGF 介导
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AngiogenesisSignal {
    /// VEGF 浓度（pg/mL），阈值约 35 pg/mL
    pub vegf_level: f32,
    /// 缺氧程度（0..1，HIF-1α 介导 VEGF 上调）
    pub hypoxia: f32,
    /// 微血管密度（相对值）
    pub microvessel_density: f32,
}

impl AngiogenesisSignal {
    pub fn new() -> Self { Self { vegf_level: 5.0, hypoxia: 0.0, microvessel_density: 0.1 } }
    /// VEGF 是否超过血管新生阈值（Folkman 1971）
    pub fn is_angiogenic(&self) -> bool { self.vegf_level > 35.0 }
    /// 缺氧上调 VEGF（HIF-1α 通路）：缺氧 → VEGF 合成增加
    pub fn update(&mut self, dt: f32) {
        // VEGF 随缺氧上升，随血管化下降
        let production = 10.0 * self.hypoxia;
        let clearance = 0.1 * self.vegf_level * self.microvessel_density;
        self.vegf_level = (self.vegf_level + (production - clearance) * dt).max(0.0);
        // 血管新生增加微血管密度
        if self.is_angiogenic() {
            self.microvessel_density = (self.microvessel_density + 0.01 * dt).min(1.0);
            self.hypoxia = (self.hypoxia - 0.005 * dt).max(0.0);
        }
    }
}
impl Default for AngiogenesisSignal { fn default() -> Self { Self::new() } }

/// 癌细胞 —— 携带突变与 hallmark 激活状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CancerCell {
    /// 突变负荷（突变数）
    pub mutation_count: u32,
    /// 增殖速率（1/天）
    pub proliferation_rate: f32,
    /// 凋亡逃逸程度（0..1）
    pub apoptosis_resistance: f32,
    /// 侵袭能力（0..1）
    pub invasiveness: f32,
}

impl CancerCell {
    pub fn new() -> Self {
        Self { mutation_count: 0, proliferation_rate: 0.5, apoptosis_resistance: 0.2, invasiveness: 0.1 }
    }
    /// 累积突变（基因组不稳定性 hallmark）—— 突变越多恶性程度越高
    pub fn accumulate_mutations(&mut self, count: u32) {
        self.mutation_count += count;
        self.apoptosis_resistance = (self.apoptosis_resistance + 0.01 * count as f32).min(1.0);
        self.invasiveness = (self.invasiveness + 0.005 * count as f32).min(1.0);
    }
}
impl Default for CancerCell { fn default() -> Self { Self::new() } }

/// 肿瘤 —— Gompertz 生长模型（Laird 1964）
/// dV/dt = a·V·ln(K/V)，V 为体积，K 为承载量，a 为生长速率
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tumor {
    /// 肿瘤体积（mm³）
    pub volume_mm3: f32,
    /// Gompertz 生长速率 a（1/天）
    pub growth_rate: f32,
    /// 承载量 K（mm³），肿瘤最大体积
    pub carrying_capacity_mm3: f32,
    /// 血管新生信号
    pub angiogenesis: AngiogenesisSignal,
    /// 癌细胞状态
    pub cell: CancerCell,
}

impl Tumor {
    pub fn new() -> Self {
        Self { volume_mm3: 1.0, growth_rate: 0.1, carrying_capacity_mm3: 1000.0,
            angiogenesis: AngiogenesisSignal::new(), cell: CancerCell::new() }
    }
    /// Gompertz 生长方程瞬时增长率（Laird 1964，公式 1）：dV/dt = a·V·ln(K/V)
    pub fn growth_velocity(&self) -> f32 {
        if self.volume_mm3 <= 0.0 { return 0.0; }
        self.growth_rate * self.volume_mm3 * (self.carrying_capacity_mm3 / self.volume_mm3).ln()
    }
    /// 推进一个时间步（显式 Euler，dt 单位：天）
    pub fn update(&mut self, dt: f32) {
        let dv = self.growth_velocity() * dt;
        self.volume_mm3 = (self.volume_mm3 + dv).max(0.0).min(self.carrying_capacity_mm3);
        // 缺氧随体积上升（接近承载量时缺氧加重）
        self.angiogenesis.hypoxia =
            (self.volume_mm3 / self.carrying_capacity_mm3).clamp(0.0, 1.0);
        self.angiogenesis.update(dt);
    }
    /// 是否进入血管期（体积 > 2 mm³ 且 VEGF 阳性，Folkman 1971）
    pub fn is_vascularized(&self) -> bool {
        self.volume_mm3 > 2.0 && self.angiogenesis.is_angiogenic()
    }
}
impl Default for Tumor { fn default() -> Self { Self::new() } }

/// 转移靶器官（Chambers 2002 转移级联）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetastaticSite {
    Lung,
    Liver,
    Bone,
    Brain,
    LymphNode,
}

/// 转移事件（Chambers 2002 —— 转移级联：侵袭→循环→定植→生长）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Metastasis {
    /// 转移概率（0..1）
    pub probability: f32,
    /// 靶器官
    pub site: MetastaticSite,
    /// 转移灶数
    pub lesion_count: u32,
}

impl Metastasis {
    pub fn new() -> Self { Self { probability: 0.0, site: MetastaticSite::Lung, lesion_count: 0 } }
    /// 根据肿瘤体积与侵袭能力计算转移概率（Chambers 2002）—— 概率 ∝ 体积 × 侵袭能力
    pub fn update_probability(&mut self, tumor: &Tumor) {
        let vol_factor = (tumor.volume_mm3 / tumor.carrying_capacity_mm3).clamp(0.0, 1.0);
        self.probability = (vol_factor * tumor.cell.invasiveness).clamp(0.0, 1.0);
    }
    /// 尝试一次转移事件（返回是否成功）
    pub fn attempt(&mut self) -> bool {
        if self.probability >= 0.5 { self.lesion_count += 1; true } else { false }
    }
}
impl Default for Metastasis { fn default() -> Self { Self::new() } }

// 序列化 trait bound 编译期校验（避免引入 serde_json 依赖）
fn _assert_ser<T: Serialize>() {}
fn _assert_de<T: for<'de> Deserialize<'de>>() {}

#[cfg(test)]
mod tests {
    use super::*;

    // -------- Hallmark --------

    #[test]
    fn test_hallmark_all_count_is_ten() {
        // Hanahan & Weinberg 2011: 十大 Hallmarks
        assert_eq!(Hallmark::all().len(), 10, "应有十大 Hallmarks");
    }

    #[test]
    fn test_hallmark_panel_default_all_inactive() {
        let p = HallmarkPanel::new();
        for v in p.activations.iter() {
            assert_eq!(*v, 0.0, "健康细胞所有 hallmark 应为 0");
        }
        assert!(!p.is_malignant());
    }

    #[test]
    fn test_hallmark_panel_set_get() {
        let mut p = HallmarkPanel::new();
        p.set(Hallmark::ResistingCellDeath, 0.8);
        assert!((p.get(Hallmark::ResistingCellDeath) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_hallmark_panel_set_clamped() {
        let mut p = HallmarkPanel::new();
        p.set(Hallmark::SustainingProliferativeSignaling, 1.5);
        assert!(p.get(Hallmark::SustainingProliferativeSignaling) <= 1.0, "应 clamp 到 1");
        p.set(Hallmark::EvadingGrowthSuppressors, -0.5);
        assert!(p.get(Hallmark::EvadingGrowthSuppressors) >= 0.0, "应 clamp 到 0");
    }

    #[test]
    fn test_hallmark_panel_malignant_detection() {
        let mut p = HallmarkPanel::new();
        p.set(Hallmark::ActivatingInvasionAndMetastasis, 0.7);
        assert!(p.is_malignant(), "任一 hallmark > 0.5 应判为恶性");
    }

    #[test]
    fn test_hallmark_panel_active_count() {
        let mut p = HallmarkPanel::new();
        p.set(Hallmark::ResistingCellDeath, 0.6);
        p.set(Hallmark::InducingAngiogenesis, 0.4);
        assert_eq!(p.active_count(0.5), 1, "仅 1 个 hallmark > 0.5");
    }

    // -------- AngiogenesisSignal --------

    #[test]
    fn test_angiogenesis_default_below_threshold() {
        let a = AngiogenesisSignal::new();
        assert!(!a.is_angiogenic(), "默认 VEGF 5 pg/mL 应低于 35 pg/mL 阈值");
    }

    #[test]
    fn test_angiogenesis_is_angiogenic_above_threshold() {
        let mut a = AngiogenesisSignal::new();
        a.vegf_level = 40.0;
        assert!(a.is_angiogenic(), "VEGF 40 pg/mL 应触发血管新生");
    }

    #[test]
    fn test_angiogenesis_hypoxia_upregulates_vegf() {
        let mut a = AngiogenesisSignal::new();
        a.hypoxia = 1.0; // 严重缺氧
        let vegf_before = a.vegf_level;
        a.update(1.0);
        assert!(a.vegf_level > vegf_before, "HIF-1α 应上调 VEGF");
    }

    #[test]
    fn test_angiogenesis_vascularization_reduces_hypoxia() {
        let mut a = AngiogenesisSignal::new();
        a.vegf_level = 50.0; // 触发血管新生
        a.hypoxia = 0.8;
        let hypoxia_before = a.hypoxia;
        a.update(10.0);
        assert!(a.hypoxia < hypoxia_before, "血管化应缓解缺氧");
    }

    // -------- CancerCell --------

    #[test]
    fn test_cancer_cell_default_low_malignancy() {
        let c = CancerCell::new();
        assert_eq!(c.mutation_count, 0);
        assert!(c.apoptosis_resistance < 0.3);
    }

    #[test]
    fn test_cancer_cell_mutations_increase_malignancy() {
        let mut c = CancerCell::new();
        c.accumulate_mutations(20);
        assert_eq!(c.mutation_count, 20);
        assert!(c.apoptosis_resistance > 0.2, "突变应增加凋亡抗性");
        assert!(c.invasiveness > 0.1, "突变应增加侵袭性");
    }

    // -------- Tumor (Gompertz 生长) --------

    #[test]
    fn test_tumor_default_params() {
        let t = Tumor::new();
        assert_eq!(t.volume_mm3, 1.0, "初始体积 1 mm³");
        assert!(t.growth_rate > 0.0);
        assert!(t.carrying_capacity_mm3 > t.volume_mm3);
    }

    #[test]
    fn test_tumor_gompertz_growth_monotonic_increase() {
        let mut t = Tumor::new();
        let mut prev = t.volume_mm3;
        for _ in 0..50 {
            t.update(1.0);
            assert!(t.volume_mm3 >= prev, "Gompertz 生长应单调递增, 实际 {} -> {}", prev, t.volume_mm3);
            prev = t.volume_mm3;
        }
    }

    #[test]
    fn test_tumor_gompertz_approaches_carrying_capacity() {
        let mut t = Tumor::new();
        for _ in 0..500 { t.update(1.0); }
        // 长时间后应接近 K
        assert!(t.volume_mm3 > t.carrying_capacity_mm3 * 0.9,
            "应接近承载量 K={}, 实际 {}", t.carrying_capacity_mm3, t.volume_mm3);
        assert!(t.volume_mm3 <= t.carrying_capacity_mm3, "不应超过承载量");
    }

    #[test]
    fn test_tumor_growth_velocity_positive_below_and_zero_at_capacity() {
        let t = Tumor::new();
        // V < K → ln(K/V) > 0 → 正增长
        assert!(t.growth_velocity() > 0.0, "V<K 时增长率应为正");
        let mut t2 = Tumor::new();
        t2.volume_mm3 = t2.carrying_capacity_mm3;
        // V = K → ln(K/V) = ln(1) = 0 → 零增长
        assert!(t2.growth_velocity().abs() < 1e-5, "V=K 时增长率应为 0");
    }

    #[test]
    fn test_tumor_vascularization_triggered() {
        let mut t = Tumor::new();
        t.angiogenesis.vegf_level = 50.0;
        t.volume_mm3 = 5.0; // > 2 mm³
        assert!(t.is_vascularized(), "体积>2mm³ 且 VEGF 阳性应判为血管化");
    }

    #[test]
    fn test_tumor_update_increases_volume() {
        let mut t = Tumor::new();
        let before = t.volume_mm3;
        t.update(1.0);
        assert!(t.volume_mm3 > before, "生长后体积应增加");
    }

    // -------- Metastasis --------

    #[test]
    fn test_metastasis_default_zero_probability() {
        let m = Metastasis::new();
        assert_eq!(m.probability, 0.0);
        assert_eq!(m.lesion_count, 0);
    }

    #[test]
    fn test_metastasis_probability_increases_with_tumor_size() {
        let mut m = Metastasis::new();
        let mut small_tumor = Tumor::new();
        small_tumor.volume_mm3 = 10.0;
        small_tumor.cell.invasiveness = 0.5;
        m.update_probability(&small_tumor);
        let p_small = m.probability;

        let mut large_tumor = Tumor::new();
        large_tumor.volume_mm3 = 800.0;
        large_tumor.cell.invasiveness = 0.5;
        m.update_probability(&large_tumor);
        let p_large = m.probability;
        assert!(p_large > p_small, "大肿瘤转移概率应更高");
    }

    #[test]
    fn test_metastasis_probability_bounded() {
        let mut m = Metastasis::new();
        let mut t = Tumor::new();
        t.volume_mm3 = 10000.0; // 远超承载量
        t.cell.invasiveness = 1.0;
        m.update_probability(&t);
        assert!(m.probability <= 1.0, "转移概率应 ≤ 1");
    }

    #[test]
    fn test_metastasis_attempt_high_probability() {
        let mut m = Metastasis::new();
        m.probability = 0.7;
        assert!(m.attempt(), "高概率应成功转移");
        assert_eq!(m.lesion_count, 1);
    }

    #[test]
    fn test_metastasis_attempt_low_probability() {
        let mut m = Metastasis::new();
        m.probability = 0.2;
        assert!(!m.attempt(), "低概率不应转移");
        assert_eq!(m.lesion_count, 0);
    }

    // -------- 序列化 trait bound 编译期校验 --------

    #[test]
    fn test_serde_traits_implemented_for_all_types() {
        _assert_ser::<Hallmark>();        _assert_de::<Hallmark>();
        _assert_ser::<HallmarkPanel>();   _assert_de::<HallmarkPanel>();
        _assert_ser::<AngiogenesisSignal>(); _assert_de::<AngiogenesisSignal>();
        _assert_ser::<CancerCell>();      _assert_de::<CancerCell>();
        _assert_ser::<Tumor>();           _assert_de::<Tumor>();
        _assert_ser::<Metastasis>();      _assert_de::<Metastasis>();
        _assert_ser::<MetastaticSite>();  _assert_de::<MetastaticSite>();
    }
}
