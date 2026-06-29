use glam::Vec3;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disease {
    pub id: Uuid,
    pub name: String,
    pub pathogen: PathogenType,
    pub transmission: Vec<TransmissionRoute>,
    pub incubation_period: f32,
    pub infectious_period: f32,
    pub recovery_period: f32,
    pub mortality_rate: f32,
    pub symptoms: Vec<Symptom>,
    pub mutation_rate: f32,
    pub radiation_boost: f32,
    pub resistance_profile: ResistanceProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathogenType {
    Virus,
    Bacteria,
    Fungus,
    Parasite,
    Prion,
    RadiationInduced,
    Nanite,
    MutagenicAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransmissionRoute {
    Airborne,
    Contact,
    FluidExchange,
    VectorBorne,
    ContaminatedFood,
    ContaminatedWater,
    RadiationExposure,
    VerticalTransmission,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symptom {
    pub name: String,
    pub severity: f32,
    pub onset_time: f32,
    pub duration: f32,
    pub effects: Vec<SymptomEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymptomEffect {
    HealthLoss { rate: f32 },
    StaminaLoss { rate: f32 },
    StatPenalty { stat: String, amount: f32 },
    VisionImpairment { intensity: f32 },
    MovementSlow { factor: f32 },
    Hallucination { frequency: f32 },
    Aggression { level: f32 },
    Mutation { chance: f32 },
    RadiationSensitivity { multiplier: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResistanceProfile {
    pub antibiotic_resistance: f32,
    pub antiviral_resistance: f32,
    pub heat_resistance: f32,
    pub cold_resistance: f32,
    pub radiation_resistance: f32,
    pub mutation_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Infection {
    pub disease_id: Uuid,
    pub host_id: Uuid,
    pub stage: InfectionStage,
    pub progress: f32,
    pub time_infected: f32,
    pub symptom_intensity: f32,
    pub contagiousness: f32,
    pub position: Vec3,
    pub mutations: Vec<DiseaseMutation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfectionStage {
    Exposed,
    Incubating,
    Infectious,
    Symptomatic,
    Severe,
    Recovering,
    Immune,
    Deceased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiseaseMutation {
    pub generation: u32,
    pub mutation_type: MutationType,
    pub effects: Vec<DiseaseMutationEffect>,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationType {
    IncreasedTransmission,
    IncreasedSeverity,
    IncreasedResistance,
    AntigenicShift,
    AntigenicDrift,
    HostAdaptation,
    RadiationInduced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiseaseMutationEffect {
    TransmissionBoost(f32),
    SeverityBoost(f32),
    ResistanceBoost(ResistanceProfile),
    IncubationChange(f32),
    SymptomChange { symptom: String, delta: f32 },
    NewRoute(TransmissionRoute),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiseaseSystem {
    pub diseases: Vec<Disease>,
    pub active_infections: Vec<Infection>,
    pub immunity_records: Vec<ImmunityRecord>,
    pub environmental_hazards: Vec<EnvironmentalHazard>,
    pub epidemic_events: Vec<EpidemicEvent>,
    pub global_health_modifiers: HealthModifiers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmunityRecord {
    pub organism_id: Uuid,
    pub disease_id: Uuid,
    pub immunity_level: f32,
    pub acquired_at: f64,
    pub duration: f32,
    pub natural: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalHazard {
    pub position: Vec3,
    pub radius: f32,
    pub hazard_type: HazardType,
    pub intensity: f32,
    pub duration: f32,
    pub remaining: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazardType {
    RadiationZone,
    ToxicSpill,
    Biohazard,
    ContaminatedWater,
    SporeCloud,
    CorpsePile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpidemicEvent {
    pub disease_id: Uuid,
    pub start_time: f64,
    pub end_time: Option<f64>,
    pub origin: Vec3,
    pub spread_radius: f32,
    pub total_infected: u32,
    pub total_deaths: u32,
    pub containment_measures: Vec<ContainmentMeasure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainmentMeasure {
    Quarantine,
    Isolation,
    Vaccination,
    Culling,
    Sanitation,
    BorderClosure,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthModifiers {
    pub global_immunity_boost: f32,
    pub antibiotic_effectiveness: f32,
    pub sanitation_level: f32,
    pub medical_supply: f32,
    pub radiation_level: f32,
    pub population_density: f32,
}

impl Default for DiseaseSystem {
    fn default() -> Self {
        Self {
            diseases: Vec::new(),
            active_infections: Vec::new(),
            immunity_records: Vec::new(),
            environmental_hazards: Vec::new(),
            epidemic_events: Vec::new(),
            global_health_modifiers: HealthModifiers {
                global_immunity_boost: 0.0,
                antibiotic_effectiveness: 1.0,
                sanitation_level: 0.5,
                medical_supply: 0.5,
                radiation_level: 0.0,
                population_density: 0.0,
            },
        }
    }
}

impl DiseaseSystem {
    pub fn update(&mut self, dt: f32, time: f64) {
        self.update_infections(dt, time);
        self.update_hazards(dt);
        self.check_transmissions(time);
        self.update_epidemics(time);
        self.cleanup_resolved();
    }

    fn update_infections(&mut self, dt: f32, _time: f64) {
        for infection in &mut self.active_infections {
            infection.time_infected += dt;

            let disease = match self.diseases.iter().find(|d| d.id == infection.disease_id) {
                Some(d) => d,
                None => continue,
            };

            infection.contagiousness = if infection.time_infected > disease.incubation_period {
                (infection.time_infected - disease.incubation_period).min(disease.infectious_period)
                    / disease.infectious_period
            } else {
                0.0
            };

            let radiation_factor =
                1.0 + self.global_health_modifiers.radiation_level * disease.radiation_boost;

            infection.progress = infection.time_infected
                / (disease.incubation_period + disease.infectious_period + disease.recovery_period);

            if infection.time_infected < disease.incubation_period {
                infection.stage = InfectionStage::Incubating;
            } else if infection.time_infected
                < disease.incubation_period + disease.infectious_period
            {
                infection.stage = InfectionStage::Infectious;
                infection.symptom_intensity = (infection.time_infected - disease.incubation_period)
                    / disease.infectious_period;
            } else if infection.time_infected
                < disease.incubation_period + disease.infectious_period + disease.recovery_period
            {
                infection.stage = InfectionStage::Severe;
                infection.symptom_intensity = 1.0
                    - (infection.time_infected
                        - disease.incubation_period
                        - disease.infectious_period)
                        / disease.recovery_period;
            } else {
                let mut rng = rand::thread_rng();
                if rng.gen::<f32>() < disease.mortality_rate * radiation_factor {
                    infection.stage = InfectionStage::Deceased;
                } else {
                    infection.stage = InfectionStage::Recovering;
                }
            }

            if disease.mutation_rate > 0.0 {
                let mut rng = rand::thread_rng();
                if rng.gen::<f32>() < disease.mutation_rate * dt * radiation_factor {
                    Self::mutate_disease(infection);
                }
            }
        }
    }

    fn mutate_disease(infection: &mut Infection) {
        let mut rng = rand::thread_rng();
        let mutation_types = [
            MutationType::IncreasedTransmission,
            MutationType::IncreasedSeverity,
            MutationType::IncreasedResistance,
            MutationType::AntigenicDrift,
            MutationType::RadiationInduced,
        ];

        let mtype = mutation_types[rng.gen_range(0..mutation_types.len())];
        let effects = match mtype {
            MutationType::IncreasedTransmission => {
                vec![DiseaseMutationEffect::TransmissionBoost(rng.gen_range(0.1..0.5))]
            },
            MutationType::IncreasedSeverity => {
                vec![DiseaseMutationEffect::SeverityBoost(rng.gen_range(0.1..0.3))]
            },
            MutationType::IncreasedResistance => {
                vec![DiseaseMutationEffect::ResistanceBoost(ResistanceProfile {
                    antibiotic_resistance: rng.gen_range(0.0..0.3),
                    antiviral_resistance: rng.gen_range(0.0..0.3),
                    heat_resistance: rng.gen_range(0.0..0.2),
                    cold_resistance: rng.gen_range(0.0..0.2),
                    radiation_resistance: rng.gen_range(0.0..0.5),
                    mutation_rate: rng.gen_range(0.0..0.1),
                })]
            },
            _ => vec![DiseaseMutationEffect::TransmissionBoost(rng.gen_range(0.05..0.15))],
        };

        infection.mutations.push(DiseaseMutation {
            generation: infection.mutations.len() as u32 + 1,
            mutation_type: mtype,
            effects,
            timestamp: infection.time_infected as f64,
        });
    }

    fn update_hazards(&mut self, dt: f32) {
        for hazard in &mut self.environmental_hazards {
            hazard.remaining -= dt;
        }
        self.environmental_hazards.retain(|h| h.remaining > 0.0);
    }

    fn check_transmissions(&mut self, _time: f64) {
        let mut new_infections = Vec::new();

        for i in 0..self.active_infections.len() {
            let infection = &self.active_infections[i];
            if infection.stage != InfectionStage::Infectious
                && infection.stage != InfectionStage::Symptomatic
            {
                continue;
            }

            let disease = match self.diseases.iter().find(|d| d.id == infection.disease_id) {
                Some(d) => d,
                None => continue,
            };

            let transmission_radius = 5.0 * infection.contagiousness;

            for j in 0..self.active_infections.len() {
                if i == j {
                    continue;
                }
                let other = &self.active_infections[j];
                if other.disease_id != infection.disease_id {
                    continue;
                }
                let dist = (other.position - infection.position).length();
                if dist < transmission_radius {
                    let mut rng = rand::thread_rng();
                    let transmission_chance =
                        infection.contagiousness * (1.0 - dist / transmission_radius);
                    if rng.gen::<f32>() < transmission_chance * 0.01 {
                        let has_immunity = self.immunity_records.iter().any(|ir| {
                            ir.organism_id == other.host_id
                                && ir.disease_id == disease.id
                                && ir.immunity_level > 0.5
                        });
                        if !has_immunity {
                            let new_infection = Infection {
                                disease_id: disease.id,
                                host_id: other.host_id,
                                stage: InfectionStage::Exposed,
                                progress: 0.0,
                                time_infected: 0.0,
                                symptom_intensity: 0.0,
                                contagiousness: 0.0,
                                position: other.position,
                                mutations: Vec::new(),
                            };
                            new_infections.push(new_infection);
                        }
                    }
                }
            }
        }

        self.active_infections.extend(new_infections);
    }

    fn update_epidemics(&mut self, _time: f64) {
        for epidemic in &mut self.epidemic_events {
            if epidemic.end_time.is_none() {
                let infected_count = self
                    .active_infections
                    .iter()
                    .filter(|i| i.disease_id == epidemic.disease_id)
                    .count() as u32;
                epidemic.total_infected = infected_count;

                let deceased_count = self
                    .active_infections
                    .iter()
                    .filter(|i| {
                        i.disease_id == epidemic.disease_id && i.stage == InfectionStage::Deceased
                    })
                    .count() as u32;
                epidemic.total_deaths = deceased_count;
            }
        }
    }

    fn cleanup_resolved(&mut self) {
        self.active_infections
            .retain(|i| i.stage != InfectionStage::Deceased && i.stage != InfectionStage::Immune);
    }

    pub fn infect_organism(&mut self, disease_id: Uuid, host_id: Uuid, position: Vec3) {
        let infection = Infection {
            disease_id,
            host_id,
            stage: InfectionStage::Exposed,
            progress: 0.0,
            time_infected: 0.0,
            symptom_intensity: 0.0,
            contagiousness: 0.0,
            position,
            mutations: Vec::new(),
        };
        self.active_infections.push(infection);
    }

    pub fn add_hazard(
        &mut self,
        position: Vec3,
        radius: f32,
        hazard_type: HazardType,
        intensity: f32,
        duration: f32,
    ) {
        self.environmental_hazards.push(EnvironmentalHazard {
            position,
            radius,
            hazard_type,
            intensity,
            duration,
            remaining: duration,
        });
    }

    pub fn grant_immunity(&mut self, organism_id: Uuid, disease_id: Uuid, natural: bool) {
        self.immunity_records.push(ImmunityRecord {
            organism_id,
            disease_id,
            immunity_level: if natural { 0.8 } else { 1.0 },
            acquired_at: 0.0,
            duration: if natural { 86400.0 * 30.0 } else { 86400.0 * 180.0 },
            natural,
        });
    }

    pub fn active_epidemic_count(&self) -> usize {
        self.epidemic_events.iter().filter(|e| e.end_time.is_none()).count()
    }

    pub fn total_infected(&self) -> usize {
        self.active_infections.len()
    }

    pub fn hazard_count(&self) -> usize {
        self.environmental_hazards.len()
    }
}

impl Disease {
    pub fn radiation_sickness() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "急性辐射综合征".to_string(),
            pathogen: PathogenType::RadiationInduced,
            transmission: vec![TransmissionRoute::RadiationExposure],
            incubation_period: 3600.0,
            infectious_period: 0.0,
            recovery_period: 86400.0 * 7.0,
            mortality_rate: 0.7,
            symptoms: vec![
                Symptom {
                    name: "细胞坏死".to_string(),
                    severity: 0.8,
                    onset_time: 3600.0,
                    duration: 86400.0 * 30.0,
                    effects: vec![SymptomEffect::HealthLoss { rate: 0.02 }],
                },
                Symptom {
                    name: "基因突变".to_string(),
                    severity: 0.5,
                    onset_time: 7200.0,
                    duration: 86400.0 * 365.0,
                    effects: vec![
                        SymptomEffect::Mutation { chance: 0.1 },
                        SymptomEffect::RadiationSensitivity { multiplier: 2.0 },
                    ],
                },
            ],
            mutation_rate: 0.0,
            radiation_boost: 0.0,
            resistance_profile: ResistanceProfile {
                antibiotic_resistance: 0.0,
                antiviral_resistance: 0.0,
                heat_resistance: 0.3,
                cold_resistance: 0.5,
                radiation_resistance: 0.0,
                mutation_rate: 0.0,
            },
        }
    }

    pub fn wasteland_plague() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "废土瘟疫".to_string(),
            pathogen: PathogenType::Bacteria,
            transmission: vec![
                TransmissionRoute::Airborne,
                TransmissionRoute::Contact,
                TransmissionRoute::ContaminatedWater,
            ],
            incubation_period: 86400.0 * 2.0,
            infectious_period: 86400.0 * 5.0,
            recovery_period: 86400.0 * 10.0,
            mortality_rate: 0.3,
            symptoms: vec![
                Symptom {
                    name: "发热".to_string(),
                    severity: 0.6,
                    onset_time: 86400.0,
                    duration: 86400.0 * 5.0,
                    effects: vec![
                        SymptomEffect::HealthLoss { rate: 0.01 },
                        SymptomEffect::StaminaLoss { rate: 0.02 },
                    ],
                },
                Symptom {
                    name: "器官衰竭".to_string(),
                    severity: 0.9,
                    onset_time: 86400.0 * 4.0,
                    duration: 86400.0 * 7.0,
                    effects: vec![
                        SymptomEffect::HealthLoss { rate: 0.05 },
                        SymptomEffect::MovementSlow { factor: 0.5 },
                    ],
                },
            ],
            mutation_rate: 0.001,
            radiation_boost: 2.0,
            resistance_profile: ResistanceProfile {
                antibiotic_resistance: 0.6,
                antiviral_resistance: 0.0,
                heat_resistance: 0.4,
                cold_resistance: 0.3,
                radiation_resistance: 0.8,
                mutation_rate: 0.001,
            },
        }
    }

    pub fn ghoul_rot() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "尸鬼化综合症".to_string(),
            pathogen: PathogenType::RadiationInduced,
            transmission: vec![TransmissionRoute::RadiationExposure, TransmissionRoute::Contact],
            incubation_period: 86400.0 * 30.0,
            infectious_period: 0.0,
            recovery_period: 86400.0 * 365.0,
            mortality_rate: 0.1,
            symptoms: vec![
                Symptom {
                    name: "皮肤坏死".to_string(),
                    severity: 0.7,
                    onset_time: 86400.0 * 10.0,
                    duration: 86400.0 * 365.0,
                    effects: vec![
                        SymptomEffect::RadiationSensitivity { multiplier: 0.1 },
                        SymptomEffect::HealthLoss { rate: 0.001 },
                    ],
                },
                Symptom {
                    name: "认知退化".to_string(),
                    severity: 0.5,
                    onset_time: 86400.0 * 60.0,
                    duration: 86400.0 * 730.0,
                    effects: vec![
                        SymptomEffect::Aggression { level: 0.3 },
                        SymptomEffect::Hallucination { frequency: 0.1 },
                    ],
                },
            ],
            mutation_rate: 0.0001,
            radiation_boost: 5.0,
            resistance_profile: ResistanceProfile {
                antibiotic_resistance: 0.0,
                antiviral_resistance: 0.0,
                heat_resistance: 0.8,
                cold_resistance: 0.9,
                radiation_resistance: 1.0,
                mutation_rate: 0.0,
            },
        }
    }
}

// =====================================================================
// 辅助查询方法 —— 纯查询，不修改状态
// =====================================================================

/// 疾病严重度分级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeverityLevel {
    Mild,
    Moderate,
    Severe,
    Critical,
}

impl SeverityLevel {
    /// 数值化严重度（0-3）
    pub fn as_index(&self) -> u8 {
        match self {
            SeverityLevel::Mild => 0,
            SeverityLevel::Moderate => 1,
            SeverityLevel::Severe => 2,
            SeverityLevel::Critical => 3,
        }
    }
}

impl Disease {
    /// 基于死亡率和症状严重度的综合分级
    pub fn severity_level(&self) -> SeverityLevel {
        let m = self.mortality_rate;
        let max_sym = self.max_symptom_severity();
        let score = m * 0.6 + max_sym * 0.4;
        if score >= 0.7 {
            SeverityLevel::Critical
        } else if score >= 0.5 {
            SeverityLevel::Severe
        } else if score >= 0.3 {
            SeverityLevel::Moderate
        } else {
            SeverityLevel::Mild
        }
    }

    /// 是否具有传染性（有传播途径且传染期 > 0）
    pub fn is_contagious(&self) -> bool {
        !self.transmission.is_empty() && self.infectious_period > 0.0
    }

    /// 是否可治疗（基于病原体类型与耐药性）
    pub fn is_treatable(&self) -> bool {
        match self.pathogen {
            PathogenType::Bacteria => self.resistance_profile.antibiotic_resistance < 0.7,
            PathogenType::Virus => self.resistance_profile.antiviral_resistance < 0.7,
            PathogenType::Fungus | PathogenType::Parasite => true,
            PathogenType::RadiationInduced => false,
            PathogenType::Prion => false,
            PathogenType::Nanite | PathogenType::MutagenicAgent => true,
        }
    }

    /// 死亡率（同 mortality_rate 字段，提供查询语义）
    pub fn mortality_fraction(&self) -> f32 {
        self.mortality_rate.clamp(0.0, 1.0)
    }

    /// 估算基本再生数 R0（基于传染期、传播途径数与突变率）
    pub fn r0_estimate(&self) -> f32 {
        if !self.is_contagious() {
            return 0.0;
        }
        let route_factor = self.transmission.len() as f32;
        let base = (self.infectious_period / 86400.0).max(0.0) * 0.3;
        let mutation_factor = 1.0 + self.mutation_rate * 100.0;
        (base * route_factor * mutation_factor).max(0.0)
    }

    /// 总病程时长（潜伏 + 传染 + 恢复，秒）
    pub fn total_duration(&self) -> f32 {
        self.incubation_period + self.infectious_period + self.recovery_period
    }

    /// 最大症状严重度
    pub fn max_symptom_severity(&self) -> f32 {
        self.symptoms
            .iter()
            .map(|s| s.severity)
            .fold(0.0f32, f32::max)
            .clamp(0.0, 1.0)
    }

    /// 是否包含某传播途径
    pub fn has_route(&self, route: TransmissionRoute) -> bool {
        self.transmission.contains(&route)
    }
}
impl Infection {
    /// 是否已死亡
    pub fn is_terminal(&self) -> bool {
        matches!(self.stage, InfectionStage::Deceased)
    }

    /// 是否已恢复（Recovering 或 Immune）
    pub fn is_recovered(&self) -> bool {
        matches!(self.stage, InfectionStage::Recovering | InfectionStage::Immune)
    }

    /// 进度分数 ∈ [0, 1]
    pub fn progress_fraction(&self) -> f32 {
        self.progress.clamp(0.0, 1.0)
    }

    /// 突变计数
    pub fn mutation_count(&self) -> usize {
        self.mutations.len()
    }

    /// 是否仍处于可传染阶段
    pub fn is_contagious_stage(&self) -> bool {
        matches!(
            self.stage,
            InfectionStage::Infectious | InfectionStage::Symptomatic
        )
    }
}

impl DiseaseSystem {
    /// 按阶段统计感染数
    pub fn infected_count_by_stage(&self, stage: InfectionStage) -> usize {
        self.active_infections
            .iter()
            .filter(|i| i.stage == stage)
            .count()
    }

    /// 累计死亡数（基于疫情事件）
    pub fn total_deaths(&self) -> u32 {
        self.epidemic_events.iter().map(|e| e.total_deaths).sum()
    }

    /// 是否处于流行状态
    pub fn is_epidemic(&self) -> bool {
        self.active_epidemic_count() > 0
    }

    /// 观察到的致死率（死亡 / 总感染）
    pub fn observed_mortality_rate(&self) -> f32 {
        let total = self.total_infected();
        if total == 0 {
            return 0.0;
        }
        let deceased = self.infected_count_by_stage(InfectionStage::Deceased);
        deceased as f32 / total as f32
    }
}

impl ResistanceProfile {
    /// 综合耐药性（所有耐药项的平均）
    pub fn overall_resistance(&self) -> f32 {
        let sum = self.antibiotic_resistance
            + self.antiviral_resistance
            + self.heat_resistance
            + self.cold_resistance
            + self.radiation_resistance;
        (sum / 5.0).clamp(0.0, 1.0)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radiation_sickness_fields() {
        let d = Disease::radiation_sickness();
        assert_eq!(d.pathogen, PathogenType::RadiationInduced);
        assert!((d.mortality_rate - 0.7).abs() < 1e-5);
        assert_eq!(d.infectious_period, 0.0);
        assert!(!d.is_contagious());
        assert!(!d.is_treatable(), "radiation sickness not treatable");
    }

    #[test]
    fn test_wasteland_plague_contagious() {
        let d = Disease::wasteland_plague();
        assert_eq!(d.pathogen, PathogenType::Bacteria);
        assert!(d.infectious_period > 0.0);
        assert!(!d.transmission.is_empty());
        assert!(d.is_contagious());
        assert!(d.is_treatable(), "antibiotic_resistance=0.6 < 0.7");
        assert!(d.has_route(TransmissionRoute::Airborne));
        assert!(d.has_route(TransmissionRoute::ContaminatedWater));
        assert!(!d.has_route(TransmissionRoute::VectorBorne));
    }

    #[test]
    fn test_ghoul_rot_resistance() {
        let d = Disease::ghoul_rot();
        assert_eq!(d.pathogen, PathogenType::RadiationInduced);
        assert!((d.resistance_profile.radiation_resistance - 1.0).abs() < 1e-5);
        assert!((d.resistance_profile.cold_resistance - 0.9).abs() < 1e-5);
    }

    #[test]
    fn test_severity_level_classification() {
        let rs = Disease::radiation_sickness();
        let lvl = rs.severity_level();
        assert_eq!(lvl, SeverityLevel::Critical);
        assert_eq!(lvl.as_index(), 3);
        let wp = Disease::wasteland_plague();
        assert_eq!(wp.severity_level(), SeverityLevel::Severe);
    }

    #[test]
    fn test_r0_estimate_positive_for_contagious() {
        let wp = Disease::wasteland_plague();
        let r0 = wp.r0_estimate();
        assert!(r0 > 0.0, "wasteland plague R0 should be positive");
        let rs = Disease::radiation_sickness();
        assert_eq!(rs.r0_estimate(), 0.0);
    }

    #[test]
    fn test_total_duration_and_max_symptom() {
        let wp = Disease::wasteland_plague();
        let total = wp.total_duration();
        assert!(total > 0.0);
        assert_eq!(total, wp.incubation_period + wp.infectious_period + wp.recovery_period);
        let max_s = wp.max_symptom_severity();
        assert!((max_s - 0.9).abs() < 1e-5);
    }

    #[test]
    fn test_mortality_fraction() {
        let rs = Disease::radiation_sickness();
        assert!((rs.mortality_fraction() - 0.7).abs() < 1e-5);
    }

    #[test]
    fn test_disease_system_default_empty() {
        let s = DiseaseSystem::default();
        assert_eq!(s.diseases.len(), 0);
        assert_eq!(s.active_infections.len(), 0);
        assert_eq!(s.immunity_records.len(), 0);
        assert_eq!(s.environmental_hazards.len(), 0);
        assert_eq!(s.epidemic_events.len(), 0);
        assert_eq!(s.total_infected(), 0);
        assert_eq!(s.hazard_count(), 0);
        assert!(!s.is_epidemic());
        assert_eq!(s.observed_mortality_rate(), 0.0);
        assert_eq!(s.total_deaths(), 0);
    }

    #[test]
    fn test_infect_organism_adds_infection() {
        let mut s = DiseaseSystem::default();
        let did = Uuid::new_v4();
        let hid = Uuid::new_v4();
        s.infect_organism(did, hid, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(s.total_infected(), 1);
        let inf = &s.active_infections[0];
        assert_eq!(inf.disease_id, did);
        assert_eq!(inf.host_id, hid);
        assert_eq!(inf.stage, InfectionStage::Exposed);
        assert_eq!(inf.progress, 0.0);
        assert_eq!(inf.mutation_count(), 0);
        assert!(!inf.is_terminal());
        assert!(!inf.is_recovered());
        assert!(!inf.is_contagious_stage());
        assert_eq!(inf.progress_fraction(), 0.0);
    }

    #[test]
    fn test_grant_immunity_and_records() {
        let mut s = DiseaseSystem::default();
        let oid = Uuid::new_v4();
        let did = Uuid::new_v4();
        s.grant_immunity(oid, did, true);
        assert_eq!(s.immunity_records.len(), 1);
        let r = &s.immunity_records[0];
        assert!(r.natural);
        assert!((r.immunity_level - 0.8).abs() < 1e-5);
        s.grant_immunity(oid, did, false);
        assert_eq!(s.immunity_records.len(), 2);
        assert!((s.immunity_records[1].immunity_level - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_add_hazard_and_count() {
        let mut s = DiseaseSystem::default();
        s.add_hazard(
            Vec3::new(1.0, 2.0, 3.0),
            5.0,
            HazardType::RadiationZone,
            0.8,
            60.0,
        );
        assert_eq!(s.hazard_count(), 1);
        let h = &s.environmental_hazards[0];
        assert_eq!(h.hazard_type, HazardType::RadiationZone);
        assert!((h.remaining - 60.0).abs() < 1e-5);
        assert!((h.intensity - 0.8).abs() < 1e-5);
    }

    #[test]
    fn test_infected_count_by_stage() {
        let mut s = DiseaseSystem::default();
        let did = Uuid::new_v4();
        s.infect_organism(did, Uuid::new_v4(), Vec3::ZERO);
        s.infect_organism(did, Uuid::new_v4(), Vec3::ZERO);
        assert_eq!(s.infected_count_by_stage(InfectionStage::Exposed), 2);
        assert_eq!(s.infected_count_by_stage(InfectionStage::Deceased), 0);
    }

    #[test]
    fn test_resistance_profile_overall() {
        let rs = Disease::radiation_sickness().resistance_profile;
        let overall = rs.overall_resistance();
        assert!(overall >= 0.0 && overall <= 1.0);
        assert!((overall - 0.16).abs() < 1e-5);
    }

    #[test]
    fn test_infection_query_methods() {
        let mut inf = Infection {
            disease_id: Uuid::new_v4(),
            host_id: Uuid::new_v4(),
            stage: InfectionStage::Infectious,
            progress: 0.5,
            time_infected: 100.0,
            symptom_intensity: 0.7,
            contagiousness: 0.8,
            position: Vec3::ZERO,
            mutations: vec![DiseaseMutation {
                generation: 1,
                mutation_type: MutationType::AntigenicDrift,
                effects: vec![],
                timestamp: 100.0,
            }],
        };
        assert!(inf.is_contagious_stage());
        assert!(!inf.is_terminal());
        assert!(!inf.is_recovered());
        assert_eq!(inf.mutation_count(), 1);
        assert!((inf.progress_fraction() - 0.5).abs() < 1e-5);
        inf.stage = InfectionStage::Deceased;
        assert!(inf.is_terminal());
        inf.stage = InfectionStage::Immune;
        assert!(inf.is_recovered());
    }
}
