//! reaction_prediction.rs - 从原子推导未知化学反应
//!
//! 核心理念：不依赖预设反应数据库，从原子性质 + 热力学 + 动力学 + 量子化学近似推导反应。
//! 用户核心需求："支持没有预先写进数据库的化学物质能够通过底层原子的能推导新的化学方程式"
//!
//! 推导流水线：
//! 1. 反应位点识别（functional_groups + 电负性/电荷分析）
//! 2. 位点配对（亲核↔亲电、酸↔碱、自由基↔自由基）
//! 3. 键断裂/形成枚举（基于位点配对，生成候选反应）
//! 4. 热力学评估（bond_enthalpy_method 估算 ΔH，估算 ΔG）
//! 5. 动力学评估（Evans-Polanyi 估算 Ea，Eyring 估算速率常数）
//! 6. 守恒律验证（质量/原子数/电荷绝对守恒）
//! 7. 可行性过滤与排序
//!
//! 参考：
//! - Carey & Sundberg, "Advanced Organic Chemistry" (机理分类)
//! - Evans-Polanyi 直线关系 (JACS 1938)
//! - Eyring 过渡态理论 (JCP 1935)
//! - Fukui 前线轨道理论 (Nobel 1981)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::conservation::ConservationChecker;
use crate::elements::Element;
use crate::functional_groups::{identify_reactive_sites, ReactiveSite, SiteType};
use crate::kinetics::{equilibrium_constant_from_dg, evans_polanyi_ea, eyring_rate_constant};
use crate::molecules::{AtomId, Bond, BondOrder, Molecule};
use crate::thermodynamics::estimate_bde;

/// 标准温度 K
const T_STD: f64 = 298.15;

// ============================================================
// 反应机理分类
// ============================================================

/// 反应机理（从原子推导，不预设产物数据库）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReactionMechanism {
    /// 单分子亲核取代（碳正离子中间体）
    SN1,
    /// 双分子亲核取代（协同）
    SN2,
    /// 单分子消除
    E1,
    /// 双分子消除（反式共平面）
    E2,
    /// 亲电加成（烯烃/炔烃）
    ElectrophilicAddition,
    /// 亲核加成（羰基等）
    NucleophilicAddition,
    /// 自由基取代
    RadicalSubstitution,
    /// 氧化还原（电子转移）
    OxidationReduction,
    /// 酸碱中和（质子转移）
    AcidBase,
    /// 燃烧（O2 氧化含 C/H 物质）
    Combustion,
    /// 水解（水断裂化学键）
    Hydrolysis,
    /// 缩合（两个分子合一，脱小分子）
    Condensation,
    /// 聚合（单体重复连接）
    Polymerization,
    /// 重排（骨架异构化）
    Rearrangement,
    /// 分解（单分子断裂）
    Decomposition,
    /// 复分解（AB + CD → AD + CB）
    DoubleDisplacement,
    /// 化合（A + B → AB）
    Synthesis,
    /// 置换（A + BC → AC + B）
    SingleDisplacement,
    /// 自定义机理（扩展用）
    Custom(String),
}

impl ReactionMechanism {
    pub fn name(&self) -> &str {
        match self {
            Self::SN1 => "SN1", Self::SN2 => "SN2",
            Self::E1 => "E1", Self::E2 => "E2",
            Self::ElectrophilicAddition => "ElectrophilicAddition",
            Self::NucleophilicAddition => "NucleophilicAddition",
            Self::RadicalSubstitution => "RadicalSubstitution",
            Self::OxidationReduction => "OxidationReduction",
            Self::AcidBase => "AcidBase",
            Self::Combustion => "Combustion",
            Self::Hydrolysis => "Hydrolysis",
            Self::Condensation => "Condensation",
            Self::Polymerization => "Polymerization",
            Self::Rearrangement => "Rearrangement",
            Self::Decomposition => "Decomposition",
            Self::DoubleDisplacement => "DoubleDisplacement",
            Self::Synthesis => "Synthesis",
            Self::SingleDisplacement => "SingleDisplacement",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// 反应可行性等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feasibility {
    /// ΔG < -10 kJ/mol，自发
    Spontaneous,
    /// -10 ≤ ΔG < 0，有利
    Favorable,
    /// |ΔG| < 10，平衡
    Equilibrium,
    /// ΔG > 10，非自发
    NonSpontaneous,
    /// Ea > 阈值，动力学受阻
    KineticallyBlocked,
}
// ============================================================
// 键变化与候选反应
// ============================================================

/// 键变化描述（断键或成键）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondChange {
    /// 产物分子索引（0=产物A, 1=产物B, ...）
    pub product_idx: usize,
    pub atom_a: AtomId,
    pub atom_b: AtomId,
    pub order: BondOrder,
}

/// 候选反应（从原子推导）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateReaction {
    pub reactants: Vec<Molecule>,
    pub products: Vec<Molecule>,
    pub mechanism: ReactionMechanism,
    pub broken_bonds: Vec<BondChange>,
    pub formed_bonds: Vec<BondChange>,
    /// 焓变 kJ/mol（负=放热）
    pub delta_h: f64,
    /// 吉布斯自由能变 kJ/mol（负=自发）
    pub delta_g: f64,
    /// 活化能 kJ/mol
    pub activation_energy: f64,
    /// 速率常数 1/s @ temperature
    pub rate_constant: f64,
    /// 平衡常数 K
    pub equilibrium_constant: f64,
    pub feasibility: Feasibility,
    /// 守恒律是否通过
    pub conservation_ok: bool,
}

impl CandidateReaction {
    /// 反应方程式字符串
    pub fn equation(&self) -> String {
        let r: Vec<String> = self.reactants.iter().map(|m| m.name.clone().unwrap_or_else(|| m.molecular_formula())).collect();
        let p: Vec<String> = self.products.iter().map(|m| m.name.clone().unwrap_or_else(|| m.molecular_formula())).collect();
        format!("{} → {}  [{}] ΔH={:.1} ΔG={:.1} Ea={:.1} K={:.3e}",
            r.join(" + "), p.join(" + "), self.mechanism.name(),
            self.delta_h, self.delta_g, self.activation_energy, self.equilibrium_constant)
    }
}

// ============================================================
// 反应预测器
// ============================================================

/// 反应预测器配置
pub struct ReactionPredictor {
    /// 反应温度 K
    pub temperature: f64,
    /// 守恒律检查器
    pub checker: ConservationChecker,
    /// 最大键变化数（控制组合爆炸）
    pub max_bond_changes: usize,
    /// ΔG 阈值 kJ/mol（超过则拒绝）
    pub dg_threshold: f64,
    /// 活化能阈值 kJ/mol（超过则标记动力学受阻）
    pub ea_threshold: f64,
    /// 最大候选数
    pub max_candidates: usize,
}

impl Default for ReactionPredictor {
    fn default() -> Self {
        Self {
            temperature: T_STD,
            checker: ConservationChecker::standard(),
            max_bond_changes: 4,
            dg_threshold: 100.0,
            ea_threshold: 250.0,
            max_candidates: 50,
        }
    }
}

impl ReactionPredictor {
    pub fn new(temperature: f64) -> Self {
        Self { temperature, ..Default::default() }
    }

    pub fn strict(temperature: f64) -> Self {
        Self {
            temperature,
            checker: ConservationChecker::strict(),
            ..Default::default()
        }
    }

    /// 主入口：预测两个分子间所有可能反应
    pub fn predict(&self, a: &Molecule, b: &Molecule) -> Vec<CandidateReaction> {
        let mut candidates = Vec::new();

        // 1. 识别反应位点
        let sites_a = identify_reactive_sites(a);
        let sites_b = identify_reactive_sites(b);

        // 2. 位点配对
        let pairs = self.match_sites(&sites_a, &sites_b);

        // 3. 对每对位点生成候选反应
        for (sa, sb) in &pairs {
            if let Some(mut cand) = self.build_candidate(a, b, sa, sb) {
                self.evaluate(&mut cand);
                if self.is_feasible(&cand) {
                    cand.conservation_ok = self.verify_conservation(&cand);
                    candidates.push(cand);
                }
            }
        }

        // 4. 特殊反应类型
        self.add_special_reactions(a, b, &mut candidates);

        // 5. 排序：按速率常数降序
        candidates.sort_by(|x, y| {
            y.rate_constant.partial_cmp(&x.rate_constant).unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(self.max_candidates);
        candidates
    }

    /// 预测单分子分解（返回所有可能的键断裂，含非自发反应）
    pub fn predict_decomposition(&self, mol: &Molecule) -> Vec<CandidateReaction> {
        let mut results = Vec::new();
        // 枚举所有可断裂的键
        for (bi, bond) in mol.bonds.iter().enumerate() {
            if let Some(mut cand) = self.build_decomposition(mol, bi, bond) {
                self.evaluate(&mut cand);
                // 分解反应不过滤热力学可行性，全部返回（标注 feasibility）
                cand.conservation_ok = self.verify_conservation(&cand);
                results.push(cand);
            }
        }
        results
    }

    /// 预测燃烧反应（含 C/H 物质 + O2 → CO2 + H2O）
    pub fn predict_combustion(&self, mol: &Molecule) -> Option<CandidateReaction> {
        let counts = mol.atom_count_by_element();
        let c = *counts.get(&Element::C).unwrap_or(&0) as f64;
        let h = *counts.get(&Element::H).unwrap_or(&0) as f64;
        if c == 0.0 && h == 0.0 { return None; }

        // 平衡：CxHy + (x+y/4) O2 → x CO2 + (y/2) H2O
        let o2_count = c + h / 4.0;
        let co2_count = c;
        let h2o_count = h / 2.0;

        let mut o2 = Molecule::new(); o2.name = Some("O2".into());
        o2.add_atom(Element::O); o2.add_atom(Element::O);
        o2.add_bond(0, 1, BondOrder::Double);

        let mut co2 = Molecule::new(); co2.name = Some("CO2".into());
        let c0 = co2.add_atom(Element::C);
        let o1 = co2.add_atom(Element::O);
        let o2a = co2.add_atom(Element::O);
        co2.add_bond(c0, o1, BondOrder::Double);
        co2.add_bond(c0, o2a, BondOrder::Double);

        let mut h2o = Molecule::new(); h2o.name = Some("H2O".into());
        let ow = h2o.add_atom(Element::O);
        let hw1 = h2o.add_atom(Element::H);
        let hw2 = h2o.add_atom(Element::H);
        h2o.add_bond(ow, hw1, BondOrder::Single);
        h2o.add_bond(ow, hw2, BondOrder::Single);

        let mut products = Vec::new();
        for _ in 0..co2_count as usize { products.push(co2.clone()); }
        for _ in 0..h2o_count as usize { products.push(h2o.clone()); }

        let mut reactants = vec![mol.clone()];
        for _ in 0..o2_count as usize { reactants.push(o2.clone()); }

        let mut cand = CandidateReaction {
            reactants, products,
            mechanism: ReactionMechanism::Combustion,
            broken_bonds: vec![], formed_bonds: vec![],
            delta_h: 0.0, delta_g: 0.0,
            activation_energy: 0.0, rate_constant: 0.0,
            equilibrium_constant: 0.0,
            feasibility: Feasibility::Equilibrium,
            conservation_ok: false,
        };
        self.evaluate(&mut cand);
        cand.conservation_ok = self.verify_conservation(&cand);
        Some(cand)
    }
    /// 预测酸碱反应（质子转移）
    pub fn predict_acid_base(&self, acid: &Molecule, base: &Molecule) -> Option<CandidateReaction> {
        let acid_sites = identify_reactive_sites(acid);
        let base_sites = identify_reactive_sites(base);
        let acid_h = acid_sites.iter().find(|s| s.site_type == SiteType::Acid)?;
        let base_n = base_sites.iter().find(|s| s.site_type == SiteType::Base)?;

        // 产物：酸去 H，碱得 H
        let mut prod_acid = acid.clone();
        let mut prod_base = base.clone();

        // 从酸移除 H（断 O-H/N-H/S-H 键）
        let h_atom = acid_h.atom_id;
        let removed = self.remove_hydrogen(&mut prod_acid, h_atom);
        if !removed { return None; }

        // 在碱上加 H（连到碱性位原子）
        let base_atom = base_n.atom_id;
        let new_h = prod_base.add_atom(Element::H);
        prod_base.add_bond(base_atom, new_h, BondOrder::Single);

        let mut cand = CandidateReaction {
            reactants: vec![acid.clone(), base.clone()],
            products: vec![prod_acid, prod_base],
            mechanism: ReactionMechanism::AcidBase,
            broken_bonds: vec![],
            formed_bonds: vec![],
            delta_h: 0.0, delta_g: 0.0,
            activation_energy: 0.0, rate_constant: 0.0,
            equilibrium_constant: 0.0,
            feasibility: Feasibility::Equilibrium,
            conservation_ok: false,
        };
        self.evaluate(&mut cand);
        cand.conservation_ok = self.verify_conservation(&cand);
        Some(cand)
    }

    /// 预测化合反应（A + B → AB）
    pub fn predict_synthesis(&self, a: &Molecule, b: &Molecule) -> Option<CandidateReaction> {
        // 找最高反应性位点
        let sites_a = identify_reactive_sites(a);
        let sites_b = identify_reactive_sites(b);
        let sa = sites_a.iter().max_by(|x, y| x.reactivity_score.partial_cmp(&y.reactivity_score).unwrap())?;
        let sb = sites_b.iter().max_by(|x, y| x.reactivity_score.partial_cmp(&y.reactivity_score).unwrap())?;

        if !self.compatible(sa, sb) { return None; }

        // 合并分子，形成新键
        let mut combined = a.clone();
        let offset = combined.atoms.len() as AtomId;
        for atom in &b.atoms {
            let mut new_atom = *atom;
            new_atom.id += offset;
            combined.atoms.push(new_atom);
        }
        for bond in &b.bonds {
            combined.add_bond(bond.a + offset, bond.b + offset, bond.order);
        }
        // 形成新键
        combined.add_bond(sa.atom_id, sb.atom_id + offset, BondOrder::Single);

        let mut cand = CandidateReaction {
            reactants: vec![a.clone(), b.clone()],
            products: vec![combined],
            mechanism: ReactionMechanism::Synthesis,
            broken_bonds: vec![],
            formed_bonds: vec![],
            delta_h: 0.0, delta_g: 0.0,
            activation_energy: 0.0, rate_constant: 0.0,
            equilibrium_constant: 0.0,
            feasibility: Feasibility::Equilibrium,
            conservation_ok: false,
        };
        self.evaluate(&mut cand);
        cand.conservation_ok = self.verify_conservation(&cand);
        Some(cand)
    }

    // ============================================================
    // 内部方法
    // ============================================================

    /// 位点配对
    fn match_sites<'a>(&self, sites_a: &'a [ReactiveSite], sites_b: &'a [ReactiveSite]) -> Vec<(&'a ReactiveSite, &'a ReactiveSite)> {
        let mut pairs = Vec::new();
        for sa in sites_a {
            for sb in sites_b {
                if self.compatible(sa, sb) {
                    pairs.push((sa, sb));
                }
            }
        }
        pairs
    }

    /// 位点兼容性判断
    fn compatible(&self, a: &ReactiveSite, b: &ReactiveSite) -> bool {
        use SiteType::*;
        matches!((a.site_type, b.site_type),
            (Nucleophile, Electrophile) | (Electrophile, Nucleophile)
            | (Acid, Base) | (Base, Acid)
            | (Radical, Radical)
            | (Nucleophile, LeavingGroup) | (LeavingGroup, Nucleophile)
            | (Base, Electrophile) | (Electrophile, Base))
    }

    /// 根据位点类型分类机理
    fn classify_mechanism(&self, sa: &ReactiveSite, sb: &ReactiveSite) -> ReactionMechanism {
        use SiteType::*;
        match (sa.site_type, sb.site_type) {
            (Acid, Base) | (Base, Acid) => ReactionMechanism::AcidBase,
            (Nucleophile, Electrophile) | (Electrophile, Nucleophile) => ReactionMechanism::NucleophilicAddition,
            (Nucleophile, LeavingGroup) | (LeavingGroup, Nucleophile) => ReactionMechanism::SN2,
            (Base, Electrophile) | (Electrophile, Base) => ReactionMechanism::NucleophilicAddition,
            (Radical, Radical) => ReactionMechanism::Synthesis,
            _ => ReactionMechanism::Custom("unknown".into()),
        }
    }

    /// 从酸分子移除一个氢原子（断键）
    fn remove_hydrogen(&self, mol: &mut Molecule, h_id: AtomId) -> bool {
        // 找到 H 所在的键
        let bond_idx = mol.bonds.iter().position(|b| b.a == h_id || b.b == h_id);
        if let Some(idx) = bond_idx {
            mol.bonds.remove(idx);
            // 也移除原子（标记为移除：用 swap_remove）
            if (h_id as usize) < mol.atoms.len() {
                mol.atoms.retain(|a| a.id != h_id);
                // 重新分配 id（保持连续）
                for (i, a) in mol.atoms.iter_mut().enumerate() {
                    let old_id = a.id;
                    a.id = i as AtomId;
                    if old_id != a.id {
                        for b in mol.bonds.iter_mut() {
                            if b.a == old_id { b.a = a.id; }
                            if b.b == old_id { b.b = a.id; }
                        }
                    }
                }
            }
            true
        } else { false }
    }

    /// 添加特殊反应
    fn add_special_reactions(&self, a: &Molecule, b: &Molecule, candidates: &mut Vec<CandidateReaction>) {
        // 燃烧：如果 b 是 O2 或 a 是 O2
        let o2 = Self::is_o2(a).then_some(b).or_else(|| Self::is_o2(b).then_some(a));
        if let Some(fuel) = o2 {
            if let Some(c) = self.predict_combustion(fuel) { candidates.push(c); }
        }
        // 酸碱
        if let Some(c) = self.predict_acid_base(a, b) { candidates.push(c); }
        // 化合
        if let Some(c) = self.predict_synthesis(a, b) { candidates.push(c); }
    }

    fn is_o2(mol: &Molecule) -> bool {
        mol.atoms.len() == 2 && mol.atoms.iter().all(|a| a.element == Element::O)
            && mol.bonds.iter().any(|b| b.order == BondOrder::Double)
    }
    /// 构建候选反应（通用路径）
    fn build_candidate(&self, a: &Molecule, b: &Molecule, sa: &ReactiveSite, sb: &ReactiveSite) -> Option<CandidateReaction> {
        let mechanism = self.classify_mechanism(sa, sb);

        // 合并 a + b 为一个产物分子图
        let mut combined = a.clone();
        let offset = combined.atoms.len() as AtomId;
        for atom in &b.atoms {
            let mut na = *atom;
            na.id += offset;
            combined.atoms.push(na);
        }
        for bond in &b.bonds {
            let mut nb = *bond;
            nb.a += offset;
            nb.b += offset;
            combined.bonds.push(nb);
        }

        let atom_a = sa.atom_id;
        let atom_b = sb.atom_id + offset;

        let mut broken = Vec::new();
        let mut formed = Vec::new();

        match &mechanism {
            ReactionMechanism::AcidBase => {
                // 质子转移：从酸位移除 H，加到碱位
                // 找酸位相连的 H
                let h_bond = combined.bonds.iter().find(|bd| {
                    (bd.a == atom_a && combined.atoms.iter().find(|at| at.id == bd.b).map_or(false, |at| at.element == Element::H))
                    || (bd.b == atom_a && combined.atoms.iter().find(|at| at.id == bd.a).map_or(false, |at| at.element == Element::H))
                });
                if let Some(hb) = h_bond {
                    let h_id = if combined.atoms.iter().find(|at| at.id == hb.b).map_or(false, |at| at.element == Element::H) { hb.b } else { hb.a };
                    // 断 O-H
                    broken.push(BondChange { product_idx: 0, atom_a: atom_a, atom_b: h_id, order: BondOrder::Single });
                    // 形成 H-碱位
                    formed.push(BondChange { product_idx: 0, atom_a: atom_b, atom_b: h_id, order: BondOrder::Single });
                }
            }
            ReactionMechanism::NucleophilicAddition | ReactionMechanism::Synthesis => {
                // 形成新键 a-b
                formed.push(BondChange { product_idx: 0, atom_a, atom_b, order: BondOrder::Single });
            }
            ReactionMechanism::SN2 => {
                // 亲核攻击 + 离去基离开
                formed.push(BondChange { product_idx: 0, atom_a, atom_b, order: BondOrder::Single });
                // 离去基键断裂：找 sb 所在原子的另一条键（非刚形成的）
                let leaving_bond = combined.bonds.iter().find(|bd| {
                    (bd.a == sb.atom_id || bd.b == sb.atom_id)
                    && !(bd.a == atom_a && bd.b == atom_b)
                    && !(bd.a == atom_b && bd.b == atom_a)
                });
                if let Some(lb) = leaving_bond {
                    broken.push(BondChange { product_idx: 0, atom_a: lb.a, atom_b: lb.b, order: lb.order });
                }
            }
            _ => {
                formed.push(BondChange { product_idx: 0, atom_a, atom_b, order: BondOrder::Single });
            }
        }

        // 应用键变化到 combined
        for bc in &broken {
            combined.bonds.retain(|b| !((b.a == bc.atom_a && b.b == bc.atom_b) || (b.a == bc.atom_b && b.b == bc.atom_a)));
        }
        for bc in &formed {
            combined.add_bond(bc.atom_a, bc.atom_b, bc.order);
        }

        Some(CandidateReaction {
            reactants: vec![a.clone(), b.clone()],
            products: vec![combined],
            mechanism,
            broken_bonds: broken,
            formed_bonds: formed,
            delta_h: 0.0, delta_g: 0.0,
            activation_energy: 0.0, rate_constant: 0.0,
            equilibrium_constant: 0.0,
            feasibility: Feasibility::Equilibrium,
            conservation_ok: false,
        })
    }

    /// 构建分解反应（断一根键）
    fn build_decomposition(&self, mol: &Molecule, bond_idx: usize, _bond: &Bond) -> Option<CandidateReaction> {
        let mut prod = mol.clone();
        let removed = prod.bonds.remove(bond_idx);
        // 分裂成两个分子（连通分量）
        let fragments = self.split_fragments(&prod);

        let broken = vec![BondChange {
            product_idx: 0, atom_a: removed.a, atom_b: removed.b, order: removed.order,
        }];

        Some(CandidateReaction {
            reactants: vec![mol.clone()],
            products: fragments,
            mechanism: ReactionMechanism::Decomposition,
            broken_bonds: broken,
            formed_bonds: vec![],
            delta_h: 0.0, delta_g: 0.0,
            activation_energy: 0.0, rate_constant: 0.0,
            equilibrium_constant: 0.0,
            feasibility: Feasibility::Equilibrium,
            conservation_ok: false,
        })
    }

    /// 将分子按连通分量分裂
    fn split_fragments(&self, mol: &Molecule) -> Vec<Molecule> {
        let n = mol.atoms.len();
        if n == 0 { return vec![]; }
        let mut visited = vec![false; n];
        let mut fragments = Vec::new();

        // 构建邻接表
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for b in &mol.bonds {
            let ai = b.a as usize;
            let bi = b.b as usize;
            if ai < n && bi < n {
                adj[ai].push(bi);
                adj[bi].push(ai);
            }
        }

        for start in 0..n {
            if visited[start] { continue; }
            // BFS
            let mut component = Vec::new();
            let mut queue = vec![start];
            visited[start] = true;
            while let Some(u) = queue.pop() {
                component.push(u);
                for &v in &adj[u] {
                    if !visited[v] {
                        visited[v] = true;
                        queue.push(v);
                    }
                }
            }
            // 构建子分子
            let mut frag = Molecule::new();
            let mut id_map = HashMap::new();
            for &orig_idx in component.iter() {
                let new_id = frag.add_atom(mol.atoms[orig_idx].element);
                id_map.insert(orig_idx as AtomId, new_id);
                // 复制形式电荷等
                if let Some(a) = frag.atoms.last_mut() {
                    a.formal_charge = mol.atoms[orig_idx].formal_charge;
                    a.hybridization = mol.atoms[orig_idx].hybridization;
                }
            }
            for b in &mol.bonds {
                let ai = b.a as usize;
                let bi = b.b as usize;
                if let (Some(&na), Some(&nb)) = (id_map.get(&(ai as AtomId)), id_map.get(&(bi as AtomId))) {
                    if component.contains(&ai) && component.contains(&bi) {
                        frag.add_bond(na, nb, b.order);
                    }
                }
            }
            if !frag.atoms.is_empty() {
                fragments.push(frag);
            }
        }
        fragments
    }
    // ============================================================
    // 热力学与动力学评估
    // ============================================================

    /// 评估候选反应的 ΔH/ΔG/Ea/k/K
    fn evaluate(&self, cand: &mut CandidateReaction) {
        // ΔH = Σ BDE(断键) - Σ BDE(成键)
        let mut dh = 0.0_f64;
        for bc in &cand.broken_bonds {
            // 查找反应物中的键 BDE
            if let Some(mol) = cand.reactants.get(bc.product_idx) {
                if let Some(bond) = mol.bonds.iter().find(|b| (b.a == bc.atom_a && b.b == bc.atom_b) || (b.a == bc.atom_b && b.b == bc.atom_a)) {
                    dh += bond.bde_kjmol;
                }
            }
        }
        for bc in &cand.formed_bonds {
            // 成键放热：查找产物原子元素
            if let Some(mol) = cand.products.get(bc.product_idx) {
                let e1 = mol.atoms.iter().find(|a| a.id == bc.atom_a).map(|a| a.element);
                let e2 = mol.atoms.iter().find(|a| a.id == bc.atom_b).map(|a| a.element);
                if let (Some(e1), Some(e2)) = (e1, e2) {
                    dh -= estimate_bde(e1, e2, bc.order);
                }
            } else {
                // 跨分子成键，用元素估算
                dh -= estimate_bde(Element::C, Element::C, bc.order);
            }
        }

        // 燃烧特殊处理：产物 CO2/H2O 生成能
        if cand.mechanism == ReactionMechanism::Combustion {
            let counts: HashMap<Element, u32> = cand.reactants[0].atom_count_by_element();
            let c = *counts.get(&Element::C).unwrap_or(&0) as f64;
            let h = *counts.get(&Element::H).unwrap_or(&0) as f64;
            // ΔH ≈ -418 kJ/mol per C (CO2) + -286 kJ/mol per 2H (H2O)
            // 简化：用燃烧焓近似
            dh = -(c * 393.5 + h * 0.5 * 285.8);
        }

        cand.delta_h = dh;

        // ΔG ≈ ΔH - T·ΔS（简化：ΔS 估算）
        // 气体反应 ΔS 典型 ±100-200 J/(mol·K)
        let n_reactants = cand.reactants.len() as f64;
        let n_products = cand.products.len() as f64;
        let dn = n_products - n_reactants;
        let ds = dn * 120.0; // 简化：每摩尔气体变化 ~120 J/(mol·K)
        cand.delta_g = dh - self.temperature * ds / 1000.0;

        // 平衡常数 K = exp(-ΔG/(RT))
        cand.equilibrium_constant = equilibrium_constant_from_dg(cand.delta_g, self.temperature);

        // 活化能：Evans-Polanyi Ea = E0 + α·ΔH
        // E0 ~ 80 kJ/mol（典型），α ~ 0.5
        let e0 = 80.0_f64;
        let alpha = 0.5_f64;
        cand.activation_energy = evans_polanyi_ea(e0, alpha, dh);

        // 燃烧活化能较低（链式反应）
        if cand.mechanism == ReactionMechanism::Combustion {
            cand.activation_energy = 30.0;
        }
        // 酸碱反应几乎无活化能
        if cand.mechanism == ReactionMechanism::AcidBase {
            cand.activation_energy = 5.0;
        }

        // 速率常数：Eyring k = κ·(kB·T/h)·exp(-Ea/(RT))
        cand.rate_constant = eyring_rate_constant(cand.activation_energy * 1000.0, self.temperature, 1.0);

        // 可行性分类
        cand.feasibility = if cand.activation_energy > self.ea_threshold {
            Feasibility::KineticallyBlocked
        } else if cand.delta_g < -10.0 {
            Feasibility::Spontaneous
        } else if cand.delta_g < 0.0 {
            Feasibility::Favorable
        } else if cand.delta_g.abs() < 10.0 {
            Feasibility::Equilibrium
        } else {
            Feasibility::NonSpontaneous
        };
    }

    /// 可行性过滤
    fn is_feasible(&self, cand: &CandidateReaction) -> bool {
        // 排除动力学完全受阻且非自发的
        if cand.feasibility == Feasibility::KineticallyBlocked && cand.delta_g > 0.0 {
            return false;
        }
        // ΔG 超过阈值
        if cand.delta_g > self.dg_threshold {
            return false;
        }
        true
    }

    // ============================================================
    // 守恒律验证
    // ============================================================

    /// 验证守恒律（质量/原子数/电荷绝对守恒）
    fn verify_conservation(&self, cand: &CandidateReaction) -> bool {
        // 构建反应物总状态
        let mut before_counts: HashMap<u8, u64> = HashMap::new();
        for mol in &cand.reactants {
            for a in &mol.atoms {
                *before_counts.entry(a.element as u8).or_insert(0) += 1;
            }
        }
        let mut after_counts: HashMap<u8, u64> = HashMap::new();
        for mol in &cand.products {
            for a in &mol.atoms {
                *after_counts.entry(a.element as u8).or_insert(0) += 1;
            }
        }
        // 原子数精确匹配（u64 整数，不阉割）
        if before_counts != after_counts { return false; }

        // 电荷守恒
        let charge_before: i32 = cand.reactants.iter().map(|m| m.charge).sum();
        let charge_after: i32 = cand.products.iter().map(|m| m.charge).sum();
        if charge_before != charge_after { return false; }

        // 质量守恒（从原子计数推算）
        let mass_before: f64 = cand.reactants.iter().flat_map(|m| m.atoms.iter()).map(|a| a.element.atomic_mass()).sum();
        let mass_after: f64 = cand.products.iter().flat_map(|m| m.atoms.iter()).map(|a| a.element.atomic_mass()).sum();
        if (mass_before - mass_after).abs() > 1e-9 { return false; }

        true
    }

    // ============================================================
    // 反应网络
    // ============================================================

    /// 构建多步反应网络（深度优先搜索）
    pub fn build_reaction_network(&self, initial: &[Molecule], max_depth: usize) -> ReactionNetwork {
        let mut network = ReactionNetwork::new();
        let mut visited: Vec<String> = Vec::new();

        let initial_key: String = initial.iter().map(|m| m.molecular_formula()).collect::<Vec<_>>().join("+");
        network.add_node(initial_key.clone());

        self.expand_network(initial, &initial_key, 0, max_depth, &mut network, &mut visited);

        network
    }

    fn expand_network(
        &self,
        species: &[Molecule],
        parent_key: &str,
        depth: usize,
        max_depth: usize,
        network: &mut ReactionNetwork,
        visited: &mut Vec<String>,
    ) {
        if depth >= max_depth { return; }

        // 两两反应
        for i in 0..species.len() {
            for j in (i+1)..species.len() {
                let cands = self.predict(&species[i], &species[j]);
                for cand in cands {
                    let product_key: String = cand.products.iter().map(|m| m.molecular_formula()).collect::<Vec<_>>().join("+");
                    let reaction_desc = cand.equation();
                    network.add_node(product_key.clone());
                    network.add_edge(parent_key.to_string(), product_key.clone(), reaction_desc);

                    if !visited.contains(&product_key) && cand.products.len() <= 5 {
                        visited.push(product_key.clone());
                        // 递归展开
                        let mut next = species.to_vec();
                        next.remove(j);
                        next.remove(i);
                        next.extend(cand.products);
                        self.expand_network(&next, &product_key, depth + 1, max_depth, network, visited);
                    }
                }
            }
            // 单分子分解
            let decomps = self.predict_decomposition(&species[i]);
            for cand in decomps {
                let product_key: String = cand.products.iter().map(|m| m.molecular_formula()).collect::<Vec<_>>().join("+");
                let reaction_desc = cand.equation();
                network.add_node(product_key.clone());
                network.add_edge(parent_key.to_string(), product_key.clone(), reaction_desc);

                if !visited.contains(&product_key) && cand.products.len() <= 5 {
                    visited.push(product_key.clone());
                    let mut next = species.to_vec();
                    next.remove(i);
                    next.extend(cand.products);
                    self.expand_network(&next, &product_key, depth + 1, max_depth, network, visited);
                }
            }
        }
    }
}
// ============================================================
// 反应网络结构
// ============================================================

/// 反应网络节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNode {
    pub key: String,
    pub formula: String,
}

/// 反应网络边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEdge {
    pub from: String,
    pub to: String,
    pub reaction: String,
}

/// 反应网络（多步反应路径图）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionNetwork {
    pub nodes: Vec<NetworkNode>,
    pub edges: Vec<NetworkEdge>,
}

impl ReactionNetwork {
    pub fn new() -> Self {
        Self { nodes: vec![], edges: vec![] }
    }

    pub fn add_node(&mut self, key: String) {
        if !self.nodes.iter().any(|n| n.key == key) {
            self.nodes.push(NetworkNode {
                formula: key.clone(),
                key,
            });
        }
    }

    pub fn add_edge(&mut self, from: String, to: String, reaction: String) {
        self.edges.push(NetworkEdge { from, to, reaction });
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }

    /// 导出为 DOT 图（Graphviz）
    pub fn to_dot(&self) -> String {
        let mut s = String::from("digraph ReactionNetwork {\n");
        s.push_str("    rankdir=LR;\n");
        s.push_str("    node [shape=box, style=filled, fillcolor=lightblue];\n");
        for n in &self.nodes {
            s.push_str(&format!("    \"{}\" [label=\"{}\"];\n", n.key, n.formula));
        }
        for e in &self.edges {
            s.push_str(&format!("    \"{}\" -> \"{}\" [label=\"{}\"];\n", e.from, e.to, e.reaction.replace('"', "'")));
        }
        s.push_str("}\n");
        s
    }
}

impl Default for ReactionNetwork {
    fn default() -> Self { Self::new() }
}

// ============================================================
// 辅助函数
// ============================================================

/// 从分子式快速构建简单分子（用于测试和 O2/H2O 等常用品）
pub fn build_simple_molecule(formula: &str) -> Option<Molecule> {
    let mut mol = Molecule::new();
    mol.name = Some(formula.to_string());
    match formula {
        "H2O" => {
            let o = mol.add_atom(Element::O);
            let h1 = mol.add_atom(Element::H);
            let h2 = mol.add_atom(Element::H);
            mol.add_bond(o, h1, BondOrder::Single);
            mol.add_bond(o, h2, BondOrder::Single);
        }
        "O2" => {
            let o1 = mol.add_atom(Element::O);
            let o2 = mol.add_atom(Element::O);
            mol.add_bond(o1, o2, BondOrder::Double);
        }
        "H2" => {
            let h1 = mol.add_atom(Element::H);
            let h2 = mol.add_atom(Element::H);
            mol.add_bond(h1, h2, BondOrder::Single);
        }
        "N2" => {
            let n1 = mol.add_atom(Element::N);
            let n2 = mol.add_atom(Element::N);
            mol.add_bond(n1, n2, BondOrder::Triple);
        }
        "CO2" => {
            let c = mol.add_atom(Element::C);
            let o1 = mol.add_atom(Element::O);
            let o2 = mol.add_atom(Element::O);
            mol.add_bond(c, o1, BondOrder::Double);
            mol.add_bond(c, o2, BondOrder::Double);
        }
        "CH4" => {
            let c = mol.add_atom(Element::C);
            for _ in 0..4 {
                let h = mol.add_atom(Element::H);
                mol.add_bond(c, h, BondOrder::Single);
            }
        }
        "HCl" => {
            let h = mol.add_atom(Element::H);
            let cl = mol.add_atom(Element::Cl);
            mol.add_bond(h, cl, BondOrder::Single);
        }
        "NaOH" => {
            let na = mol.add_atom(Element::Na);
            let o = mol.add_atom(Element::O);
            let h = mol.add_atom(Element::H);
            mol.add_bond(na, o, BondOrder::Ionic);
            mol.add_bond(o, h, BondOrder::Single);
        }
        "NH3" => {
            let n = mol.add_atom(Element::N);
            for _ in 0..3 {
                let h = mol.add_atom(Element::H);
                mol.add_bond(n, h, BondOrder::Single);
            }
        }
        _ => return None,
    }
    Some(mol)
}

/// 解析分子式字符串为 Molecule（简化版，支持 C/H/O/N/Cl/S 等单字母+数字）
pub fn parse_formula(formula: &str) -> Option<Molecule> {
    if let Some(m) = build_simple_molecule(formula) {
        return Some(m);
    }
    let mut mol = Molecule::new();
    mol.name = Some(formula.to_string());
    let bytes = formula.as_bytes();
    let mut i = 0;
    // 解析元素+计数
    let mut elements: Vec<(Element, u32)> = Vec::new();
    while i < bytes.len() {
        let c = bytes[i] as char;
        if !c.is_ascii_uppercase() { i += 1; continue; }
        let sym = if i + 1 < bytes.len() && (bytes[i+1] as char).is_ascii_lowercase() {
            i += 2;
            format!("{}{}", c, bytes[i-1] as char)
        } else {
            i += 1;
            c.to_string()
        };
        let mut count = 0u32;
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            count = count * 10 + (bytes[i] - b'0') as u32;
            i += 1;
        }
        if count == 0 { count = 1; }
        // 查找元素
        if let Some(elem) = Element::from_symbol(&sym) {
            elements.push((elem, count));
        }
    }
    if elements.is_empty() { return None; }
    // 简化构建：只加原子，不连键（实际结构需更多信息）
    for (elem, count) in elements {
        for _ in 0..count {
            mol.add_atom(elem);
        }
    }
    Some(mol)
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictor_default() {
        let p = ReactionPredictor::default();
        assert!((p.temperature - 298.15).abs() < 1e-6);
        assert_eq!(p.max_bond_changes, 4);
    }

    #[test]
    fn test_combustion_methane() {
        let p = ReactionPredictor::default();
        let ch4 = build_simple_molecule("CH4").unwrap();
        let o2 = build_simple_molecule("O2").unwrap();
        let results = p.predict(&ch4, &o2);
        // 应至少有一个燃烧反应
        let has_combustion = results.iter().any(|r| r.mechanism == ReactionMechanism::Combustion);
        assert!(has_combustion, "应预测到燃烧反应");
        let comb = results.iter().find(|r| r.mechanism == ReactionMechanism::Combustion).unwrap();
        // 甲烷燃烧 ΔH ≈ -890 kJ/mol
        assert!(comb.delta_h < -500.0, "燃烧应强放热，got ΔH={}", comb.delta_h);
        // 守恒
        assert!(comb.conservation_ok, "燃烧必须守恒");
    }

    #[test]
    fn test_acid_base() {
        let p = ReactionPredictor::default();
        let hcl = build_simple_molecule("HCl").unwrap();
        let naoh = build_simple_molecule("NaOH").unwrap();
        if let Some(ab) = p.predict_acid_base(&hcl, &naoh) {
            assert_eq!(ab.mechanism, ReactionMechanism::AcidBase);
            // 酸碱反应活化能低
            assert!(ab.activation_energy < 30.0);
        }
    }

    #[test]
    fn test_decomposition() {
        let p = ReactionPredictor::default();
        let h2o = build_simple_molecule("H2O").unwrap();
        let decomps = p.predict_decomposition(&h2o);
        // 水应能分解为 H + OH
        assert!(!decomps.is_empty(), "水应能分解");
        for d in &decomps {
            assert!(d.conservation_ok, "分解必须守恒");
        }
    }

    #[test]
    fn test_conservation_verification() {
        let p = ReactionPredictor::default();
        let ch4 = build_simple_molecule("CH4").unwrap();
        let comb = p.predict_combustion(&ch4).unwrap();
        assert!(p.verify_conservation(&comb), "燃烧必须守恒");
    }

    #[test]
    fn test_reaction_network() {
        let p = ReactionPredictor::default();
        let ch4 = build_simple_molecule("CH4").unwrap();
        let o2 = build_simple_molecule("O2").unwrap();
        let net = p.build_reaction_network(&[ch4, o2], 2);
        assert!(net.node_count() > 0);
        assert!(net.edge_count() > 0);
        // DOT 输出可读
        let dot = net.to_dot();
        assert!(dot.contains("digraph"));
    }

    #[test]
    fn test_parse_formula() {
        let mol = parse_formula("H2O").unwrap();
        assert_eq!(mol.atom_count(), 3);
        let mol2 = parse_formula("CH4").unwrap();
        assert_eq!(mol2.atom_count(), 5);
    }

    #[test]
    fn test_equation_string() {
        let p = ReactionPredictor::default();
        let ch4 = build_simple_molecule("CH4").unwrap();
        if let Some(c) = p.predict_combustion(&ch4) {
            let s = c.equation();
            assert!(s.contains("Combustion"));
            assert!(s.contains("ΔH"));
        }
    }
}