use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrganType {
    Heart,
    Lung,
    Liver,
    Kidney,
    Brain,
    Stomach,
    Intestine,
    Skin,
    Muscle,
    Bone,
    Eye,
    Ear,
}

impl OrganType {
    pub fn default_max_health(&self) -> f32 {
        match self {
            Self::Heart => 100.0,
            Self::Lung => 100.0,
            Self::Liver => 120.0,
            Self::Kidney => 100.0,
            Self::Brain => 150.0,
            Self::Stomach => 80.0,
            Self::Intestine => 90.0,
            Self::Skin => 200.0,
            Self::Muscle => 150.0,
            Self::Bone => 200.0,
            Self::Eye => 50.0,
            Self::Ear => 40.0,
        }
    }

    pub fn default_regeneration_rate(&self) -> f32 {
        match self {
            Self::Liver => 0.5,
            Self::Skin => 0.3,
            Self::Bone => 0.1,
            Self::Muscle => 0.15,
            Self::Lung => 0.05,
            Self::Heart => 0.02,
            Self::Brain => 0.01,
            Self::Kidney => 0.03,
            Self::Stomach => 0.1,
            Self::Intestine => 0.1,
            Self::Eye => 0.01,
            Self::Ear => 0.01,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organ {
    pub id: Uuid,
    pub organ_type: OrganType,
    pub health: f32,
    pub max_health: f32,
    pub efficiency: f32,
    pub damage_accumulation: f32,
    pub regeneration_rate: f32,
    pub infection_level: f32,
    pub is_functional: bool,
}

impl Organ {
    pub fn new(organ_type: OrganType) -> Self {
        let max_health = organ_type.default_max_health();
        Self {
            id: Uuid::new_v4(),
            organ_type,
            health: max_health,
            max_health,
            efficiency: 1.0,
            damage_accumulation: 0.0,
            regeneration_rate: organ_type.default_regeneration_rate(),
            infection_level: 0.0,
            is_functional: true,
        }
    }

    pub fn health_percent(&self) -> f32 {
        (self.health / self.max_health).clamp(0.0, 1.0)
    }

    pub fn is_critical(&self) -> bool {
        self.health_percent() < 0.2
    }

    pub fn is_destroyed(&self) -> bool {
        self.health <= 0.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganSystem {
    pub organs: Vec<Organ>,
}

impl OrganSystem {
    pub fn new() -> Self {
        Self { organs: Vec::new() }
    }

    pub fn create_humanoid_organs(&mut self) -> Vec<Uuid> {
        let organ_types = vec![
            OrganType::Heart,
            OrganType::Lung,
            OrganType::Lung,
            OrganType::Liver,
            OrganType::Kidney,
            OrganType::Kidney,
            OrganType::Brain,
            OrganType::Stomach,
            OrganType::Intestine,
            OrganType::Skin,
            OrganType::Muscle,
            OrganType::Bone,
            OrganType::Eye,
            OrganType::Eye,
            OrganType::Ear,
            OrganType::Ear,
        ];

        let mut ids = Vec::new();
        for ot in organ_types {
            let organ = Organ::new(ot);
            let id = organ.id;
            ids.push(id);
            self.organs.push(organ);
        }
        ids
    }

    pub fn create_animal_organs(&mut self, species: &str) -> Vec<Uuid> {
        let mut ids = Vec::new();

        let base_organs = match species {
            "mammal" => vec![
                OrganType::Heart,
                OrganType::Lung,
                OrganType::Lung,
                OrganType::Liver,
                OrganType::Kidney,
                OrganType::Kidney,
                OrganType::Brain,
                OrganType::Stomach,
                OrganType::Intestine,
                OrganType::Skin,
                OrganType::Muscle,
                OrganType::Bone,
                OrganType::Eye,
                OrganType::Eye,
                OrganType::Ear,
                OrganType::Ear,
            ],
            "reptile" => vec![
                OrganType::Heart,
                OrganType::Lung,
                OrganType::Lung,
                OrganType::Liver,
                OrganType::Kidney,
                OrganType::Kidney,
                OrganType::Brain,
                OrganType::Stomach,
                OrganType::Intestine,
                OrganType::Skin,
                OrganType::Muscle,
                OrganType::Bone,
                OrganType::Eye,
                OrganType::Eye,
            ],
            "bird" => vec![
                OrganType::Heart,
                OrganType::Lung,
                OrganType::Lung,
                OrganType::Liver,
                OrganType::Kidney,
                OrganType::Kidney,
                OrganType::Brain,
                OrganType::Stomach,
                OrganType::Intestine,
                OrganType::Muscle,
                OrganType::Bone,
                OrganType::Eye,
                OrganType::Eye,
            ],
            "insect" => vec![
                OrganType::Brain,
                OrganType::Stomach,
                OrganType::Intestine,
                OrganType::Muscle,
                OrganType::Eye,
                OrganType::Eye,
            ],
            _ => vec![
                OrganType::Heart,
                OrganType::Brain,
                OrganType::Stomach,
                OrganType::Intestine,
                OrganType::Skin,
                OrganType::Muscle,
                OrganType::Bone,
            ],
        };

        for ot in base_organs {
            let mut organ = Organ::new(ot);
            if species == "insect" {
                organ.max_health *= 0.3;
                organ.regeneration_rate *= 0.5;
            }
            let id = organ.id;
            ids.push(id);
            self.organs.push(organ);
        }
        ids
    }

    pub fn damage_organ(&mut self, id: Uuid, amount: f32) -> bool {
        if let Some(organ) = self.organs.iter_mut().find(|o| o.id == id) {
            organ.health = (organ.health - amount).max(0.0);
            organ.damage_accumulation += amount;
            organ.efficiency = organ.health_percent();
            organ.is_functional = organ.health > 0.0;
            true
        } else {
            false
        }
    }

    pub fn heal_organ(&mut self, id: Uuid, amount: f32) -> bool {
        if let Some(organ) = self.organs.iter_mut().find(|o| o.id == id) {
            organ.health = (organ.health + amount).min(organ.max_health);
            organ.efficiency = organ.health_percent();
            true
        } else {
            false
        }
    }

    pub fn infect_organ(&mut self, id: Uuid, amount: f32) -> bool {
        if let Some(organ) = self.organs.iter_mut().find(|o| o.id == id) {
            organ.infection_level = (organ.infection_level + amount).min(1.0);
            true
        } else {
            false
        }
    }

    pub fn get_total_health(&self) -> f32 {
        if self.organs.is_empty() {
            return 0.0;
        }
        let total: f32 = self.organs.iter().map(|o| o.health_percent()).sum();
        total / self.organs.len() as f32
    }

    pub fn get_organ_efficiency(&self, organ_type: OrganType) -> f32 {
        let matching: Vec<f32> = self
            .organs
            .iter()
            .filter(|o| o.organ_type == organ_type)
            .map(|o| o.efficiency)
            .collect();

        if matching.is_empty() {
            return 0.0;
        }
        matching.iter().sum::<f32>() / matching.len() as f32
    }

    pub fn tick_regeneration(&mut self, dt: f32) {
        for organ in &mut self.organs {
            if organ.health < organ.max_health && organ.is_functional {
                let regen = organ.regeneration_rate * dt * organ.max_health;
                let infection_penalty = 1.0 - organ.infection_level * 0.8;
                let actual_regen = regen * infection_penalty;

                organ.health = (organ.health + actual_regen).min(organ.max_health);
                organ.efficiency = organ.health_percent();

                organ.infection_level = (organ.infection_level - 0.01 * dt).max(0.0);
            }

            if organ.health <= 0.0 {
                organ.is_functional = false;
                organ.efficiency = 0.0;
            }
        }
    }

    pub fn get_vital_status(&self) -> VitalStatus {
        let heart_eff = self.get_organ_efficiency(OrganType::Heart);
        let brain_eff = self.get_organ_efficiency(OrganType::Brain);
        let lung_eff = self.get_organ_efficiency(OrganType::Lung);

        if brain_eff <= 0.0 || heart_eff <= 0.0 {
            VitalStatus::Dead
        } else if heart_eff < 0.3 || brain_eff < 0.3 || lung_eff < 0.3 {
            VitalStatus::Critical
        } else if self.get_total_health() < 0.5 {
            VitalStatus::Injured
        } else {
            VitalStatus::Healthy
        }
    }

    pub fn organ_count(&self) -> usize {
        self.organs.len()
    }

    pub fn functional_count(&self) -> usize {
        self.organs.iter().filter(|o| o.is_functional).count()
    }

    pub fn get_organ(&self, id: Uuid) -> Option<&Organ> {
        self.organs.iter().find(|o| o.id == id)
    }

    pub fn get_organs_of_type(&self, organ_type: OrganType) -> Vec<&Organ> {
        self.organs.iter().filter(|o| o.organ_type == organ_type).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VitalStatus {
    Healthy,
    Injured,
    Critical,
    Dead,
}

impl Default for OrganSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_humanoid_organs() {
        let mut system = OrganSystem::new();
        let ids = system.create_humanoid_organs();
        assert_eq!(ids.len(), 16);
        assert_eq!(system.organ_count(), 16);
    }

    #[test]
    fn test_damage_heal() {
        let mut system = OrganSystem::new();
        let _ids = system.create_humanoid_organs();

        let heart_id = system.get_organs_of_type(OrganType::Heart)[0].id;
        assert!(system.damage_organ(heart_id, 50.0));
        let organ = system.get_organ(heart_id).unwrap();
        assert_eq!(organ.health, 50.0);

        assert!(system.heal_organ(heart_id, 30.0));
        let organ = system.get_organ(heart_id).unwrap();
        assert_eq!(organ.health, 80.0);
    }

    #[test]
    fn test_regeneration() {
        let mut system = OrganSystem::new();
        let _ids = system.create_humanoid_organs();

        let liver_id = system.get_organs_of_type(OrganType::Liver)[0].id;
        system.damage_organ(liver_id, 50.0);
        let before = system.get_organ(liver_id).unwrap().health;

        system.tick_regeneration(10.0);
        let after = system.get_organ(liver_id).unwrap().health;
        assert!(after > before);
    }

    #[test]
    fn test_vital_status() {
        let mut system = OrganSystem::new();
        system.create_humanoid_organs();
        assert_eq!(system.get_vital_status(), VitalStatus::Healthy);

        let heart_id = system.get_organs_of_type(OrganType::Heart)[0].id;
        system.damage_organ(heart_id, 100.0);
        assert_eq!(system.get_vital_status(), VitalStatus::Dead);
    }

    #[test]
    fn test_create_animal_organs() {
        let mut system = OrganSystem::new();
        let ids = system.create_animal_organs("reptile");
        assert_eq!(ids.len(), 14);
    }

    #[test]
    fn test_get_organ_efficiency() {
        let mut system = OrganSystem::new();
        system.create_humanoid_organs();

        let eff = system.get_organ_efficiency(OrganType::Heart);
        assert_eq!(eff, 1.0);
    }

    #[test]
    fn test_infection() {
        let mut system = OrganSystem::new();
        let _ids = system.create_humanoid_organs();

        let skin_id = system.get_organs_of_type(OrganType::Skin)[0].id;
        system.damage_organ(skin_id, 20.0);
        system.infect_organ(skin_id, 0.5);

        let before = system.get_organ(skin_id).unwrap().health;
        system.tick_regeneration(10.0);
        let after = system.get_organ(skin_id).unwrap().health;

        assert!(after >= before);
    }
}
