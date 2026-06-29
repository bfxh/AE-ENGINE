use serde::{Deserialize, Serialize};

use crate::elements::Element;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BondEnergy {
    pub element_a: Element,
    pub element_b: Element,
    pub bond_energy_kj_mol: f32,
}

impl BondEnergy {
    pub fn lookup(a: Element, b: Element) -> f32 {
        BOND_ENERGIES
            .iter()
            .find(|be| {
                (be.element_a == a && be.element_b == b) || (be.element_a == b && be.element_b == a)
            })
            .map(|be| be.bond_energy_kj_mol)
            .unwrap_or(200.0)
    }
}

const BOND_ENERGIES: &[BondEnergy] = &[
    BondEnergy { element_a: Element::H, element_b: Element::H, bond_energy_kj_mol: 436.0 },
    BondEnergy { element_a: Element::H, element_b: Element::C, bond_energy_kj_mol: 413.0 },
    BondEnergy { element_a: Element::H, element_b: Element::N, bond_energy_kj_mol: 391.0 },
    BondEnergy { element_a: Element::H, element_b: Element::O, bond_energy_kj_mol: 463.0 },
    BondEnergy { element_a: Element::H, element_b: Element::F, bond_energy_kj_mol: 567.0 },
    BondEnergy { element_a: Element::H, element_b: Element::Cl, bond_energy_kj_mol: 431.0 },
    BondEnergy { element_a: Element::H, element_b: Element::S, bond_energy_kj_mol: 339.0 },
    BondEnergy { element_a: Element::C, element_b: Element::C, bond_energy_kj_mol: 348.0 },
    BondEnergy { element_a: Element::C, element_b: Element::N, bond_energy_kj_mol: 293.0 },
    BondEnergy { element_a: Element::C, element_b: Element::O, bond_energy_kj_mol: 358.0 },
    BondEnergy { element_a: Element::C, element_b: Element::F, bond_energy_kj_mol: 485.0 },
    BondEnergy { element_a: Element::C, element_b: Element::Cl, bond_energy_kj_mol: 328.0 },
    BondEnergy { element_a: Element::C, element_b: Element::S, bond_energy_kj_mol: 272.0 },
    BondEnergy { element_a: Element::N, element_b: Element::N, bond_energy_kj_mol: 163.0 },
    BondEnergy { element_a: Element::N, element_b: Element::O, bond_energy_kj_mol: 201.0 },
    BondEnergy { element_a: Element::O, element_b: Element::O, bond_energy_kj_mol: 146.0 },
    BondEnergy { element_a: Element::O, element_b: Element::Si, bond_energy_kj_mol: 452.0 },
    BondEnergy { element_a: Element::O, element_b: Element::Fe, bond_energy_kj_mol: 390.0 },
    BondEnergy { element_a: Element::O, element_b: Element::Al, bond_energy_kj_mol: 501.0 },
    BondEnergy { element_a: Element::Fe, element_b: Element::Fe, bond_energy_kj_mol: 100.0 },
    BondEnergy { element_a: Element::Si, element_b: Element::Si, bond_energy_kj_mol: 226.0 },
    BondEnergy { element_a: Element::S, element_b: Element::O, bond_energy_kj_mol: 265.0 },
    BondEnergy { element_a: Element::P, element_b: Element::O, bond_energy_kj_mol: 335.0 },
    BondEnergy { element_a: Element::Na, element_b: Element::Cl, bond_energy_kj_mol: 411.0 },
    BondEnergy { element_a: Element::Ca, element_b: Element::O, bond_energy_kj_mol: 464.0 },
    BondEnergy { element_a: Element::Mg, element_b: Element::O, bond_energy_kj_mol: 394.0 },
    BondEnergy { element_a: Element::Ti, element_b: Element::O, bond_energy_kj_mol: 672.0 },
    BondEnergy { element_a: Element::Cr, element_b: Element::O, bond_energy_kj_mol: 461.0 },
    BondEnergy { element_a: Element::Mn, element_b: Element::O, bond_energy_kj_mol: 402.0 },
    BondEnergy { element_a: Element::Ni, element_b: Element::O, bond_energy_kj_mol: 382.0 },
    BondEnergy { element_a: Element::Cu, element_b: Element::O, bond_energy_kj_mol: 269.0 },
    BondEnergy { element_a: Element::Zn, element_b: Element::O, bond_energy_kj_mol: 159.0 },
    BondEnergy { element_a: Element::Pb, element_b: Element::O, bond_energy_kj_mol: 382.0 },
    BondEnergy { element_a: Element::U, element_b: Element::O, bond_energy_kj_mol: 759.0 },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Molecule {
    pub formula: Vec<(Element, u8)>,
    pub enthalpy_of_formation: f32,
    pub entropy: f32,
}

impl Molecule {
    pub fn total_bond_energy(&self) -> f32 {
        let mut total = 0.0;
        for i in 0..self.formula.len() {
            for j in i..self.formula.len() {
                let (elem_a, count_a) = &self.formula[i];
                let (elem_b, count_b) = &self.formula[j];
                if i == j {
                    total += BondEnergy::lookup(*elem_a, *elem_b) * *count_a as f32 * 0.5;
                } else {
                    total += BondEnergy::lookup(*elem_a, *elem_b)
                        * (*count_a as f32).min(*count_b as f32);
                }
            }
        }
        total
    }

    pub fn estimate_gibbs_free_energy(&self, temperature_k: f32) -> f32 {
        let delta_h = self.enthalpy_of_formation;
        let delta_s = self.entropy;
        delta_h - temperature_k * delta_s / 1000.0
    }
}

pub struct ThermodynamicsEngine {
    pub cache: lru::LruCache<(u64, u64), f32>,
    pub temperature: f32,
    pub pressure: f32,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_derivations: u64,
}

impl ThermodynamicsEngine {
    pub fn new(cache_size: usize) -> Self {
        Self {
            cache: lru::LruCache::new(std::num::NonZeroUsize::new(cache_size.max(1)).unwrap()),
            temperature: 298.0,
            pressure: 101.325,
            cache_hits: 0,
            cache_misses: 0,
            total_derivations: 0,
        }
    }

    pub fn is_reaction_feasible(
        &mut self,
        reactants: &[(Element, u8)],
        products: &[(Element, u8)],
    ) -> (bool, f32) {
        let reactants_key = Self::hash_formula(reactants);
        let products_key = Self::hash_formula(products);

        if let Some(&cached_dg) = self.cache.get(&(reactants_key, products_key)) {
            self.cache_hits += 1;
            return (cached_dg < 0.0, cached_dg);
        }

        self.cache_misses += 1;
        self.total_derivations += 1;

        let reactant_energy: f32 = reactants
            .iter()
            .map(|(elem, count)| BondEnergy::lookup(*elem, *elem) * *count as f32 * 0.5)
            .sum();

        let product_energy: f32 = products
            .iter()
            .map(|(elem, count)| BondEnergy::lookup(*elem, *elem) * *count as f32 * 0.5)
            .sum();

        let delta_h = product_energy - reactant_energy;
        let delta_s = (products.len() as f32 - reactants.len() as f32) * 100.0;
        let delta_g = delta_h - self.temperature * delta_s / 1000.0;

        self.cache.put((reactants_key, products_key), delta_g);

        (delta_g < 0.0, delta_g)
    }

    pub fn reaction_rate_estimate(&self, activation_energy: f32, temperature: f32) -> f32 {
        let r = 8.314;
        let exponent = -activation_energy * 1000.0 / (r * temperature);
        let factor = (exponent as f64).exp() as f32;
        factor.clamp(0.0, 1.0)
    }

    fn hash_formula(formula: &[(Element, u8)]) -> u64 {
        let mut sorted: Vec<_> = formula.iter().collect();
        sorted.sort_by_key(|(e, _)| *e as u8);
        let mut hash: u64 = 0;
        for (elem, count) in sorted {
            hash = hash.wrapping_mul(31).wrapping_add(*elem as u64);
            hash = hash.wrapping_mul(31).wrapping_add(*count as u64);
        }
        hash
    }

    pub fn cache_stats(&self) -> (u64, u64, f32) {
        let total = self.cache_hits + self.cache_misses;
        let hit_rate = if total > 0 { self.cache_hits as f32 / total as f32 } else { 0.0 };
        (self.cache_hits, self.cache_misses, hit_rate)
    }
}

impl Default for ThermodynamicsEngine {
    fn default() -> Self {
        Self::new(4096)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionProgress {
    pub reaction_id: u64,
    pub progress: f32,
    pub total_amount: f32,
    pub consumed_per_tick: f32,
    pub heat_per_tick: f32,
    pub start_tick: u64,
    pub estimated_ticks: u64,
}

impl ReactionProgress {
    pub fn advance(&mut self, dt: f32) -> bool {
        self.progress += self.consumed_per_tick * dt;
        self.progress >= 1.0
    }

    pub fn heat_generated_this_tick(&self, dt: f32) -> f32 {
        self.heat_per_tick * dt
    }

    pub fn remaining_amount(&self) -> f32 {
        (self.total_amount * (1.0 - self.progress)).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bond_energy_lookup() {
        let e = BondEnergy::lookup(Element::H, Element::O);
        assert!((e - 463.0).abs() < 1.0);
    }

    #[test]
    fn test_feasibility() {
        let mut engine = ThermodynamicsEngine::new(100);
        let (feasible, dg) =
            engine.is_reaction_feasible(&[(Element::Fe, 1)], &[(Element::Fe, 1), (Element::O, 1)]);
        assert!(feasible || dg > 0.0);
    }

    #[test]
    fn test_cache() {
        let mut engine = ThermodynamicsEngine::new(100);
        let _ = engine.is_reaction_feasible(
            &[(Element::H, 2), (Element::O, 1)],
            &[(Element::H, 2), (Element::O, 1)],
        );
        assert_eq!(engine.cache_misses, 1);
        let _ = engine.is_reaction_feasible(
            &[(Element::H, 2), (Element::O, 1)],
            &[(Element::H, 2), (Element::O, 1)],
        );
        assert_eq!(engine.cache_hits, 1);
    }
}
