use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionRule {
    pub id: u64,
    pub name: String,
    pub reactant_smarts: Vec<String>,
    pub product_smarts: Vec<String>,
    pub delta_h: f32,
    pub activation_energy: f32,
    pub temp_range: (f32, f32),
    pub pressure_range: (f32, f32),
    pub ph_range: (f32, f32),
    pub catalyst_smarts: Vec<String>,
    pub solvent_smarts: Vec<String>,
    pub reaction_type: ReactionRuleType,
    pub hazard_flags: u32,
    pub functional_groups: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReactionRuleType {
    AcidBase,
    OxidationReduction,
    Combustion,
    Precipitation,
    Complexation,
    Polymerization,
    Decomposition,
    Substitution,
    Addition,
    Elimination,
    Rearrangement,
    Biochemical,
    Photochemical,
    Electrochemical,
    Nuclear,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HazardFlags: u32 {
        const NONE = 0;
        const TOXIC = 1 << 0;
        const CORROSIVE = 1 << 1;
        const FLAMMABLE = 1 << 2;
        const EXPLOSIVE = 1 << 3;
        const RADIOACTIVE = 1 << 4;
        const OXIDIZING = 1 << 5;
        const BIOHAZARD = 1 << 6;
        const ASPHYXIANT = 1 << 7;
        const CARCINOGENIC = 1 << 8;
        const MUTAGENIC = 1 << 9;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalGroup {
    pub id: u64,
    pub name: String,
    pub smarts: String,
    pub reactivity: f32,
    pub polarity: f32,
    pub acid_base: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Substance {
    pub id: u64,
    pub name: String,
    pub smiles: String,
    pub inchi: String,
    pub molecular_weight: f32,
    pub density: f32,
    pub melting_point: f32,
    pub boiling_point: f32,
    pub functional_groups: Vec<u64>,
    pub hazard_flags: u32,
    pub state_std: MatterState,
    pub solubility: f32,
    pub reactivity_index: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatterState {
    Solid,
    Liquid,
    Gas,
    Plasma,
    Supercritical,
    BoseEinsteinCondensate,
}

#[derive(Debug, Clone)]
pub struct ReactionMatcher {
    pub rules: Vec<ReactionRule>,
    pub substances: HashMap<u64, Substance>,
    pub functional_groups: HashMap<u64, FunctionalGroup>,
    pub group_index: HashMap<u64, Vec<usize>>,
    pub result_cache: lru::LruCache<(u64, u64, u64), Vec<MatchedReaction>>,
}

#[derive(Debug, Clone)]
pub struct MatchedReaction {
    pub rule_index: usize,
    pub confidence: f32,
    pub expected_products: Vec<String>,
    pub energy_released: f32,
    pub hazards: HazardFlags,
    pub rate_estimate: f32,
}

#[derive(Debug, Clone)]
pub struct ReactionConditions {
    pub temperature: f32,
    pub pressure: f32,
    pub ph: f32,
    pub catalysts: Vec<u64>,
    pub solvents: Vec<u64>,
}

impl Default for ReactionConditions {
    fn default() -> Self {
        Self {
            temperature: 298.15,
            pressure: 1.0,
            ph: 7.0,
            catalysts: Vec::new(),
            solvents: Vec::new(),
        }
    }
}

impl ReactionMatcher {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            substances: HashMap::new(),
            functional_groups: HashMap::new(),
            group_index: HashMap::new(),
            result_cache: lru::LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()),
        }
    }

    pub fn add_rule(&mut self, rule: ReactionRule) {
        let index = self.rules.len();

        for group_id in &rule.functional_groups {
            self.group_index.entry(*group_id).or_default().push(index);
        }

        self.rules.push(rule);
    }

    pub fn add_substance(&mut self, substance: Substance) {
        self.substances.insert(substance.id, substance);
    }

    pub fn add_functional_group(&mut self, group: FunctionalGroup) {
        self.group_index.entry(group.id).or_default();
        self.functional_groups.insert(group.id, group);
    }

    pub fn match_reactions(
        &mut self,
        substance_a: u64,
        substance_b: u64,
        conditions: &ReactionConditions,
    ) -> Vec<MatchedReaction> {
        let cache_key = (substance_a, substance_b, Self::hash_conditions(conditions));

        if let Some(cached) = self.result_cache.get(&cache_key) {
            return cached.clone();
        }

        let sub_a = match self.substances.get(&substance_a) {
            Some(s) => s,
            None => return Vec::new(),
        };
        let sub_b = match self.substances.get(&substance_b) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut candidate_indices = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for group_a in &sub_a.functional_groups {
            if let Some(indices) = self.group_index.get(group_a) {
                for &idx in indices {
                    if seen.insert(idx) {
                        candidate_indices.push(idx);
                    }
                }
            }
        }
        for group_b in &sub_b.functional_groups {
            if let Some(indices) = self.group_index.get(group_b) {
                for &idx in indices {
                    if seen.insert(idx) {
                        candidate_indices.push(idx);
                    }
                }
            }
        }

        let mut matches = Vec::new();

        for &idx in &candidate_indices {
            let rule = &self.rules[idx];

            if conditions.temperature < rule.temp_range.0
                || conditions.temperature > rule.temp_range.1
            {
                continue;
            }
            if conditions.pressure < rule.pressure_range.0
                || conditions.pressure > rule.pressure_range.1
            {
                continue;
            }
            if conditions.ph < rule.ph_range.0 || conditions.ph > rule.ph_range.1 {
                continue;
            }

            let has_groups_a =
                rule.functional_groups.iter().any(|g| sub_a.functional_groups.contains(g));
            let has_groups_b =
                rule.functional_groups.iter().any(|g| sub_b.functional_groups.contains(g));

            if !has_groups_a && !has_groups_b {
                continue;
            }

            let delta_g = Self::estimate_delta_g(rule, conditions.temperature);
            if delta_g >= 0.0 {
                continue;
            }

            let energy_available = conditions.temperature * 0.008314;
            let rate = if rule.activation_energy <= energy_available {
                (energy_available / rule.activation_energy.max(0.001)).min(1.0)
            } else {
                0.0
            };

            let confidence = (has_groups_a as u32 as f32 * 0.5 + has_groups_b as u32 as f32 * 0.5)
                * (-delta_g / 100.0).min(1.0)
                * rate;

            if confidence > 0.1 {
                let hazards = HazardFlags::from_bits_truncate(rule.hazard_flags);

                matches.push(MatchedReaction {
                    rule_index: idx,
                    confidence,
                    expected_products: rule.product_smarts.clone(),
                    energy_released: -delta_g,
                    hazards,
                    rate_estimate: rate,
                });
            }
        }

        matches.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
        });

        self.result_cache.put(cache_key, matches.clone());
        matches
    }

    fn estimate_delta_g(rule: &ReactionRule, temperature: f32) -> f32 {
        let delta_s = -0.1;
        rule.delta_h - temperature * delta_s
    }

    fn hash_conditions(conditions: &ReactionConditions) -> u64 {
        let mut hash: u64 = 0;
        hash ^= (conditions.temperature.to_bits() as u64).wrapping_mul(0x9E3779B97F4A7C15);
        hash ^= (conditions.pressure.to_bits() as u64).wrapping_mul(0x9E3779B97F4A7C15);
        hash ^= (conditions.ph.to_bits() as u64).wrapping_mul(0x9E3779B97F4A7C15);
        hash
    }

    pub fn get_substance(&self, id: u64) -> Option<&Substance> {
        self.substances.get(&id)
    }

    pub fn get_substance_by_name(&self, name: &str) -> Option<&Substance> {
        self.substances.values().find(|s| s.name.eq_ignore_ascii_case(name))
    }

    pub fn query_hazards(&self, substance_id: u64) -> HazardFlags {
        self.substances
            .get(&substance_id)
            .map(|s| HazardFlags::from_bits_truncate(s.hazard_flags))
            .unwrap_or(HazardFlags::NONE)
    }
}

impl Default for ReactionMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_substance_and_match() {
        let mut matcher = ReactionMatcher::new();

        let water = Substance {
            id: 1,
            name: "Water".to_string(),
            smiles: "O".to_string(),
            inchi: "InChI=1S/H2O/h1H2".to_string(),
            molecular_weight: 18.015,
            density: 1.0,
            melting_point: 273.15,
            boiling_point: 373.15,
            functional_groups: vec![1],
            hazard_flags: 0,
            state_std: MatterState::Liquid,
            solubility: 1.0,
            reactivity_index: 0.1,
        };

        let iron = Substance {
            id: 2,
            name: "Iron".to_string(),
            smiles: "[Fe]".to_string(),
            inchi: "InChI=1S/Fe".to_string(),
            molecular_weight: 55.845,
            density: 7.874,
            melting_point: 1811.0,
            boiling_point: 3134.0,
            functional_groups: vec![2],
            hazard_flags: 0,
            state_std: MatterState::Solid,
            solubility: 0.0,
            reactivity_index: 0.5,
        };

        matcher.add_substance(water);
        matcher.add_substance(iron);

        let rule = ReactionRule {
            id: 1,
            name: "Iron Rusting".to_string(),
            reactant_smarts: vec!["[Fe]".to_string(), "[O]".to_string()],
            product_smarts: vec!["[Fe2O3]".to_string()],
            delta_h: -824.2,
            activation_energy: 1.0,
            temp_range: (200.0, 2000.0),
            pressure_range: (0.1, 100.0),
            ph_range: (0.0, 14.0),
            catalyst_smarts: vec![],
            solvent_smarts: vec![],
            reaction_type: ReactionRuleType::OxidationReduction,
            hazard_flags: HazardFlags::CORROSIVE.bits(),
            functional_groups: vec![1, 2],
        };

        matcher.add_rule(rule);

        let conditions = ReactionConditions::default();
        let matches = matcher.match_reactions(1, 2, &conditions);

        assert!(!matches.is_empty());
        assert!(matches[0].confidence > 0.0);
        assert!(matches[0].hazards.contains(HazardFlags::CORROSIVE));
    }
}
