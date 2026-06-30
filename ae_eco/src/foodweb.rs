use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrophicLevel {
    Producer,
    PrimaryConsumer,
    SecondaryConsumer,
    TertiaryConsumer,
    ApexPredator,
    Decomposer,
}

impl TrophicLevel {
    pub fn energy_efficiency(&self) -> f32 {
        match self {
            TrophicLevel::Producer => 0.01,
            TrophicLevel::PrimaryConsumer => 0.1,
            TrophicLevel::SecondaryConsumer => 0.1,
            TrophicLevel::TertiaryConsumer => 0.1,
            TrophicLevel::ApexPredator => 0.05,
            TrophicLevel::Decomposer => 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Species {
    pub id: String,
    pub name: String,
    pub trophic_level: TrophicLevel,
    pub biomass: f32,
    pub energy_content: f32,
    pub metabolic_rate: f32,
    pub reproduction_rate: f32,
    pub lifespan: f32,
}

impl Species {
    pub fn new(name: &str, trophic_level: TrophicLevel, biomass: f32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            trophic_level,
            biomass,
            energy_content: 20000.0,
            metabolic_rate: match trophic_level {
                TrophicLevel::Producer => 0.01,
                TrophicLevel::PrimaryConsumer => 0.1,
                TrophicLevel::SecondaryConsumer => 0.2,
                TrophicLevel::TertiaryConsumer => 0.3,
                TrophicLevel::ApexPredator => 0.4,
                TrophicLevel::Decomposer => 0.05,
            },
            reproduction_rate: 0.5,
            lifespan: 365.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodChain {
    pub links: Vec<FoodLink>,
    pub energy_flow: f32,
    pub stability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodLink {
    pub predator_id: String,
    pub prey_id: String,
    pub consumption_rate: f32,
    pub preference: f32,
    pub energy_transfer: f32,
}

impl FoodLink {
    pub fn new(predator_id: &str, prey_id: &str, consumption_rate: f32) -> Self {
        Self {
            predator_id: predator_id.to_string(),
            prey_id: prey_id.to_string(),
            consumption_rate,
            preference: 0.5,
            energy_transfer: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodWeb {
    pub species: Vec<Species>,
    pub links: Vec<FoodLink>,
    pub connectance: f32,
    pub omnivory_index: f32,
}

impl FoodWeb {
    pub fn new() -> Self {
        Self { species: Vec::new(), links: Vec::new(), connectance: 0.0, omnivory_index: 0.0 }
    }

    pub fn add_species(&mut self, species: Species) {
        self.species.push(species);
        self.update_metrics();
    }

    pub fn add_link(&mut self, link: FoodLink) {
        self.links.push(link);
        self.update_metrics();
    }

    pub fn update_metrics(&mut self) {
        let s = self.species.len() as f32;
        if s > 1.0 {
            self.connectance = self.links.len() as f32 / (s * s);
        }

        let omnivory_count = self
            .species
            .iter()
            .filter(|sp| self.links.iter().filter(|l| l.predator_id == sp.id).count() > 1)
            .count();
        self.omnivory_index = omnivory_count as f32 / s.max(1.0);
    }

    pub fn trophic_position(&self, species_id: &str) -> f32 {
        let prey_links: Vec<&FoodLink> =
            self.links.iter().filter(|l| l.predator_id == species_id).collect();

        if prey_links.is_empty() {
            return 1.0;
        }

        let avg_prey_position: f32 =
            prey_links.iter().map(|l| self.trophic_position(&l.prey_id)).sum::<f32>()
                / prey_links.len() as f32;

        1.0 + avg_prey_position
    }

    pub fn energy_flow(&self, species_id: &str) -> f32 {
        let incoming: f32 = self
            .links
            .iter()
            .filter(|l| l.predator_id == species_id)
            .map(|l| {
                let prey = self.species.iter().find(|s| s.id == l.prey_id);
                match prey {
                    Some(p) => l.consumption_rate * p.biomass * l.energy_transfer,
                    None => 0.0,
                }
            })
            .sum();

        let outgoing: f32 =
            self.links.iter().filter(|l| l.prey_id == species_id).map(|l| l.consumption_rate).sum();

        incoming - outgoing
    }

    pub fn remove_species(&mut self, species_id: &str) -> Vec<String> {
        self.links.retain(|l| l.predator_id != species_id && l.prey_id != species_id);
        self.species.retain(|s| s.id != species_id);

        let affected: Vec<String> = self
            .links
            .iter()
            .filter(|l| {
                !self.species.iter().any(|s| s.id == l.predator_id)
                    || !self.species.iter().any(|s| s.id == l.prey_id)
            })
            .map(|l| l.predator_id.clone())
            .collect();

        self.links.retain(|l| {
            self.species.iter().any(|s| s.id == l.predator_id)
                && self.species.iter().any(|s| s.id == l.prey_id)
        });

        self.update_metrics();
        affected
    }
}

impl Default for FoodWeb {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_food_web_creation() {
        let web = FoodWeb::new();
        assert!(web.species.is_empty());
        assert!(web.links.is_empty());
        assert_eq!(web.connectance, 0.0);
    }

    #[test]
    fn test_food_web_species_and_links() {
        let mut web = FoodWeb::new();
        let grass = Species::new("草", TrophicLevel::Producer, 1000.0);
        let rabbit = Species::new("兔", TrophicLevel::PrimaryConsumer, 100.0);
        let fox = Species::new("狐", TrophicLevel::SecondaryConsumer, 10.0);

        let grass_id = grass.id.clone();
        let rabbit_id = rabbit.id.clone();
        let fox_id = fox.id.clone();

        web.add_species(grass);
        web.add_species(rabbit);
        web.add_species(fox);

        web.add_link(FoodLink::new(&rabbit_id, &grass_id, 0.5));
        web.add_link(FoodLink::new(&fox_id, &rabbit_id, 0.3));

        assert_eq!(web.species.len(), 3);
        assert_eq!(web.links.len(), 2);
        assert!(web.connectance > 0.0);
    }

    #[test]
    fn test_food_web_trophic_position() {
        let mut web = FoodWeb::new();
        let grass = Species::new("草", TrophicLevel::Producer, 1000.0);
        let rabbit = Species::new("兔", TrophicLevel::PrimaryConsumer, 100.0);

        let grass_id = grass.id.clone();
        let rabbit_id = rabbit.id.clone();

        web.add_species(grass);
        web.add_species(rabbit);
        web.add_link(FoodLink::new(&rabbit_id, &grass_id, 0.5));

        let pos = web.trophic_position(&grass_id);
        assert_eq!(pos, 1.0);
        let rabbit_pos = web.trophic_position(&rabbit_id);
        assert!(rabbit_pos > 1.0);
    }
}
