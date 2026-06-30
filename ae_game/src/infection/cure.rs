//! 治疗系统

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::entity::InfectionWorld;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CureKind {
    Drug,
    Purge,
    Amputate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CureResult {
    pub kind: CureKind,
    pub target: Uuid,
    pub reduced: f32,
    pub success: bool,
}

#[derive(Debug, Default)]
pub struct CureSystem {
    pub drug_reduce_amount: f32,
    pub purge_radius: f32,
    pub drug_cooldown: f32,
    pub cooldowns: hashbrown::HashMap<Uuid, f32>,
}

impl CureSystem {
    pub fn new() -> Self {
        Self {
            drug_reduce_amount: 30.0,
            purge_radius: 5.0,
            drug_cooldown: 10.0,
            cooldowns: hashbrown::HashMap::new(),
        }
    }

    pub fn use_drug(&mut self, world: &mut InfectionWorld, target: Uuid) -> CureResult {
        let on_cooldown = self.cooldowns.get(&target).copied().unwrap_or(0.0) > 0.0;
        if on_cooldown {
            return CureResult { kind: CureKind::Drug, target, reduced: 0.0, success: false };
        }
        if let Some(s) = world.infections.get_mut(&target) {
            let before = s.level;
            s.reduce(self.drug_reduce_amount / 100.0);
            let reduced = before - s.level;
            self.cooldowns.insert(target, self.drug_cooldown);
            return CureResult { kind: CureKind::Drug, target, reduced, success: true };
        }
        CureResult { kind: CureKind::Drug, target, reduced: 0.0, success: false }
    }

    pub fn step(&mut self, dt: f32) {
        for cd in self.cooldowns.values_mut() {
            *cd = (*cd - dt).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drug_cure() {
        let mut world = InfectionWorld::new();
        let id = Uuid::new_v4();
        world.add_entity(id);
        world.expose(id, 5.0);

        let mut cure = CureSystem::new();
        let result = cure.use_drug(&mut world, id);
        assert!(result.success);
        assert!(result.reduced > 0.0);
    }

    #[test]
    fn test_drug_cooldown() {
        let mut world = InfectionWorld::new();
        let id = Uuid::new_v4();
        world.add_entity(id);
        world.expose(id, 5.0);

        let mut cure = CureSystem::new();
        cure.use_drug(&mut world, id);
        let result2 = cure.use_drug(&mut world, id);
        assert!(!result2.success);
    }
}
