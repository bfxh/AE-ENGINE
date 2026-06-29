//! 表观遗传学模块
//!
//! 实现表观遗传学核心机制，包括 DNA 甲基化、组蛋白修饰、染色质重塑和基因组印记。
//!
//! # 生物学背景
//!
//! 表观遗传学研究基因表达的可遗传变化，这些变化不涉及 DNA 序列的改变。
//! 主要机制包括：
//! - DNA 甲基化：CpG 位点的 5-甲基胞嘧啶（5mC）修饰
//! - 组蛋白修饰：组蛋白尾部翻译后修饰（PTM）
//! - 染色质重塑：染色质结构的动态调控
//! - 基因组印记：亲本特异性基因表达
//!
//! # 参考文献
//!
//! - Bird A. (2002) "DNA methylation patterns and epigenetic memory." Genes Dev.
//! - Kouzarides T. (2007) "Chromatin modifications and their function." Cell.
//! - Feil R., Berger F. (2007) "Convergent evolution of genomic imprinting in plants and mammals." Trends Genet.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CpG 位点甲基化状态
///
/// 参考：Lister R. et al. (2009) "Human DNA methylomes at base resolution show widespread epigenomic differences." Nature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MethylationStatus {
    /// 未甲基化
    Unmethylated,
    /// 半甲基化（单链）
    Hemimethylated,
    /// 完全甲基化
    Methylated,
}

/// DNA 甲基化模式
///
/// CpG 位点的甲基化状态记录，用于基因表达调控。
/// CpG 岛通常位于基因启动子区，甲基化导致基因沉默。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethylationPattern {
    /// CpG 位点 ID
    pub cpg_id: u32,
    /// 基因组位置（染色体:位置）
    pub position: String,
    /// 甲基化状态
    pub status: MethylationStatus,
    /// 甲基化水平 (0.0-1.0)
    pub level: f32,
    /// 是否位于 CpG 岛
    pub in_cpg_island: bool,
    /// 关联基因
    pub associated_gene: Option<String>,
}

impl Default for MethylationPattern {
    fn default() -> Self {
        Self {
            cpg_id: 0,
            position: String::new(),
            status: MethylationStatus::Unmethylated,
            level: 0.0,
            in_cpg_island: false,
            associated_gene: None,
        }
    }
}

impl MethylationPattern {
    /// 创建新的 CpG 位点
    pub fn new(cpg_id: u32, position: String, in_cpg_island: bool) -> Self {
        Self {
            cpg_id,
            position,
            status: MethylationStatus::Unmethylated,
            level: 0.0,
            in_cpg_island,
            associated_gene: None,
        }
    }

    /// 甲基化该位点
    ///
    /// 甲基化水平通常在细胞分裂过程中逐渐建立。
    pub fn methylate(&mut self, level: f32) {
        self.level = level.clamp(0.0, 1.0);
        self.status = if self.level > 0.5 {
            MethylationStatus::Methylated
        } else if self.level > 0.0 {
            MethylationStatus::Hemimethylated
        } else {
            MethylationStatus::Unmethylated
        };
    }

    /// 去甲基化
    ///
    /// 主动去甲基化通过 TET 酶介导的氧化反应实现。
    /// 参考：Tahiliani M. et al. (2009) "Conversion of 5-methylcytosine to 5-hydroxymethylcytosine in mammalian DNA." Science.
    pub fn demethylate(&mut self) {
        self.status = MethylationStatus::Unmethylated;
        self.level = 0.0;
    }

    /// 检查是否抑制基因表达
    ///
    /// 启动子区 CpG 岛甲基化通常导致基因沉默。
    pub fn is_gene_silencing(&self) -> bool {
        self.in_cpg_island && self.status == MethylationStatus::Methylated
    }
}

/// 组蛋白修饰类型
///
/// 组蛋白修饰通过影响染色质结构调控基因表达。
/// 参考：Kouzarides T. (2007) "Chromatin modifications and their function." Cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HistoneMark {
    /// H3K4me3 - 活化标记，常见于活性基因启动子
    H3K4me3,
    /// H3K27me3 - 抑制标记，Polycomb 介导的沉默
    H3K27me3,
    /// H3K9me3 - 异染色质标记
    H3K9me3,
    /// H3K36me3 - 基因体标记，转录延伸
    H3K36me3,
    /// H3K27ac - 活化标记，增强子活性
    H3K27ac,
    /// H3K9ac - 活化标记
    H3K9ac,
    /// H4K20me1 - DNA 复制相关
    H4K20me1,
}

impl HistoneMark {
    /// 判断是否为活化标记
    pub fn is_activating(&self) -> bool {
        matches!(self, Self::H3K4me3 | Self::H3K36me3 | Self::H3K27ac | Self::H3K9ac)
    }

    /// 判断是否为抑制标记
    pub fn is_repressing(&self) -> bool {
        matches!(self, Self::H3K27me3 | Self::H3K9me3)
    }
}

/// 组蛋白修饰
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoneModification {
    /// 修饰类型
    pub mark: HistoneMark,
    /// 组蛋白尾部氨基酸位置
    pub position: String,
    /// 修饰丰度 (0.0-1.0)
    pub abundance: f32,
    /// 染色体区域
    pub chromosomal_region: String,
    /// 关联基因
    pub associated_gene: Option<String>,
}

impl Default for HistoneModification {
    fn default() -> Self {
        Self {
            mark: HistoneMark::H3K4me3,
            position: "H3K4".to_string(),
            abundance: 0.0,
            chromosomal_region: String::new(),
            associated_gene: None,
        }
    }
}

impl HistoneModification {
    /// 创建新的组蛋白修饰
    pub fn new(mark: HistoneMark, chromosomal_region: String) -> Self {
        let position = match mark {
            HistoneMark::H3K4me3 => "H3K4".to_string(),
            HistoneMark::H3K27me3 => "H3K27".to_string(),
            HistoneMark::H3K9me3 => "H3K9".to_string(),
            HistoneMark::H3K36me3 => "H3K36".to_string(),
            HistoneMark::H3K27ac => "H3K27".to_string(),
            HistoneMark::H3K9ac => "H3K9".to_string(),
            HistoneMark::H4K20me1 => "H4K20".to_string(),
        };
        Self {
            mark,
            position,
            abundance: 0.0,
            chromosomal_region,
            associated_gene: None,
        }
    }

    /// 设置修饰丰度
    pub fn set_abundance(&mut self, abundance: f32) {
        self.abundance = abundance.clamp(0.0, 1.0);
    }

    /// 增加修饰丰度
    ///
    /// 组蛋白修饰酶（如 HAT、HMT）催化修饰添加。
    pub fn increase_abundance(&mut self, delta: f32) {
        self.abundance = (self.abundance + delta).min(1.0);
    }

    /// 减少修饰丰度
    ///
    /// 组蛋白去修饰酶（如 HDAC、HDM）催化修饰移除。
    pub fn decrease_abundance(&mut self, delta: f32) {
        self.abundance = (self.abundance - delta).max(0.0);
    }
}

/// 染色质状态
///
/// 染色质可分为常染色质（开放）和异染色质（紧密）。
/// 参考：Allis C.D., Jenuwein T. (2016) "The molecular hallmarks of epigenetic control." Nat Rev Genet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChromatinState {
    /// 常染色质 - 开放，允许转录
    Euchromatin,
    /// 兼性异染色质 - 可逆沉默
    FacultativeHeterochromatin,
    /// 组成型异染色质 - 永久沉默
    ConstitutiveHeterochromatin,
    /// 活性增强子
    ActiveEnhancer,
    /// 沉默增强子
    PoisedEnhancer,
}

impl ChromatinState {
    /// 判断染色质是否开放（允许转录）
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Euchromatin | Self::ActiveEnhancer)
    }

    /// 判断染色质是否关闭（转录抑制）
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::FacultativeHeterochromatin | Self::ConstitutiveHeterochromatin)
    }

    /// 判断是否为增强子区域
    pub fn is_enhancer(&self) -> bool {
        matches!(self, Self::ActiveEnhancer | Self::PoisedEnhancer)
    }
}

/// 染色质区域状态记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromatinRegion {
    /// 区域 ID
    pub id: u32,
    /// 染色体位置
    pub position: String,
    /// 染色质状态
    pub state: ChromatinState,
    /// 可及性分数 (0.0-1.0)
    pub accessibility: f32,
    /// 核小体占据密度
    pub nucleosome_density: f32,
    /// 组蛋白修饰列表
    pub histone_marks: Vec<HistoneModification>,
}

impl Default for ChromatinRegion {
    fn default() -> Self {
        Self {
            id: 0,
            position: String::new(),
            state: ChromatinState::Euchromatin,
            accessibility: 1.0,
            nucleosome_density: 0.5,
            histone_marks: Vec::new(),
        }
    }
}

impl ChromatinRegion {
    /// 创建新的染色质区域
    pub fn new(id: u32, position: String, state: ChromatinState) -> Self {
        let (accessibility, nucleosome_density) = match state {
            ChromatinState::Euchromatin => (0.8, 0.3),
            ChromatinState::FacultativeHeterochromatin => (0.3, 0.7),
            ChromatinState::ConstitutiveHeterochromatin => (0.1, 0.9),
            ChromatinState::ActiveEnhancer => (0.9, 0.2),
            ChromatinState::PoisedEnhancer => (0.4, 0.6),
        };
        Self {
            id,
            position,
            state,
            accessibility,
            nucleosome_density,
            histone_marks: Vec::new(),
        }
    }

    /// 添加组蛋白修饰
    pub fn add_histone_mark(&mut self, modification: HistoneModification) {
        self.histone_marks.push(modification);
        self.update_state_from_marks();
    }

    /// 根据组蛋白修饰更新染色质状态
    fn update_state_from_marks(&mut self) {
        let mut activating_score = 0.0;
        let mut repressing_score = 0.0;

        for mark in &self.histone_marks {
            let score = mark.abundance;
            if mark.mark.is_activating() {
                activating_score += score;
            } else if mark.mark.is_repressing() {
                repressing_score += score;
            }
        }

        // 根据修饰分数更新可及性
        self.accessibility = (self.accessibility + activating_score * 0.1 - repressing_score * 0.1).clamp(0.0, 1.0);
        self.nucleosome_density = (self.nucleosome_density - activating_score * 0.05 + repressing_score * 0.05).clamp(0.0, 1.0);

        // 更新状态
        if self.accessibility > 0.7 {
            self.state = ChromatinState::Euchromatin;
        } else if self.accessibility < 0.3 {
            self.state = ChromatinState::FacultativeHeterochromatin;
        }
    }
}

/// 基因组印记
///
/// 印记基因根据亲本来源差异表达。
/// 参考：Barlow D.P., Bartolomei M.S. (2014) "Genomic imprinting in mammals." Cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParentalOrigin {
    /// 母系遗传
    Maternal,
    /// 父系遗传
    Paternal,
}

/// 印记基因记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Imprinting {
    /// 基因名称
    pub gene_name: String,
    /// 染色体位置
    pub position: String,
    /// 表达的亲本等位基因
    pub expressed_allele: ParentalOrigin,
    /// 沉默的亲本等位基因
    pub silenced_allele: ParentalOrigin,
    /// 印记控制区域 (ICR)
    pub icr_position: String,
    /// 印记稳定性 (0.0-1.0)
    pub stability: f32,
}

impl Default for Imprinting {
    fn default() -> Self {
        Self {
            gene_name: String::new(),
            position: String::new(),
            expressed_allele: ParentalOrigin::Maternal,
            silenced_allele: ParentalOrigin::Paternal,
            icr_position: String::new(),
            stability: 1.0,
        }
    }
}

impl Imprinting {
    /// 创建新的印记基因
    pub fn new(gene_name: String, position: String, expressed: ParentalOrigin) -> Self {
        let silenced = match expressed {
            ParentalOrigin::Maternal => ParentalOrigin::Paternal,
            ParentalOrigin::Paternal => ParentalOrigin::Maternal,
        };
        Self {
            gene_name,
            position,
            expressed_allele: expressed,
            silenced_allele: silenced,
            icr_position: String::new(),
            stability: 1.0,
        }
    }

    /// 检查等位基因是否表达
    pub fn is_expressed(&self, origin: ParentalOrigin) -> bool {
        origin == self.expressed_allele
    }

    /// 遗传印记到下一代
    ///
    /// 印记在生殖系中被擦除并重新建立。
    /// 参考：Lee J.T. (2015) "Regulation of X-chromosome inactivation and epigenetic reprogramming." Curr Opin Cell Biol.
    pub fn inherit(&self) -> Self {
        Self {
            gene_name: self.gene_name.clone(),
            position: self.position.clone(),
            expressed_allele: self.expressed_allele,
            silenced_allele: self.silenced_allele,
            icr_position: self.icr_position.clone(),
            stability: self.stability * 0.95, // 轻微降低稳定性
        }
    }

    /// 重置印记（生殖系重编程）
    pub fn reset(&mut self) {
        self.stability = 1.0;
    }
}

/// 表观基因组
///
/// 细胞完整的表观遗传状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Epigenome {
    /// CpG 甲基化位点
    pub methylation_sites: HashMap<u32, MethylationPattern>,
    /// 染色质区域
    pub chromatin_regions: HashMap<u32, ChromatinRegion>,
    /// 印记基因
    pub imprinted_genes: HashMap<String, Imprinting>,
    /// 全局甲基化水平
    pub global_methylation: f32,
    /// 细胞类型
    pub cell_type: String,
}

impl Default for Epigenome {
    fn default() -> Self {
        Self {
            methylation_sites: HashMap::new(),
            chromatin_regions: HashMap::new(),
            imprinted_genes: HashMap::new(),
            global_methylation: 0.5,
            cell_type: String::new(),
        }
    }
}

impl Epigenome {
    /// 创建新的表观基因组
    pub fn new(cell_type: String) -> Self {
        Self {
            methylation_sites: HashMap::new(),
            chromatin_regions: HashMap::new(),
            imprinted_genes: HashMap::new(),
            global_methylation: 0.5,
            cell_type,
        }
    }

    /// 添加甲基化位点
    pub fn add_methylation_site(&mut self, site: MethylationPattern) {
        self.methylation_sites.insert(site.cpg_id, site);
        self.update_global_methylation();
    }

    /// 更新全局甲基化水平
    fn update_global_methylation(&mut self) {
        if self.methylation_sites.is_empty() {
            self.global_methylation = 0.5;
            return;
        }
        let total: f32 = self.methylation_sites.values().map(|s| s.level).sum();
        self.global_methylation = total / self.methylation_sites.len() as f32;
    }

    /// 添加染色质区域
    pub fn add_chromatin_region(&mut self, region: ChromatinRegion) {
        self.chromatin_regions.insert(region.id, region);
    }

    /// 添加印记基因
    pub fn add_imprinted_gene(&mut self, imprinting: Imprinting) {
        self.imprinted_genes.insert(imprinting.gene_name.clone(), imprinting);
    }

    /// 计算基因表达概率
    ///
    /// 综合甲基化状态和染色质可及性。
    pub fn calculate_expression_probability(&self, gene_name: &str) -> f32 {
        let methylation_effect = self.get_methylation_effect(gene_name);
        let chromatin_effect = self.get_chromatin_effect(gene_name);
        let imprinting_effect = self.get_imprinting_effect(gene_name);

        (methylation_effect * 0.4 + chromatin_effect * 0.4 + imprinting_effect * 0.2).clamp(0.0, 1.0)
    }

    fn get_methylation_effect(&self, gene_name: &str) -> f32 {
        let sites: Vec<_> = self.methylation_sites.values()
            .filter(|s| s.associated_gene.as_deref() == Some(gene_name))
            .collect();

        if sites.is_empty() {
            return 0.5;
        }

        // 启动子甲基化抑制表达
        let promoter_methylation: f32 = sites.iter()
            .filter(|s| s.in_cpg_island)
            .map(|s| s.level)
            .sum::<f32>() / sites.iter().filter(|s| s.in_cpg_island).count().max(1) as f32;

        1.0 - promoter_methylation
    }

    fn get_chromatin_effect(&self, gene_name: &str) -> f32 {
        let regions: Vec<_> = self.chromatin_regions.values()
            .filter(|r| r.histone_marks.iter().any(|m| m.associated_gene.as_deref() == Some(gene_name)))
            .collect();

        if regions.is_empty() {
            return 0.5;
        }

        regions.iter().map(|r| r.accessibility).sum::<f32>() / regions.len() as f32
    }

    fn get_imprinting_effect(&self, gene_name: &str) -> f32 {
        self.imprinted_genes.get(gene_name)
            .map(|i| i.stability)
            .unwrap_or(0.5)
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 甲基化测试 =====

    #[test]
    fn test_methylation_pattern_default() {
        let pattern = MethylationPattern::default();
        assert_eq!(pattern.status, MethylationStatus::Unmethylated);
        assert_eq!(pattern.level, 0.0);
        assert!(!pattern.in_cpg_island);
    }

    #[test]
    fn test_cpg_methylation_level() {
        let mut pattern = MethylationPattern::new(1, "chr1:1000".to_string(), true);
        assert_eq!(pattern.status, MethylationStatus::Unmethylated);

        pattern.methylate(0.3);
        assert_eq!(pattern.status, MethylationStatus::Hemimethylated);
        assert!((pattern.level - 0.3).abs() < 0.01);

        pattern.methylate(0.8);
        assert_eq!(pattern.status, MethylationStatus::Methylated);
        assert!((pattern.level - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_methylation_clamp() {
        let mut pattern = MethylationPattern::default();
        pattern.methylate(1.5);
        assert!((pattern.level - 1.0).abs() < 0.01);

        pattern.methylate(-0.5);
        assert!((pattern.level - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_demethylation() {
        let mut pattern = MethylationPattern::default();
        pattern.methylate(0.9);
        assert_eq!(pattern.status, MethylationStatus::Methylated);

        pattern.demethylate();
        assert_eq!(pattern.status, MethylationStatus::Unmethylated);
        assert_eq!(pattern.level, 0.0);
    }

    #[test]
    fn test_gene_silencing() {
        let mut pattern = MethylationPattern::new(1, "chr1:1000".to_string(), true);
        assert!(!pattern.is_gene_silencing());

        pattern.methylate(0.9);
        assert!(pattern.is_gene_silencing());
    }

    #[test]
    fn test_non_cpg_island_no_silencing() {
        let mut pattern = MethylationPattern::new(1, "chr1:1000".to_string(), false);
        pattern.methylate(0.9);
        assert!(!pattern.is_gene_silencing());
    }

    // ===== 组蛋白修饰测试 =====

    #[test]
    fn test_histone_mark_activation_status() {
        assert!(HistoneMark::H3K4me3.is_activating());
        assert!(HistoneMark::H3K27ac.is_activating());
        assert!(HistoneMark::H3K36me3.is_activating());
        assert!(!HistoneMark::H3K27me3.is_activating());
    }

    #[test]
    fn test_histone_mark_repression_status() {
        assert!(HistoneMark::H3K27me3.is_repressing());
        assert!(HistoneMark::H3K9me3.is_repressing());
        assert!(!HistoneMark::H3K4me3.is_repressing());
    }

    #[test]
    fn test_histone_modification_default() {
        let modi = HistoneModification::default();
        assert_eq!(modi.abundance, 0.0);
        assert!(modi.associated_gene.is_none());
    }

    #[test]
    fn test_histone_modification_abundance() {
        let mut modi = HistoneModification::new(HistoneMark::H3K4me3, "chr1:1000-2000".to_string());
        modi.set_abundance(0.7);
        assert!((modi.abundance - 0.7).abs() < 0.01);

        modi.increase_abundance(0.5);
        assert!((modi.abundance - 1.0).abs() < 0.01);

        modi.decrease_abundance(0.3);
        assert!((modi.abundance - 0.7).abs() < 0.01);
    }

    // ===== 染色质状态测试 =====

    #[test]
    fn test_chromatin_state_openness() {
        assert!(ChromatinState::Euchromatin.is_open());
        assert!(ChromatinState::ActiveEnhancer.is_open());
        assert!(!ChromatinState::FacultativeHeterochromatin.is_open());
        assert!(!ChromatinState::ConstitutiveHeterochromatin.is_open());
    }

    #[test]
    fn test_chromatin_state_closedness() {
        assert!(ChromatinState::FacultativeHeterochromatin.is_closed());
        assert!(ChromatinState::ConstitutiveHeterochromatin.is_closed());
        assert!(!ChromatinState::Euchromatin.is_closed());
    }

    #[test]
    fn test_chromatin_state_enhancer() {
        assert!(ChromatinState::ActiveEnhancer.is_enhancer());
        assert!(ChromatinState::PoisedEnhancer.is_enhancer());
        assert!(!ChromatinState::Euchromatin.is_enhancer());
    }

    #[test]
    fn test_chromatin_region_default() {
        let region = ChromatinRegion::default();
        assert_eq!(region.state, ChromatinState::Euchromatin);
        assert!((region.accessibility - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_chromatin_region_creation() {
        let region = ChromatinRegion::new(1, "chr1:1000".to_string(), ChromatinState::ConstitutiveHeterochromatin);
        assert!(region.accessibility < 0.3);
        assert!(region.nucleosome_density > 0.7);
    }

    #[test]
    fn test_chromatin_region_histone_marks() {
        let mut region = ChromatinRegion::new(1, "chr1:1000".to_string(), ChromatinState::Euchromatin);
        let activating = HistoneModification::new(HistoneMark::H3K4me3, "chr1:1000".to_string());
        region.add_histone_mark(activating);
        assert_eq!(region.histone_marks.len(), 1);
    }

    // ===== 印记测试 =====

    #[test]
    fn test_imprinting_default() {
        let imprint = Imprinting::default();
        assert_eq!(imprint.expressed_allele, ParentalOrigin::Maternal);
        assert_eq!(imprint.silenced_allele, ParentalOrigin::Paternal);
        assert!((imprint.stability - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_imprinting_expression() {
        let imprint = Imprinting::new("IGF2".to_string(), "chr11:2000000".to_string(), ParentalOrigin::Paternal);
        assert!(imprint.is_expressed(ParentalOrigin::Paternal));
        assert!(!imprint.is_expressed(ParentalOrigin::Maternal));
    }

    #[test]
    fn test_imprinting_inheritance() {
        let imprint = Imprinting::new("H19".to_string(), "chr11:2000000".to_string(), ParentalOrigin::Maternal);
        let inherited = imprint.inherit();
        assert!((inherited.stability - 0.95).abs() < 0.01);
        assert_eq!(inherited.gene_name, "H19");
    }

    #[test]
    fn test_imprinting_reset() {
        let mut imprint = Imprinting::new("IGF2".to_string(), "chr11:2000000".to_string(), ParentalOrigin::Paternal);
        imprint.stability = 0.8;
        imprint.reset();
        assert!((imprint.stability - 1.0).abs() < 0.01);
    }

    // ===== 表观基因组测试 =====

    #[test]
    fn test_epigenome_default() {
        let epi = Epigenome::default();
        assert!((epi.global_methylation - 0.5).abs() < 0.01);
        assert!(epi.methylation_sites.is_empty());
    }

    #[test]
    fn test_epigenome_methylation_update() {
        let mut epi = Epigenome::new("hepatocyte".to_string());
        let mut site1 = MethylationPattern::new(1, "chr1:1000".to_string(), true);
        site1.methylate(0.8);
        let mut site2 = MethylationPattern::new(2, "chr1:2000".to_string(), false);
        site2.methylate(0.4);

        epi.add_methylation_site(site1);
        epi.add_methylation_site(site2);

        assert!((epi.global_methylation - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_epigenome_expression_probability() {
        let mut epi = Epigenome::new("neuron".to_string());
        let mut site = MethylationPattern::new(1, "chr1:1000".to_string(), true);
        site.associated_gene = Some("GeneA".to_string());
        site.methylate(0.2);
        epi.add_methylation_site(site);

        let prob = epi.calculate_expression_probability("GeneA");
        assert!(prob > 0.5); // 低甲基化应该有较高表达概率
    }
}