use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HormoneType {
    Adrenaline,
    Cortisol,
    Testosterone,
    Estrogen,
    Dopamine,
    Serotonin,
    Endorphin,
    Insulin,
    GrowthHormone,
    Melatonin,
    Oxytocin,
}

impl HormoneType {
    pub fn default_concentration(&self) -> f32 {
        match self {
            Self::Adrenaline => 0.1,
            Self::Cortisol => 0.2,
            Self::Testosterone => 0.5,
            Self::Estrogen => 0.4,
            Self::Dopamine => 0.3,
            Self::Serotonin => 0.4,
            Self::Endorphin => 0.1,
            Self::Insulin => 0.5,
            Self::GrowthHormone => 0.2,
            Self::Melatonin => 0.1,
            Self::Oxytocin => 0.2,
        }
    }

    pub fn default_half_life(&self) -> f32 {
        match self {
            Self::Adrenaline => 120.0,
            Self::Cortisol => 3600.0,
            Self::Testosterone => 7200.0,
            Self::Estrogen => 5400.0,
            Self::Dopamine => 180.0,
            Self::Serotonin => 300.0,
            Self::Endorphin => 600.0,
            Self::Insulin => 300.0,
            Self::GrowthHormone => 1800.0,
            Self::Melatonin => 3600.0,
            Self::Oxytocin => 180.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hormone {
    pub hormone_type: HormoneType,
    pub concentration: f32,
    pub half_life: f32,
    pub secretion_rate: f32,
    pub target_organs: Vec<String>,
}

impl Hormone {
    pub fn new(hormone_type: HormoneType) -> Self {
        Self {
            hormone_type,
            concentration: hormone_type.default_concentration(),
            half_life: hormone_type.default_half_life(),
            secretion_rate: 0.01,
            target_organs: Vec::new(),
        }
    }

    pub fn with_targets(mut self, targets: Vec<&str>) -> Self {
        self.target_organs = targets.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn is_elevated(&self) -> bool {
        self.concentration > self.hormone_type.default_concentration() * 2.0
    }

    pub fn is_depleted(&self) -> bool {
        self.concentration < self.hormone_type.default_concentration() * 0.1
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HormoneEffects {
    pub focus_mod: f32,
    pub strength_mod: f32,
    pub speed_mod: f32,
    pub mood_mod: f32,
    pub healing_mod: f32,
    pub sleep_mod: f32,
    pub aggression_mod: f32,
    pub social_mod: f32,
    pub stress_mod: f32,
}

impl Default for HormoneEffects {
    fn default() -> Self {
        Self {
            focus_mod: 1.0,
            strength_mod: 1.0,
            speed_mod: 1.0,
            mood_mod: 0.0,
            healing_mod: 1.0,
            sleep_mod: 0.0,
            aggression_mod: 0.0,
            social_mod: 0.0,
            stress_mod: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HormoneSystem {
    pub hormones: Vec<Hormone>,
}

impl HormoneSystem {
    pub fn new() -> Self {
        Self { hormones: Vec::new() }
    }

    pub fn create_default_system(&mut self) {
        let hormone_types = vec![
            HormoneType::Adrenaline,
            HormoneType::Cortisol,
            HormoneType::Testosterone,
            HormoneType::Estrogen,
            HormoneType::Dopamine,
            HormoneType::Serotonin,
            HormoneType::Endorphin,
            HormoneType::Insulin,
            HormoneType::GrowthHormone,
            HormoneType::Melatonin,
            HormoneType::Oxytocin,
        ];

        for ht in hormone_types {
            let hormone = match ht {
                HormoneType::Adrenaline => {
                    Hormone::new(ht).with_targets(vec!["heart", "muscle", "lung"])
                },
                HormoneType::Cortisol => {
                    Hormone::new(ht).with_targets(vec!["liver", "muscle", "brain"])
                },
                HormoneType::Testosterone => {
                    Hormone::new(ht).with_targets(vec!["muscle", "bone", "brain"])
                },
                HormoneType::Estrogen => {
                    Hormone::new(ht).with_targets(vec!["bone", "skin", "brain"])
                },
                HormoneType::Dopamine => Hormone::new(ht).with_targets(vec!["brain"]),
                HormoneType::Serotonin => Hormone::new(ht).with_targets(vec!["brain", "intestine"]),
                HormoneType::Endorphin => Hormone::new(ht).with_targets(vec!["brain", "muscle"]),
                HormoneType::Insulin => {
                    Hormone::new(ht).with_targets(vec!["liver", "muscle", "fat"])
                },
                HormoneType::GrowthHormone => {
                    Hormone::new(ht).with_targets(vec!["bone", "muscle", "liver"])
                },
                HormoneType::Melatonin => Hormone::new(ht).with_targets(vec!["brain"]),
                HormoneType::Oxytocin => Hormone::new(ht).with_targets(vec!["brain", "heart"]),
            };
            self.hormones.push(hormone);
        }
    }

    pub fn update(&mut self, dt: f32, stress: f32, activity: f32, health: f32) {
        for hormone in &mut self.hormones {
            let decay = (2.0f32).ln() / hormone.half_life.max(1.0);
            hormone.concentration *= (-decay * dt).exp();

            match hormone.hormone_type {
                HormoneType::Adrenaline => {
                    hormone.concentration += stress * hormone.secretion_rate * dt * 10.0;
                },
                HormoneType::Cortisol => {
                    hormone.concentration += stress * hormone.secretion_rate * dt * 5.0;
                },
                HormoneType::Dopamine => {
                    hormone.concentration += activity * hormone.secretion_rate * dt * 3.0;
                },
                HormoneType::Serotonin => {
                    hormone.concentration += (1.0 - stress) * hormone.secretion_rate * dt * 2.0;
                },
                HormoneType::Endorphin => {
                    hormone.concentration += activity * hormone.secretion_rate * dt * 5.0;
                },
                HormoneType::Insulin => {
                    hormone.concentration += activity * hormone.secretion_rate * dt * 2.0;
                },
                HormoneType::GrowthHormone => {
                    hormone.concentration += (1.0 - health) * hormone.secretion_rate * dt * 3.0;
                },
                HormoneType::Melatonin => {
                    hormone.concentration += (1.0 - activity) * hormone.secretion_rate * dt;
                },
                _ => {
                    hormone.concentration += hormone.secretion_rate * dt * 0.5;
                },
            }

            hormone.concentration = hormone.concentration.clamp(0.0, 10.0);
        }
    }

    pub fn get_hormone_level(&self, hormone_type: HormoneType) -> f32 {
        self.hormones
            .iter()
            .find(|h| h.hormone_type == hormone_type)
            .map(|h| h.concentration)
            .unwrap_or(0.0)
    }

    pub fn inject_hormone(&mut self, hormone_type: HormoneType, amount: f32) -> bool {
        if let Some(hormone) = self.hormones.iter_mut().find(|h| h.hormone_type == hormone_type) {
            hormone.concentration = (hormone.concentration + amount).min(10.0);
            true
        } else {
            false
        }
    }

    pub fn get_effects(&self) -> HormoneEffects {
        let adrenaline = self.get_hormone_level(HormoneType::Adrenaline);
        let cortisol = self.get_hormone_level(HormoneType::Cortisol);
        let testosterone = self.get_hormone_level(HormoneType::Testosterone);
        let dopamine = self.get_hormone_level(HormoneType::Dopamine);
        let serotonin = self.get_hormone_level(HormoneType::Serotonin);
        let endorphin = self.get_hormone_level(HormoneType::Endorphin);
        let growth = self.get_hormone_level(HormoneType::GrowthHormone);
        let melatonin = self.get_hormone_level(HormoneType::Melatonin);
        let oxytocin = self.get_hormone_level(HormoneType::Oxytocin);

        let def_conc = |ht: HormoneType| ht.default_concentration();

        HormoneEffects {
            focus_mod: 1.0 + (adrenaline - def_conc(HormoneType::Adrenaline)) * 0.5
                - (cortisol - def_conc(HormoneType::Cortisol)) * 0.3
                + (dopamine - def_conc(HormoneType::Dopamine)) * 0.4,
            strength_mod: 1.0
                + (adrenaline - def_conc(HormoneType::Adrenaline)) * 0.3
                + (testosterone - def_conc(HormoneType::Testosterone)) * 0.5,
            speed_mod: 1.0
                + (adrenaline - def_conc(HormoneType::Adrenaline)) * 0.4
                + (dopamine - def_conc(HormoneType::Dopamine)) * 0.2,
            mood_mod: (dopamine - def_conc(HormoneType::Dopamine)) * 0.5
                + (serotonin - def_conc(HormoneType::Serotonin)) * 0.5
                - (cortisol - def_conc(HormoneType::Cortisol)) * 0.4
                + (endorphin - def_conc(HormoneType::Endorphin)) * 0.3,
            healing_mod: 1.0 + (growth - def_conc(HormoneType::GrowthHormone)) * 0.5
                - (cortisol - def_conc(HormoneType::Cortisol)) * 0.2,
            sleep_mod: (melatonin - def_conc(HormoneType::Melatonin)) * 0.5
                - (cortisol - def_conc(HormoneType::Cortisol)) * 0.3
                - (adrenaline - def_conc(HormoneType::Adrenaline)) * 0.2,
            aggression_mod: (adrenaline - def_conc(HormoneType::Adrenaline)) * 0.3
                + (testosterone - def_conc(HormoneType::Testosterone)) * 0.4
                - (serotonin - def_conc(HormoneType::Serotonin)) * 0.2,
            social_mod: (oxytocin - def_conc(HormoneType::Oxytocin)) * 0.5
                + (dopamine - def_conc(HormoneType::Dopamine)) * 0.3
                - (cortisol - def_conc(HormoneType::Cortisol)) * 0.3,
            stress_mod: (cortisol - def_conc(HormoneType::Cortisol)) * 0.5
                - (endorphin - def_conc(HormoneType::Endorphin)) * 0.3
                - (oxytocin - def_conc(HormoneType::Oxytocin)) * 0.2,
        }
    }

    pub fn hormone_count(&self) -> usize {
        self.hormones.len()
    }

    pub fn get_hormone(&self, hormone_type: HormoneType) -> Option<&Hormone> {
        self.hormones.iter().find(|h| h.hormone_type == hormone_type)
    }

    pub fn get_elevated_hormones(&self) -> Vec<HormoneType> {
        self.hormones.iter().filter(|h| h.is_elevated()).map(|h| h.hormone_type).collect()
    }
}

impl Default for HormoneSystem {
    fn default() -> Self {
        let mut system = Self::new();
        system.create_default_system();
        system
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_system() {
        let system = HormoneSystem::default();
        assert_eq!(system.hormone_count(), 11);
    }

    #[test]
    fn test_get_hormone_level() {
        let system = HormoneSystem::default();
        let level = system.get_hormone_level(HormoneType::Adrenaline);
        assert!(level > 0.0);
    }

    #[test]
    fn test_inject_hormone() {
        let mut system = HormoneSystem::default();
        let before = system.get_hormone_level(HormoneType::Dopamine);
        assert!(system.inject_hormone(HormoneType::Dopamine, 1.0));
        let after = system.get_hormone_level(HormoneType::Dopamine);
        assert!(after > before);
    }

    #[test]
    fn test_update_decay() {
        let mut system = HormoneSystem::default();
        system.inject_hormone(HormoneType::Adrenaline, 5.0);

        let before = system.get_hormone_level(HormoneType::Adrenaline);
        system.update(60.0, 0.0, 0.0, 1.0);
        let after = system.get_hormone_level(HormoneType::Adrenaline);
        assert!(after < before);
    }

    #[test]
    fn test_stress_response() {
        let mut system = HormoneSystem::default();
        let before_cortisol = system.get_hormone_level(HormoneType::Cortisol);
        let before_adrenaline = system.get_hormone_level(HormoneType::Adrenaline);

        system.update(10.0, 1.0, 0.0, 1.0);

        let after_cortisol = system.get_hormone_level(HormoneType::Cortisol);
        let after_adrenaline = system.get_hormone_level(HormoneType::Adrenaline);

        assert!(after_cortisol > before_cortisol || after_adrenaline > before_adrenaline);
    }

    #[test]
    fn test_get_effects() {
        let mut system = HormoneSystem::default();
        system.inject_hormone(HormoneType::Adrenaline, 2.0);
        system.inject_hormone(HormoneType::Dopamine, 2.0);

        let effects = system.get_effects();
        assert!(effects.strength_mod > 1.0);
        assert!(effects.mood_mod > 0.0);
    }

    #[test]
    fn test_elevated_detection() {
        let mut system = HormoneSystem::default();
        system.inject_hormone(HormoneType::Adrenaline, 5.0);

        let elevated = system.get_elevated_hormones();
        assert!(elevated.contains(&HormoneType::Adrenaline));
    }
}
