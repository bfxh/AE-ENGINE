//! 衰老模块 —— 端粒缩短、Hayflick 极限、细胞衰老与 Gompertz 死亡率
//!
//! 科学来源：
//! - Hayflick, L. (1965). Exp. Cell Res. 37: 614-636. —— Hayflick 极限 ~50 次分裂
//! - Olovnikov, A. M. (1973). J. Theor. Biol. 41: 181-190. —— 端粒"末端复制问题"
//! - Harley, C. B. et al. (1990). Nature 345: 458-460. —— 缩短速率 50-100 bp/分裂
//! - Gompertz, B. (1825). Phil. Trans. R. Soc. 115: 513-585. —— μ(t) = a·e^(b·t)
//! - Campisi, J. (2013). Annu. Rev. Physiol. 75: 685-705. —— SASP
//! - Cortopassi, G. A. & Arnheim, N. (1990). Nucleic Acids Res. 18: 6927. —— mtDNA 损伤
//!
//! 核心规律：端粒缩短 50-100 bp/分裂；Hayflick ~50 次；Gompertz a≈10⁻⁴/年，b≈0.085/年；
//! 死亡率倍增时间 ln2/b ≈ 8.15 年；SASP 因子含 IL-6、IL-8、MMP、TGF-β。

use serde::{Deserialize, Serialize};

/// 端粒 —— 染色体末端保护性结构（Olovnikov 1973 末端复制问题）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Telomere {
    /// 端粒长度（bp），人类新生儿约 10000 bp
    pub length_bp: f32,
    /// 每次分裂缩短速率（bp/分裂），默认 75 bp（Harley 1990 区间中值）
    pub shortening_rate_bp: f32,
    /// 触发衰老的临界长度（bp），~4000 bp
    pub critical_length_bp: f32,
    /// 已经历分裂次数
    pub divisions: u32,
}

impl Telomere {
    /// 默认新生儿端粒：10000 bp，每次缩短 75 bp，临界 4000 bp
    pub fn new() -> Self {
        Self { length_bp: 10000.0, shortening_rate_bp: 75.0, critical_length_bp: 4000.0, divisions: 0 }
    }
    /// 推进一次细胞分裂 —— 端粒缩短（Olovnikov 1973）
    /// 返回 true 表示分裂成功，false 表示已达 Hayflick 极限
    pub fn divide(&mut self) -> bool {
        if self.is_senescent() { return false; }
        self.length_bp = (self.length_bp - self.shortening_rate_bp).max(0.0);
        self.divisions += 1;
        true
    }
    /// 是否已触发 Hayflick 极限（端粒短于临界长度）
    pub fn is_senescent(&self) -> bool { self.length_bp <= self.critical_length_bp }
    /// 剩余可分裂次数（Hayflick 1965）
    pub fn remaining_divisions(&self) -> u32 {
        if self.length_bp <= self.critical_length_bp { return 0; }
        ((self.length_bp - self.critical_length_bp) / self.shortening_rate_bp).floor() as u32
    }
}
impl Default for Telomere { fn default() -> Self { Self::new() } }

/// SASP 因子谱（Campisi 2013）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SaspFactor { Il6, Il8, Mmp, TgfBeta, Igfbp, Vegf }

/// 衰老相关分子标志物
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SenescenceMarker { P16Ink4a, P21Cip1, SaBetaGal, GammaH2ax }

/// 衰老细胞 —— 已达 Hayflick 极限或 DNA 损伤诱导的永久细胞周期阻滞（Campisi 2013）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SenescentCell {
    pub is_senescent: bool,
    /// p16^INK4a 表达水平（0..1，正常 <0.1，衰老 >0.5）
    pub p16_level: f32,
    /// p21^CIP1 表达水平（0..1）
    pub p21_level: f32,
    /// SA-β-gal 活性（0..1）
    pub sa_beta_gal: f32,
    /// SASP 分泌强度（0..1）
    pub sasp_intensity: f32,
    /// DNA 损伤焦点数（γH2AX foci）
    pub dna_damage_foci: u32,
    /// 衰老持续时长（年）
    pub senescence_age_years: f32,
}

impl SenescentCell {
    pub fn new() -> Self {
        Self { is_senescent: false, p16_level: 0.05, p21_level: 0.05, sa_beta_gal: 0.0,
            sasp_intensity: 0.0, dna_damage_foci: 0, senescence_age_years: 0.0 }
    }
    /// 诱导细胞进入衰老状态
    pub fn induce_senescence(&mut self, dna_damage_foci: u32) {
        self.is_senescent = true;
        self.p16_level = 0.7;
        self.p21_level = 0.6;
        self.sa_beta_gal = 0.8;
        self.sasp_intensity = 0.5;
        self.dna_damage_foci = dna_damage_foci;
    }
    /// SASP 因子分泌速率 —— 饱和曲线 1 - e^(-age/τ)，τ≈2 年（Campisi 2013）
    pub fn sasp_secretion_rate(&self) -> f32 {
        if !self.is_senescent { return 0.0; }
        self.sasp_intensity * (1.0 - (-self.senescence_age_years / 2.0).exp())
    }
    /// 衰老推进（显式 Euler，dt 单位：年）
    pub fn update(&mut self, dt: f32) {
        if self.is_senescent {
            self.senescence_age_years += dt;
            self.sasp_intensity = (self.sasp_intensity + 0.01 * dt).min(1.0);
        }
    }
}
impl Default for SenescentCell { fn default() -> Self { Self::new() } }

/// 线粒体 DNA 损伤累积 —— 衰老的"线粒体理论"（Cortopassi & Arnheim 1990）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MitochondrialDamage {
    /// mtDNA 缺失突变比例（0..1），老年人组织可达 0.01-0.1
    pub deletion_fraction: f32,
    /// 点突变比例（0..1）
    pub point_mutation_fraction: f32,
    /// ROS 产生速率（相对值 0..1）
    pub ros_production_rate: f32,
    /// mtDNA 拷贝数（人类细胞约 1000-10000 拷贝）
    pub copy_number: f32,
    /// 损伤累积速率系数（1/年）
    pub accumulation_rate: f32,
}

impl MitochondrialDamage {
    pub fn new() -> Self {
        Self { deletion_fraction: 0.0, point_mutation_fraction: 0.0, ros_production_rate: 0.1,
            copy_number: 5000.0, accumulation_rate: 0.002 }
    }
    /// 推进损伤累积（显式 Euler，dt 单位：年）
    /// 线性累积 + ROS 正反馈：损伤越多 → ROS 越多 → 损伤更快
    pub fn update(&mut self, dt: f32) {
        let acc = self.accumulation_rate * (1.0 + self.ros_production_rate) * dt;
        self.deletion_fraction = (self.deletion_fraction + acc).min(1.0);
        self.point_mutation_fraction = (self.point_mutation_fraction + acc * 0.5).min(1.0);
        self.ros_production_rate = (self.ros_production_rate + 0.01 * self.deletion_fraction * dt).min(1.0);
        self.copy_number = (self.copy_number - 5.0 * self.deletion_fraction * dt).max(0.0);
    }
    /// 是否达到功能障碍阈值（>5% 缺失即显著影响氧化磷酸化）
    pub fn is_dysfunctional(&self) -> bool { self.deletion_fraction > 0.05 }
}
impl Default for MitochondrialDamage { fn default() -> Self { Self::new() } }

/// Gompertz 死亡率模型（Gompertz 1825）
/// μ(t) = a · e^(b·t)；a 基底死亡率（1/年）≈10⁻⁴；b 衰老速率（1/年）≈0.085
/// 死亡率倍增时间 T₂ = ln(2)/b ≈ 8.15 年
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GompertzMortality {
    /// 基底死亡率 a（1/年）
    pub baseline_mortality: f32,
    /// 衰老速率系数 b（1/年）
    pub aging_rate: f32,
    /// 当前年龄（年）
    pub age_years: f32,
}

impl GompertzMortality {
    pub fn new() -> Self {
        Self { baseline_mortality: 1.0e-4, aging_rate: 0.085, age_years: 0.0 }
    }
    /// 自定义参数构造
    pub fn with_params(baseline: f32, rate: f32) -> Self {
        Self { baseline_mortality: baseline, aging_rate: rate, age_years: 0.0 }
    }
    /// 给定年龄的瞬时死亡率 μ(t)（Gompertz 1825，公式 1）
    pub fn mortality_at(&self, age: f32) -> f32 {
        self.baseline_mortality * (self.aging_rate * age).exp()
    }
    /// 当前年龄的瞬时死亡率
    pub fn current_mortality(&self) -> f32 { self.mortality_at(self.age_years) }
    /// 死亡率倍增时间 T₂ = ln(2)/b
    pub fn mortality_doubling_time(&self) -> f32 {
        if self.aging_rate <= 0.0 { f32::INFINITY } else { core::f32::consts::LN_2 / self.aging_rate }
    }
    /// 推进年龄（dt 单位：年）
    pub fn update(&mut self, dt: f32) { self.age_years += dt; }
}
impl Default for GompertzMortality { fn default() -> Self { Self::new() } }

/// 衰老累积器 —— 集成端粒、SASP、mtDNA 损伤与 Gompertz 死亡率
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AgingAccumulator {
    pub telomere: Telomere,
    pub senescent_cell: SenescentCell,
    pub mtdna: MitochondrialDamage,
    pub mortality: GompertzMortality,
    /// 总衰老负担（0..1，综合指标）
    pub aging_burden: f32,
}

impl AgingAccumulator {
    pub fn new() -> Self {
        Self { telomere: Telomere::new(), senescent_cell: SenescentCell::new(),
            mtdna: MitochondrialDamage::new(), mortality: GompertzMortality::new(), aging_burden: 0.0 }
    }
    /// 推进一个时间步（dt 单位：年）
    pub fn update(&mut self, dt: f32) {
        self.mortality.update(dt);
        self.mtdna.update(dt);
        self.senescent_cell.update(dt);
        if self.telomere.is_senescent() && !self.senescent_cell.is_senescent {
            self.senescent_cell.induce_senescence(self.senescent_cell.dna_damage_foci);
        }
        let telomere_burden = 1.0 - (self.telomere.length_bp / 10000.0).clamp(0.0, 1.0);
        let sasp_burden = self.senescent_cell.sasp_secretion_rate();
        let mtdna_burden = self.mtdna.deletion_fraction;
        self.aging_burden = (0.4 * telomere_burden + 0.3 * sasp_burden + 0.3 * mtdna_burden).clamp(0.0, 1.0);
    }
    /// 推进一次细胞分裂（端粒层面）
    pub fn cell_division(&mut self) -> bool {
        let ok = self.telomere.divide();
        if !ok && !self.senescent_cell.is_senescent {
            self.senescent_cell.induce_senescence(5);
        }
        ok
    }
}
impl Default for AgingAccumulator { fn default() -> Self { Self::new() } }

// 序列化 trait bound 编译期校验（避免引入 serde_json 依赖）
fn _assert_ser<T: Serialize>() {}
fn _assert_de<T: for<'de> Deserialize<'de>>() {}

#[cfg(test)]
mod tests {
    use super::*;

    // -------- Telomere --------

    #[test]
    fn test_telomere_default_length() {
        let t = Telomere::new();
        assert_eq!(t.length_bp, 10000.0, "新生儿端粒默认 10000 bp");
    }

    #[test]
    fn test_telomere_shortening_rate_in_harley_range() {
        let t = Telomere::new();
        assert!(t.shortening_rate_bp >= 50.0 && t.shortening_rate_bp <= 100.0,
            "Harley 1990: 缩短速率应在 50-100 bp/分裂, 实际 {}", t.shortening_rate_bp);
    }

    #[test]
    fn test_telomere_shortens_after_division() {
        let mut t = Telomere::new();
        let before = t.length_bp;
        assert!(t.divide());
        assert_eq!(t.length_bp, before - 75.0);
        assert_eq!(t.divisions, 1);
    }

    #[test]
    fn test_telomere_hayflick_limit_triggers_senescence() {
        let mut t = Telomere::new();
        t.length_bp = t.critical_length_bp;
        assert!(t.is_senescent(), "端粒达临界应触发 Hayflick 极限");
    }

    #[test]
    fn test_telomere_division_blocked_when_senescent() {
        let mut t = Telomere::new();
        t.length_bp = t.critical_length_bp;
        assert!(!t.divide(), "已达 Hayflick 极限不能再分裂");
        assert_eq!(t.divisions, 0, "失败的分裂不应计数");
    }

    #[test]
    fn test_telomere_remaining_divisions_hayflick_scale() {
        let t = Telomere::new();
        let r = t.remaining_divisions();
        // (10000 - 4000) / 75 = 80 次理论剩余，Hayflick 1965 实测 ~50
        assert!(r > 30 && r < 120, "剩余分裂次数 {}", r);
    }

    // -------- SenescentCell --------

    #[test]
    fn test_senescent_cell_default_not_senescent() {
        let c = SenescentCell::new();
        assert!(!c.is_senescent);
        assert!(c.p16_level < 0.1, "正常细胞 p16 应低表达");
    }

    #[test]
    fn test_senescent_cell_induce_sets_markers() {
        let mut c = SenescentCell::new();
        c.induce_senescence(10);
        assert!(c.is_senescent);
        assert!(c.p16_level > 0.5, "衰老细胞 p16 应高表达");
        assert!(c.sa_beta_gal > 0.5, "SA-β-gal 应阳性");
        assert_eq!(c.dna_damage_foci, 10);
    }

    #[test]
    fn test_sasp_secretion_zero_when_not_senescent() {
        assert_eq!(SenescentCell::new().sasp_secretion_rate(), 0.0);
    }

    #[test]
    fn test_sasp_secretion_positive_when_senescent() {
        let mut c = SenescentCell::new();
        c.induce_senescence(5);
        c.senescence_age_years = 1.0;
        assert!(c.sasp_secretion_rate() > 0.0, "衰老细胞应分泌 SASP");
    }

    // -------- MitochondrialDamage --------

    #[test]
    fn test_mtdna_default_zero_damage() {
        let m = MitochondrialDamage::new();
        assert_eq!(m.deletion_fraction, 0.0, "新生 mtDNA 无缺失");
        assert!(m.copy_number > 1000.0, "正常拷贝数应高");
    }

    #[test]
    fn test_mtdna_damage_accumulates_over_time() {
        let mut m = MitochondrialDamage::new();
        let before = m.deletion_fraction;
        m.update(1.0);
        assert!(m.deletion_fraction > before, "mtDNA 损伤应随时间累积");
    }

    #[test]
    fn test_mtdna_dysfunctional_threshold() {
        let mut m = MitochondrialDamage::new();
        m.deletion_fraction = 0.06;
        assert!(m.is_dysfunctional(), ">5% 缺失应判为功能障碍");
    }

    #[test]
    fn test_mtdna_ros_positive_feedback() {
        let mut m = MitochondrialDamage::new();
        m.deletion_fraction = 0.5;
        let ros_before = m.ros_production_rate;
        m.update(1.0);
        assert!(m.ros_production_rate > ros_before, "ROS 应随损伤正反馈上升");
    }

    // -------- GompertzMortality --------

    #[test]
    fn test_gompertz_default_aging_rate_matches_literature() {
        let g = GompertzMortality::new();
        assert!((g.aging_rate - 0.085).abs() < 0.01, "b 应约 0.085, 实际 {}", g.aging_rate);
    }

    #[test]
    fn test_gompertz_mortality_increases_with_age() {
        let g = GompertzMortality::new();
        assert!(g.mortality_at(80.0) > g.mortality_at(20.0), "Gompertz 死亡率应随年龄上升");
    }

    #[test]
    fn test_gompertz_mortality_at_age_zero_equals_baseline() {
        let g = GompertzMortality::new();
        // μ(0) = a · e^0 = a
        assert!((g.mortality_at(0.0) - g.baseline_mortality).abs() < 1e-6, "μ(0) 应等于 a");
    }

    #[test]
    fn test_gompertz_mortality_doubling_time_about_8_years() {
        let g = GompertzMortality::new();
        // ln(2)/0.085 ≈ 8.15 年
        assert!((g.mortality_doubling_time() - 8.0).abs() < 1.0,
            "死亡率倍增时间应约 8 年, 实际 {}", g.mortality_doubling_time());
    }

    #[test]
    fn test_gompertz_mortality_actually_doubles_in_t2() {
        let g = GompertzMortality::new();
        let t2 = g.mortality_doubling_time();
        let ratio = g.mortality_at(50.0 + t2) / g.mortality_at(50.0);
        // m2/m1 = e^(b·t2) = e^(ln2) = 2
        assert!((ratio - 2.0).abs() < 0.01, "倍增时间内死亡率应翻倍, 比例 {}", ratio);
    }

    // -------- AgingAccumulator --------

    #[test]
    fn test_accumulator_default_clean() {
        let a = AgingAccumulator::new();
        assert_eq!(a.aging_burden, 0.0);
        assert!(!a.senescent_cell.is_senescent);
        assert_eq!(a.telomere.length_bp, 10000.0);
    }

    #[test]
    fn test_accumulator_update_advances_age_and_mtdna() {
        let mut a = AgingAccumulator::new();
        let mtdna_before = a.mtdna.deletion_fraction;
        a.update(2.0);
        assert!((a.mortality.age_years - 2.0).abs() < 1e-5);
        assert!(a.mtdna.deletion_fraction > mtdna_before);
    }

    #[test]
    fn test_accumulator_cell_division_until_hayflick() {
        let mut a = AgingAccumulator::new();
        let mut divisions = 0;
        while a.cell_division() {
            divisions += 1;
            if divisions > 200 { break; }
        }
        assert!(divisions > 30, "应能分裂数十次, 实际 {}", divisions);
        assert!(a.senescent_cell.is_senescent, "端粒耗尽应诱导衰老");
    }

    #[test]
    fn test_accumulator_aging_burden_rises_and_bounded() {
        let mut a = AgingAccumulator::new();
        let initial = a.aging_burden;
        for _ in 0..100 { a.update(1.0); }
        assert!(a.aging_burden > initial, "衰老负担应随时间上升");
        assert!(a.aging_burden <= 1.0, "负担应被 clamp 到 1");
    }

    // -------- 序列化 trait bound 编译期校验 --------

    #[test]
    fn test_serde_traits_implemented_for_all_types() {
        // 若类型未实现 Serialize/Deserialize，此处编译失败
        _assert_ser::<Telomere>();         _assert_de::<Telomere>();
        _assert_ser::<SenescentCell>();    _assert_de::<SenescentCell>();
        _assert_ser::<MitochondrialDamage>(); _assert_de::<MitochondrialDamage>();
        _assert_ser::<GompertzMortality>(); _assert_de::<GompertzMortality>();
        _assert_ser::<AgingAccumulator>(); _assert_de::<AgingAccumulator>();
        _assert_ser::<SaspFactor>();       _assert_de::<SaspFactor>();
        _assert_ser::<SenescenceMarker>(); _assert_de::<SenescenceMarker>();
    }
}
