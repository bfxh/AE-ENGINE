//! molecules.rs - 分子图结构（占位实现）
//!
//! 提供原子节点 + 化学键边的分子图，以及 BDE 键能数据。
//! 完整官能团识别在 functional_groups.rs 中。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::elements::Element;

/// 原子 ID
pub type AtomId = u32;

/// 化学键级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondOrder {
    Single,
    Double,
    Triple,
    Aromatic,
    Ionic,
    Metallic,
    Hydrogen,
    Coordinate,
}

impl BondOrder {
    /// 数值键级（用于量子化学计算）
    pub fn order(&self) -> f64 {
        match self {
            BondOrder::Single => 1.0,
            BondOrder::Double => 2.0,
            BondOrder::Triple => 3.0,
            BondOrder::Aromatic => 1.5,
            BondOrder::Ionic => 0.0,
            BondOrder::Metallic => 0.0,
            BondOrder::Hydrogen => 0.1,
            BondOrder::Coordinate => 1.0,
        }
    }
}

/// 原子节点
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Atom {
    pub element: Element,
    pub id: AtomId,
    pub formal_charge: i8,
    pub position: [f64; 3], // 坐标 (Å)，默认 [0,0,0]
    pub hybridization: Hybridization,
}

impl Atom {
    pub fn new(id: AtomId, element: Element) -> Self {
        Self {
            element, id, formal_charge: 0,
            position: [0.0, 0.0, 0.0],
            hybridization: Hybridization::Unknown,
        }
    }
}

/// 杂化类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hybridization {
    SP, SP2, SP3, SP3D, SP3D2, Unknown,
}

/// 化学键
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bond {
    pub a: AtomId,
    pub b: AtomId,
    pub order: BondOrder,
    pub length_pm: f64,
    pub bde_kjmol: f64, // 键解离能 kJ/mol
}

impl Bond {
    pub fn new(a: AtomId, b: AtomId, order: BondOrder) -> Self {
        let (length_pm, bde_kjmol) = default_bond_params(a, b, order);
        Self { a, b, order, length_pm, bde_kjmol }
    }
}

/// 默认键长 (pm) 和键解离能 (kJ/mol)
fn default_bond_params(_a: AtomId, _b: AtomId, order: BondOrder) -> (f64, f64) {
    match order {
        BondOrder::Single => (154.0, 346.0),
        BondOrder::Double => (134.0, 614.0),
        BondOrder::Triple => (120.0, 839.0),
        BondOrder::Aromatic => (140.0, 505.0),
        BondOrder::Hydrogen => (180.0, 20.0),
        _ => (150.0, 300.0),
    }
}

/// 分子图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Molecule {
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    pub charge: i32,
    pub name: Option<String>,
}

impl Default for Molecule {
    fn default() -> Self {
        Self { atoms: vec![], bonds: vec![], charge: 0, name: None }
    }
}

impl Molecule {
    pub fn new() -> Self { Self::default() }

    pub fn add_atom(&mut self, element: Element) -> AtomId {
        let id = self.atoms.len() as AtomId;
        self.atoms.push(Atom::new(id, element));
        id
    }

    pub fn add_bond(&mut self, a: AtomId, b: AtomId, order: BondOrder) {
        self.bonds.push(Bond::new(a, b, order));
    }

    /// 分子式（Hill 顺序）
    pub fn molecular_formula(&self) -> String {
        let counts = self.atom_count_by_element();
        let mut elems: Vec<_> = counts.iter().collect();
        // Hill: C 优先，H 次之，其余按字母序
        elems.sort_by(|a, b| {
            match (a.0, b.0) {
                (Element::C, _) => std::cmp::Ordering::Less,
                (_, Element::C) => std::cmp::Ordering::Greater,
                (Element::H, _) => std::cmp::Ordering::Less,
                (_, Element::H) => std::cmp::Ordering::Greater,
                _ => a.0.symbol().cmp(b.0.symbol()),
            }
        });
        let mut s = String::new();
        for (e, &c) in elems {
            s.push_str(e.symbol());
            if c > 1 { s.push_str(&c.to_string()); }
        }
        s
    }

    /// 分子质量 (g/mol)
    pub fn molecular_mass(&self) -> f64 {
        self.atoms.iter().map(|a| a.element.atomic_mass()).sum()
    }

    /// 按元素统计原子数
    pub fn atom_count_by_element(&self) -> HashMap<Element, u32> {
        let mut m = HashMap::new();
        for a in &self.atoms {
            *m.entry(a.element).or_insert(0) += 1;
        }
        m
    }

    /// 返回某原子的所有键
    pub fn bonds_of(&self, id: AtomId) -> Vec<&Bond> {
        self.bonds.iter().filter(|b| b.a == id || b.b == id).collect()
    }

    /// 返回某原子的邻居
    pub fn neighbors(&self, id: AtomId) -> Vec<AtomId> {
        let mut ns: Vec<AtomId> = self.bonds.iter()
            .filter_map(|b| {
                if b.a == id { Some(b.b) }
                else if b.b == id { Some(b.a) }
                else { None }
            })
            .collect();
        ns.sort_unstable();
        ns.dedup();
        ns
    }

    /// 原子总数
    pub fn atom_count(&self) -> usize { self.atoms.len() }

    /// 键总数
    pub fn bond_count(&self) -> usize { self.bonds.len() }

    /// 估算总键能 (kJ/mol)
    pub fn total_bde(&self) -> f64 {
        self.bonds.iter().map(|b| b.bde_kjmol).sum()
    }
}

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

    #[test]
    fn test_water_formula() {
        let w = build_water();
        assert_eq!(w.molecular_formula(), "H2O");
    }

    #[test]
    fn test_water_mass() {
        let w = build_water();
        let expected = 2.0 * 1.008 + 15.999;
        assert!((w.molecular_mass() - expected).abs() < 1e-6);
    }

    #[test]
    fn test_neighbors() {
        let w = build_water();
        let o = 0u32;
        let ns = w.neighbors(o);
        assert_eq!(ns.len(), 2);
    }

    #[test]
    fn test_bond_order_value() {
        assert_eq!(BondOrder::Single.order(), 1.0);
        assert_eq!(BondOrder::Aromatic.order(), 1.5);
        assert_eq!(BondOrder::Triple.order(), 3.0);
    }
}
