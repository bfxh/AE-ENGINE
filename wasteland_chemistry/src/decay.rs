use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Radioisotope {
    pub id: Uuid,
    pub name: String,
    pub symbol: String,
    pub atomic_number: u32,
    pub mass_number: u32,
    pub half_life: f32,
    pub decay_type: DecayType,
    pub decay_energy: f32,
    pub daughter_isotope: Option<String>,
    pub branching_ratio: f32,
    pub gamma_energy: f32,
    pub neutron_emission: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecayType {
    Alpha,
    BetaMinus,
    BetaPlus,
    ElectronCapture,
    Gamma,
    SpontaneousFission,
    NeutronEmission,
    ProtonEmission,
    IsomericTransition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayChain {
    pub id: Uuid,
    pub name: String,
    pub isotopes: Vec<Radioisotope>,
    pub start_isotope: String,
    pub stable_endpoint: String,
    pub total_steps: u32,
    pub total_energy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioactiveSample {
    pub isotope_id: Uuid,
    pub position: Vec3,
    pub initial_amount: f32,
    pub current_amount: f32,
    pub activity: f32,
    pub accumulated_dose: f32,
    pub start_time: f64,
    pub shielding: ShieldingMaterial,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShieldingMaterial {
    None,
    Paper,
    Aluminum { thickness: f32 },
    Lead { thickness: f32 },
    Concrete { thickness: f32 },
    Water { thickness: f32 },
    BoratedPolyethylene { thickness: f32 },
    DepletedUranium { thickness: f32 },
}

impl ShieldingMaterial {
    pub fn attenuation_factor(&self, decay_type: DecayType, _energy: f32) -> f32 {
        match (self, decay_type) {
            (Self::None, _) => 1.0,
            (Self::Paper, DecayType::Alpha) => 0.0,
            (Self::Aluminum { thickness }, DecayType::BetaMinus | DecayType::BetaPlus) => {
                (-*thickness / 5.0).exp()
            },
            (Self::Lead { thickness }, DecayType::Gamma) => (-*thickness / 1.0).exp(),
            (Self::Concrete { thickness }, DecayType::Gamma) => (-*thickness / 6.0).exp(),
            (Self::Concrete { thickness }, DecayType::NeutronEmission) => {
                (-*thickness / 10.0).exp()
            },
            (Self::Water { thickness }, DecayType::NeutronEmission) => (-*thickness / 5.0).exp(),
            (Self::BoratedPolyethylene { thickness }, DecayType::NeutronEmission) => {
                (-*thickness / 3.0).exp()
            },
            (Self::DepletedUranium { thickness }, DecayType::Gamma) => (-*thickness / 0.5).exp(),
            _ => 1.0,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::None => "无防护",
            Self::Paper => "纸张",
            Self::Aluminum { .. } => "铝板",
            Self::Lead { .. } => "铅板",
            Self::Concrete { .. } => "混凝土",
            Self::Water { .. } => "水",
            Self::BoratedPolyethylene { .. } => "硼化聚乙烯",
            Self::DepletedUranium { .. } => "贫铀",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiationField {
    pub position: Vec3,
    pub radius: f32,
    pub dose_rate: f32,
    pub decay_chain: Option<Uuid>,
    pub source_isotope: Option<Uuid>,
    pub half_life: f32,
    pub created_at: f64,
    pub field_type: RadiationFieldType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadiationFieldType {
    PointSource,
    AreaContamination,
    FalloutZone,
    ReactorLeak,
    DirtyBomb,
    NuclearDetonation,
    NaturalDeposit,
    WasteStorage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioactiveDecaySystem {
    pub samples: Vec<RadioactiveSample>,
    pub radiation_fields: Vec<RadiationField>,
    pub decay_chains: Vec<DecayChain>,
    pub global_background_radiation: f32,
    pub active_decay_events: Vec<DecayEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayEvent {
    pub sample_id: Uuid,
    pub from_isotope: String,
    pub to_isotope: String,
    pub decay_type: DecayType,
    pub energy_released: f32,
    pub position: Vec3,
    pub timestamp: f64,
}

impl Default for RadioactiveDecaySystem {
    fn default() -> Self {
        Self {
            samples: Vec::new(),
            radiation_fields: Vec::new(),
            decay_chains: Vec::new(),
            global_background_radiation: 0.001,
            active_decay_events: Vec::new(),
        }
    }
}

impl RadioactiveDecaySystem {
    pub fn update(&mut self, dt: f32, time: f64) {
        self.update_samples(dt, time);
        self.update_radiation_fields(dt, time);
        self.cleanup_depleted();
    }

    fn update_samples(&mut self, dt: f32, time: f64) {
        for sample in &mut self.samples {
            let isotope = match self
                .decay_chains
                .iter()
                .flat_map(|c| c.isotopes.iter())
                .find(|i| i.id == sample.isotope_id)
            {
                Some(i) => i.clone(),
                None => continue,
            };

            let decay_constant = (2.0f32).ln() / isotope.half_life;
            let decayed = sample.current_amount * (1.0 - (-decay_constant * dt).exp());
            sample.current_amount -= decayed;
            sample.activity = decayed / dt;

            let shield_factor =
                sample.shielding.attenuation_factor(isotope.decay_type, isotope.decay_energy);
            sample.accumulated_dose += decayed * isotope.decay_energy * shield_factor * 0.01;

            if decayed > 0.001 && isotope.daughter_isotope.is_some() {
                self.active_decay_events.push(DecayEvent {
                    sample_id: sample.isotope_id,
                    from_isotope: isotope.symbol.clone(),
                    to_isotope: isotope.daughter_isotope.clone().unwrap_or_default(),
                    decay_type: isotope.decay_type,
                    energy_released: isotope.decay_energy * decayed,
                    position: sample.position,
                    timestamp: time,
                });
            }
        }
    }

    fn update_radiation_fields(&mut self, dt: f32, _time: f64) {
        for field in &mut self.radiation_fields {
            let decay_constant = (2.0f32).ln() / field.half_life;
            field.dose_rate *= (-decay_constant * dt).exp();

            if field.dose_rate < 0.001 {
                field.dose_rate = 0.0;
            }
        }
    }

    fn cleanup_depleted(&mut self) {
        self.samples.retain(|s| s.current_amount > 1e-6);
        self.radiation_fields.retain(|f| f.dose_rate > 0.001);
        if self.active_decay_events.len() > 1000 {
            self.active_decay_events.drain(0..500);
        }
    }

    pub fn add_sample(
        &mut self,
        isotope: Radioisotope,
        position: Vec3,
        amount: f32,
        shielding: ShieldingMaterial,
    ) {
        let activity = amount * (2.0f32).ln() / isotope.half_life;
        self.samples.push(RadioactiveSample {
            isotope_id: isotope.id,
            position,
            initial_amount: amount,
            current_amount: amount,
            activity,
            accumulated_dose: 0.0,
            start_time: 0.0,
            shielding,
        });
    }

    pub fn add_radiation_field(
        &mut self,
        position: Vec3,
        radius: f32,
        dose_rate: f32,
        half_life: f32,
        field_type: RadiationFieldType,
    ) {
        self.radiation_fields.push(RadiationField {
            position,
            radius,
            dose_rate,
            decay_chain: None,
            source_isotope: None,
            half_life,
            created_at: 0.0,
            field_type,
        });
    }

    pub fn dose_at_position(&self, position: Vec3) -> f32 {
        let mut total_dose = self.global_background_radiation;

        for field in &self.radiation_fields {
            let dist = (position - field.position).length();
            if dist < field.radius {
                let falloff = 1.0 - dist / field.radius;
                total_dose += field.dose_rate * falloff * falloff;
            }
        }

        for sample in &self.samples {
            let dist = (position - sample.position).length();
            if dist < 50.0 {
                let falloff = 1.0 - dist / 50.0;
                total_dose += sample.activity * falloff * 0.01;
            }
        }

        total_dose
    }

    pub fn create_fallout_field(&mut self, position: Vec3, radius: f32, yield_kt: f32) {
        let dose_rate = yield_kt * 100.0;
        let half_life = 86400.0 * 7.0;

        self.add_radiation_field(
            position,
            radius,
            dose_rate,
            half_life,
            RadiationFieldType::FalloutZone,
        );
    }

    pub fn active_samples(&self) -> usize {
        self.samples.len()
    }

    pub fn active_fields(&self) -> usize {
        self.radiation_fields.iter().filter(|f| f.dose_rate > 0.001).count()
    }

    pub fn total_activity(&self) -> f32 {
        self.samples.iter().map(|s| s.activity).sum()
    }
}

impl Radioisotope {
    pub fn uranium_238() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "铀-238".to_string(),
            symbol: "U-238".to_string(),
            atomic_number: 92,
            mass_number: 238,
            half_life: 1.41e17,
            decay_type: DecayType::Alpha,
            decay_energy: 4.27,
            daughter_isotope: Some("Th-234".to_string()),
            branching_ratio: 1.0,
            gamma_energy: 0.05,
            neutron_emission: false,
        }
    }

    pub fn plutonium_239() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "钚-239".to_string(),
            symbol: "Pu-239".to_string(),
            atomic_number: 94,
            mass_number: 239,
            half_life: 7.6e11,
            decay_type: DecayType::Alpha,
            decay_energy: 5.24,
            daughter_isotope: Some("U-235".to_string()),
            branching_ratio: 1.0,
            gamma_energy: 0.13,
            neutron_emission: true,
        }
    }

    pub fn cesium_137() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "铯-137".to_string(),
            symbol: "Cs-137".to_string(),
            atomic_number: 55,
            mass_number: 137,
            half_life: 9.5e8,
            decay_type: DecayType::BetaMinus,
            decay_energy: 1.17,
            daughter_isotope: Some("Ba-137m".to_string()),
            branching_ratio: 0.946,
            gamma_energy: 0.662,
            neutron_emission: false,
        }
    }

    pub fn strontium_90() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "锶-90".to_string(),
            symbol: "Sr-90".to_string(),
            atomic_number: 38,
            mass_number: 90,
            half_life: 9.1e8,
            decay_type: DecayType::BetaMinus,
            decay_energy: 0.546,
            daughter_isotope: Some("Y-90".to_string()),
            branching_ratio: 1.0,
            gamma_energy: 0.0,
            neutron_emission: false,
        }
    }

    pub fn iodine_131() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "碘-131".to_string(),
            symbol: "I-131".to_string(),
            atomic_number: 53,
            mass_number: 131,
            half_life: 6.95e5,
            decay_type: DecayType::BetaMinus,
            decay_energy: 0.971,
            daughter_isotope: Some("Xe-131".to_string()),
            branching_ratio: 1.0,
            gamma_energy: 0.364,
            neutron_emission: false,
        }
    }

    pub fn cobalt_60() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "钴-60".to_string(),
            symbol: "Co-60".to_string(),
            atomic_number: 27,
            mass_number: 60,
            half_life: 1.66e8,
            decay_type: DecayType::BetaMinus,
            decay_energy: 2.82,
            daughter_isotope: Some("Ni-60".to_string()),
            branching_ratio: 1.0,
            gamma_energy: 1.33,
            neutron_emission: false,
        }
    }
}

impl DecayChain {
    pub fn uranium_series() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "铀衰变链".to_string(),
            isotopes: vec![
                Radioisotope::uranium_238(),
                Radioisotope {
                    id: Uuid::new_v4(),
                    name: "钍-234".to_string(),
                    symbol: "Th-234".to_string(),
                    atomic_number: 90,
                    mass_number: 234,
                    half_life: 2.08e6,
                    decay_type: DecayType::BetaMinus,
                    decay_energy: 0.27,
                    daughter_isotope: Some("Pa-234m".to_string()),
                    branching_ratio: 1.0,
                    gamma_energy: 0.09,
                    neutron_emission: false,
                },
            ],
            start_isotope: "U-238".to_string(),
            stable_endpoint: "Pb-206".to_string(),
            total_steps: 14,
            total_energy: 51.7,
        }
    }
}
