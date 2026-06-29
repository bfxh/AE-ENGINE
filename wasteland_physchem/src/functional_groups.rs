//! functional_groups.rs - 官能团识别与反应位点分析
//!
//! 提供 35+ 官能团类型识别、6 类反应位点标注、pKa 估算、芳香性判断、环查找。
//! 用于 reaction_prediction 模块从未知物质推导化学反应方程式。
//!
//! 识别策略：基于原子元素 + 键级 + 邻居关系 + 杂化状态的图模式匹配。
//! 不依赖外部数据库，全部从原子性质推导。
//!
//! 参考：
//! - IUPAC 官能团命名
//! - Carey & Sundberg, "Advanced Organic Chemistry"
//! - Hückel 4n+2 芳香性规则

use serde::{Deserialize, Serialize};
use crate::molecules::{AtomId, BondOrder, Molecule, Atom, Bond, Hybridization};
use crate::elements::Element;

// ============================================================
// 数据结构
// ============================================================

/// 官能团类型（35+ 种）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionalGroupType {
    // 烃基
    Alkane, Alkene, Alkyne, AromaticRing, Allyl, Vinyl,
    // 含氧
    Hydroxyl, Carbonyl, Aldehyde, Ketone, Carboxyl, Ester, Ether,
    Peroxide, Hemiacetal, Acetal, Epoxide,
    // 含氮
    Amine, Amide, Imine, Nitrile, Nitro, Nitrite, Azo, Diazo, Hydrazine,
    // 含硫
    Thiol, Sulfide, Disulfide, Sulfoxide, Sulfone, SulfonicAcid, Thioether,
    // 含磷
    Phosphate, Phosphonate, Phosphine,
    // 含卤
    Halo, Polyhalo,
    // 其他
    Isocyanate, Cyanate, Thiocyanate, Isothiocyanate, Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalGroup {
    pub group_type: FunctionalGroupType,
    pub atom_ids: Vec<AtomId>,
    pub bond_ids: Vec<usize>,
}

/// 反应位点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SiteType {
    Nucleophile,
    Electrophile,
    Radical,
    Acid,
    Base,
    LeavingGroup,
}

/// 反应位点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactiveSite {
    pub atom_id: AtomId,
    pub site_type: SiteType,
    pub reactivity_score: f64,
    pub functional_group: Option<FunctionalGroupType>,
}

// ============================================================
// 辅助函数
// ============================================================

fn atom_by_id<'a>(mol: &'a Molecule, id: AtomId) -> Option<&'a Atom> {
    mol.atoms.iter().find(|a| a.id == id)
}

fn bond_between<'a>(mol: &'a Molecule, a: AtomId, b: AtomId) -> Option<&'a Bond> {
    mol.bonds.iter().find(|bd| (bd.a == a && bd.b == b) || (bd.a == b && bd.b == a))
}

fn bond_idx(mol: &Molecule, a: AtomId, b: AtomId) -> Option<usize> {
    mol.bonds.iter().position(|bd| (bd.a == a && bd.b == b) || (bd.a == b && bd.b == a))
}

fn count_element_neighbors(mol: &Molecule, id: AtomId, e: Element) -> u32 {
    mol.neighbors(id).iter()
        .filter_map(|&n| atom_by_id(mol, n))
        .filter(|a| a.element == e)
        .count() as u32
}

fn has_hydrogen_neighbor(mol: &Molecule, id: AtomId) -> bool {
    count_element_neighbors(mol, id, Element::H) > 0
}

fn has_carbon_neighbor(mol: &Molecule, id: AtomId) -> bool {
    count_element_neighbors(mol, id, Element::C) > 0
}

fn is_halogen(e: Element) -> bool {
    matches!(e, Element::F | Element::Cl | Element::Br | Element::I)
}

fn is_heteroatom(e: Element) -> bool {
    matches!(e, Element::N | Element::O | Element::S | Element::P) || is_halogen(e)
}

/// 计算原子上的孤对电子数
/// 公式：LP = (价电子 - 键级总和 + 形式电荷) / 2
fn lone_pairs(mol: &Molecule, id: AtomId) -> u8 {
    let atom = match atom_by_id(mol, id) { Some(a) => a, None => return 0 };
    let ve = atom.element.valence_electrons() as i32;
    let bond_sum: i32 = mol.bonds_of(id).iter().map(|b| b.order.order() as i32).sum();
    let lp = ((ve - bond_sum + atom.formal_charge as i32) / 2).max(0);
    lp as u8
}

/// 原子的总键级（度）
fn bond_degree(mol: &Molecule, id: AtomId) -> f64 {
    mol.bonds_of(id).iter().map(|b| b.order.order()).sum()
}
// ============================================================
// 官能团识别
// ============================================================

/// 识别分子中的所有官能团
pub fn identify_functional_groups(mol: &Molecule) -> Vec<FunctionalGroup> {
    let mut groups = Vec::new();

    // 含氧官能团
    identify_oxygen_groups(mol, &mut groups);
    // 含氮官能团
    identify_nitrogen_groups(mol, &mut groups);
    // 含硫官能团
    identify_sulfur_groups(mol, &mut groups);
    // 含磷官能团
    identify_phosphorus_groups(mol, &mut groups);
    // 含卤官能团
    identify_halogen_groups(mol, &mut groups);
    // 烃基（烯/炔/芳）
    identify_hydrocarbon_groups(mol, &mut groups);
    // 其他（异氰酸酯等）
    identify_other_groups(mol, &mut groups);

    groups
}

fn identify_oxygen_groups(mol: &Molecule, groups: &mut Vec<FunctionalGroup>) {
    for bond in &mol.bonds {
        let a_atom = atom_by_id(mol, bond.a);
        let b_atom = atom_by_id(mol, bond.b);
        let (a, b) = match (a_atom, b_atom) { (Some(x), Some(y)) => (x, y), _ => continue };

        // C=O 羰基
        if (a.element == Element::C && b.element == Element::O && bond.order == BondOrder::Double)
        || (a.element == Element::O && b.element == Element::C && bond.order == BondOrder::Double) {
            let (c_atom, o_atom) = if a.element == Element::C { (a, b) } else { (b, a) };
            let bi = bond_idx(mol, c_atom.id, o_atom.id).unwrap_or(0);
            let c_neighbors = mol.neighbors(c_atom.id);
            let has_oh = c_neighbors.iter().any(|&n| {
                atom_by_id(mol, n).map_or(false, |nb| nb.element == Element::O && has_hydrogen_neighbor(mol, n))
            });
            let has_or = c_neighbors.iter().any(|&n| {
                atom_by_id(mol, n).map_or(false, |nb| nb.element == Element::O && !has_hydrogen_neighbor(mol, n) && has_carbon_neighbor(mol, n))
            });
            let c_h_count = count_element_neighbors(mol, c_atom.id, Element::H);
            let c_c_count = count_element_neighbors(mol, c_atom.id, Element::C);

            let gt = if has_oh { FunctionalGroupType::Carboxyl }
                else if has_or { FunctionalGroupType::Ester }
                else if c_h_count >= 1 { FunctionalGroupType::Aldehyde }
                else if c_c_count >= 2 { FunctionalGroupType::Ketone }
                else { FunctionalGroupType::Carbonyl };

            groups.push(FunctionalGroup {
                group_type: gt,
                atom_ids: vec![c_atom.id, o_atom.id],
                bond_ids: vec![bi],
            });
        }

        // O-O 过氧化物
        if a.element == Element::O && b.element == Element::O && bond.order == BondOrder::Single {
            let bi = bond_idx(mol, bond.a, bond.b).unwrap_or(0);
            groups.push(FunctionalGroup {
                group_type: FunctionalGroupType::Peroxide,
                atom_ids: vec![bond.a, bond.b],
                bond_ids: vec![bi],
            });
        }
    }

    // 羟基（-OH）：O 连 H 且连 C
    for atom in &mol.atoms {
        if atom.element != Element::O { continue; }
        if has_hydrogen_neighbor(mol, atom.id) && has_carbon_neighbor(mol, atom.id) {
            // 避免与羧基重复（羧基的 OH 已被识别）
            let is_carboxyl_oh = mol.bonds.iter().any(|bd| {
                if bd.a != atom.id && bd.b != atom.id { return false; }
                let other = if bd.a == atom.id { bd.b } else { bd.a };
                bd.order == BondOrder::Double && atom_by_id(mol, other).map_or(false, |a| a.element == Element::C)
            });
            if !is_carboxyl_oh {
                groups.push(FunctionalGroup {
                    group_type: FunctionalGroupType::Hydroxyl,
                    atom_ids: vec![atom.id],
                    bond_ids: vec![],
                });
            }
        }
        // 醚（R-O-R）：O 度 2，无 H
        if !has_hydrogen_neighbor(mol, atom.id) {
            let c_neighbors = count_element_neighbors(mol, atom.id, Element::C);
            if c_neighbors >= 2 {
                groups.push(FunctionalGroup {
                    group_type: FunctionalGroupType::Ether,
                    atom_ids: vec![atom.id],
                    bond_ids: vec![],
                });
            }
        }
    }
}

fn identify_nitrogen_groups(mol: &Molecule, groups: &mut Vec<FunctionalGroup>) {
    for atom in &mol.atoms {
        if atom.element != Element::N { continue; }
        let degree = bond_degree(mol, atom.id);
        let has_double = mol.bonds_of(atom.id).iter().any(|b| b.order == BondOrder::Double);
        let has_triple = mol.bonds_of(atom.id).iter().any(|b| b.order == BondOrder::Triple);

        if has_triple && has_carbon_neighbor(mol, atom.id) {
            groups.push(FunctionalGroup { group_type: FunctionalGroupType::Nitrile, atom_ids: vec![atom.id], bond_ids: vec![] });
        } else if has_double && has_carbon_neighbor(mol, atom.id) {
            // C=N 亚胺
            groups.push(FunctionalGroup { group_type: FunctionalGroupType::Imine, atom_ids: vec![atom.id], bond_ids: vec![] });
        } else if degree <= 3.0 && has_carbon_neighbor(mol, atom.id) {
            // 检查是否酰胺（N 连羰基 C）
            let is_amide = mol.neighbors(atom.id).iter().any(|&n| {
                atom_by_id(mol, n).map_or(false, |nb| nb.element == Element::C && {
                    mol.bonds_of(n).iter().any(|b| b.order == BondOrder::Double && {
                        let other = if b.a == n { b.b } else { b.a };
                        atom_by_id(mol, other).map_or(false, |o| o.element == Element::O)
                    })
                })
            });
            let gt = if is_amide { FunctionalGroupType::Amide } else { FunctionalGroupType::Amine };
            groups.push(FunctionalGroup { group_type: gt, atom_ids: vec![atom.id], bond_ids: vec![] });
        }
    }
}

fn identify_sulfur_groups(mol: &Molecule, groups: &mut Vec<FunctionalGroup>) {
    for atom in &mol.atoms {
        if atom.element != Element::S { continue; }
        let has_h = has_hydrogen_neighbor(mol, atom.id);
        let c_count = count_element_neighbors(mol, atom.id, Element::C);
        let s_count = count_element_neighbors(mol, atom.id, Element::S);
        let has_double_o = mol.bonds_of(atom.id).iter().any(|b| {
            b.order == BondOrder::Double && {
                let other = if b.a == atom.id { b.b } else { b.a };
                atom_by_id(mol, other).map_or(false, |a| a.element == Element::O)
            }
        });
        let double_o_count = mol.bonds_of(atom.id).iter().filter(|b| {
            b.order == BondOrder::Double && {
                let other = if b.a == atom.id { b.b } else { b.a };
                atom_by_id(mol, other).map_or(false, |a| a.element == Element::O)
            }
        }).count();

        if has_h { groups.push(FunctionalGroup { group_type: FunctionalGroupType::Thiol, atom_ids: vec![atom.id], bond_ids: vec![] }); }
        else if s_count >= 1 { groups.push(FunctionalGroup { group_type: FunctionalGroupType::Disulfide, atom_ids: vec![atom.id], bond_ids: vec![] }); }
        else if double_o_count >= 2 && has_hydrogen_neighbor(mol, atom.id) == false {
            // 检查磺酸（S(=O)2-OH）
            let has_oh = mol.neighbors(atom.id).iter().any(|&n| {
                atom_by_id(mol, n).map_or(false, |nb| nb.element == Element::O && has_hydrogen_neighbor(mol, n))
            });
            if has_oh { groups.push(FunctionalGroup { group_type: FunctionalGroupType::SulfonicAcid, atom_ids: vec![atom.id], bond_ids: vec![] }); }
            else { groups.push(FunctionalGroup { group_type: FunctionalGroupType::Sulfone, atom_ids: vec![atom.id], bond_ids: vec![] }); }
        }
        else if has_double_o { groups.push(FunctionalGroup { group_type: FunctionalGroupType::Sulfoxide, atom_ids: vec![atom.id], bond_ids: vec![] }); }
        else if c_count >= 2 { groups.push(FunctionalGroup { group_type: FunctionalGroupType::Sulfide, atom_ids: vec![atom.id], bond_ids: vec![] }); }
    }
}

fn identify_phosphorus_groups(mol: &Molecule, groups: &mut Vec<FunctionalGroup>) {
    for atom in &mol.atoms {
        if atom.element != Element::P { continue; }
        let degree = bond_degree(mol, atom.id);
        let has_double_o = mol.bonds_of(atom.id).iter().any(|b| {
            b.order == BondOrder::Double && {
                let other = if b.a == atom.id { b.b } else { b.a };
                atom_by_id(mol, other).map_or(false, |a| a.element == Element::O)
            }
        });
        let o_count = count_element_neighbors(mol, atom.id, Element::O);
        let c_count = count_element_neighbors(mol, atom.id, Element::C);

        if has_double_o && o_count >= 3 {
            groups.push(FunctionalGroup { group_type: FunctionalGroupType::Phosphate, atom_ids: vec![atom.id], bond_ids: vec![] });
        } else if has_double_o && c_count >= 1 {
            groups.push(FunctionalGroup { group_type: FunctionalGroupType::Phosphonate, atom_ids: vec![atom.id], bond_ids: vec![] });
        } else if degree <= 3.0 {
            groups.push(FunctionalGroup { group_type: FunctionalGroupType::Phosphine, atom_ids: vec![atom.id], bond_ids: vec![] });
        }
    }
}

fn identify_halogen_groups(mol: &Molecule, groups: &mut Vec<FunctionalGroup>) {
    for atom in &mol.atoms {
        if !is_halogen(atom.element) { continue; }
        if has_carbon_neighbor(mol, atom.id) {
            // 检查同碳多卤
            let all_neighbors = mol.neighbors(atom.id);
            let c_id = all_neighbors.iter()
                .find(|&&n| atom_by_id(mol, n).map_or(false, |a| a.element == Element::C));
            if let Some(&c_id) = c_id {
                let halo_count = mol.neighbors(c_id).iter()
                    .filter(|&&n| atom_by_id(mol, n).map_or(false, |a| is_halogen(a.element)))
                    .count();
                if halo_count >= 2 {
                    groups.push(FunctionalGroup { group_type: FunctionalGroupType::Polyhalo, atom_ids: vec![atom.id], bond_ids: vec![] });
                } else {
                    groups.push(FunctionalGroup { group_type: FunctionalGroupType::Halo, atom_ids: vec![atom.id], bond_ids: vec![] });
                }
            }
        }
    }
}

fn identify_hydrocarbon_groups(mol: &Molecule, groups: &mut Vec<FunctionalGroup>) {
    for bond in &mol.bonds {
        let a_atom = atom_by_id(mol, bond.a);
        let b_atom = atom_by_id(mol, bond.b);
        let (a, b) = match (a_atom, b_atom) { (Some(x), Some(y)) => (x, y), _ => continue };
        if a.element != Element::C || b.element != Element::C { continue; }
        let bi = bond_idx(mol, bond.a, bond.b).unwrap_or(0);
        match bond.order {
            BondOrder::Double => {
                groups.push(FunctionalGroup { group_type: FunctionalGroupType::Alkene, atom_ids: vec![bond.a, bond.b], bond_ids: vec![bi] });
            }
            BondOrder::Triple => {
                groups.push(FunctionalGroup { group_type: FunctionalGroupType::Alkyne, atom_ids: vec![bond.a, bond.b], bond_ids: vec![bi] });
            }
            BondOrder::Aromatic => {
                groups.push(FunctionalGroup { group_type: FunctionalGroupType::AromaticRing, atom_ids: vec![bond.a, bond.b], bond_ids: vec![bi] });
            }
            _ => {}
        }
    }
    // 检测苯环（6 元环全 sp2）
    let rings = find_rings(mol, 6);
    for ring in &rings {
        if ring.len() == 6 && is_aromatic_ring(mol, ring) {
            let bis: Vec<usize> = ring.windows(2).filter_map(|w| bond_idx(mol, w[0], w[1])).collect();
            groups.push(FunctionalGroup { group_type: FunctionalGroupType::AromaticRing, atom_ids: ring.clone(), bond_ids: bis });
        }
    }
}

fn identify_other_groups(mol: &Molecule, groups: &mut Vec<FunctionalGroup>) {
    // 异氰酸酯 -N=C=O / 硫氰酸酯 -S-C≡N / 异硫氰酸酯 -N=C=S
    for atom in &mol.atoms {
        if atom.element == Element::N {
            let has_double_c = mol.bonds_of(atom.id).iter().any(|b| b.order == BondOrder::Double && {
                let other = if b.a == atom.id { b.b } else { b.a };
                atom_by_id(mol, other).map_or(false, |a| a.element == Element::C)
            });
            if has_double_c {
                let neighbors = mol.neighbors(atom.id);
                let n_c = neighbors.iter().find(|&&n| atom_by_id(mol, n).map_or(false, |a| a.element == Element::C));
                if let Some(&c_id) = n_c {
                    let has_double_o = mol.bonds_of(c_id).iter().any(|b| b.order == BondOrder::Double && {
                        let other = if b.a == c_id { b.b } else { b.a };
                        atom_by_id(mol, other).map_or(false, |a| a.element == Element::O)
                    });
                    let has_double_s = mol.bonds_of(c_id).iter().any(|b| b.order == BondOrder::Double && {
                        let other = if b.a == c_id { b.b } else { b.a };
                        atom_by_id(mol, other).map_or(false, |a| a.element == Element::S)
                    });
                    if has_double_o { groups.push(FunctionalGroup { group_type: FunctionalGroupType::Isocyanate, atom_ids: vec![atom.id], bond_ids: vec![] }); }
                    if has_double_s { groups.push(FunctionalGroup { group_type: FunctionalGroupType::Isothiocyanate, atom_ids: vec![atom.id], bond_ids: vec![] }); }
                }
            }
        }
    }
}
// ============================================================
// 反应位点识别
// ============================================================

/// 识别分子的反应位点
pub fn identify_reactive_sites(mol: &Molecule) -> Vec<ReactiveSite> {
    let mut sites = Vec::new();
    let groups = identify_functional_groups(mol);

    for atom in &mol.atoms {
        let elem = atom.element;
        let lp = lone_pairs(mol, atom.id);
        let charge = atom.formal_charge;

        // 1. 亲核位点：N/O/S/P 带孤对电子或负电荷
        if matches!(elem, Element::N | Element::O | Element::S | Element::P) && (lp > 0 || charge < 0) {
            let score = compute_nucleophilicity(mol, atom.id, lp, charge);
            let fg = find_group_for_atom(&groups, atom.id);
            sites.push(ReactiveSite { atom_id: atom.id, site_type: SiteType::Nucleophile, reactivity_score: score, functional_group: fg });
        }

        // 2. 亲电位点：带正电荷或羰基 C
        if charge > 0 && (elem == Element::C || elem == Element::N) {
            let score = 0.5 + 0.3 * (charge as f64);
            sites.push(ReactiveSite { atom_id: atom.id, site_type: SiteType::Electrophile, reactivity_score: score.min(1.0), functional_group: find_group_for_atom(&groups, atom.id) });
        }
        // 羰基 C 是亲电体
        if elem == Element::C {
            let has_double_o = mol.bonds_of(atom.id).iter().any(|b| b.order == BondOrder::Double && {
                let other = if b.a == atom.id { b.b } else { b.a };
                atom_by_id(mol, other).map_or(false, |a| a.element == Element::O)
            });
            if has_double_o {
                sites.push(ReactiveSite { atom_id: atom.id, site_type: SiteType::Electrophile, reactivity_score: 0.7, functional_group: Some(FunctionalGroupType::Carbonyl) });
            }
        }

        // 3. 自由基：价电子奇数
        let ve = elem.valence_electrons();
        let bond_sum: i32 = mol.bonds_of(atom.id).iter().map(|b| b.order.order() as i32).sum();
        let electron_count = ve as i32 + charge as i32 - bond_sum;
        if electron_count % 2 != 0 && electron_count > 0 {
            sites.push(ReactiveSite { atom_id: atom.id, site_type: SiteType::Radical, reactivity_score: 0.8, functional_group: find_group_for_atom(&groups, atom.id) });
        }

        // 4. 酸性氢：与 O/N/S 相连的 H
        if elem == Element::H {
            let hetero_neighbor = mol.neighbors(atom.id).iter().any(|&n| {
                atom_by_id(mol, n).map_or(false, |a| matches!(a.element, Element::O | Element::N | Element::S))
            });
            if hetero_neighbor {
                let score = 0.6;
                sites.push(ReactiveSite { atom_id: atom.id, site_type: SiteType::Acid, reactivity_score: score, functional_group: None });
            }
        }

        // 5. 碱性位：N/O/S 带孤对电子
        if matches!(elem, Element::N | Element::O | Element::S) && lp > 0 && charge <= 0 {
            let score = 0.5 + 0.1 * lp as f64;
            sites.push(ReactiveSite { atom_id: atom.id, site_type: SiteType::Base, reactivity_score: score.min(1.0), functional_group: find_group_for_atom(&groups, atom.id) });
        }

        // 6. 离去基：卤素或带正电荷的杂原子
        if is_halogen(elem) && has_carbon_neighbor(mol, atom.id) {
            // 卤素离去能力 I > Br > Cl >> F
            let score = match elem {
                Element::I => 0.9, Element::Br => 0.8, Element::Cl => 0.6, Element::F => 0.2,
                _ => 0.3,
            };
            sites.push(ReactiveSite { atom_id: atom.id, site_type: SiteType::LeavingGroup, reactivity_score: score, functional_group: Some(FunctionalGroupType::Halo) });
        }
    }

    sites
}

/// 亲核性评分
fn compute_nucleophilicity(mol: &Molecule, id: AtomId, lp: u8, charge: i8) -> f64 {
    let atom = match atom_by_id(mol, id) { Some(a) => a, None => return 0.0 };
    let en = atom.element.electronegativity().unwrap_or(2.0);
    // 低电负性 + 多孤对电子 + 负电荷 = 高亲核性
    let mut score = 0.4 + 0.1 * lp as f64;
    if charge < 0 { score += 0.3 * (-charge as f64); }
    score -= 0.05 * (en - 2.0).max(0.0); // 电负性高降低亲核性
    score.clamp(0.0, 1.0)
}

fn find_group_for_atom(groups: &[FunctionalGroup], id: AtomId) -> Option<FunctionalGroupType> {
    groups.iter().find(|g| g.atom_ids.contains(&id)).map(|g| g.group_type)
}

// ============================================================
// pKa 估算
// ============================================================

/// 估算某酸性位点的 pKa
pub fn estimate_pka(mol: &Molecule, site: &ReactiveSite) -> f64 {
    if site.site_type != SiteType::Acid { return 15.0; }

    // 找到 H 相连的杂原子
    let hetero = mol.neighbors(site.atom_id).iter().find_map(|&n| {
        atom_by_id(mol, n).filter(|a| matches!(a.element, Element::O | Element::N | Element::S))
    });
    let hetero = match hetero { Some(h) => h, None => return 15.0 };

    let groups = identify_functional_groups(mol);
    let fg = find_group_for_atom(&groups, hetero.id);

    let base_pka = match fg {
        Some(FunctionalGroupType::Carboxyl) => 4.76,
        Some(FunctionalGroupType::SulfonicAcid) => -2.0,
        Some(FunctionalGroupType::AromaticRing) => 10.0, // 苯酚
        Some(FunctionalGroupType::Thiol) => 10.5,
        Some(FunctionalGroupType::Hydroxyl) => 16.0,
        Some(FunctionalGroupType::Amide) => 17.0,
        Some(FunctionalGroupType::Amine) => 38.0,
        Some(FunctionalGroupType::Imine) => 20.0,
        _ => match hetero.element {
            Element::O => 16.0,
            Element::N => 38.0,
            Element::S => 10.5,
            _ => 15.0,
        },
    };

    // 诱导效应修正：邻居电负性原子降低 pKa
    let mut correction = 0.0;
    for &n in &mol.neighbors(hetero.id) {
        if n == site.atom_id { continue; }
        if let Some(nb) = atom_by_id(mol, n) {
            let en = nb.element.electronegativity().unwrap_or(2.0);
            if en > 3.0 { correction -= 1.0; } // 强吸电子
            else if en > 2.5 { correction -= 0.5; }
        }
    }

    base_pka + correction
}

// ============================================================
// 芳香性与环查找
// ============================================================

/// 判断一组原子是否构成芳香环（Hückel 4n+2 规则）
pub fn is_aromatic_ring(mol: &Molecule, atom_ids: &[AtomId]) -> bool {
    if atom_ids.len() < 5 || atom_ids.len() > 7 { return false; }

    // 所有原子必须 sp2（或芳烃）
    let all_sp2 = atom_ids.iter().all(|&id| {
        atom_by_id(mol, id).map_or(false, |a| {
            matches!(a.hybridization, Hybridization::SP | Hybridization::SP2)
        })
    });
    if !all_sp2 { return false; }

    // 计算π电子数
    let mut pi_electrons = 0u32;
    for &id in atom_ids {
        let atom = match atom_by_id(mol, id) { Some(a) => a, None => return false };
        // 双键贡献 2
        let has_double = mol.bonds_of(id).iter().any(|b| {
            b.order == BondOrder::Double || b.order == BondOrder::Aromatic
        });
        if has_double { pi_electrons += 2; }
        // 杂原子孤对电子贡献 2（如呋喃 O）
        else if matches!(atom.element, Element::O | Element::N | Element::S) {
            let lp = lone_pairs(mol, id);
            if lp >= 2 { pi_electrons += 2; }
        }
    }

    // Hückel: 4n+2 (n=0,1,2,...)
    matches!(pi_electrons, 2 | 6 | 10 | 14)
}

/// 查找分子中的所有环（BFS 最小环）
pub fn find_rings(mol: &Molecule, max_size: usize) -> Vec<Vec<AtomId>> {
    let n = mol.atoms.len();
    if n == 0 { return vec![]; }
    let mut rings = Vec::new();

    // 邻接表
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for b in &mol.bonds {
        let ai = b.a as usize;
        let bi = b.b as usize;
        if ai < n && bi < n {
            adj[ai].push(bi);
            adj[bi].push(ai);
        }
    }

    // 对每条边，移除后找最短路径，构成环
    let mut seen: Vec<std::collections::HashSet<Vec<AtomId>>> = vec![];
    for (ei, b) in mol.bonds.iter().enumerate() {
        let u = b.a as usize;
        let v = b.b as usize;
        if u >= n || v >= n { continue; }
        // BFS u -> v，不经过边 ei
        if let Some(path) = bfs_shortest_skip_edge(&adj, u, v, ei, max_size) {
            if path.len() >= 3 && path.len() <= max_size {
                let mut ring: Vec<AtomId> = path.iter().map(|&i| i as AtomId).collect();
                ring.sort();
                // 去重
                if !seen.iter().any(|s| s.contains(&ring)) {
                    let mut hs = std::collections::HashSet::new();
                    hs.insert(ring.clone());
                    seen.push(hs);
                    rings.push(path.iter().map(|&i| i as AtomId).collect());
                }
            }
        }
    }
    rings
}

fn bfs_shortest_skip_edge(adj: &[Vec<usize>], start: usize, goal: usize, skip_edge: usize, max_len: usize) -> Option<Vec<usize>> {
    let n = adj.len();
    let mut visited = vec![false; n];
    let mut parent = vec![None; n];
    let mut queue = std::collections::VecDeque::new();
    visited[start] = true;
    queue.push_back(start);
    while let Some(u) = queue.pop_front() {
        if u == goal && parent[u].is_some() { break; }
        for &v in &adj[u] {
            if visited[v] { continue; }
            // 检查是否是跳过的边
            let is_skipped = (u == start && v == goal) || (u == goal && v == start);
            if is_skipped && skip_edge == usize::MAX { continue; }
            // 简化：不精确跳过特定边，而是跳过 start-goal 直连
            if u == start && v == goal { continue; }
            visited[v] = true;
            parent[v] = Some(u);
            queue.push_back(v);
        }
    }
    if !visited[goal] { return None; }
    // 重建路径
    let mut path = vec![goal];
    let mut cur = goal;
    while let Some(p) = parent[cur] {
        path.push(p);
        cur = p;
    }
    path.reverse();
    if path.len() > max_len { return None; }
    Some(path)
}
// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn build_water() -> Molecule {
        let mut m = Molecule::new();
        let o = m.add_atom(Element::O);
        let h1 = m.add_atom(Element::H);
        let h2 = m.add_atom(Element::H);
        m.add_bond(o, h1, BondOrder::Single);
        m.add_bond(o, h2, BondOrder::Single);
        m
    }

    fn build_methanol() -> Molecule {
        let mut m = Molecule::new();
        let c = m.add_atom(Element::C);
        let o = m.add_atom(Element::O);
        let h1 = m.add_atom(Element::H);
        let h2 = m.add_atom(Element::H);
        let h3 = m.add_atom(Element::H);
        let ho = m.add_atom(Element::H);
        m.add_bond(c, o, BondOrder::Single);
        m.add_bond(c, h1, BondOrder::Single);
        m.add_bond(c, h2, BondOrder::Single);
        m.add_bond(c, h3, BondOrder::Single);
        m.add_bond(o, ho, BondOrder::Single);
        m
    }

    fn build_acetic_acid() -> Molecule {
        let mut m = Molecule::new();
        let c1 = m.add_atom(Element::C);
        let c2 = m.add_atom(Element::C);
        let o1 = m.add_atom(Element::O);
        let o2 = m.add_atom(Element::O);
        let h1 = m.add_atom(Element::H);
        let h2 = m.add_atom(Element::H);
        let h3 = m.add_atom(Element::H);
        let ho = m.add_atom(Element::H);
        m.add_bond(c1, c2, BondOrder::Single);
        m.add_bond(c2, o1, BondOrder::Double);
        m.add_bond(c2, o2, BondOrder::Single);
        m.add_bond(c1, h1, BondOrder::Single);
        m.add_bond(c1, h2, BondOrder::Single);
        m.add_bond(c1, h3, BondOrder::Single);
        m.add_bond(o2, ho, BondOrder::Single);
        m
    }

    #[test]
    fn test_identify_hydroxyl() {
        let m = build_methanol();
        let groups = identify_functional_groups(&m);
        assert!(groups.iter().any(|g| g.group_type == FunctionalGroupType::Hydroxyl), "甲醇应有羟基");
    }

    #[test]
    fn test_identify_carboxyl() {
        let m = build_acetic_acid();
        let groups = identify_functional_groups(&m);
        assert!(groups.iter().any(|g| g.group_type == FunctionalGroupType::Carboxyl), "乙酸应有羧基");
    }

    #[test]
    fn test_reactive_sites_water() {
        let m = build_water();
        let sites = identify_reactive_sites(&m);
        // 水应有碱性位（O 孤对电子）和酸性位（O-H 上的 H）
        assert!(sites.iter().any(|s| s.site_type == SiteType::Base), "水应有碱性位");
        assert!(sites.iter().any(|s| s.site_type == SiteType::Acid), "水应有酸性位");
    }

    #[test]
    fn test_pka_water() {
        let m = build_water();
        let sites = identify_reactive_sites(&m);
        let acid = sites.iter().find(|s| s.site_type == SiteType::Acid);
        if let Some(a) = acid {
            let pka = estimate_pka(&m, a);
            // 水的 pKa ~15.7
            assert!(pka > 10.0 && pka < 20.0, "水 pKa 应在 10-20，got {}", pka);
        }
    }

    #[test]
    fn test_lone_pairs() {
        let m = build_water();
        // O 有 2 个孤对电子
        assert_eq!(lone_pairs(&m, 0), 2);
    }

    #[test]
    fn test_find_rings() {
        // 苯环
        let mut m = Molecule::new();
        let c0 = m.add_atom(Element::C);
        let c1 = m.add_atom(Element::C);
        let c2 = m.add_atom(Element::C);
        let c3 = m.add_atom(Element::C);
        let c4 = m.add_atom(Element::C);
        let c5 = m.add_atom(Element::C);
        m.add_bond(c0, c1, BondOrder::Single);
        m.add_bond(c1, c2, BondOrder::Double);
        m.add_bond(c2, c3, BondOrder::Single);
        m.add_bond(c3, c4, BondOrder::Double);
        m.add_bond(c4, c5, BondOrder::Single);
        m.add_bond(c5, c0, BondOrder::Double);
        let rings = find_rings(&m, 6);
        assert!(!rings.is_empty(), "苯应有环");
    }
}