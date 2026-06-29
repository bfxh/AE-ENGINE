use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuccessionStage {
    Bare,
    Pioneer,
    EarlySeral,
    MidSeral,
    LateSeral,
    Climax,
    Disturbed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcologicalSuccession {
    pub stage: SuccessionStage,
    pub progress: f32,
    pub time_in_stage: f32,
    pub species_richness: f32,
    pub soil_depth: f32,
    pub soil_organic_matter: f32,
    pub canopy_cover: f32,
    pub disturbance_history: Vec<DisturbanceEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisturbanceEvent {
    pub event_type: DisturbanceType,
    pub severity: f32,
    pub time_since: f32,
    pub area_affected: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisturbanceType {
    Fire,
    Flood,
    Landslide,
    VolcanicEruption,
    HumanActivity,
    Storm,
    Disease,
    Drought,
}

impl Default for EcologicalSuccession {
    fn default() -> Self {
        Self {
            stage: SuccessionStage::Bare,
            progress: 0.0,
            time_in_stage: 0.0,
            species_richness: 0.0,
            soil_depth: 0.01,
            soil_organic_matter: 0.001,
            canopy_cover: 0.0,
            disturbance_history: Vec::new(),
        }
    }
}

impl EcologicalSuccession {
    pub fn stage_threshold(&self) -> f32 {
        match self.stage {
            SuccessionStage::Bare => 0.1,
            SuccessionStage::Pioneer => 0.3,
            SuccessionStage::EarlySeral => 0.5,
            SuccessionStage::MidSeral => 0.7,
            SuccessionStage::LateSeral => 0.9,
            SuccessionStage::Climax => 1.0,
            SuccessionStage::Disturbed => 0.0,
        }
    }

    pub fn progress_rate(&self) -> f32 {
        let base_rate = 0.001;
        let soil_factor = self.soil_depth * 10.0;
        let organic_factor = self.soil_organic_matter * 5.0;
        let richness_factor = (self.species_richness * 0.1).min(1.0);
        base_rate * (1.0 + soil_factor + organic_factor + richness_factor)
    }

    pub fn update(&mut self, dt: f32) {
        self.time_in_stage += dt;

        self.soil_depth += 0.0001 * dt;
        self.soil_depth = self.soil_depth.min(2.0);

        self.soil_organic_matter += 0.00001 * self.species_richness * dt;
        self.soil_organic_matter = self.soil_organic_matter.min(0.5);

        self.canopy_cover = self.canopy_cover.clamp(0.0, 1.0);

        if self.stage != SuccessionStage::Climax && self.stage != SuccessionStage::Disturbed {
            self.progress += self.progress_rate() * dt;
            let threshold = self.stage_threshold();

            if self.progress >= threshold {
                self.progress = 0.0;
                self.stage = self.next_stage();
            }
        }

        self.species_richness = match self.stage {
            SuccessionStage::Bare => 0.0,
            SuccessionStage::Pioneer => 5.0,
            SuccessionStage::EarlySeral => 15.0,
            SuccessionStage::MidSeral => 30.0,
            SuccessionStage::LateSeral => 50.0,
            SuccessionStage::Climax => 80.0,
            SuccessionStage::Disturbed => self.species_richness * 0.5,
        };

        self.disturbance_history.retain(|d| d.time_since < 36500.0);
        for d in &mut self.disturbance_history {
            d.time_since += dt;
        }
    }

    pub fn next_stage(&self) -> SuccessionStage {
        match self.stage {
            SuccessionStage::Bare => SuccessionStage::Pioneer,
            SuccessionStage::Pioneer => SuccessionStage::EarlySeral,
            SuccessionStage::EarlySeral => SuccessionStage::MidSeral,
            SuccessionStage::MidSeral => SuccessionStage::LateSeral,
            SuccessionStage::LateSeral => SuccessionStage::Climax,
            SuccessionStage::Climax => SuccessionStage::Climax,
            SuccessionStage::Disturbed => SuccessionStage::Pioneer,
        }
    }

    pub fn apply_disturbance(&mut self, dist_type: DisturbanceType, severity: f32, area: f32) {
        self.disturbance_history.push(DisturbanceEvent {
            event_type: dist_type,
            severity,
            time_since: 0.0,
            area_affected: area,
        });

        if severity > 0.5 {
            self.stage = match dist_type {
                DisturbanceType::VolcanicEruption => SuccessionStage::Bare,
                DisturbanceType::Fire => SuccessionStage::Pioneer,
                DisturbanceType::Landslide => SuccessionStage::Bare,
                DisturbanceType::Flood => SuccessionStage::EarlySeral,
                DisturbanceType::HumanActivity => SuccessionStage::Bare,
                DisturbanceType::Storm => SuccessionStage::MidSeral,
                DisturbanceType::Disease => SuccessionStage::LateSeral,
                DisturbanceType::Drought => SuccessionStage::EarlySeral,
            };
            self.progress = 0.0;
            self.species_richness *= 1.0 - severity;
            self.soil_depth *= 1.0 - severity * 0.5;
            self.soil_organic_matter *= 1.0 - severity * 0.3;
        }
    }

    pub fn biodiversity_index(&self) -> f32 {
        let richness_norm = (self.species_richness / 80.0).min(1.0);
        let canopy_factor = self.canopy_cover;
        let soil_factor = (self.soil_depth / 2.0).min(1.0);
        (richness_norm + canopy_factor + soil_factor) / 3.0
    }
}
