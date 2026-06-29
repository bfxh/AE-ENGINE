use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kinetics::KineticModel;
use crate::reactions::ChemicalReaction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalyst {
    pub id: Uuid,
    pub name: String,
    pub energy_barrier_reduction: f32,
    pub selectivity: f32,
    pub surface_area: f32,
    pub poison_level: f32,
    pub temperature_range: (f32, f32),
    pub active: bool,
    pub lifespan: f32,
    pub age: f32,
    pub regeneration_possible: bool,
    pub specificity: Vec<String>,
}

impl Catalyst {
    pub fn new(name: &str, energy_barrier_reduction: f32, selectivity: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            energy_barrier_reduction,
            selectivity,
            surface_area: 1.0,
            poison_level: 0.0,
            temperature_range: (200.0, 800.0),
            active: true,
            lifespan: 1000.0,
            age: 0.0,
            regeneration_possible: false,
            specificity: Vec::new(),
        }
    }

    pub fn with_surface_area(mut self, area: f32) -> Self {
        self.surface_area = area;
        self
    }

    pub fn with_temperature_range(mut self, min: f32, max: f32) -> Self {
        self.temperature_range = (min, max);
        self
    }

    pub fn with_lifespan(mut self, lifespan: f32) -> Self {
        self.lifespan = lifespan;
        self
    }

    pub fn with_regeneration(mut self) -> Self {
        self.regeneration_possible = true;
        self
    }

    pub fn with_specificity(mut self, targets: Vec<String>) -> Self {
        self.specificity = targets;
        self
    }

    pub fn is_poisoned(&self) -> bool {
        self.poison_level >= 1.0
    }

    pub fn is_in_temperature_range(&self, temperature: f32) -> bool {
        temperature >= self.temperature_range.0 && temperature <= self.temperature_range.1
    }

    pub fn efficiency(&self) -> f32 {
        if self.is_poisoned() || !self.active {
            return 0.0;
        }
        let age_factor = (1.0 - self.age / self.lifespan).max(0.0);
        let poison_factor = (1.0 - self.poison_level).max(0.0);
        self.surface_area * self.selectivity * age_factor * poison_factor
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalysisSystem {
    pub catalysts: Vec<Catalyst>,
}

impl CatalysisSystem {
    pub fn new() -> Self {
        Self { catalysts: Vec::new() }
    }

    pub fn add_catalyst(&mut self, catalyst: Catalyst) -> Uuid {
        let id = catalyst.id;
        self.catalysts.push(catalyst);
        id
    }

    pub fn remove_catalyst(&mut self, id: Uuid) -> bool {
        let len_before = self.catalysts.len();
        self.catalysts.retain(|c| c.id != id);
        self.catalysts.len() < len_before
    }

    pub fn get_catalyst(&self, id: Uuid) -> Option<&Catalyst> {
        self.catalysts.iter().find(|c| c.id == id)
    }

    pub fn get_catalyst_mut(&mut self, id: Uuid) -> Option<&mut Catalyst> {
        self.catalysts.iter_mut().find(|c| c.id == id)
    }

    pub fn apply_catalyst(&self, reaction: &ChemicalReaction, catalyst: &Catalyst) -> f32 {
        if catalyst.is_poisoned() || !catalyst.active {
            return 1.0;
        }

        let temp = reaction.conditions.min_temperature;
        if !catalyst.is_in_temperature_range(temp) {
            return 1.0;
        }

        let base_factor = 1.0 + catalyst.energy_barrier_reduction * catalyst.efficiency();
        base_factor.max(1.0)
    }

    pub fn calculate_modified_rate(
        &self,
        model: &KineticModel,
        reaction: &ChemicalReaction,
        catalyst: &Catalyst,
        temperature: f32,
    ) -> f32 {
        let rate = model.calculate_rate(&[1.0], temperature);
        let factor = self.apply_catalyst(reaction, catalyst);
        rate * factor
    }

    pub fn is_catalyst_poisoned(&self, id: Uuid) -> bool {
        self.get_catalyst(id).map(|c| c.is_poisoned()).unwrap_or(false)
    }

    pub fn poison_catalyst(&mut self, id: Uuid, amount: f32) -> bool {
        if let Some(catalyst) = self.get_catalyst_mut(id) {
            catalyst.poison_level = (catalyst.poison_level + amount).min(1.0);
            true
        } else {
            false
        }
    }

    pub fn regenerate_catalyst(&mut self, id: Uuid) -> bool {
        if let Some(catalyst) = self.get_catalyst_mut(id) {
            if catalyst.regeneration_possible {
                catalyst.poison_level = 0.0;
                catalyst.age = 0.0;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn age_catalysts(&mut self, dt: f32) {
        for catalyst in &mut self.catalysts {
            if catalyst.active {
                catalyst.age += dt;
                if catalyst.age >= catalyst.lifespan {
                    catalyst.active = false;
                }
            }
        }
    }

    pub fn find_best_catalyst(
        &self,
        reaction: &ChemicalReaction,
        temperature: f32,
    ) -> Option<&Catalyst> {
        self.catalysts
            .iter()
            .filter(|c| c.active && !c.is_poisoned() && c.is_in_temperature_range(temperature))
            .max_by(|a, b| {
                let factor_a = self.apply_catalyst(reaction, a);
                let factor_b = self.apply_catalyst(reaction, b);
                factor_a.partial_cmp(&factor_b).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn catalyst_count(&self) -> usize {
        self.catalysts.len()
    }

    pub fn active_count(&self) -> usize {
        self.catalysts.iter().filter(|c| c.active && !c.is_poisoned()).count()
    }
}

impl Default for CatalysisSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactions::{ChemicalReaction, ReactionConditions, ReactionType};

    fn make_test_reaction() -> ChemicalReaction {
        ChemicalReaction {
            reactants: vec![],
            products: vec![],
            activation_energy: 50.0,
            reaction_enthalpy: -100.0,
            reaction_rate: 0.5,
            catalyst: None,
            reaction_type: ReactionType::Synthesis,
            conditions: ReactionConditions { min_temperature: 300.0, ..Default::default() },
        }
    }

    #[test]
    fn test_add_remove_catalyst() {
        let mut system = CatalysisSystem::new();
        let cat = Catalyst::new("platinum", 5.0, 0.9);
        let id = system.add_catalyst(cat);
        assert_eq!(system.catalyst_count(), 1);
        assert!(system.get_catalyst(id).is_some());
        assert!(system.remove_catalyst(id));
        assert_eq!(system.catalyst_count(), 0);
    }

    #[test]
    fn test_apply_catalyst() {
        let mut system = CatalysisSystem::new();
        let cat = Catalyst::new("platinum", 5.0, 0.9).with_temperature_range(200.0, 800.0);
        system.add_catalyst(cat);

        let reaction = make_test_reaction();
        let catalyst = system.catalysts.first().unwrap();
        let factor = system.apply_catalyst(&reaction, catalyst);
        assert!(factor > 1.0);
    }

    #[test]
    fn test_catalyst_poison() {
        let mut system = CatalysisSystem::new();
        let cat = Catalyst::new("iron", 2.0, 0.7);
        let id = system.add_catalyst(cat);

        assert!(!system.is_catalyst_poisoned(id));
        system.poison_catalyst(id, 1.0);
        assert!(system.is_catalyst_poisoned(id));
    }

    #[test]
    fn test_catalyst_regeneration() {
        let mut system = CatalysisSystem::new();
        let cat = Catalyst::new("zeolite", 3.0, 0.8).with_regeneration();
        let id = system.add_catalyst(cat);

        system.poison_catalyst(id, 1.0);
        assert!(system.is_catalyst_poisoned(id));

        assert!(system.regenerate_catalyst(id));
        assert!(!system.is_catalyst_poisoned(id));
    }

    #[test]
    fn test_find_best_catalyst() {
        let mut system = CatalysisSystem::new();
        let cat1 = Catalyst::new("platinum", 5.0, 0.9).with_temperature_range(200.0, 800.0);
        let cat2 = Catalyst::new("iron", 2.0, 0.7).with_temperature_range(200.0, 800.0);
        system.add_catalyst(cat1);
        system.add_catalyst(cat2);

        let reaction = make_test_reaction();
        let best = system.find_best_catalyst(&reaction, 500.0);
        assert!(best.is_some());
        assert_eq!(best.unwrap().name, "platinum");
    }

    #[test]
    fn test_catalyst_aging() {
        let mut system = CatalysisSystem::new();
        let cat = Catalyst::new("short_lived", 1.0, 0.5).with_lifespan(10.0);
        let id = system.add_catalyst(cat);

        system.age_catalysts(15.0);
        let catalyst = system.get_catalyst(id).unwrap();
        assert!(!catalyst.active);
    }
}
