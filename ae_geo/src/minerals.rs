use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mineral {
    pub name: String,
    pub hardness: f32,
    pub density: f32,
    pub luster: f32,
    pub rarity: f32,
    pub cleavage_quality: f32,
    pub fracture_toughness: f32,
}

pub static MINERAL_QUARTZ: LazyLock<Mineral> = LazyLock::new(|| Mineral {
    name: "Quartz".to_string(),
    hardness: 7.0,
    density: 2650.0,
    luster: 0.5,
    rarity: 0.1,
    cleavage_quality: 0.1,
    fracture_toughness: 1.0,
});

pub static MINERAL_FELDSPAR: LazyLock<Mineral> = LazyLock::new(|| Mineral {
    name: "Feldspar".to_string(),
    hardness: 6.0,
    density: 2600.0,
    luster: 0.4,
    rarity: 0.15,
    cleavage_quality: 0.8,
    fracture_toughness: 0.6,
});

pub static MINERAL_CALCITE: LazyLock<Mineral> = LazyLock::new(|| Mineral {
    name: "Calcite".to_string(),
    hardness: 3.0,
    density: 2710.0,
    luster: 0.6,
    rarity: 0.2,
    cleavage_quality: 1.0,
    fracture_toughness: 0.3,
});

pub static MINERAL_MICA: LazyLock<Mineral> = LazyLock::new(|| Mineral {
    name: "Mica".to_string(),
    hardness: 2.5,
    density: 2800.0,
    luster: 0.9,
    rarity: 0.25,
    cleavage_quality: 1.0,
    fracture_toughness: 0.2,
});

pub static MINERAL_OLIVINE: LazyLock<Mineral> = LazyLock::new(|| Mineral {
    name: "Olivine".to_string(),
    hardness: 6.5,
    density: 3300.0,
    luster: 0.4,
    rarity: 0.08,
    cleavage_quality: 0.5,
    fracture_toughness: 0.8,
});

pub static MINERAL_PYRITE: LazyLock<Mineral> = LazyLock::new(|| Mineral {
    name: "Pyrite".to_string(),
    hardness: 6.0,
    density: 5000.0,
    luster: 0.95,
    rarity: 0.05,
    cleavage_quality: 0.2,
    fracture_toughness: 0.4,
});

pub static MINERAL_MAGNETITE: LazyLock<Mineral> = LazyLock::new(|| Mineral {
    name: "Magnetite".to_string(),
    hardness: 5.5,
    density: 5200.0,
    luster: 0.7,
    rarity: 0.04,
    cleavage_quality: 0.3,
    fracture_toughness: 0.5,
});

pub static MINERAL_GALENA: LazyLock<Mineral> = LazyLock::new(|| Mineral {
    name: "Galena".to_string(),
    hardness: 2.5,
    density: 7500.0,
    luster: 0.9,
    rarity: 0.03,
    cleavage_quality: 1.0,
    fracture_toughness: 0.1,
});

pub static ALL_MINERALS: LazyLock<Vec<Mineral>> = LazyLock::new(|| {
    vec![
        MINERAL_QUARTZ.clone(),
        MINERAL_FELDSPAR.clone(),
        MINERAL_CALCITE.clone(),
        MINERAL_MICA.clone(),
        MINERAL_OLIVINE.clone(),
        MINERAL_PYRITE.clone(),
        MINERAL_MAGNETITE.clone(),
        MINERAL_GALENA.clone(),
    ]
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OreDeposit {
    pub mineral: Mineral,
    pub grade: f32,
    pub volume: f32,
    pub depth: f32,
}

impl OreDeposit {
    pub fn extractable_mass(&self) -> f32 {
        self.volume * self.mineral.density * self.grade
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mineral_quartz() {
        let q = &*MINERAL_QUARTZ;
        assert_eq!(q.name, "Quartz");
        assert_eq!(q.hardness, 7.0);
        assert_eq!(q.density, 2650.0);
    }

    #[test]
    fn test_mineral_pyrite() {
        let p = &*MINERAL_PYRITE;
        assert_eq!(p.name, "Pyrite");
        assert_eq!(p.hardness, 6.0);
        assert_eq!(p.density, 5000.0);
        assert!(p.luster > 0.9);
    }

    #[test]
    fn test_ore_deposit() {
        let deposit = OreDeposit {
            mineral: MINERAL_MAGNETITE.clone(),
            grade: 0.6,
            volume: 100.0,
            depth: 50.0,
        };
        let mass = deposit.extractable_mass();
        assert_eq!(mass, 100.0 * 5200.0 * 0.6);
    }
}
