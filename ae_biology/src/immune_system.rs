//! 免疫系统模块 — 先天免疫 + 适应性免疫
//!
//! 生物学背景:
//!   哺乳动物免疫系统分为先天免疫 (innate) 与适应性免疫 (adaptive) 两层。
//!   先天免疫通过模式识别受体 (PRR) 识别病原体相关分子模式 (PAMP),
//!   在数分钟至数小时内激活。适应性免疫通过 T/B 淋巴细胞的抗原特异性受体
//!   (TCR/BCR) 识别抗原,需要数天完成克隆扩增 (clonal expansion),并形成
//!   长期免疫记忆 (memory cell)。
//!
//! 论文来源:
//!   - Janeway C.A., Travers P., Walport M., Shlomchik M. (2001).
//!     "Immunobiology: The Immune System in Health and Disease" 5th ed. Garland.
//!   - Perelson A.S. (2002). "Modelling viral and immune system dynamics."
//!     Phil. Trans. R. Soc. Lond. B 357:1065-1071. (动力学方程)
//!   - Burnet F.M. (1959). "The Clonal Selection Theory of Acquired Immunity."
//!     Vanderbilt Univ. Press. (克隆选择学说, 1960 Nobel)
//!   - Mosmann T.R., Coffman R.L. (1989). "TH1 and TH2 cells: different patterns
//!     of lymphokine secretion lead to different functional properties."
//!     Annu. Rev. Immunol. 7:145-173.

use serde::{Deserialize, Serialize};

/// 细胞因子类型 (基于 Mosmann 1989 TH1/TH2 分类)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CytokineType {
    /// 促炎, TH1 型 (TNF-α, IL-1β, IL-6, IL-12)
    ProInflammatory,
    /// 抗炎, TH2 型 (IL-4, IL-5, IL-10, IL-13)
    AntiInflammatory,
    /// 趋化因子 (IL-8/CXCL8)
    Chemokine,
    /// 干扰素 (IFN-α/β/γ)
    Interferon,
    /// 集落刺激因子 (CSF)
    GrowthFactor,
}

/// 细胞因子 (信号分子)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cytokine {
    /// 类型
    pub kind: CytokineType,
    /// 血清浓度 (pg/mL)
    pub concentration_pg_ml: f32,
    /// 半衰期 (s)
    pub half_life_s: f32,
}

impl Cytokine {
    pub fn new(kind: CytokineType) -> Self {
        // 浓度与半衰期参考 Janeway 2001 Table 2-3
        let (concentration_pg_ml, half_life_s) = match kind {
            CytokineType::ProInflammatory => (10.0, 3600.0),
            CytokineType::AntiInflammatory => (5.0, 7200.0),
            CytokineType::Chemokine => (50.0, 1800.0),
            CytokineType::Interferon => (20.0, 5400.0),
            CytokineType::GrowthFactor => (30.0, 10800.0),
        };
        Self { kind, concentration_pg_ml, half_life_s }
    }
}

impl Default for Cytokine {
    fn default() -> Self { Self::new(CytokineType::ProInflammatory) }
}

/// 抗原 (病原体表面分子)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Antigen {
    /// 抗原 ID (模拟表位哈希)
    pub epitope_id: u32,
    /// 致病性 (0..1, 1=致死)
    pub pathogenicity: f32,
    /// 复制率 (1/s, 病原体复制)
    pub replication_rate: f32,
}

impl Antigen {
    pub fn new(epitope_id: u32) -> Self {
        Self { epitope_id, pathogenicity: 0.5, replication_rate: 0.1 }
    }
}

impl Default for Antigen {
    fn default() -> Self { Self::new(0) }
}

/// T 细胞亚型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TCellSubset {
    /// 辅助 T 细胞 (CD4+)
    Helper,
    /// 细胞毒 T 细胞 (CD8+)
    Cytotoxic,
    /// 调节 T 细胞 (Treg)
    Regulatory,
    /// 记忆 T 细胞
    Memory,
}

/// T 细胞
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TCell {
    pub subset: TCellSubset,
    /// 数量 (cells/μL)
    pub count: f32,
    /// 亲和力 (0..1)
    pub affinity: f32,
    /// 是否已激活
    pub activated: bool,
}

impl TCell {
    pub fn new(subset: TCellSubset) -> Self {
        // 典型外周血计数参考值 (Janeway 2001 Appendix)
        let count = match subset {
            TCellSubset::Helper => 1000.0,
            TCellSubset::Cytotoxic => 500.0,
            TCellSubset::Regulatory => 50.0,
            TCellSubset::Memory => 10.0,
        };
        Self { subset, count, affinity: 0.5, activated: false }
    }
}

impl Default for TCell {
    fn default() -> Self { Self::new(TCellSubset::Helper) }
}

/// B 细胞
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BCell {
    /// 数量 (cells/μL)
    pub count: f32,
    /// 抗体亲和力 (0..1)
    pub affinity: f32,
    /// 是否分化为浆细胞
    pub is_plasma: bool,
    /// 是否为记忆 B 细胞
    pub is_memory: bool,
}

impl Default for BCell {
    fn default() -> Self {
        Self { count: 300.0, affinity: 0.5, is_plasma: false, is_memory: false }
    }
}

/// 免疫响应状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImmuneResponse {
    /// 静息
    Resting,
    /// 识别期 (数小时)
    Recognition,
    /// 激活期 (克隆扩增)
    Activation,
    /// 效应期 (清除病原)
    Effector,
    /// 记忆期 (长期)
    Memory,
}

/// 先天免疫系统
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InnateImmuneSystem {
    /// 巨噬细胞数量 (cells/μL)
    pub macrophages: f32,
    /// 中性粒细胞数量 (cells/μL)
    pub neutrophils: f32,
    /// NK 细胞数量 (cells/μL)
    pub nk_cells: f32,
    /// 补体活性 (0..1)
    pub complement_activity: f32,
    /// 炎症指数 (0..1)
    pub inflammation: f32,
}

impl Default for InnateImmuneSystem {
    fn default() -> Self {
        Self {
            macrophages: 200.0,
            neutrophils: 4000.0,
            nk_cells: 100.0,
            complement_activity: 0.8,
            inflammation: 0.0,
        }
    }
}

impl InnateImmuneSystem {
    /// 识别抗原 (PRR-PAMP 结合,数分钟内启动)
    /// 返回识别强度 (0..1)
    pub fn recognize_antigen(&mut self, antigen: &Antigen) -> f32 {
        // 识别强度与致病性正相关 (Janeway 2001 Ch.2)
        let recognition = (antigen.pathogenicity * 0.7 + 0.3).min(1.0);
        // 激活炎症级联
        self.inflammation = (self.inflammation + antigen.pathogenicity * 0.3).min(1.0);
        recognition
    }

    /// 释放促炎细胞因子
    /// Perelson 2002 动力学: d[C]/dt = k_release * N_macro * inflammation - k_decay * [C]
    /// 显式 Euler 积分
    pub fn cytokine_release(&mut self, dt: f32) -> Cytokine {
        let k_release = 0.5;     // pg/(cell·s)
        let k_decay = 0.0001;    // 1/s
        let production = k_release * self.macrophages * self.inflammation;
        let mut cyto = Cytokine::new(CytokineType::ProInflammatory);
        cyto.concentration_pg_ml +=
            production * dt - k_decay * cyto.concentration_pg_ml * dt;
        cyto.concentration_pg_ml = cyto.concentration_pg_ml.max(0.0);
        cyto
    }

    /// 中性粒细胞吞噬 (病原体负载降低)
    pub fn phagocytosis(&mut self, antigen: &Antigen, dt: f32) -> f32 {
        // 吞噬率 ~ 中性粒细胞数 * 识别概率
        let rate = self.neutrophils * 0.0001 * (1.0 - antigen.pathogenicity * 0.3);
        rate * dt
    }
}

/// 适应性免疫系统
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AdaptiveImmuneSystem {
    pub helper_t: TCell,
    pub cytotoxic_t: TCell,
    pub regulatory_t: TCell,
    pub memory_t: TCell,
    pub b_cells: BCell,
    /// 当前响应阶段
    pub phase: ImmuneResponse,
    /// 已识别抗原 ID
    pub recognized_antigen_id: u32,
    /// 累计克隆扩增代数
    pub expansion_generations: u32,
}

impl Default for AdaptiveImmuneSystem {
    fn default() -> Self {
        Self {
            helper_t: TCell::new(TCellSubset::Helper),
            cytotoxic_t: TCell::new(TCellSubset::Cytotoxic),
            regulatory_t: TCell::new(TCellSubset::Regulatory),
            memory_t: TCell::new(TCellSubset::Memory),
            b_cells: BCell::default(),
            phase: ImmuneResponse::Resting,
            recognized_antigen_id: 0,
            expansion_generations: 0,
        }
    }
}

impl AdaptiveImmuneSystem {
    /// 识别抗原 (TCR/BCR-MHC-肽段结合,数小时)
    /// 返回是否成功识别
    pub fn recognize_antigen(&mut self, antigen: &Antigen) -> bool {
        let threshold = 0.3;
        let recognized = self.helper_t.affinity >= threshold || antigen.pathogenicity > 0.5;
        if recognized {
            self.phase = ImmuneResponse::Recognition;
            self.recognized_antigen_id = antigen.epitope_id;
            self.helper_t.activated = true;
        }
        recognized
    }

    /// 克隆扩增 (Burnet 1959 克隆选择)
    /// Logistic 增长: dN/dt = r * N * (1 - N/K), K = 10000 cells/μL
    /// 显式 Euler 积分
    pub fn clonal_expansion(&mut self, dt: f32) {
        if self.phase != ImmuneResponse::Recognition && self.phase != ImmuneResponse::Activation {
            return;
        }
        self.phase = ImmuneResponse::Activation;
        let r = 0.5;            // 扩增率 1/s
        let k = 10000.0;        // 承载量 cells/μL
        // Helper T 扩增
        let n_h = self.helper_t.count;
        self.helper_t.count += r * n_h * (1.0 - n_h / k) * dt;
        // Cytotoxic T 扩增
        let n_c = self.cytotoxic_t.count;
        self.cytotoxic_t.count += r * n_c * (1.0 - n_c / k) * dt;
        // B 细胞扩增
        let n_b = self.b_cells.count;
        self.b_cells.count += r * n_b * (1.0 - n_b / k) * dt;
        self.expansion_generations += 1;
        // 进入效应期阈值
        if self.helper_t.count > 2000.0 {
            self.phase = ImmuneResponse::Effector;
            self.cytotoxic_t.activated = true;
            self.b_cells.is_plasma = true;
        }
    }

    /// 形成记忆细胞 (抗原清除后,5-10% 效应细胞转为记忆)
    pub fn memory_formation(&mut self) {
        if self.phase != ImmuneResponse::Effector {
            return;
        }
        let memory_fraction = 0.07;
        self.memory_t.count += self.cytotoxic_t.count * memory_fraction;
        self.memory_t.affinity = self.cytotoxic_t.affinity;
        self.b_cells.is_memory = true;
        self.b_cells.is_plasma = false;
        self.phase = ImmuneResponse::Memory;
    }

    /// 二次免疫应答 (记忆细胞快速识别同一抗原)
    pub fn recall_antigen(&mut self, antigen: &Antigen) -> bool {
        if self.phase == ImmuneResponse::Memory
            && antigen.epitope_id == self.recognized_antigen_id
        {
            // 记忆细胞数量快速膨胀 (10-100 倍),跳过识别期
            self.helper_t.count *= 5.0;
            self.cytotoxic_t.count *= 5.0;
            self.helper_t.activated = true;
            self.cytotoxic_t.activated = true;
            self.phase = ImmuneResponse::Effector;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_innate_default() {
        let s = InnateImmuneSystem::default();
        assert!(s.macrophages > 0.0);
        assert!(s.neutrophils > 0.0);
        assert!(s.nk_cells > 0.0);
        assert_eq!(s.inflammation, 0.0);
        assert!(s.complement_activity > 0.0 && s.complement_activity <= 1.0);
    }

    #[test]
    fn test_adaptive_default_phase_resting() {
        let a = AdaptiveImmuneSystem::default();
        assert_eq!(a.phase, ImmuneResponse::Resting);
        assert!(!a.helper_t.activated);
        assert!(!a.cytotoxic_t.activated);
        assert_eq!(a.expansion_generations, 0);
    }

    #[test]
    fn test_cytokine_default_concentrations() {
        let p = Cytokine::new(CytokineType::ProInflammatory);
        assert!(p.concentration_pg_ml > 0.0);
        assert!(p.half_life_s > 0.0);
    }

    #[test]
    fn test_cytokine_types_distinct() {
        let p = Cytokine::new(CytokineType::ProInflammatory);
        let a = Cytokine::new(CytokineType::AntiInflammatory);
        assert_ne!(p.kind, a.kind);
        assert_ne!(p.concentration_pg_ml, a.concentration_pg_ml);
    }

    #[test]
    fn test_antigen_default() {
        let a = Antigen::default();
        assert!(a.pathogenicity > 0.0 && a.pathogenicity <= 1.0);
        assert!(a.replication_rate >= 0.0);
    }

    #[test]
    fn test_tcell_subsets_count() {
        let h = TCell::new(TCellSubset::Helper);
        let c = TCell::new(TCellSubset::Cytotoxic);
        let r = TCell::new(TCellSubset::Regulatory);
        let m = TCell::new(TCellSubset::Memory);
        assert!(h.count > c.count);
        assert!(c.count > r.count);
        assert!(r.count > m.count);
    }

    #[test]
    fn test_bcell_default() {
        let b = BCell::default();
        assert!(b.count > 0.0);
        assert!(!b.is_plasma);
        assert!(!b.is_memory);
        assert!(b.affinity > 0.0 && b.affinity <= 1.0);
    }

    #[test]
    fn test_innate_recognize_high_pathogen() {
        let mut s = InnateImmuneSystem::default();
        let a = Antigen { epitope_id: 1, pathogenicity: 0.8, replication_rate: 0.2 };
        let r = s.recognize_antigen(&a);
        assert!(r > 0.5);
        assert!(s.inflammation > 0.0);
    }

    #[test]
    fn test_innate_recognize_low_pathogen() {
        let mut s = InnateImmuneSystem::default();
        let a = Antigen { epitope_id: 1, pathogenicity: 0.1, replication_rate: 0.0 };
        let r = s.recognize_antigen(&a);
        assert!(r >= 0.3 && r < 0.6);
    }

    #[test]
    fn test_innate_inflammation_caps_at_one() {
        let mut s = InnateImmuneSystem::default();
        let a = Antigen { epitope_id: 1, pathogenicity: 1.0, replication_rate: 0.1 };
        for _ in 0..10 {
            s.recognize_antigen(&a);
        }
        assert!(s.inflammation <= 1.0);
    }

    #[test]
    fn test_cytokine_release_with_inflammation() {
        let mut s = InnateImmuneSystem::default();
        s.inflammation = 0.5;
        let c = s.cytokine_release(1.0);
        assert!(c.concentration_pg_ml > 0.0);
        assert_eq!(c.kind, CytokineType::ProInflammatory);
    }

    #[test]
    fn test_cytokine_release_zero_inflammation() {
        let mut s = InnateImmuneSystem::default();
        s.inflammation = 0.0;
        let c = s.cytokine_release(1.0);
        // 基础浓度来自 Cytokine::new,production 为 0,decay 微负
        assert!(c.concentration_pg_ml >= 0.0);
    }

    #[test]
    fn test_phagocytosis_returns_positive() {
        let mut s = InnateImmuneSystem::default();
        let a = Antigen { epitope_id: 1, pathogenicity: 0.5, replication_rate: 0.1 };
        let cleared = s.phagocytosis(&a, 1.0);
        assert!(cleared > 0.0);
    }

    #[test]
    fn test_adaptive_recognize_antigen_high_pathogen() {
        let mut a = AdaptiveImmuneSystem::default();
        let ag = Antigen { epitope_id: 42, pathogenicity: 0.7, replication_rate: 0.1 };
        assert!(a.recognize_antigen(&ag));
        assert_eq!(a.phase, ImmuneResponse::Recognition);
        assert_eq!(a.recognized_antigen_id, 42);
        assert!(a.helper_t.activated);
    }

    #[test]
    fn test_adaptive_no_recognize_low_affinity_and_pathogen() {
        let mut a = AdaptiveImmuneSystem::default();
        a.helper_t.affinity = 0.1;
        let ag = Antigen { epitope_id: 99, pathogenicity: 0.1, replication_rate: 0.0 };
        assert!(!a.recognize_antigen(&ag));
        assert_eq!(a.phase, ImmuneResponse::Resting);
    }

    #[test]
    fn test_clonal_expansion_increases_count() {
        let mut a = AdaptiveImmuneSystem::default();
        a.phase = ImmuneResponse::Recognition;
        let before = a.helper_t.count;
        a.clonal_expansion(1.0);
        assert!(a.helper_t.count > before);
        assert!(a.expansion_generations > 0);
    }

    #[test]
    fn test_clonal_expansion_resting_no_change() {
        let mut a = AdaptiveImmuneSystem::default();
        let before = a.helper_t.count;
        a.clonal_expansion(1.0);
        assert_eq!(a.helper_t.count, before);
        assert_eq!(a.expansion_generations, 0);
    }

    #[test]
    fn test_clonal_expansion_transitions_to_effector() {
        let mut a = AdaptiveImmuneSystem::default();
        a.phase = ImmuneResponse::Activation;
        a.helper_t.count = 5000.0;
        a.clonal_expansion(0.001);
        assert_eq!(a.phase, ImmuneResponse::Effector);
        assert!(a.cytotoxic_t.activated);
        assert!(a.b_cells.is_plasma);
    }

    #[test]
    fn test_memory_formation_from_effector() {
        let mut a = AdaptiveImmuneSystem::default();
        a.phase = ImmuneResponse::Effector;
        a.cytotoxic_t.count = 3000.0;
        a.cytotoxic_t.affinity = 0.9;
        let before = a.memory_t.count;
        a.memory_formation();
        assert!(a.memory_t.count > before);
        assert_eq!(a.phase, ImmuneResponse::Memory);
        assert!(a.b_cells.is_memory);
        assert!(!a.b_cells.is_plasma);
    }

    #[test]
    fn test_memory_formation_only_in_effector() {
        let mut a = AdaptiveImmuneSystem::default();
        a.phase = ImmuneResponse::Activation;
        let before = a.memory_t.count;
        a.memory_formation();
        assert_eq!(a.memory_t.count, before);
        assert_eq!(a.phase, ImmuneResponse::Activation);
    }

    #[test]
    fn test_recall_antigen_memory_phase() {
        let mut a = AdaptiveImmuneSystem::default();
        a.phase = ImmuneResponse::Memory;
        a.recognized_antigen_id = 7;
        let ag = Antigen { epitope_id: 7, pathogenicity: 0.5, replication_rate: 0.1 };
        let before = a.helper_t.count;
        assert!(a.recall_antigen(&ag));
        assert!(a.helper_t.count > before);
        assert_eq!(a.phase, ImmuneResponse::Effector);
    }

    #[test]
    fn test_recall_antigen_unknown_id() {
        let mut a = AdaptiveImmuneSystem::default();
        a.phase = ImmuneResponse::Memory;
        a.recognized_antigen_id = 7;
        let ag = Antigen { epitope_id: 99, pathogenicity: 0.5, replication_rate: 0.1 };
        assert!(!a.recall_antigen(&ag));
    }

    #[test]
    fn test_recall_antigen_not_in_memory_phase() {
        let mut a = AdaptiveImmuneSystem::default();
        a.phase = ImmuneResponse::Resting;
        let ag = Antigen { epitope_id: 0, pathogenicity: 0.5, replication_rate: 0.1 };
        assert!(!a.recall_antigen(&ag));
    }

    #[test]
    fn test_immune_response_phases_distinct() {
        let phases = [
            ImmuneResponse::Resting,
            ImmuneResponse::Recognition,
            ImmuneResponse::Activation,
            ImmuneResponse::Effector,
            ImmuneResponse::Memory,
        ];
        for i in 0..phases.len() {
            for j in (i + 1)..phases.len() {
                assert_ne!(phases[i], phases[j]);
            }
        }
    }

    #[test]
    fn test_clonal_expansion_respects_carrying_capacity() {
        let mut a = AdaptiveImmuneSystem::default();
        a.phase = ImmuneResponse::Activation;
        a.helper_t.count = 9999.0; // 接近 K=10000
        for _ in 0..1000 {
            a.clonal_expansion(0.1);
        }
        // Logistic 增长不会显著超过 K
        assert!(a.helper_t.count < 10001.0);
    }
}
