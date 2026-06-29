//! 高级生殖模块 — 动情周期、精子发生、配子发生与受精
//!
//! 生物学背景:
//!   生殖生物学研究雌性动情周期（月经周期）、精子发生、配子发生与受精过程。
//!   人类月经周期约 28 天，由 FSH（卵泡刺激素）和 LH（黄体生成素）调控，
//!   分卵泡期（14 天）、排卵、黄体期（14 天）。精子发生约 64-74 天，
//!   经历精原细胞 → 初级精母细胞 → 次级精母细胞 → 精细胞 → 精子。
//!   受精时父母双方各贡献 23 条染色体（单倍体），形成 46 条染色体的二倍体合子。
//!
//! 论文来源:
//! - Knobil, E., Neill, J. D. (2006). "Knobil and Neill's Physiology of Reproduction."
//!   3rd ed. Academic Press. (FSH/LH 调控, 月经周期)
//! - Corker, C. S., Davidson, D. W. (1978). "A computer model of the human menstrual
//!   cycle." J. Steroid Biochem. 9(8): 827-834. (周期建模)
//! - Clermont, Y. (1972). "Kinetics of spermatogenesis in mammals: seminiferous
//!   epithelium cycle and spermatogonial renewal." Physiol. Rev. 52(1): 198-236.
//!   (精子发生 64-74 天, 阶段)
//! - McGinnis, L. K., et al. (2009). "Oocyte vitrification." Fertil. Steril.
//!   (受精染色体组合)
//! - World Health Organization (2002). "WHO Manual for the Laboratory Analysis
//!   of Human Semen." 4th ed. (精液参数)
//!
//! 物理量单位:
//!   - 时间: day (天)
//!   - 激素浓度: mIU/mL (FSH, LH)
//!   - 染色体数: 条 (整数)

use serde::{Deserialize, Serialize};

/// 动情/月经周期阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EstrousPhase {
    /// 月经期 (Day 1-5)
    Menstrual,
    /// 卵泡期 (Day 6-14)
    Follicular,
    /// 排卵期 (Day 14-15)
    Ovulation,
    /// 黄体期 (Day 15-28)
    Luteal,
}

impl EstrousPhase {
    /// 由周期日数 (1..28) 判断阶段
    /// 来源: Knobil & Neill 2006
    pub fn from_cycle_day(day: u32) -> Self {
        let d = if day == 0 { 28 } else { day };
        let d = ((d - 1) % 28) + 1;
        if d <= 5 {
            Self::Menstrual
        } else if d <= 13 {
            Self::Follicular
        } else if d <= 15 {
            Self::Ovulation
        } else {
            Self::Luteal
        }
    }
}

/// 动情/月经周期状态
/// 来源: Knobil & Neill 2006
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EstrousCycle {
    /// 周期长度 (天, 默认 28)
    pub cycle_length_days: u32,
    /// 当前周期内日数 (1..cycle_length_days)
    pub current_day: u32,
    /// FSH 水平 (mIU/mL, 基线 ~ 5-20)
    pub fsh_miu_per_ml: f32,
    /// LH 水平 (mIU/mL, 基线 ~ 5-25, 峰值 ~ 50-100)
    pub lh_miu_per_ml: f32,
    /// 雌二醇水平 (pg/mL)
    pub estradiol_pg_per_ml: f32,
    /// 黄体酮水平 (ng/mL)
    pub progesterone_ng_per_ml: f32,
    /// 是否已排卵
    pub ovulated: bool,
}

impl Default for EstrousCycle {
    fn default() -> Self {
        Self {
            cycle_length_days: 28,
            current_day: 1,
            fsh_miu_per_ml: 10.0,
            lh_miu_per_ml: 10.0,
            estradiol_pg_per_ml: 50.0,
            progesterone_ng_per_ml: 1.0,
            ovulated: false,
        }
    }
}

impl EstrousCycle {
    /// 当前阶段
    pub fn current_phase(&self) -> EstrousPhase {
        EstrousPhase::from_cycle_day(self.current_day)
    }

    /// FSH 刺激卵泡生长 — 增加 FSH 提升雌二醇
    /// 来源: Knobil & Neill 2006
    pub fn fsh_stimulates_follicle(&mut self, fsh_increase: f32) {
        self.fsh_miu_per_ml += fsh_increase;
        // FSH 促进颗粒细胞分泌雌二醇
        self.estradiol_pg_per_ml += fsh_increase * 20.0;
    }

    /// LH 峰触发排卵 — 当 LH 超过阈值 (>= 40 mIU/mL) 时排卵
    /// 来源: Knobil & Neill 2006
    pub fn lh_peak_triggers_ovulation(&mut self) -> bool {
        if self.lh_miu_per_ml >= 40.0
            && !self.ovulated
            && self.current_phase() == EstrousPhase::Ovulation
        {
            self.ovulated = true;
            true
        } else {
            false
        }
    }

    /// 显式 Euler 单步推进一天
    pub fn step_one_day(&mut self) {
        self.current_day += 1;
        if self.current_day > self.cycle_length_days {
            self.current_day = 1;
            self.ovulated = false;
        }
        // 根据阶段调整激素
        match self.current_phase() {
            EstrousPhase::Menstrual => {
                self.fsh_miu_per_ml = 12.0;
                self.lh_miu_per_ml = 8.0;
                self.estradiol_pg_per_ml = 30.0;
                self.progesterone_ng_per_ml = 0.5;
            }
            EstrousPhase::Follicular => {
                self.fsh_miu_per_ml = 10.0;
                self.lh_miu_per_ml = 12.0;
                self.estradiol_pg_per_ml = 100.0;
                self.progesterone_ng_per_ml = 1.0;
            }
            EstrousPhase::Ovulation => {
                self.fsh_miu_per_ml = 15.0;
                self.lh_miu_per_ml = 60.0; // LH 峰
                self.estradiol_pg_per_ml = 200.0;
                self.progesterone_ng_per_ml = 2.0;
            }
            EstrousPhase::Luteal => {
                self.fsh_miu_per_ml = 6.0;
                self.lh_miu_per_ml = 8.0;
                self.estradiol_pg_per_ml = 120.0;
                self.progesterone_ng_per_ml = 15.0; // 黄体酮主导
            }
        }
        // 自动排卵判定
        let _ = self.lh_peak_triggers_ovulation();
    }
}

/// 精子发生阶段
/// 来源: Clermont 1972
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpermatogenesisStage {
    /// 精原细胞 (Mitosis)
    Spermatogonia,
    /// 初级精母细胞 (Meiosis I)
    PrimarySpermatocyte,
    /// 次级精母细胞 (Meiosis II 完成)
    SecondarySpermatocyte,
    /// 精细胞 (分化)
    Spermatid,
    /// 成熟精子
    Spermatozoon,
}

impl SpermatogenesisStage {
    /// 各阶段顺序索引 (0..5)
    pub fn order_index(&self) -> u8 {
        match self {
            Self::Spermatogonia => 0,
            Self::PrimarySpermatocyte => 1,
            Self::SecondarySpermatocyte => 2,
            Self::Spermatid => 3,
            Self::Spermatozoon => 4,
        }
    }

    /// 各阶段持续天数 (人, Clermont 1972)
    /// 总计约 64-74 天
    pub fn duration_days(&self) -> f32 {
        match self {
            Self::Spermatogonia => 16.0,
            Self::PrimarySpermatocyte => 23.0,
            Self::SecondarySpermatocyte => 1.0,
            Self::Spermatid => 22.0,
            Self::Spermatozoon => 2.0,
        }
    }
}

/// 精子发生状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Spermatogenesis {
    /// 当前阶段
    pub stage: SpermatogenesisStage,
    /// 当前阶段累计天数
    pub stage_days: f32,
    /// 总累计天数
    pub total_days: f32,
    /// 已生成精子数 (相对于 1 个精原细胞 → 4 个精子)
    pub sperm_count: u32,
}

impl Default for Spermatogenesis {
    fn default() -> Self {
        Self {
            stage: SpermatogenesisStage::Spermatogonia,
            stage_days: 0.0,
            total_days: 0.0,
            sperm_count: 0,
        }
    }
}

impl Spermatogenesis {
    /// 显式 Euler 单步推进一天
    /// 来源: Clermont 1972
    pub fn step_one_day(&mut self) {
        self.stage_days += 1.0;
        self.total_days += 1.0;
        let dur = self.stage.duration_days();
        if self.stage_days >= dur {
            self.stage_days -= dur;
            self.advance_stage();
        }
    }

    /// 推进到下一阶段
    pub fn advance_stage(&mut self) {
        use SpermatogenesisStage::*;
        let prev = self.stage;
        self.stage = match self.stage {
            Spermatogonia => PrimarySpermatocyte,
            PrimarySpermatocyte => SecondarySpermatocyte,
            SecondarySpermatocyte => Spermatid,
            Spermatid => {
                // 一个精原细胞经减数分裂产生 4 个精子
                self.sperm_count += 4;
                Spermatozoon
            }
            Spermatozoon => Spermatozoon, // 终态
        };
        // 进入新阶段时记录变化 (避免 unused 警告)
        let _ = prev;
    }

    /// 是否完成 (产生成熟精子)
    pub fn is_complete(&self) -> bool {
        self.stage == SpermatogenesisStage::Spermatozoon && self.sperm_count > 0
    }
}

/// 配子发生类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GametogenesisType {
    /// 卵子发生
    Oogenesis,
    /// 精子发生
    Spermatogenesis,
}

/// 配子发生状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Gametogenesis {
    pub kind: GametogenesisType,
    /// 染色体倍性 (1 = 单倍体, 2 = 二倍体)
    pub ploidy: u8,
    /// 染色体数
    pub chromosome_count: u8,
}

impl Default for Gametogenesis {
    fn default() -> Self {
        Self {
            kind: GametogenesisType::Spermatogenesis,
            ploidy: 1,
            chromosome_count: 23,
        }
    }
}

impl Gametogenesis {
    /// 人类配子: 单倍体 23 条染色体
    pub fn human_haploid(kind: GametogenesisType) -> Self {
        Self {
            kind,
            ploidy: 1,
            chromosome_count: 23,
        }
    }

    /// 人类体细胞: 二倍体 46 条染色体
    pub fn human_diploid() -> Self {
        Self {
            kind: GametogenesisType::Oogenesis, // 占位, 体细胞不严格分类
            ploidy: 2,
            chromosome_count: 46,
        }
    }
}

/// 受精 — 配子融合形成合子
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Fertilization {
    /// 父方染色体数
    pub paternal_chromosomes: u8,
    /// 母方染色体数
    pub maternal_chromosomes: u8,
    /// 合子染色体数
    pub zygote_chromosomes: u8,
    /// 合子倍性
    pub zygote_ploidy: u8,
}

impl Fertilization {
    /// 由两个配子构造受精
    /// 合子染色体数 = 父 + 母
    pub fn from_gametes(sperm: &Gametogenesis, egg: &Gametogenesis) -> Self {
        let zygote_chromosomes = sperm.chromosome_count + egg.chromosome_count;
        let zygote_ploidy = sperm.ploidy + egg.ploidy;
        Self {
            paternal_chromosomes: sperm.chromosome_count,
            maternal_chromosomes: egg.chromosome_count,
            zygote_chromosomes,
            zygote_ploidy,
        }
    }

    /// 是否产生正常人类二倍体合子 (2n = 46)
    pub fn is_normal_human_diploid(&self) -> bool {
        self.zygote_ploidy == 2 && self.zygote_chromosomes == 46
    }
}

/// 生殖激素
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReproductiveHormones {
    /// FSH (mIU/mL)
    pub fsh_miu_per_ml: f32,
    /// LH (mIU/mL)
    pub lh_miu_per_ml: f32,
    /// 雌二醇 (pg/mL)
    pub estradiol_pg_per_ml: f32,
    /// 黄体酮 (ng/mL)
    pub progesterone_ng_per_ml: f32,
    /// 睾酮 (ng/dL)
    pub testosterone_ng_per_dl: f32,
}

impl Default for ReproductiveHormones {
    fn default() -> Self {
        Self {
            fsh_miu_per_ml: 10.0,
            lh_miu_per_ml: 10.0,
            estradiol_pg_per_ml: 50.0,
            progesterone_ng_per_ml: 1.0,
            testosterone_ng_per_dl: 500.0,
        }
    }
}

impl ReproductiveHormones {
    /// FSH/LH 比值 (诊断 PCOS 等用, 正常 ~ 1.0)
    pub fn fsh_lh_ratio(&self) -> f32 {
        if self.lh_miu_per_ml > 0.0 {
            self.fsh_miu_per_ml / self.lh_miu_per_ml
        } else {
            0.0
        }
    }

    /// 是否处于绝经期 (FSH > 25 mIU/mL)
    pub fn is_menopausal(&self) -> bool {
        self.fsh_miu_per_ml > 25.0
    }

    /// 是否处于 LH 峰 (LH > 40 mIU/mL)
    pub fn is_lh_peak(&self) -> bool {
        self.lh_miu_per_ml > 40.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estrous_cycle_default_28_days() {
        let ec = EstrousCycle::default();
        assert_eq!(ec.cycle_length_days, 28);
        assert_eq!(ec.current_day, 1);
        assert!(!ec.ovulated);
    }

    #[test]
    fn test_estrous_phase_from_cycle_day_menstrual() {
        assert_eq!(EstrousPhase::from_cycle_day(1), EstrousPhase::Menstrual);
        assert_eq!(EstrousPhase::from_cycle_day(5), EstrousPhase::Menstrual);
    }

    #[test]
    fn test_estrous_phase_from_cycle_day_follicular() {
        assert_eq!(EstrousPhase::from_cycle_day(6), EstrousPhase::Follicular);
        assert_eq!(EstrousPhase::from_cycle_day(13), EstrousPhase::Follicular);
    }

    #[test]
    fn test_estrous_phase_from_cycle_day_ovulation() {
        assert_eq!(EstrousPhase::from_cycle_day(14), EstrousPhase::Ovulation);
        assert_eq!(EstrousPhase::from_cycle_day(15), EstrousPhase::Ovulation);
    }

    #[test]
    fn test_estrous_phase_from_cycle_day_luteal() {
        assert_eq!(EstrousPhase::from_cycle_day(16), EstrousPhase::Luteal);
        assert_eq!(EstrousPhase::from_cycle_day(28), EstrousPhase::Luteal);
    }

    #[test]
    fn test_estrous_phase_wraps_around() {
        // day 0 应映射为 28 (黄体期)
        assert_eq!(EstrousPhase::from_cycle_day(0), EstrousPhase::Luteal);
        // day 29 = day 1
        assert_eq!(EstrousPhase::from_cycle_day(29), EstrousPhase::Menstrual);
        assert_eq!(EstrousPhase::from_cycle_day(56), EstrousPhase::Luteal);
    }

    #[test]
    fn test_fsh_stimulates_follicle_increases_estradiol() {
        let mut ec = EstrousCycle::default();
        let initial_e2 = ec.estradiol_pg_per_ml;
        ec.fsh_stimulates_follicle(5.0);
        assert!(ec.estradiol_pg_per_ml > initial_e2);
        // FSH 增加 5 → 雌二醇增加 5*20 = 100
        assert!((ec.estradiol_pg_per_ml - initial_e2 - 100.0).abs() < 1e-3);
    }

    #[test]
    fn test_lh_peak_triggers_ovulation_at_correct_phase() {
        let mut ec = EstrousCycle {
            current_day: 14, // 排卵期
            lh_miu_per_ml: 60.0,
            ..EstrousCycle::default()
        };
        let ovulated = ec.lh_peak_triggers_ovulation();
        assert!(ovulated);
        assert!(ec.ovulated);
    }

    #[test]
    fn test_lh_peak_does_not_trigger_in_wrong_phase() {
        let mut ec = EstrousCycle {
            current_day: 5, // 月经期, 不是排卵期
            lh_miu_per_ml: 60.0,
            ..EstrousCycle::default()
        };
        let ovulated = ec.lh_peak_triggers_ovulation();
        assert!(!ovulated);
        assert!(!ec.ovulated);
    }

    #[test]
    fn test_lh_below_threshold_does_not_trigger() {
        let mut ec = EstrousCycle {
            current_day: 14, // 排卵期
            lh_miu_per_ml: 30.0, // 低于阈值
            ..EstrousCycle::default()
        };
        let ovulated = ec.lh_peak_triggers_ovulation();
        assert!(!ovulated);
        assert!(!ec.ovulated);
    }

    #[test]
    fn test_lh_peak_does_not_trigger_twice() {
        let mut ec = EstrousCycle {
            current_day: 14,
            lh_miu_per_ml: 60.0,
            ..EstrousCycle::default()
        };
        let first = ec.lh_peak_triggers_ovulation();
        let second = ec.lh_peak_triggers_ovulation();
        assert!(first);
        assert!(!second);
    }

    #[test]
    fn test_estrous_cycle_step_one_day_advances_and_wraps() {
        let mut ec = EstrousCycle {
            cycle_length_days: 28,
            current_day: 28,
            ..EstrousCycle::default()
        };
        ec.step_one_day();
        assert_eq!(ec.current_day, 1);
    }

    #[test]
    fn test_estrous_cycle_step_resets_ovulated_at_new_cycle() {
        let mut ec = EstrousCycle {
            cycle_length_days: 28,
            current_day: 28,
            ovulated: true,
            ..EstrousCycle::default()
        };
        ec.step_one_day();
        assert_eq!(ec.current_day, 1);
        assert!(!ec.ovulated);
    }

    #[test]
    fn test_estrous_cycle_step_sets_lh_peak_on_ovulation_day() {
        let mut ec = EstrousCycle {
            cycle_length_days: 28,
            current_day: 13, // 下一天是 14
            ..EstrousCycle::default()
        };
        ec.step_one_day();
        assert_eq!(ec.current_day, 14);
        assert_eq!(ec.current_phase(), EstrousPhase::Ovulation);
        // LH 应被设为峰值 (60)
        assert!((ec.lh_miu_per_ml - 60.0).abs() < 1e-3);
    }

    #[test]
    fn test_spermatogenesis_default_starts_at_spermatogonia() {
        let s = Spermatogenesis::default();
        assert_eq!(s.stage, SpermatogenesisStage::Spermatogonia);
        assert!(!s.is_complete());
    }

    #[test]
    fn test_spermatogenesis_stage_order() {
        assert!(SpermatogenesisStage::Spermatogonia.order_index() < SpermatogenesisStage::PrimarySpermatocyte.order_index());
        assert!(SpermatogenesisStage::Spermatid.order_index() < SpermatogenesisStage::Spermatozoon.order_index());
    }

    #[test]
    fn test_spermatogenesis_total_duration_64_to_74_days() {
        let total = SpermatogenesisStage::Spermatogonia.duration_days()
            + SpermatogenesisStage::PrimarySpermatocyte.duration_days()
            + SpermatogenesisStage::SecondarySpermatocyte.duration_days()
            + SpermatogenesisStage::Spermatid.duration_days()
            + SpermatogenesisStage::Spermatozoon.duration_days();
        // 16 + 23 + 1 + 22 + 2 = 64 天
        assert!(total >= 64.0 && total <= 74.0);
    }

    #[test]
    fn test_spermatogenesis_advance_stage_progresses_through_stages() {
        let mut s = Spermatogenesis::default();
        s.advance_stage();
        assert_eq!(s.stage, SpermatogenesisStage::PrimarySpermatocyte);
        s.advance_stage();
        assert_eq!(s.stage, SpermatogenesisStage::SecondarySpermatocyte);
        s.advance_stage();
        assert_eq!(s.stage, SpermatogenesisStage::Spermatid);
        s.advance_stage();
        assert_eq!(s.stage, SpermatogenesisStage::Spermatozoon);
        assert_eq!(s.sperm_count, 4);
        assert!(s.is_complete());
    }

    #[test]
    fn test_spermatogenesis_spermatozoon_is_terminal() {
        let mut s = Spermatogenesis {
            stage: SpermatogenesisStage::Spermatozoon,
            stage_days: 0.0,
            total_days: 64.0,
            sperm_count: 4,
        };
        s.advance_stage();
        assert_eq!(s.stage, SpermatogenesisStage::Spermatozoon);
    }

    #[test]
    fn test_spermatogenesis_step_one_day_progresses_to_completion() {
        let mut s = Spermatogenesis::default();
        // 跑 70 天应能完成
        for _ in 0..70 {
            s.step_one_day();
        }
        assert!(s.total_days >= 60.0);
        assert!(s.is_complete());
    }

    #[test]
    fn test_gametogenesis_human_haploid() {
        let sperm = Gametogenesis::human_haploid(GametogenesisType::Spermatogenesis);
        assert_eq!(sperm.ploidy, 1);
        assert_eq!(sperm.chromosome_count, 23);
        let egg = Gametogenesis::human_haploid(GametogenesisType::Oogenesis);
        assert_eq!(egg.ploidy, 1);
        assert_eq!(egg.chromosome_count, 23);
    }

    #[test]
    fn test_gametogenesis_human_diploid() {
        let somatic = Gametogenesis::human_diploid();
        assert_eq!(somatic.ploidy, 2);
        assert_eq!(somatic.chromosome_count, 46);
    }

    #[test]
    fn test_fertilization_combines_chromosomes() {
        let sperm = Gametogenesis::human_haploid(GametogenesisType::Spermatogenesis);
        let egg = Gametogenesis::human_haploid(GametogenesisType::Oogenesis);
        let zygote = Fertilization::from_gametes(&sperm, &egg);
        assert_eq!(zygote.paternal_chromosomes, 23);
        assert_eq!(zygote.maternal_chromosomes, 23);
        assert_eq!(zygote.zygote_chromosomes, 46);
        assert_eq!(zygote.zygote_ploidy, 2);
        assert!(zygote.is_normal_human_diploid());
    }

    #[test]
    fn test_fertilization_abnormal_ploidy_detected() {
        // 异常: 两个二倍体配子 (理论上不能存活)
        let abnormal_sperm = Gametogenesis {
            kind: GametogenesisType::Spermatogenesis,
            ploidy: 2,
            chromosome_count: 46,
        };
        let abnormal_egg = Gametogenesis {
            kind: GametogenesisType::Oogenesis,
            ploidy: 2,
            chromosome_count: 46,
        };
        let zygote = Fertilization::from_gametes(&abnormal_sperm, &abnormal_egg);
        // 4 倍体 92 条, 非正常人类二倍体
        assert_eq!(zygote.zygote_ploidy, 4);
        assert_eq!(zygote.zygote_chromosomes, 92);
        assert!(!zygote.is_normal_human_diploid());
    }

    #[test]
    fn test_reproductive_hormones_default() {
        let h = ReproductiveHormones::default();
        assert!((h.fsh_miu_per_ml - 10.0).abs() < 1e-5);
        assert!((h.lh_miu_per_ml - 10.0).abs() < 1e-5);
        assert!(!h.is_menopausal());
        assert!(!h.is_lh_peak());
    }

    #[test]
    fn test_reproductive_hormones_fsh_lh_ratio_normal() {
        let h = ReproductiveHormones::default();
        assert!((h.fsh_lh_ratio() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_reproductive_hormones_menopausal_high_fsh() {
        let h = ReproductiveHormones {
            fsh_miu_per_ml: 40.0,
            ..ReproductiveHormones::default()
        };
        assert!(h.is_menopausal());
    }

    #[test]
    fn test_reproductive_hormones_lh_peak_high_lh() {
        let h = ReproductiveHormones {
            lh_miu_per_ml: 60.0,
            ..ReproductiveHormones::default()
        };
        assert!(h.is_lh_peak());
    }

    #[test]
    fn test_reproductive_hormones_fsh_lh_ratio_zero_lh() {
        let h = ReproductiveHormones {
            lh_miu_per_ml: 0.0,
            ..ReproductiveHormones::default()
        };
        assert!((h.fsh_lh_ratio() - 0.0).abs() < 1e-5);
    }
}
