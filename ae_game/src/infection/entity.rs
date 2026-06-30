//! 感染实体

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::stage::InfectionStage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfectionStatus {
    pub entity_id: Uuid,
    pub level: f32,
    pub stage: InfectionStage,
    pub time_in_stage: f32,
    pub growth_rate: f32,
    pub spore_exposure: f32,
    pub is_amputated: bool,
}

impl InfectionStatus {
    pub fn new(entity_id: Uuid) -> Self {
        Self {
            entity_id,
            level: 0.0,
            stage: InfectionStage::Latent,
            time_in_stage: 0.0,
            growth_rate: 0.01,
            spore_exposure: 0.0,
            is_amputated: false,
        }
    }

    pub fn expose_to_spores(&mut self, amount: f32) {
        self.spore_exposure += amount;
        self.level = (self.level + amount * 0.1).min(1.0);
        self.update_stage();
    }

    pub fn reduce(&mut self, amount: f32) {
        self.level = (self.level - amount).max(0.0);
        self.update_stage();
    }

    pub fn advance(&mut self, dt: f32) {
        if self.is_amputated { return; }
        self.time_in_stage += dt;
        self.level = (self.level + self.growth_rate * dt).min(1.0);
        self.update_stage();
    }

    fn update_stage(&mut self) {
        let new_stage = InfectionStage::from_infection_level(self.level);
        if new_stage != self.stage {
            self.stage = new_stage;
            self.time_in_stage = 0.0;
        }
    }
}

#[derive(Debug, Default)]
pub struct InfectionWorld {
    pub infections: hashbrown::HashMap<Uuid, InfectionStatus>,
}

impl InfectionWorld {
    pub fn new() -> Self { Self::default() }

    pub fn add_entity(&mut self, id: Uuid) {
        self.infections.insert(id, InfectionStatus::new(id));
    }

    pub fn expose(&mut self, id: Uuid, amount: f32) {
        if let Some(s) = self.infections.get_mut(&id) {
            s.expose_to_spores(amount);
        }
    }

    pub fn step(&mut self, dt: f32) {
        for s in self.infections.values_mut() {
            s.advance(dt);
        }
    }

    pub fn infected_count(&self) -> usize {
        self.infections.values().filter(|s| s.level > 0.0).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infection_status() {
        let id = Uuid::new_v4();
        let mut status = InfectionStatus::new(id);
        assert_eq!(status.level, 0.0);

        status.expose_to_spores(1.0);
        assert!(status.level > 0.0);
        assert!(status.stage != InfectionStage::Latent || status.level < 0.25);
    }

    #[test]
    fn test_infection_world() {
        let mut world = InfectionWorld::new();
        let id = Uuid::new_v4();
        world.add_entity(id);
        world.expose(id, 2.0);
        world.step(1.0);
        assert!(world.infected_count() > 0);
    }
}
