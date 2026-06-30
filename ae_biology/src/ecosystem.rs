use glam::Vec3;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::organisms::{Diet, Organism, OrganismState, Species};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ecosystem {
    pub id: Uuid,
    pub name: String,
    pub biome: Biome,
    pub organisms: Vec<Organism>,
    pub flora: Vec<Flora>,
    pub resources: Vec<Resource>,
    pub temperature: f32,
    pub humidity: f32,
    pub radiation_level: f32,
    pub soil_quality: f32,
    pub water_purity: f32,
    pub carrying_capacity: usize,
    pub bounds: EcosystemBounds,
    pub time: f64,
    pub population_history: Vec<PopulationSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Biome {
    Wasteland,
    RuinedCity,
    Underground,
    RadioactiveMarsh,
    Desert,
    ToxicForest,
    MountainPass,
    CoastalWreck,
    IndustrialZone,
    Farmland,
    MilitaryBase,
    Crater,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flora {
    pub id: Uuid,
    pub species: FloraSpecies,
    pub position: Vec3,
    pub health: f32,
    pub growth: f32,
    pub max_growth: f32,
    pub fruit_bearing: bool,
    pub radiation_absorbed: f32,
    pub mutation_level: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FloraSpecies {
    MutatedGrass,
    GlowingMushroom,
    ThornBush,
    DeadTree,
    MutfruitTree,
    TatoPlant,
    Razorgrain,
    Hubflower,
    Bloodleaf,
    BrainFungus,
    Firecap,
    Tarberry,
    AshBlossom,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub position: Vec3,
    pub amount: f32,
    pub max_amount: f32,
    pub regeneration_rate: f32,
    pub quality: f32,
    pub contamination: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    Water,
    Food,
    Wood,
    Stone,
    Metal,
    Oil,
    Chemical,
    Electricity,
    Medicine,
    Ammo,
    Fuel,
    Fertilizer,
    Parts,
    Scrap,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EcosystemBounds {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationSnapshot {
    pub time: f64,
    pub total_organisms: usize,
    pub species_counts: std::collections::HashMap<Species, usize>,
    pub average_health: f32,
    pub average_radiation: f32,
    pub mutation_rate: f32,
    pub deaths: u32,
    pub births: u32,
}

impl Ecosystem {
    pub fn new(name: String, biome: Biome, bounds: EcosystemBounds) -> Self {
        let carrying_capacity = match biome {
            Biome::Wasteland => 200,
            Biome::RuinedCity => 500,
            Biome::Underground => 300,
            Biome::RadioactiveMarsh => 100,
            Biome::Desert => 50,
            Biome::ToxicForest => 150,
            Biome::MountainPass => 100,
            Biome::CoastalWreck => 250,
            Biome::IndustrialZone => 300,
            Biome::Farmland => 400,
            Biome::MilitaryBase => 200,
            Biome::Crater => 50,
            Biome::Custom(_) => 200,
        };

        Self {
            id: Uuid::new_v4(),
            name,
            biome,
            organisms: Vec::new(),
            flora: Vec::new(),
            resources: Vec::new(),
            temperature: 293.0,
            humidity: 0.5,
            radiation_level: 0.0,
            soil_quality: 0.5,
            water_purity: 0.5,
            carrying_capacity,
            bounds,
            time: 0.0,
            population_history: Vec::new(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt as f64;

        let mut deaths = 0u32;
        let mut births = 0u32;

        let mut rng = rand::thread_rng();

        self.organisms.retain(|org| {
            if org.state == OrganismState::Dead {
                deaths += 1;
                return false;
            }
            true
        });

        for i in 0..self.organisms.len() {
            self.organisms[i].sensory.ambient_radiation = self.radiation_level;
            self.organisms[i].sensory.ambient_temperature = self.temperature;
        }

        for organism in &mut self.organisms {
            organism.radiation_dose += self.radiation_level * 0.01 * dt;
            organism.reproductive_cooldown =
                (organism.reproductive_cooldown - dt / 3600.0).max(0.0);
            organism.update(dt, self.time);
        }

        let mut new_offspring = Vec::new();
        for i in 0..self.organisms.len() {
            if !self.organisms[i].can_reproduce() {
                continue;
            }
            for j in (i + 1)..self.organisms.len() {
                if self.organisms[j].species != self.organisms[i].species {
                    continue;
                }
                if !self.organisms[j].can_reproduce() {
                    continue;
                }
                let dist = (self.organisms[i].position - self.organisms[j].position).length();
                if dist < 10.0 && rng.gen::<f32>() < 0.001 * dt {
                    let offspring = self.organisms[i].create_offspring(&self.organisms[j]);
                    new_offspring.push(offspring);
                    self.organisms[i].reproductive_cooldown = self.organisms[i].max_age * 0.05;
                    self.organisms[j].reproductive_cooldown = self.organisms[j].max_age * 0.05;
                    self.organisms[i].offspring_count += 1;
                    self.organisms[j].offspring_count += 1;
                    births += 1;
                    break;
                }
            }
        }
        self.organisms.extend(new_offspring);

        let organism_count = self.organisms.len();
        if organism_count < self.carrying_capacity {
            let spawn_chance =
                (1.0 - organism_count as f32 / self.carrying_capacity as f32) * 0.001 * dt;
            if rng.gen::<f32>() < spawn_chance {
                let species = self.select_spawn_species();
                let pos = self.random_position_in_bounds();
                let mut org = Organism::new(species, pos);
                org.radiation_dose = self.radiation_level * 0.1;
                self.organisms.push(org);
                births += 1;
            }
        }

        self.update_flora(dt);
        self.update_resources(dt);
        self.update_predator_prey(dt);
        self.record_population_snapshot(deaths, births);
        self.maintain_equilibrium();
    }

    fn select_spawn_species(&self) -> Species {
        let mut rng = rand::thread_rng();
        let species_pool = match self.biome {
            Biome::Wasteland => {
                vec![Species::Radroach, Species::Molerat, Species::Bloatfly, Species::Gecko]
            },
            Biome::RuinedCity => {
                vec![Species::Ghoul, Species::Radroach, Species::MutantHound, Species::Human]
            },
            Biome::Underground => vec![Species::Molerat, Species::Radroach, Species::GiantAnt],
            Biome::RadioactiveMarsh => {
                vec![Species::Bloatfly, Species::Mantis, Species::Radscorpion]
            },
            Biome::ToxicForest => {
                vec![Species::YaoGuai, Species::Cazador, Species::Gecko, Species::Mantis]
            },
            Biome::Desert => vec![Species::Radscorpion, Species::Gecko],
            Biome::MountainPass => vec![Species::YaoGuai, Species::Deathclaw, Species::Brahmin],
            Biome::CoastalWreck => vec![Species::Molerat, Species::MutantHound, Species::Human],
            Biome::IndustrialZone => {
                vec![Species::Ghoul, Species::SuperMutant, Species::MutantHuman]
            },
            Biome::Farmland => vec![Species::Brahmin, Species::Human, Species::Molerat],
            Biome::MilitaryBase => {
                vec![Species::SuperMutant, Species::Deathclaw, Species::MutantHound]
            },
            Biome::Crater => vec![Species::Deathclaw, Species::Radscorpion],
            Biome::Custom(_) => vec![Species::Radroach, Species::Molerat],
        };
        species_pool[rng.gen_range(0..species_pool.len())]
    }

    fn random_position_in_bounds(&self) -> Vec3 {
        let mut rng = rand::thread_rng();
        Vec3::new(
            rng.gen_range(self.bounds.min.x..self.bounds.max.x),
            self.bounds.min.y,
            rng.gen_range(self.bounds.min.z..self.bounds.max.z),
        )
    }

    fn update_flora(&mut self, dt: f32) {
        for plant in &mut self.flora {
            if plant.health <= 0.0 {
                continue;
            }
            plant.growth = (plant.growth + 0.01 * dt * self.soil_quality).min(plant.max_growth);
            plant.radiation_absorbed += self.radiation_level * 0.001 * dt;
            plant.health -= plant.radiation_absorbed * 0.01 * dt;
            plant.mutation_level += plant.radiation_absorbed * 0.0001;
        }
    }

    fn update_resources(&mut self, dt: f32) {
        for resource in &mut self.resources {
            resource.amount =
                (resource.amount + resource.regeneration_rate * dt).min(resource.max_amount);
        }
    }

    fn update_predator_prey(&mut self, dt: f32) {
        let mut predation_events = Vec::new();

        for (i, predator) in self.organisms.iter().enumerate() {
            if predator.state == OrganismState::Dead {
                continue;
            }
            if predator.species.diet() != Diet::Carnivore
                && predator.species.diet() != Diet::Omnivore
            {
                continue;
            }

            for (j, prey) in self.organisms.iter().enumerate() {
                if i == j || prey.state == OrganismState::Dead {
                    continue;
                }
                if prey.species == predator.species {
                    continue;
                }
                let dist = (predator.position - prey.position).length();
                let hunt_range = 10.0 * predator.size;

                if dist < hunt_range {
                    let mut rng = rand::thread_rng();
                    if rng.gen::<f32>() < 0.01 * dt {
                        let damage = predator.size * 20.0;
                        predation_events.push((j, damage));
                    }
                }
            }
        }

        for (prey_idx, damage) in &predation_events {
            if let Some(prey) = self.organisms.get_mut(*prey_idx) {
                prey.take_damage(*damage, "physical");
            }
        }
    }

    fn record_population_snapshot(&mut self, deaths: u32, births: u32) {
        let mut species_counts = std::collections::HashMap::new();
        let mut total_health = 0.0f32;
        let mut total_radiation = 0.0f32;
        let mut mutation_count = 0u32;

        for org in &self.organisms {
            *species_counts.entry(org.species).or_insert(0) += 1;
            total_health += org.health;
            total_radiation += org.radiation_dose;
            mutation_count += org.mutations.len() as u32;
        }

        let count = self.organisms.len();
        let snapshot = PopulationSnapshot {
            time: self.time,
            total_organisms: count,
            species_counts,
            average_health: if count > 0 { total_health / count as f32 } else { 0.0 },
            average_radiation: if count > 0 { total_radiation / count as f32 } else { 0.0 },
            mutation_rate: if count > 0 { mutation_count as f32 / count as f32 } else { 0.0 },
            deaths,
            births,
        };

        self.population_history.push(snapshot);
        if self.population_history.len() > 1000 {
            self.population_history.drain(0..500);
        }
    }

    fn maintain_equilibrium(&mut self) {
        if self.organisms.len() > self.carrying_capacity * 2 {
            let excess = self.organisms.len() - self.carrying_capacity;
            let mut rng = rand::thread_rng();
            for _ in 0..excess.min(10) {
                let idx = rng.gen_range(0..self.organisms.len());
                if let Some(org) = self.organisms.get_mut(idx) {
                    org.health -= 50.0;
                }
            }
        }
    }

    pub fn add_organism(&mut self, organism: Organism) {
        self.organisms.push(organism);
    }

    pub fn add_flora(&mut self, flora: Flora) {
        self.flora.push(flora);
    }

    pub fn add_resource(&mut self, resource: Resource) {
        self.resources.push(resource);
    }

    pub fn set_radiation(&mut self, level: f32) {
        self.radiation_level = level;
    }

    pub fn organism_count(&self) -> usize {
        self.organisms.iter().filter(|o| o.state != OrganismState::Dead).count()
    }

    pub fn species_diversity(&self) -> f32 {
        let mut species_counts = std::collections::HashMap::new();
        let total = self.organism_count();
        if total == 0 {
            return 0.0;
        }
        for org in &self.organisms {
            if org.state != OrganismState::Dead {
                *species_counts.entry(org.species).or_insert(0) += 1;
            }
        }
        let mut diversity = 0.0f32;
        for count in species_counts.values() {
            let p = *count as f32 / total as f32;
            // Shannon 指数 H = -Σ p·ln(p)；p∈(0,1] 时 -p·ln(p) ≥ 0
            let contribution = -p * p.ln();
            diversity += contribution.max(0.0);
        }
        diversity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn test_bounds() -> EcosystemBounds {
        EcosystemBounds {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(100.0, 10.0, 100.0),
        }
    }

    #[test]
    fn test_ecosystem_new_ae_capacity() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.carrying_capacity, 200);
    }

    #[test]
    fn test_ecosystem_new_ruined_city_capacity() {
        let eco = Ecosystem::new("test".to_string(), Biome::RuinedCity, test_bounds());
        assert_eq!(eco.carrying_capacity, 500);
    }

    #[test]
    fn test_ecosystem_new_desert_low_capacity() {
        let eco = Ecosystem::new("test".to_string(), Biome::Desert, test_bounds());
        assert_eq!(eco.carrying_capacity, 50);
    }

    #[test]
    fn test_ecosystem_new_crater_low_capacity() {
        let eco = Ecosystem::new("test".to_string(), Biome::Crater, test_bounds());
        assert_eq!(eco.carrying_capacity, 50);
    }

    #[test]
    fn test_ecosystem_new_farmland_high_capacity() {
        let eco = Ecosystem::new("test".to_string(), Biome::Farmland, test_bounds());
        assert_eq!(eco.carrying_capacity, 400);
    }

    #[test]
    fn test_ecosystem_new_underground_capacity() {
        let eco = Ecosystem::new("test".to_string(), Biome::Underground, test_bounds());
        assert_eq!(eco.carrying_capacity, 300);
    }

    #[test]
    fn test_ecosystem_new_custom_capacity() {
        let eco = Ecosystem::new("test".to_string(), Biome::Custom(99), test_bounds());
        assert_eq!(eco.carrying_capacity, 200);
    }

    #[test]
    fn test_ecosystem_all_biome_capacities() {
        let cases = vec![
            (Biome::Wasteland, 200),
            (Biome::RuinedCity, 500),
            (Biome::Underground, 300),
            (Biome::RadioactiveMarsh, 100),
            (Biome::Desert, 50),
            (Biome::ToxicForest, 150),
            (Biome::MountainPass, 100),
            (Biome::CoastalWreck, 250),
            (Biome::IndustrialZone, 300),
            (Biome::Farmland, 400),
            (Biome::MilitaryBase, 200),
            (Biome::Crater, 50),
        ];
        for (biome, expected) in cases {
            let eco = Ecosystem::new("t".to_string(), biome, test_bounds());
            assert_eq!(eco.carrying_capacity, expected, "Biome {:?} capacity mismatch", biome);
        }
    }

    #[test]
    fn test_ecosystem_default_temperature() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.temperature, 293.0);
    }

    #[test]
    fn test_ecosystem_default_humidity() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.humidity, 0.5);
    }

    #[test]
    fn test_ecosystem_default_radiation_zero() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.radiation_level, 0.0);
    }

    #[test]
    fn test_ecosystem_default_soil_quality() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.soil_quality, 0.5);
    }

    #[test]
    fn test_ecosystem_default_water_purity() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.water_purity, 0.5);
    }

    #[test]
    fn test_ecosystem_initial_time_zero() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.time, 0.0);
    }

    #[test]
    fn test_ecosystem_initial_population_history_empty() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert!(eco.population_history.is_empty());
    }

    #[test]
    fn test_ecosystem_empty_organism_count_zero() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.organism_count(), 0);
    }

    #[test]
    fn test_ecosystem_empty_species_diversity_zero() {
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.species_diversity(), 0.0);
    }

    #[test]
    fn test_ecosystem_name_preserved() {
        let eco = Ecosystem::new("Wasteland Alpha".to_string(), Biome::Wasteland, test_bounds());
        assert_eq!(eco.name, "Wasteland Alpha");
    }

    #[test]
    fn test_ecosystem_biome_preserved() {
        let eco = Ecosystem::new("test".to_string(), Biome::RadioactiveMarsh, test_bounds());
        assert_eq!(eco.biome, Biome::RadioactiveMarsh);
    }

    #[test]
    fn test_ecosystem_set_radiation() {
        let mut eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        eco.set_radiation(50.0);
        assert_eq!(eco.radiation_level, 50.0);
    }

    #[test]
    fn test_ecosystem_add_organism_increases_count() {
        let mut eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        let org = Organism::new(Species::Radroach, Vec3::new(1.0, 0.0, 1.0));
        eco.add_organism(org);
        assert_eq!(eco.organism_count(), 1);
    }

    #[test]
    fn test_ecosystem_add_flora_increases_count() {
        let mut eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        let flora = Flora {
            id: Uuid::new_v4(),
            species: FloraSpecies::MutatedGrass,
            position: Vec3::new(1.0, 0.0, 1.0),
            health: 100.0,
            growth: 0.0,
            max_growth: 10.0,
            fruit_bearing: false,
            radiation_absorbed: 0.0,
            mutation_level: 0.0,
        };
        eco.add_flora(flora);
        assert_eq!(eco.flora.len(), 1);
    }

    #[test]
    fn test_ecosystem_add_resource_increases_count() {
        let mut eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        let resource = Resource {
            resource_type: ResourceType::Water,
            position: Vec3::new(1.0, 0.0, 1.0),
            amount: 100.0,
            max_amount: 100.0,
            regeneration_rate: 1.0,
            quality: 1.0,
            contamination: 0.0,
        };
        eco.add_resource(resource);
        assert_eq!(eco.resources.len(), 1);
    }

    #[test]
    fn test_ecosystem_organism_count_excludes_dead() {
        let mut eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        let mut org = Organism::new(Species::Radroach, Vec3::new(1.0, 0.0, 1.0));
        org.state = OrganismState::Dead;
        eco.add_organism(org);
        assert_eq!(eco.organism_count(), 0);
        assert_eq!(eco.organisms.len(), 1);
    }

    #[test]
    fn test_ecosystem_species_diversity_single_species() {
        let mut eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        eco.add_organism(Organism::new(Species::Radroach, Vec3::new(1.0, 0.0, 1.0)));
        eco.add_organism(Organism::new(Species::Radroach, Vec3::new(2.0, 0.0, 2.0)));
        // 单一物种：Shannon 多样性 = 0
        assert_eq!(eco.species_diversity(), 0.0);
    }

    #[test]
    fn test_ecosystem_species_diversity_multiple_species_positive() {
        let mut eco = Ecosystem::new("test".to_string(), Biome::Wasteland, test_bounds());
        eco.add_organism(Organism::new(Species::Radroach, Vec3::new(1.0, 0.0, 1.0)));
        eco.add_organism(Organism::new(Species::Molerat, Vec3::new(2.0, 0.0, 2.0)));
        // 多物种：Shannon 多样性 > 0
        assert!(eco.species_diversity() > 0.0);
    }

    #[test]
    fn test_biome_equality() {
        assert_eq!(Biome::Wasteland, Biome::Wasteland);
        assert_ne!(Biome::Wasteland, Biome::Desert);
        assert_eq!(Biome::Custom(42), Biome::Custom(42));
        assert_ne!(Biome::Custom(42), Biome::Custom(43));
    }

    #[test]
    fn test_flora_species_equality() {
        assert_eq!(FloraSpecies::GlowingMushroom, FloraSpecies::GlowingMushroom);
        assert_ne!(FloraSpecies::GlowingMushroom, FloraSpecies::BrainFungus);
    }

    #[test]
    fn test_resource_type_equality() {
        assert_eq!(ResourceType::Water, ResourceType::Water);
        assert_ne!(ResourceType::Water, ResourceType::Metal);
    }

    #[test]
    fn test_ecosystem_bounds_stored() {
        let bounds = EcosystemBounds {
            min: Vec3::new(-10.0, -5.0, -10.0),
            max: Vec3::new(50.0, 5.0, 50.0),
        };
        let eco = Ecosystem::new("test".to_string(), Biome::Wasteland, bounds);
        assert_eq!(eco.bounds.min, Vec3::new(-10.0, -5.0, -10.0));
        assert_eq!(eco.bounds.max, Vec3::new(50.0, 5.0, 50.0));
    }
}
