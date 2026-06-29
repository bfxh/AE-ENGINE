//! 发育生物学模块 — 胚胎发生、形态发生素梯度与 HOX 基因
//!
//! 生物学背景:
//!   胚胎发生经历卵裂（受精卵 → 多细胞囊胚）、原肠胚形成（三胚层建立）、
//!   神经胚形成（神经管发育）等关键阶段。形态发生素（如 Bicoid、Nodal、Shh、Wnt）
//!   通过浓度梯度提供位置信息，决定细胞命运。HOX 基因簇则通过共线性表达
//!   沿前后轴定义体节身份。
//!
//! 论文来源:
//! - Driever, W., Nusslein-Volhard, C. (1988). "A gradient of bicoid protein in
//!   Drosophila embryos." Cell 54(1): 83-93. (Bicoid 指数衰减梯度)
//! - Lewis, E. B. (1978). "A gene complex controlling segmentation in Drosophila."
//!   Nature 276: 565-570. (Bithorax 复合体, 1995 Nobel)
//! - McGinnis, W., Krumlauf, R. (1992). "Homeobox genes and axial patterning."
//!   Cell 68(2): 283-302. (HOX 共线性)
//! - Wolpert, L. (1969). "Positional information and the spatial pattern of
//!   cellular differentiation." J. Theor. Biol. 25(1): 1-47. (位置信息理论)
//! - Gilbert, S. F. (2014). "Developmental Biology." 10th ed. Sinauer.
//!
//! 物理量单位:
//!   - 长度: 卵细胞无量纲前后位置 x ∈ [0, 1] (0=前, 1=后)
//!   - 浓度: 相对浓度 0..1
//!   - 时间: 发育小时 (DevHr, Drosophila 25°C 下的发育时间)

use serde::{Deserialize, Serialize};

/// 发育阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevelopmentalStage {
    /// 合子 (受精卵)
    Zygote,
    /// 卵裂期 (1 → 2 → 4 → 8 → 16 → 32)
    Cleavage,
    /// 桑椹胚
    Morula,
    /// 囊胚 / 胚泡
    Blastula,
    /// 原肠胚
    Gastrula,
    /// 神经胚
    Neurula,
    /// 器官发生
    Organogenesis,
    /// 胎儿/成体形态
    Fetus,
}

impl DevelopmentalStage {
    /// 阶段在序列中的顺序索引 (0..8)
    pub fn order_index(&self) -> u8 {
        match self {
            Self::Zygote => 0,
            Self::Cleavage => 1,
            Self::Morula => 2,
            Self::Blastula => 3,
            Self::Gastrula => 4,
            Self::Neurula => 5,
            Self::Organogenesis => 6,
            Self::Fetus => 7,
        }
    }

    /// 是否进入三胚层已建立阶段 (Gastrula 及以后)
    pub fn has_germ_layers(&self) -> bool {
        self.order_index() >= Self::Gastrula.order_index()
    }
}

/// 发育时钟 — 推进发育阶段
/// 来源: Gilbert 2014 DevBio
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DevelopmentalClock {
    /// 当前阶段
    pub stage: DevelopmentalStage,
    /// 发育小时数 (25°C Drosophila)
    pub dev_hours: f32,
    /// 当前阶段累计小时
    pub stage_hours: f32,
}

impl Default for DevelopmentalClock {
    fn default() -> Self {
        Self {
            stage: DevelopmentalStage::Zygote,
            dev_hours: 0.0,
            stage_hours: 0.0,
        }
    }
}

impl DevelopmentalClock {
    /// 各阶段持续时长 (Drosophila 25°C, h)
    fn stage_duration_hours(stage: DevelopmentalStage) -> f32 {
        match stage {
            DevelopmentalStage::Zygote => 0.5,
            DevelopmentalStage::Cleavage => 2.5,
            DevelopmentalStage::Morula => 1.0,
            DevelopmentalStage::Blastula => 2.0,
            DevelopmentalStage::Gastrula => 3.0,
            DevelopmentalStage::Neurula => 3.0,
            DevelopmentalStage::Organogenesis => 12.0,
            DevelopmentalStage::Fetus => 0.0, // 终态
        }
    }

    /// 显式 Euler 单步推进
    pub fn step(&mut self, dt_h: f32) {
        self.dev_hours += dt_h;
        self.stage_hours += dt_h;
        let dur = Self::stage_duration_hours(self.stage);
        if self.stage_hours >= dur && self.stage != DevelopmentalStage::Fetus {
            self.stage_hours -= dur;
            self.advance_stage();
        }
    }

    /// 直接推进到下一阶段
    pub fn advance_stage(&mut self) {
        use DevelopmentalStage::*;
        self.stage = match self.stage {
            Zygote => Cleavage,
            Cleavage => Morula,
            Morula => Blastula,
            Blastula => Gastrula,
            Gastrula => Neurula,
            Neurula => Organogenesis,
            Organogenesis => Fetus,
            Fetus => Fetus,
        };
        self.stage_hours = 0.0;
    }
}

/// 卵裂期信息
/// 来源: Gilbert 2014
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CleavageStage {
    /// 当前细胞数 (1, 2, 4, 8, 16, 32 ...)
    pub cell_count: u32,
    /// 卵裂次数
    pub cleavage_divisions: u32,
}

impl Default for CleavageStage {
    fn default() -> Self {
        Self {
            cell_count: 1,
            cleavage_divisions: 0,
        }
    }
}

impl CleavageStage {
    /// 一次卵裂 — 细胞数翻倍
    /// 来源: 经典卵裂模式 1 → 2 → 4 → 8 → 16 → 32
    pub fn divide(&mut self) {
        self.cell_count = self.cell_count.saturating_mul(2);
        self.cleavage_divisions = self.cleavage_divisions.saturating_add(1);
    }

    /// 是否到达囊胚 (典型 ~32 细胞)
    pub fn is_blastula_ready(&self) -> bool {
        self.cell_count >= 32
    }

    /// 通过细胞数推算卵裂次数 (log2)
    pub fn divisions_from_count(cell_count: u32) -> u32 {
        if cell_count == 0 {
            return 0;
        }
        31 - cell_count.leading_zeros()
    }
}

/// 形态发生素梯度 — Bicoid 指数衰减模型
/// 来源: Driever & Nusslein-Volhard 1988 Cell
/// C(x) = C_0 * exp(-x / lambda)
/// x ∈ [0, 1] 为相对前后位置, 0 = 前端 (高浓度), 1 = 后端 (低浓度)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MorphogenGradient {
    /// 形态发生素类型
    pub morphogen: Morphogen,
    /// 前端初始浓度 C_0 (相对单位)
    pub c0: f32,
    /// 衰减长度 lambda (相对单位, Bicoid ~ 0.2 卵长)
    pub lambda: f32,
}

/// 形态发生素种类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Morphogen {
    /// Bicoid (Drosophila 前轴)
    Bicoid,
    /// Nanos (后轴)
    Nanos,
    /// Nodal (中胚层诱导)
    Nodal,
    /// Sonic Hedgehog (神经管腹轴)
    Shh,
    /// Wnt (后轴/尾轴)
    Wnt,
}

impl Default for MorphogenGradient {
    fn default() -> Self {
        Self {
            morphogen: Morphogen::Bicoid,
            c0: 1.0,
            lambda: 0.2,
        }
    }
}

impl MorphogenGradient {
    /// 给定相对位置 x ∈ [0, 1] 返回相对浓度
    /// 来源: Driever & Nusslein-Volhard 1988 Eq. 1
    pub fn concentration_at(&self, x: f32) -> f32 {
        let x_clamped = x.clamp(0.0, 1.0);
        if self.lambda > 0.0 {
            self.c0 * (-x_clamped / self.lambda).exp()
        } else {
            0.0
        }
    }

    /// 形态发生素阈值位置 — 浓度等于 threshold 时的 x
    /// x_threshold = -lambda * ln(threshold / C_0)
    pub fn threshold_position(&self, threshold: f32) -> f32 {
        if self.c0 <= 0.0 || threshold <= 0.0 || threshold >= self.c0 {
            return 0.0;
        }
        -self.lambda * (threshold / self.c0).ln()
    }

    /// 浓度梯度方向 — 前端是否高于后端
    pub fn is_anterior_high(&self) -> bool {
        self.concentration_at(0.0) > self.concentration_at(1.0)
    }
}

/// HOX 基因 (共线性表达)
/// 来源: McGinnis & Krumlauf 1992 Cell
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HoxGene {
    /// Hox1 (前部表达, 1号 paralog)
    Hox1,
    Hox2,
    Hox3,
    Hox4,
    Hox5,
    Hox6,
    Hox7,
    /// Hox8 (后部表达, 8号 paralog)
    Hox8,
    Hox9,
    Hox10,
    Hox11,
    /// Hox13 (最尾部)
    Hox13,
}

impl HoxGene {
    /// 共线性顺序索引 (1..13)
    pub fn colinear_index(&self) -> u8 {
        match self {
            Self::Hox1 => 1,
            Self::Hox2 => 2,
            Self::Hox3 => 3,
            Self::Hox4 => 4,
            Self::Hox5 => 5,
            Self::Hox6 => 6,
            Self::Hox7 => 7,
            Self::Hox8 => 8,
            Self::Hox9 => 9,
            Self::Hox10 => 10,
            Self::Hox11 => 11,
            Self::Hox13 => 13,
        }
    }

    /// 表达边界 — 沿前后轴的位置 (0=前, 1=后)
    /// 共线性: 索引越大, 表达越靠后
    /// 来源: McGinnis & Krumlauf 1992
    pub fn anterior_boundary(&self) -> f32 {
        (self.colinear_index() as f32) / 14.0
    }
}

/// HOX 基因表达状态 — 跟踪簇中哪些基因已被激活
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HoxExpressionState {
    /// 已激活到的最高 HOX 索引 (0 = 未激活)
    pub max_activated: u8,
}

impl Default for HoxExpressionState {
    fn default() -> Self {
        Self { max_activated: 0 }
    }
}

impl HoxExpressionState {
    /// 按共线性顺序激活到指定 HOX 基因 (含其前面的全部)
    pub fn activate_up_to(&mut self, gene: HoxGene) {
        let idx = gene.colinear_index();
        if idx > self.max_activated {
            self.max_activated = idx;
        }
    }

    /// 检查指定 HOX 基因是否已被激活
    pub fn is_active(&self, gene: HoxGene) -> bool {
        gene.colinear_index() <= self.max_activated
    }

    /// 全部 13 个 HOX 是否激活
    pub fn is_complete(&self) -> bool {
        self.max_activated >= 13
    }
}

/// 胚胎状态综合
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Embryo {
    pub clock: DevelopmentalClock,
    pub cleavage: CleavageStage,
    pub bicoid: MorphogenGradient,
    pub hox_state: HoxExpressionState,
}

impl Default for Embryo {
    fn default() -> Self {
        Self {
            clock: DevelopmentalClock::default(),
            cleavage: CleavageStage::default(),
            bicoid: MorphogenGradient::default(),
            hox_state: HoxExpressionState::default(),
        }
    }
}

impl Embryo {
    /// 原肠胚形成 — 推进到 Gastrula 阶段
    pub fn gastrulate(&mut self) {
        while self.clock.stage != DevelopmentalStage::Gastrula
            && self.clock.stage != DevelopmentalStage::Fetus
        {
            self.clock.advance_stage();
        }
    }

    /// 卵裂一次
    pub fn cleave_once(&mut self) {
        self.cleavage.divide();
        if self.cleavage.is_blastula_ready()
            && self.clock.stage == DevelopmentalStage::Cleavage
        {
            self.clock.advance_stage();
        }
    }

    /// 沿 HOX 共线性激活下一个基因
    pub fn activate_next_hox(&mut self) -> Option<HoxGene> {
        let next_idx = self.hox_state.max_activated + 1;
        if next_idx > 13 {
            return None;
        }
        let gene = match next_idx {
            1 => HoxGene::Hox1,
            2 => HoxGene::Hox2,
            3 => HoxGene::Hox3,
            4 => HoxGene::Hox4,
            5 => HoxGene::Hox5,
            6 => HoxGene::Hox6,
            7 => HoxGene::Hox7,
            8 => HoxGene::Hox8,
            9 => HoxGene::Hox9,
            10 => HoxGene::Hox10,
            11 => HoxGene::Hox11,
            13 => HoxGene::Hox13,
            _ => return None,
        };
        self.hox_state.activate_up_to(gene);
        Some(gene)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embryo_default_state() {
        let e = Embryo::default();
        assert_eq!(e.clock.stage, DevelopmentalStage::Zygote);
        assert_eq!(e.cleavage.cell_count, 1);
        assert!((e.bicoid.c0 - 1.0).abs() < 1e-5);
        assert_eq!(e.hox_state.max_activated, 0);
    }

    #[test]
    fn test_developmental_stage_order() {
        assert!(DevelopmentalStage::Zygote.order_index() < DevelopmentalStage::Cleavage.order_index());
        assert!(DevelopmentalStage::Cleavage.order_index() < DevelopmentalStage::Gastrula.order_index());
        assert!(DevelopmentalStage::Gastrula.order_index() < DevelopmentalStage::Fetus.order_index());
    }

    #[test]
    fn test_developmental_stage_germ_layers() {
        assert!(!DevelopmentalStage::Zygote.has_germ_layers());
        assert!(!DevelopmentalStage::Cleavage.has_germ_layers());
        assert!(!DevelopmentalStage::Blastula.has_germ_layers());
        assert!(DevelopmentalStage::Gastrula.has_germ_layers());
        assert!(DevelopmentalStage::Neurula.has_germ_layers());
        assert!(DevelopmentalStage::Fetus.has_germ_layers());
    }

    #[test]
    fn test_cleavage_default_single_cell() {
        let c = CleavageStage::default();
        assert_eq!(c.cell_count, 1);
        assert_eq!(c.cleavage_divisions, 0);
        assert!(!c.is_blastula_ready());
    }

    #[test]
    fn test_cleavage_division_doubles_cell_count() {
        let mut c = CleavageStage::default();
        c.divide();
        assert_eq!(c.cell_count, 2);
        c.divide();
        assert_eq!(c.cell_count, 4);
        c.divide();
        assert_eq!(c.cell_count, 8);
        c.divide();
        assert_eq!(c.cell_count, 16);
        c.divide();
        assert_eq!(c.cell_count, 32);
        assert!(c.is_blastula_ready());
        assert_eq!(c.cleavage_divisions, 5);
    }

    #[test]
    fn test_cleavage_divisions_from_count() {
        assert_eq!(CleavageStage::divisions_from_count(1), 0);
        assert_eq!(CleavageStage::divisions_from_count(2), 1);
        assert_eq!(CleavageStage::divisions_from_count(8), 3);
        assert_eq!(CleavageStage::divisions_from_count(32), 5);
    }

    #[test]
    fn test_cleavage_divisions_from_zero_returns_zero() {
        assert_eq!(CleavageStage::divisions_from_count(0), 0);
    }

    #[test]
    fn test_bicoid_default_gradient() {
        let m = MorphogenGradient::default();
        assert_eq!(m.morphogen, Morphogen::Bicoid);
        assert!((m.c0 - 1.0).abs() < 1e-5);
        assert!((m.lambda - 0.2).abs() < 1e-5);
    }

    #[test]
    fn test_bicoid_concentration_anterior_high_posterior_low() {
        let m = MorphogenGradient::default();
        let anterior = m.concentration_at(0.0);
        let posterior = m.concentration_at(1.0);
        assert!((anterior - 1.0).abs() < 1e-5); // exp(0) = 1
        assert!(posterior < anterior);
        assert!(posterior > 0.0);
        assert!(m.is_anterior_high());
    }

    #[test]
    fn test_bicoid_concentration_at_x_eq_1() {
        // C(1) = 1 * exp(-1/0.2) = exp(-5) ≈ 0.00674
        let m = MorphogenGradient::default();
        let c = m.concentration_at(1.0);
        assert!((c - (-5.0f32).exp()).abs() < 1e-5);
    }

    #[test]
    fn test_bicoid_concentration_monotonic_decreasing() {
        let m = MorphogenGradient::default();
        let c0 = m.concentration_at(0.0);
        let c25 = m.concentration_at(0.25);
        let c50 = m.concentration_at(0.5);
        let c75 = m.concentration_at(0.75);
        let c100 = m.concentration_at(1.0);
        assert!(c0 > c25);
        assert!(c25 > c50);
        assert!(c50 > c75);
        assert!(c75 > c100);
    }

    #[test]
    fn test_bicoid_concentration_clamps_x() {
        let m = MorphogenGradient::default();
        // x 超出 [0,1] 应被 clamp
        let c_neg = m.concentration_at(-0.5);
        let c0 = m.concentration_at(0.0);
        assert!((c_neg - c0).abs() < 1e-5);
        let c_big = m.concentration_at(2.0);
        let c1 = m.concentration_at(1.0);
        assert!((c_big - c1).abs() < 1e-5);
    }

    #[test]
    fn test_bicoid_threshold_position() {
        let m = MorphogenGradient::default();
        // C_0=1, lambda=0.2, threshold=exp(-5) ≈ 0.00674 → x = 1.0
        let threshold = (-5.0f32).exp();
        let x = m.threshold_position(threshold);
        assert!((x - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_bicoid_threshold_invalid_returns_zero() {
        let m = MorphogenGradient::default();
        assert_eq!(m.threshold_position(0.0), 0.0);
        assert_eq!(m.threshold_position(2.0), 0.0); // threshold > c0
    }

    #[test]
    fn test_hox_gene_colinear_index() {
        assert_eq!(HoxGene::Hox1.colinear_index(), 1);
        assert_eq!(HoxGene::Hox8.colinear_index(), 8);
        assert_eq!(HoxGene::Hox13.colinear_index(), 13);
    }

    #[test]
    fn test_hox_anterior_boundary_increases_with_index() {
        let b1 = HoxGene::Hox1.anterior_boundary();
        let b8 = HoxGene::Hox8.anterior_boundary();
        let b13 = HoxGene::Hox13.anterior_boundary();
        assert!(b1 < b8);
        assert!(b8 < b13);
    }

    #[test]
    fn test_hox_expression_default_inactive() {
        let s = HoxExpressionState::default();
        assert_eq!(s.max_activated, 0);
        assert!(!s.is_active(HoxGene::Hox1));
        assert!(!s.is_complete());
    }

    #[test]
    fn test_hox_expression_activate_up_to() {
        let mut s = HoxExpressionState::default();
        s.activate_up_to(HoxGene::Hox5);
        assert!(s.is_active(HoxGene::Hox1));
        assert!(s.is_active(HoxGene::Hox5));
        assert!(!s.is_active(HoxGene::Hox6));
        assert!(!s.is_complete());
    }

    #[test]
    fn test_hox_expression_complete_at_hox13() {
        let mut s = HoxExpressionState::default();
        s.activate_up_to(HoxGene::Hox13);
        assert!(s.is_active(HoxGene::Hox13));
        assert!(s.is_complete());
    }

    #[test]
    fn test_hox_expression_activate_lower_after_higher_is_noop() {
        let mut s = HoxExpressionState::default();
        s.activate_up_to(HoxGene::Hox8);
        let before = s.max_activated;
        s.activate_up_to(HoxGene::Hox3);
        assert_eq!(s.max_activated, before);
    }

    #[test]
    fn test_developmental_clock_default_starts_at_zygote() {
        let c = DevelopmentalClock::default();
        assert_eq!(c.stage, DevelopmentalStage::Zygote);
        assert!((c.dev_hours - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_developmental_clock_advance_stage() {
        let mut c = DevelopmentalClock::default();
        c.advance_stage();
        assert_eq!(c.stage, DevelopmentalStage::Cleavage);
        c.advance_stage();
        assert_eq!(c.stage, DevelopmentalStage::Morula);
        c.advance_stage();
        assert_eq!(c.stage, DevelopmentalStage::Blastula);
        c.advance_stage();
        assert_eq!(c.stage, DevelopmentalStage::Gastrula);
    }

    #[test]
    fn test_developmental_clock_step_advances_through_stages() {
        let mut c = DevelopmentalClock::default();
        // Zygote 0.5h, Cleavage 2.5h, Morula 1.0h = 4.0h 进 Blastula
        // 跑 5 小时应进入 Blastula（浮点累加余量）
        for _ in 0..500 {
            c.step(0.01);
        }
        assert!(c.dev_hours >= 4.5);
        assert!(c.stage.order_index() >= DevelopmentalStage::Blastula.order_index());
    }

    #[test]
    fn test_developmental_clock_fetus_is_terminal() {
        let mut c = DevelopmentalClock {
            stage: DevelopmentalStage::Fetus,
            dev_hours: 100.0,
            stage_hours: 0.0,
        };
        c.advance_stage();
        assert_eq!(c.stage, DevelopmentalStage::Fetus);
    }

    #[test]
    fn test_embryo_cleave_once_doubles_cells() {
        let mut e = Embryo::default();
        e.cleave_once();
        assert_eq!(e.cleavage.cell_count, 2);
        e.cleave_once();
        assert_eq!(e.cleavage.cell_count, 4);
    }

    #[test]
    fn test_embryo_gastrulate_reaches_gastrula() {
        let mut e = Embryo::default();
        e.gastrulate();
        assert_eq!(e.clock.stage, DevelopmentalStage::Gastrula);
    }

    #[test]
    fn test_embryo_activate_next_hox_in_order() {
        let mut e = Embryo::default();
        let g1 = e.activate_next_hox();
        assert_eq!(g1, Some(HoxGene::Hox1));
        let g2 = e.activate_next_hox();
        assert_eq!(g2, Some(HoxGene::Hox2));
        assert!(e.hox_state.is_active(HoxGene::Hox1));
        assert!(e.hox_state.is_active(HoxGene::Hox2));
        assert!(!e.hox_state.is_active(HoxGene::Hox3));
    }
}
