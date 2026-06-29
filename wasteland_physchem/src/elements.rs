//! elements.rs - 元素性质数据库（占位实现）
//!
//! 完整 NIST/IUPAC 数据待补充。本文件提供 3 个物理层模块所依赖的元素 API。
//! 常用元素（H-Kr, 1-36）提供精确数据，其余元素提供合理默认值。

use serde::{Deserialize, Serialize};

/// 118 元素枚举（原子序数即 discriminant）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum Element {
    H = 1, He,
    Li, Be, B, C, N, O, F, Ne,
    Na, Mg, Al, Si, P, S, Cl, Ar,
    K, Ca, Sc, Ti, V, Cr, Mn, Fe, Co, Ni, Cu, Zn,
    Ga, Ge, As, Se, Br, Kr,
    Rb, Sr, Y, Zr, Nb, Mo, Tc, Ru, Rh, Pd, Ag, Cd,
    In, Sn, Sb, Te, I, Xe,
    Cs, Ba, La, Ce, Pr, Nd, Pm, Sm, Eu, Gd, Tb, Dy, Ho, Er, Tm, Yb, Lu,
    Hf, Ta, W, Re, Os, Ir, Pt, Au, Hg, Tl, Pb, Bi, Po, At, Rn,
    Fr, Ra, Ac, Th, Pa, U, Np, Pu, Am, Cm, Bk, Cf, Es, Fm, Md, No, Lr,
    Rf, Db, Sg, Bh, Hs, Mt, Ds, Rg, Cn, Nh, Fl, Mc, Lv, Ts, Og,
}

impl Element {
    /// 从原子序数构造元素
    pub fn from_atomic_number(z: u8) -> Option<Self> {
        if (1..=118).contains(&z) {
            // SAFETY: Element is repr(u8) with discriminants 1..=118
            Some(unsafe { std::mem::transmute(z) })
        } else {
            None
        }
    }

    /// 从元素符号查找元素（如 "H" => H, "Fe" => Fe）
    pub fn from_symbol(sym: &str) -> Option<Self> {
        for z in 1..=118u8 {
            if let Some(e) = Self::from_atomic_number(z) {
                if e.symbol().eq_ignore_ascii_case(sym) {
                    return Some(e);
                }
            }
        }
        None
    }

    pub fn atomic_number(&self) -> u8 {
        *self as u8
    }

    /// 元素符号
    pub fn symbol(&self) -> &'static str {
        match self {
            Element::H => "H", Element::He => "He",
            Element::Li => "Li", Element::Be => "Be", Element::B => "B",
            Element::C => "C", Element::N => "N", Element::O => "O",
            Element::F => "F", Element::Ne => "Ne",
            Element::Na => "Na", Element::Mg => "Mg", Element::Al => "Al",
            Element::Si => "Si", Element::P => "P", Element::S => "S",
            Element::Cl => "Cl", Element::Ar => "Ar",
            Element::K => "K", Element::Ca => "Ca", Element::Sc => "Sc",
            Element::Ti => "Ti", Element::V => "V", Element::Cr => "Cr",
            Element::Mn => "Mn", Element::Fe => "Fe", Element::Co => "Co",
            Element::Ni => "Ni", Element::Cu => "Cu", Element::Zn => "Zn",
            Element::Ga => "Ga", Element::Ge => "Ge", Element::As => "As",
            Element::Se => "Se", Element::Br => "Br", Element::Kr => "Kr",
            Element::Rb => "Rb", Element::Sr => "Sr", Element::Y => "Y",
            Element::Zr => "Zr", Element::Nb => "Nb", Element::Mo => "Mo",
            Element::Tc => "Tc", Element::Ru => "Ru", Element::Rh => "Rh",
            Element::Pd => "Pd", Element::Ag => "Ag", Element::Cd => "Cd",
            Element::In => "In", Element::Sn => "Sn", Element::Sb => "Sb",
            Element::Te => "Te", Element::I => "I", Element::Xe => "Xe",
            Element::Cs => "Cs", Element::Ba => "Ba", Element::La => "La",
            Element::Ce => "Ce", Element::Pr => "Pr", Element::Nd => "Nd",
            Element::Pm => "Pm", Element::Sm => "Sm", Element::Eu => "Eu",
            Element::Gd => "Gd", Element::Tb => "Tb", Element::Dy => "Dy",
            Element::Ho => "Ho", Element::Er => "Er", Element::Tm => "Tm",
            Element::Yb => "Yb", Element::Lu => "Lu",
            Element::Hf => "Hf", Element::Ta => "Ta", Element::W => "W",
            Element::Re => "Re", Element::Os => "Os", Element::Ir => "Ir",
            Element::Pt => "Pt", Element::Au => "Au", Element::Hg => "Hg",
            Element::Tl => "Tl", Element::Pb => "Pb", Element::Bi => "Bi",
            Element::Po => "Po", Element::At => "At", Element::Rn => "Rn",
            Element::Fr => "Fr", Element::Ra => "Ra", Element::Ac => "Ac",
            Element::Th => "Th", Element::Pa => "Pa", Element::U => "U",
            Element::Np => "Np", Element::Pu => "Pu", Element::Am => "Am",
            Element::Cm => "Cm", Element::Bk => "Bk", Element::Cf => "Cf",
            Element::Es => "Es", Element::Fm => "Fm", Element::Md => "Md",
            Element::No => "No", Element::Lr => "Lr",
            Element::Rf => "Rf", Element::Db => "Db", Element::Sg => "Sg",
            Element::Bh => "Bh", Element::Hs => "Hs", Element::Mt => "Mt",
            Element::Ds => "Ds", Element::Rg => "Rg", Element::Cn => "Cn",
            Element::Nh => "Nh", Element::Fl => "Fl", Element::Mc => "Mc",
            Element::Lv => "Lv", Element::Ts => "Ts", Element::Og => "Og",
        }
    }

    /// 原子质量 (u, 即 g/mol)
    pub fn atomic_mass(&self) -> f64 {
        match self {
            Element::H => 1.008, Element::He => 4.0026,
            Element::Li => 6.94, Element::Be => 9.0122, Element::B => 10.81,
            Element::C => 12.011, Element::N => 14.007, Element::O => 15.999,
            Element::F => 18.998, Element::Ne => 20.180,
            Element::Na => 22.990, Element::Mg => 24.305, Element::Al => 26.982,
            Element::Si => 28.085, Element::P => 30.974, Element::S => 32.06,
            Element::Cl => 35.45, Element::Ar => 39.948,
            Element::K => 39.098, Element::Ca => 40.078, Element::Sc => 44.956,
            Element::Ti => 47.867, Element::V => 50.942, Element::Cr => 51.996,
            Element::Mn => 54.938, Element::Fe => 55.845, Element::Co => 58.933,
            Element::Ni => 58.693, Element::Cu => 63.546, Element::Zn => 65.38,
            Element::Ga => 69.723, Element::Ge => 72.630, Element::As => 74.922,
            Element::Se => 78.971, Element::Br => 79.904, Element::Kr => 83.798,
            Element::Rb => 85.468, Element::Sr => 87.62, Element::Y => 88.906,
            Element::Zr => 91.224, Element::Nb => 92.906, Element::Mo => 95.95,
            Element::Tc => 98.0, Element::Ru => 101.07, Element::Rh => 102.91,
            Element::Pd => 106.42, Element::Ag => 107.87, Element::Cd => 112.41,
            Element::In => 114.82, Element::Sn => 118.71, Element::Sb => 121.76,
            Element::Te => 127.60, Element::I => 126.90, Element::Xe => 131.29,
            Element::Cs => 132.91, Element::Ba => 137.33, Element::La => 138.91,
            Element::Ce => 140.12, Element::Pr => 140.91, Element::Nd => 144.24,
            Element::Pm => 145.0, Element::Sm => 150.36, Element::Eu => 151.96,
            Element::Gd => 157.25, Element::Tb => 158.93, Element::Dy => 162.50,
            Element::Ho => 164.93, Element::Er => 167.26, Element::Tm => 168.93,
            Element::Yb => 173.05, Element::Lu => 174.97,
            Element::Hf => 178.49, Element::Ta => 180.95, Element::W => 183.84,
            Element::Re => 186.21, Element::Os => 190.23, Element::Ir => 192.22,
            Element::Pt => 195.08, Element::Au => 196.97, Element::Hg => 200.59,
            Element::Tl => 204.38, Element::Pb => 207.2, Element::Bi => 208.98,
            Element::Po => 209.0, Element::At => 210.0, Element::Rn => 222.0,
            Element::Fr => 223.0, Element::Ra => 226.0, Element::Ac => 227.0,
            Element::Th => 232.04, Element::Pa => 231.04, Element::U => 238.03,
            _ => 240.0, // 超铀元素默认
        }
    }

    /// 电负性（Pauling 标度），稀有气体返回 None
    pub fn electronegativity(&self) -> Option<f64> {
        match self {
            Element::H => Some(2.20),
            Element::Li => Some(0.98), Element::Be => Some(1.57), Element::B => Some(2.04),
            Element::C => Some(2.55), Element::N => Some(3.04), Element::O => Some(3.44),
            Element::F => Some(3.98),
            Element::Na => Some(0.93), Element::Mg => Some(1.31), Element::Al => Some(1.61),
            Element::Si => Some(1.90), Element::P => Some(2.19), Element::S => Some(2.58),
            Element::Cl => Some(3.16),
            Element::K => Some(0.82), Element::Ca => Some(1.00), Element::Sc => Some(1.36),
            Element::Ti => Some(1.54), Element::V => Some(1.63), Element::Cr => Some(1.66),
            Element::Mn => Some(1.55), Element::Fe => Some(1.83), Element::Co => Some(1.88),
            Element::Ni => Some(1.91), Element::Cu => Some(1.90), Element::Zn => Some(1.65),
            Element::Ga => Some(1.81), Element::Ge => Some(2.01), Element::As => Some(2.18),
            Element::Se => Some(2.55), Element::Br => Some(2.96),
            Element::Rb => Some(0.82), Element::Sr => Some(0.95), Element::Y => Some(1.22),
            Element::Zr => Some(1.33), Element::Nb => Some(1.6), Element::Mo => Some(2.16),
            Element::Tc => Some(1.9), Element::Ru => Some(2.2), Element::Rh => Some(2.28),
            Element::Pd => Some(2.20), Element::Ag => Some(1.93), Element::Cd => Some(1.69),
            Element::In => Some(1.78), Element::Sn => Some(1.96), Element::Sb => Some(2.05),
            Element::Te => Some(2.1), Element::I => Some(2.66),
            Element::Cs => Some(0.79), Element::Ba => Some(0.89),
            Element::Au => Some(2.54), Element::Hg => Some(2.00),
            Element::Tl => Some(1.62), Element::Pb => Some(2.33), Element::Bi => Some(2.02),
            _ => None,
        }
    }

    /// 第一电离能 (eV)
    pub fn ionization_energy_ev(&self) -> f64 {
        match self {
            Element::H => 13.598, Element::He => 24.587,
            Element::Li => 5.392, Element::Be => 9.323, Element::B => 8.298,
            Element::C => 11.260, Element::N => 14.534, Element::O => 13.618,
            Element::F => 17.423, Element::Ne => 21.565,
            Element::Na => 5.139, Element::Mg => 7.646, Element::Al => 5.986,
            Element::Si => 8.152, Element::P => 10.487, Element::S => 10.360,
            Element::Cl => 12.968, Element::Ar => 15.760,
            Element::K => 4.341, Element::Ca => 6.113, Element::Sc => 6.561,
            Element::Ti => 6.828, Element::V => 6.746, Element::Cr => 6.767,
            Element::Mn => 7.434, Element::Fe => 7.902, Element::Co => 7.881,
            Element::Ni => 7.640, Element::Cu => 7.726, Element::Zn => 9.394,
            Element::Ga => 5.999, Element::Ge => 7.900, Element::As => 9.815,
            Element::Se => 9.752, Element::Br => 11.814, Element::Kr => 13.999,
            Element::Rb => 4.177, Element::Sr => 5.695, Element::Y => 6.217,
            Element::Zr => 6.634, Element::Nb => 6.759, Element::Mo => 7.092,
            Element::Tc => 7.28, Element::Ru => 7.361, Element::Rh => 7.459,
            Element::Pd => 8.337, Element::Ag => 7.576, Element::Cd => 8.994,
            Element::In => 5.786, Element::Sn => 7.344, Element::Sb => 8.608,
            Element::Te => 9.010, Element::I => 10.451, Element::Xe => 12.130,
            Element::Cs => 3.894, Element::Ba => 5.212,
            _ => 7.0,
        }
    }

    /// 电子亲和能 (eV)
    pub fn electron_affinity_ev(&self) -> f64 {
        match self {
            Element::H => 0.754, Element::He => -0.5,
            Element::Li => 0.618, Element::Be => -0.5, Element::B => 0.277,
            Element::C => 1.263, Element::N => -0.07, Element::O => 1.461,
            Element::F => 3.401, Element::Ne => -1.2,
            Element::Na => 0.548, Element::Mg => -0.4, Element::Al => 0.441,
            Element::Si => 1.385, Element::P => 0.747, Element::S => 2.077,
            Element::Cl => 3.617, Element::Ar => -1.0,
            Element::K => 0.502, Element::Ca => 0.024, Element::Sc => 0.188,
            Element::Ti => 0.079, Element::V => 0.525, Element::Cr => 0.666,
            Element::Mn => -0.5, Element::Fe => 0.153, Element::Co => 0.662,
            Element::Ni => 1.156, Element::Cu => 1.236, Element::Zn => -0.6,
            Element::Ga => 0.30, Element::Ge => 1.233, Element::As => 0.814,
            Element::Se => 2.021, Element::Br => 3.364, Element::Kr => -1.0,
            Element::I => 3.059,
            _ => 0.0,
        }
    }

    /// 共价半径 (pm)
    pub fn covalent_radius_pm(&self) -> f64 {
        match self {
            Element::H => 31.0, Element::He => 28.0,
            Element::Li => 128.0, Element::Be => 96.0, Element::B => 84.0,
            Element::C => 76.0, Element::N => 71.0, Element::O => 66.0,
            Element::F => 57.0, Element::Ne => 58.0,
            Element::Na => 166.0, Element::Mg => 141.0, Element::Al => 121.0,
            Element::Si => 111.0, Element::P => 107.0, Element::S => 105.0,
            Element::Cl => 102.0, Element::Ar => 106.0,
            Element::K => 203.0, Element::Ca => 176.0, Element::Sc => 170.0,
            Element::Ti => 160.0, Element::V => 153.0, Element::Cr => 139.0,
            Element::Mn => 139.0, Element::Fe => 132.0, Element::Co => 126.0,
            Element::Ni => 124.0, Element::Cu => 132.0, Element::Zn => 122.0,
            Element::Ga => 122.0, Element::Ge => 120.0, Element::As => 119.0,
            Element::Se => 120.0, Element::Br => 120.0, Element::Kr => 116.0,
            Element::I => 139.0,
            _ => 150.0,
        }
    }

    /// 氧化态
    pub fn oxidation_states(&self) -> &'static [i8] {
        match self {
            Element::H => &[1, -1],
            Element::O => &[-2, -1, 0, 1, 2],
            Element::C => &[-4, -3, -2, -1, 0, 1, 2, 3, 4],
            Element::N => &[-3, -2, -1, 0, 1, 2, 3, 4, 5],
            Element::S => &[-2, -1, 0, 2, 4, 6],
            Element::P => &[-3, -2, -1, 0, 1, 2, 3, 4, 5],
            Element::Cl => &[-1, 0, 1, 3, 4, 5, 7],
            Element::Na => &[1], Element::K => &[1],
            Element::Mg => &[2], Element::Ca => &[2],
            Element::Fe => &[2, 3, 6],
            Element::Cu => &[1, 2, 3],
            Element::Zn => &[2],
            Element::Al => &[3],
            _ => &[0],
        }
    }

    /// 价电子数（主族元素按族号，过渡元素按 d 电子）
    pub fn valence_electrons(&self) -> u8 {
        match self {
            Element::H | Element::Li | Element::Na | Element::K | Element::Rb | Element::Cs | Element::Fr => 1,
            Element::Be | Element::Mg | Element::Ca | Element::Sr | Element::Ba | Element::Ra => 2,
            Element::B | Element::Al | Element::Ga | Element::In | Element::Tl | Element::Nh => 3,
            Element::C | Element::Si | Element::Ge | Element::Sn | Element::Pb | Element::Fl => 4,
            Element::N | Element::P | Element::As | Element::Sb | Element::Bi | Element::Mc => 5,
            Element::O | Element::S | Element::Se | Element::Te | Element::Po | Element::Lv => 6,
            Element::F | Element::Cl | Element::Br | Element::I | Element::At | Element::Ts => 7,
            Element::He | Element::Ne | Element::Ar | Element::Kr | Element::Xe | Element::Rn | Element::Og => 8,
            _ => 2, // 过渡金属简化
        }
    }

    pub fn properties(&self) -> ElementProperties {
        ElementProperties {
            atomic_number: self.atomic_number(),
            atomic_mass: self.atomic_mass(),
            electronegativity: self.electronegativity(),
            covalent_radius_pm: self.covalent_radius_pm(),
            ionization_energy_ev: self.ionization_energy_ev(),
            electron_affinity_ev: self.electron_affinity_ev(),
            oxidation_states: self.oxidation_states().to_vec(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementProperties {
    pub atomic_number: u8,
    pub atomic_mass: f64,
    pub electronegativity: Option<f64>,
    pub covalent_radius_pm: f64,
    pub ionization_energy_ev: f64,
    pub electron_affinity_ev: f64,
    pub oxidation_states: Vec<i8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_atomic_number() {
        assert_eq!(Element::from_atomic_number(1), Some(Element::H));
        assert_eq!(Element::from_atomic_number(6), Some(Element::C));
        assert_eq!(Element::from_atomic_number(118), Some(Element::Og));
        assert_eq!(Element::from_atomic_number(0), None);
        assert_eq!(Element::from_atomic_number(119), None);
    }

    #[test]
    fn test_symbol_and_mass() {
        assert_eq!(Element::H.symbol(), "H");
        assert_eq!(Element::C.symbol(), "C");
        assert_eq!(Element::Au.symbol(), "Au");
        assert!((Element::C.atomic_mass() - 12.011).abs() < 1e-6);
        assert!((Element::H.atomic_mass() - 1.008).abs() < 1e-6);
    }

    #[test]
    fn test_electronegativity() {
        assert_eq!(Element::He.electronegativity(), None);
        assert!((Element::F.electronegativity().unwrap() - 3.98).abs() < 1e-6);
    }

    #[test]
    fn test_properties() {
        let p = Element::C.properties();
        assert_eq!(p.atomic_number, 6);
        assert!((p.atomic_mass - 12.011).abs() < 1e-6);
        assert!(p.electronegativity.is_some());
    }
}
