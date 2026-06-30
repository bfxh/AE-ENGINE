use glam::Vec3;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEntity {
    pub id: Uuid,
    pub version: u64,
    pub position: Vec3,
    pub rotation: glam::Quat,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub physics: PhysicsAttributes,
    pub chemistry: ChemistryAttributes,
    pub biology: BiologyAttributes,
    pub state: MetaEntityState,
    pub structural_field: Option<StructuralFieldParams>,
    pub spawn_tick: u64,
    pub parent_id: Option<Uuid>,
    pub children: SmallVec<[Uuid; 4]>,
    pub extensions: HashMap<String, ExtensionValue>,
    pub mpss_index: Option<usize>, // Phase 6: unified particle field index
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionValue {
    Float(f32),
    Int(i64),
    String(String),
    Bool(bool),
    Vec3(Vec3),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityChanges {
    pub entity_id: Uuid,
    pub expected_version: u64,
    pub position: Option<Vec3>,
    pub velocity: Option<Vec3>,
    pub angular_velocity: Option<Vec3>,
    pub physics_changes: HashMap<String, f32>,
    pub chemistry_changes: HashMap<String, f32>,
    pub biology_changes: HashMap<String, f32>,
    pub state_change: Option<MetaEntityState>,
    pub extension_changes: HashMap<String, ExtensionValue>,
    pub spawn_children: Vec<MetaEntity>,
    pub despawn: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhysicsAttributes {
    pub mass: f32,
    pub density: f32,
    pub hardness: f32,
    pub toughness: f32,
    pub elastic_modulus: f32,
    pub yield_strength: f32,
    pub ultimate_strength: f32,
    pub poisson_ratio: f32,
    pub friction_coefficient: f32,
    pub restitution: f32,
    pub temperature: f32,
    pub thermal_conductivity: f32,
    pub specific_heat_capacity: f32,
    pub electrical_conductivity: f32,
    pub magnetic_permeability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemistryAttributes {
    pub elemental_composition: Vec<ElementFraction>,
    pub bond_types: Vec<ChemicalBond>,
    pub reactivity: f32,
    pub ph: f32,
    pub redox_potential: f32,
    pub oxidation_state: f32,
    pub corrosion_depth: f32,
    pub chemical_stain: u8,
    pub solubility: f32,
    pub flammability: f32,
    pub toxicity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiologyAttributes {
    pub gene_tokens: Vec<GeneToken>,
    pub metabolic_rate: f32,
    pub growth_rate: f32,
    pub repair_rate: f32,
    pub neural_signal_strength: f32,
    pub health: f32,
    pub max_health: f32,
    pub radiation_dose: f32,
    pub toxin_level: f32,
    pub nutrient_level: f32,
    pub hydration: f32,
    pub cell_type: CellType,
    pub tissue_density: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetaEntityState {
    Active,
    Sleeping,
    Frozen,
    Destroyed,
    PhaseTransitioning,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ElementFraction {
    pub element: Element,
    pub fraction: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Element {
    H,
    He,
    Li,
    Be,
    B,
    C,
    N,
    O,
    F,
    Ne,
    Na,
    Mg,
    Al,
    Si,
    P,
    S,
    Cl,
    Ar,
    K,
    Ca,
    Sc,
    Ti,
    V,
    Cr,
    Mn,
    Fe,
    Co,
    Ni,
    Cu,
    Zn,
    Ga,
    Ge,
    As,
    Se,
    Br,
    Kr,
    Rb,
    Sr,
    Y,
    Zr,
    Nb,
    Mo,
    Tc,
    Ru,
    Rh,
    Pd,
    Ag,
    Cd,
    In,
    Sn,
    Sb,
    Te,
    I,
    Xe,
    Cs,
    Ba,
    La,
    Ce,
    Pr,
    Nd,
    Pm,
    Sm,
    Eu,
    Gd,
    Tb,
    Dy,
    Ho,
    Er,
    Tm,
    Yb,
    Lu,
    Hf,
    Ta,
    W,
    Re,
    Os,
    Ir,
    Pt,
    Au,
    Hg,
    Tl,
    Pb,
    Bi,
    Po,
    At,
    Rn,
    Fr,
    Ra,
    Ac,
    Th,
    Pa,
    U,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChemicalBond {
    Ionic,
    Covalent,
    Metallic,
    Hydrogen,
    VanDerWaals,
    PiBond,
    SigmaBond,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneToken {
    pub name: String,
    pub expression_level: f32,
    pub mutation_state: MutationState,
    pub epigenetic_markers: Vec<String>,
    pub dominant: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationState {
    Normal,
    Mutated,
    Silenced,
    Overexpressed,
    Damaged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u64)]
pub enum CellType {
    Undefined,
    Prokaryotic,
    EukaryoticAnimal,
    EukaryoticPlant,
    EukaryoticFungal,
    Synthetic,
    Mycelial,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StructuralFieldParams {
    pub depth_in_hierarchy: u32,
    pub structure_group: u32,
    pub is_critical_node: bool,
    pub upstream_neighbors: [Option<Uuid>; 4],
    pub downstream_neighbors: [Option<Uuid>; 4],
    pub max_stress_before_yield: f32,
    pub max_stress_before_fracture: f32,
    pub current_stress: f32,
}

impl Default for StructuralFieldParams {
    fn default() -> Self {
        Self {
            depth_in_hierarchy: 0,
            structure_group: 0,
            is_critical_node: false,
            upstream_neighbors: [None; 4],
            downstream_neighbors: [None; 4],
            max_stress_before_yield: 0.0,
            max_stress_before_fracture: 0.0,
            current_stress: 0.0,
        }
    }
}

impl Default for PhysicsAttributes {
    fn default() -> Self {
        Self {
            mass: 1.0,
            density: 1000.0,
            hardness: 5.0,
            toughness: 1.0,
            elastic_modulus: 1e9,
            yield_strength: 1e7,
            ultimate_strength: 2e7,
            poisson_ratio: 0.3,
            friction_coefficient: 0.5,
            restitution: 0.3,
            temperature: 293.0,
            thermal_conductivity: 1.0,
            specific_heat_capacity: 1000.0,
            electrical_conductivity: 0.0,
            magnetic_permeability: 1.0,
        }
    }
}

impl Default for ChemistryAttributes {
    fn default() -> Self {
        Self {
            elemental_composition: Vec::new(),
            bond_types: Vec::new(),
            reactivity: 0.0,
            ph: 7.0,
            redox_potential: 0.0,
            oxidation_state: 0.0,
            corrosion_depth: 0.0,
            chemical_stain: 0,
            solubility: 0.0,
            flammability: 0.0,
            toxicity: 0.0,
        }
    }
}

impl Default for BiologyAttributes {
    fn default() -> Self {
        Self {
            gene_tokens: Vec::new(),
            metabolic_rate: 1.0,
            growth_rate: 0.01,
            repair_rate: 0.1,
            neural_signal_strength: 0.0,
            health: 100.0,
            max_health: 100.0,
            radiation_dose: 0.0,
            toxin_level: 0.0,
            nutrient_level: 100.0,
            hydration: 100.0,
            cell_type: CellType::Undefined,
            tissue_density: 1.0,
        }
    }
}

impl MetaEntity {
    pub fn new(position: Vec3, physics: PhysicsAttributes, tick: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            version: 0,
            position,
            rotation: glam::Quat::IDENTITY,
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            physics,
            chemistry: ChemistryAttributes::default(),
            biology: BiologyAttributes::default(),
            state: MetaEntityState::Active,
            structural_field: None,
            spawn_tick: tick,
            parent_id: None,
            children: SmallVec::new(),
            extensions: HashMap::new(),
            mpss_index: None,
        }
    }

    pub fn with_chemistry(mut self, chemistry: ChemistryAttributes) -> Self {
        self.chemistry = chemistry;
        self
    }

    pub fn with_biology(mut self, biology: BiologyAttributes) -> Self {
        self.biology = biology;
        self
    }

    pub fn with_structural_field(mut self, sf: StructuralFieldParams) -> Self {
        self.structural_field = Some(sf);
        self
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, MetaEntityState::Active)
    }

    pub fn is_destroyed(&self) -> bool {
        matches!(self.state, MetaEntityState::Destroyed)
    }

    pub fn distance_to(&self, other: &MetaEntity) -> f32 {
        (self.position - other.position).length()
    }

    pub fn apply_force(&mut self, force: Vec3) {
        self.version += 1;
        let acceleration = force / self.physics.mass;
        self.velocity += acceleration;
    }

    pub fn apply_heat(&mut self, delta_temp: f32) {
        self.version += 1;
        self.physics.temperature = (self.physics.temperature + delta_temp).max(0.0);
    }

    pub fn apply_damage(&mut self, amount: f32) {
        self.version += 1;
        self.biology.health = (self.biology.health - amount).max(0.0);
        if self.biology.health <= 0.0 {
            self.state = MetaEntityState::Destroyed;
        }
    }

    pub fn apply_corrosion(&mut self, depth: f32) {
        self.version += 1;
        self.chemistry.corrosion_depth += depth;
        self.chemistry.oxidation_state = (self.chemistry.oxidation_state + depth * 0.01).min(1.0);
    }

    pub fn phase_transition(&mut self) {
        self.version += 1;
        self.state = MetaEntityState::PhaseTransitioning;
    }

    pub fn current_stress(&self) -> f32 {
        self.structural_field.as_ref().map(|sf| sf.current_stress).unwrap_or(0.0)
    }

    pub fn complete_phase_transition(&mut self, new_physics: PhysicsAttributes) {
        self.version += 1;
        self.physics = new_physics;
        self.chemistry = ChemistryAttributes::default();
        self.biology = BiologyAttributes::default();
        self.state = MetaEntityState::Active;
    }

    pub fn apply_changes(&mut self, changes: EntityChanges) -> Result<u64, &'static str> {
        if changes.expected_version != self.version {
            return Err("version mismatch");
        }

        if let Some(pos) = changes.position {
            self.position = pos;
        }
        if let Some(vel) = changes.velocity {
            self.velocity = vel;
        }
        if let Some(av) = changes.angular_velocity {
            self.angular_velocity = av;
        }
        for (key, val) in changes.physics_changes {
            match key.as_str() {
                "mass" => self.physics.mass = val,
                "density" => self.physics.density = val,
                "hardness" => self.physics.hardness = val,
                "toughness" => self.physics.toughness = val,
                "elastic_modulus" => self.physics.elastic_modulus = val,
                "yield_strength" => self.physics.yield_strength = val,
                "temperature" => self.physics.temperature = val,
                "friction_coefficient" => self.physics.friction_coefficient = val,
                "restitution" => self.physics.restitution = val,
                _ => {},
            }
        }
        for (key, val) in changes.chemistry_changes {
            match key.as_str() {
                "reactivity" => self.chemistry.reactivity = val,
                "ph" => self.chemistry.ph = val,
                "oxidation_state" => self.chemistry.oxidation_state = val,
                "corrosion_depth" => self.chemistry.corrosion_depth = val,
                "solubility" => self.chemistry.solubility = val,
                "flammability" => self.chemistry.flammability = val,
                "toxicity" => self.chemistry.toxicity = val,
                _ => {},
            }
        }
        for (key, val) in changes.biology_changes {
            match key.as_str() {
                "health" => self.biology.health = val,
                "metabolic_rate" => self.biology.metabolic_rate = val,
                "growth_rate" => self.biology.growth_rate = val,
                "repair_rate" => self.biology.repair_rate = val,
                "radiation_dose" => self.biology.radiation_dose = val,
                "toxin_level" => self.biology.toxin_level = val,
                "nutrient_level" => self.biology.nutrient_level = val,
                "hydration" => self.biology.hydration = val,
                _ => {},
            }
        }
        if let Some(state) = changes.state_change {
            self.state = state;
        }
        for (key, val) in changes.extension_changes {
            self.extensions.insert(key, val);
        }
        if changes.despawn {
            self.state = MetaEntityState::Destroyed;
        }

        self.version += 1;
        Ok(self.version)
    }

    pub fn attribute_hash(&self) -> u64 {
        let mut h: u64 = 0x9E3779B97F4A7C15;
        h = h.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(self.physics.mass.to_bits() as u64);
        h = h.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(self.physics.density.to_bits() as u64);
        h = h.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(self.physics.hardness.to_bits() as u64);
        h = h
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add(self.chemistry.reactivity.to_bits() as u64);
        h = h.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(self.chemistry.ph.to_bits() as u64);
        h = h
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add(self.biology.metabolic_rate.to_bits() as u64);
        h = h.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(self.biology.cell_type as u64);
        h
    }

    pub fn set_extension(&mut self, key: &str, value: ExtensionValue) {
        self.extensions.insert(key.to_string(), value);
    }

    pub fn get_extension(&self, key: &str) -> Option<&ExtensionValue> {
        self.extensions.get(key)
    }

    pub fn apply_microstructure(
        &mut self,
        micro: &ae_materials::microstructure::Microstructure,
    ) {
        let props = ae_materials::properties::DerivedProperties::from_microstructure(micro);
        self.physics.hardness = props.hardness;
        self.physics.toughness = props.toughness;
        self.physics.yield_strength = props.yield_strength;
        self.physics.ultimate_strength = props.yield_strength * 1.5;
        self.physics.elastic_modulus = props.elastic_modulus;
        self.physics.density = props.density;
        self.physics.thermal_conductivity = props.thermal_conductivity;
    }

    pub fn apply_quench(
        &mut self,
        temp: f32,
        medium: ae_materials::manufacturing::QuenchMedium,
    ) {
        let mut micro = ae_materials::microstructure::Microstructure::new(0.2);
        ae_materials::manufacturing::quench(&mut micro, temp, medium);
        self.apply_microstructure(&micro);
    }

    pub fn apply_temper(&mut self, temp: f32, duration: f32) {
        let mut micro = ae_materials::microstructure::Microstructure::new(0.2);
        ae_materials::manufacturing::quench(
            &mut micro,
            1100.0,
            ae_materials::manufacturing::QuenchMedium::Water,
        );
        ae_materials::manufacturing::temper(&mut micro, temp, duration);
        self.apply_microstructure(&micro);
    }

    pub fn apply_anneal(&mut self, temp: f32, cool_rate: f32) {
        let mut micro = ae_materials::microstructure::Microstructure::new(0.2);
        ae_materials::manufacturing::anneal(&mut micro, temp, cool_rate);
        self.apply_microstructure(&micro);
    }

    pub fn apply_cold_work(&mut self, strain: f32) {
        let mut micro = ae_materials::microstructure::Microstructure::new(0.2);
        ae_materials::manufacturing::cold_work(&mut micro, strain);
        self.apply_microstructure(&micro);
    }

    pub fn apply_forge(&mut self, temp: f32, strain: f32) {
        let mut micro = ae_materials::microstructure::Microstructure::new(0.2);
        ae_materials::manufacturing::forge(&mut micro, temp, strain);
        self.apply_microstructure(&micro);
    }

    pub fn apply_carburize(&mut self, temp: f32, duration: f32) {
        let mut micro = ae_materials::microstructure::Microstructure::new(0.2);
        ae_materials::manufacturing::carburize(&mut micro, temp, duration);
        self.apply_microstructure(&micro);
    }

    pub fn apply_age_harden(&mut self, temp: f32, duration: f32) {
        let mut micro = ae_materials::microstructure::Microstructure::new(0.2);
        ae_materials::manufacturing::age_harden(&mut micro, temp, duration);
        self.apply_microstructure(&micro);
    }
}

impl MetaEntity {
    pub fn iron(position: Vec3, tick: u64) -> Self {
        Self::new(
            position,
            PhysicsAttributes {
                density: 7874.0,
                hardness: 4.0,
                toughness: 50.0,
                elastic_modulus: 2.1e11,
                yield_strength: 2.5e8,
                ultimate_strength: 4.0e8,
                poisson_ratio: 0.29,
                thermal_conductivity: 80.0,
                specific_heat_capacity: 450.0,
                electrical_conductivity: 1e7,
                magnetic_permeability: 200.0,
                ..Default::default()
            },
            tick,
        )
        .with_chemistry(ChemistryAttributes {
            elemental_composition: vec![ElementFraction { element: Element::Fe, fraction: 1.0 }],
            bond_types: vec![ChemicalBond::Metallic],
            reactivity: 0.3,
            ph: 7.0,
            redox_potential: -0.44,
            oxidation_state: 0.0,
            corrosion_depth: 0.0,
            chemical_stain: 0,
            solubility: 0.0,
            flammability: 0.0,
            toxicity: 0.0,
        })
    }

    pub fn water(position: Vec3, tick: u64) -> Self {
        Self::new(
            position,
            PhysicsAttributes {
                density: 1000.0,
                hardness: 0.0,
                toughness: 0.0,
                elastic_modulus: 2.2e9,
                yield_strength: 0.0,
                ultimate_strength: 0.0,
                thermal_conductivity: 0.6,
                specific_heat_capacity: 4184.0,
                ..Default::default()
            },
            tick,
        )
        .with_chemistry(ChemistryAttributes {
            elemental_composition: vec![
                ElementFraction { element: Element::H, fraction: 0.112 },
                ElementFraction { element: Element::O, fraction: 0.888 },
            ],
            bond_types: vec![ChemicalBond::Covalent, ChemicalBond::Hydrogen],
            reactivity: 0.1,
            ph: 7.0,
            redox_potential: 0.0,
            oxidation_state: 0.0,
            corrosion_depth: 0.0,
            chemical_stain: 0,
            solubility: 1.0,
            flammability: 0.0,
            toxicity: 0.0,
        })
    }

    pub fn concrete(position: Vec3, tick: u64) -> Self {
        Self::new(
            position,
            PhysicsAttributes {
                density: 2400.0,
                hardness: 6.0,
                toughness: 0.2,
                elastic_modulus: 3e10,
                yield_strength: 3e7,
                ultimate_strength: 4e7,
                poisson_ratio: 0.2,
                friction_coefficient: 0.6,
                thermal_conductivity: 1.7,
                specific_heat_capacity: 880.0,
                ..Default::default()
            },
            tick,
        )
        .with_chemistry(ChemistryAttributes {
            elemental_composition: vec![
                ElementFraction { element: Element::Ca, fraction: 0.3 },
                ElementFraction { element: Element::Si, fraction: 0.2 },
                ElementFraction { element: Element::O, fraction: 0.45 },
                ElementFraction { element: Element::Al, fraction: 0.05 },
            ],
            bond_types: vec![ChemicalBond::Ionic, ChemicalBond::Covalent],
            reactivity: 0.2,
            ph: 12.0,
            redox_potential: 0.0,
            oxidation_state: 0.0,
            corrosion_depth: 0.0,
            chemical_stain: 0,
            solubility: 0.01,
            flammability: 0.0,
            toxicity: 0.0,
        })
    }

    pub fn wood(position: Vec3, tick: u64) -> Self {
        Self::new(
            position,
            PhysicsAttributes {
                density: 600.0,
                hardness: 2.0,
                toughness: 5.0,
                elastic_modulus: 1e10,
                yield_strength: 5e7,
                ultimate_strength: 1e8,
                poisson_ratio: 0.35,
                friction_coefficient: 0.4,
                thermal_conductivity: 0.15,
                specific_heat_capacity: 2000.0,
                ..Default::default()
            },
            tick,
        )
        .with_chemistry(ChemistryAttributes {
            elemental_composition: vec![
                ElementFraction { element: Element::C, fraction: 0.5 },
                ElementFraction { element: Element::H, fraction: 0.06 },
                ElementFraction { element: Element::O, fraction: 0.44 },
            ],
            bond_types: vec![ChemicalBond::Covalent, ChemicalBond::Hydrogen],
            reactivity: 0.4,
            ph: 5.0,
            redox_potential: 0.0,
            oxidation_state: 0.0,
            corrosion_depth: 0.0,
            chemical_stain: 0,
            solubility: 0.0,
            flammability: 0.8,
            toxicity: 0.0,
        })
        .with_biology(BiologyAttributes {
            cell_type: CellType::EukaryoticPlant,
            tissue_density: 0.6,
            ..Default::default()
        })
    }

    pub fn clone_organism(position: Vec3, tick: u64) -> Self {
        Self::new(
            position,
            PhysicsAttributes {
                density: 1060.0,
                hardness: 0.5,
                toughness: 1.0,
                elastic_modulus: 1e6,
                yield_strength: 1e5,
                ultimate_strength: 2e5,
                poisson_ratio: 0.45,
                friction_coefficient: 0.8,
                thermal_conductivity: 0.5,
                specific_heat_capacity: 3500.0,
                ..Default::default()
            },
            tick,
        )
        .with_biology(BiologyAttributes {
            gene_tokens: vec![
                GeneToken {
                    name: "ACTN3".into(),
                    expression_level: 1.0,
                    mutation_state: MutationState::Normal,
                    epigenetic_markers: vec!["cognitive_filter".into()],
                    dominant: true,
                },
                GeneToken {
                    name: "MSTN".into(),
                    expression_level: 0.5,
                    mutation_state: MutationState::Normal,
                    epigenetic_markers: Vec::new(),
                    dominant: false,
                },
            ],
            cell_type: CellType::EukaryoticAnimal,
            health: 100.0,
            max_health: 100.0,
            metabolic_rate: 1.0,
            repair_rate: 0.5,
            neural_signal_strength: 1.0,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_entity_creation() {
        let entity = MetaEntity::iron(Vec3::ZERO, 0);
        assert_eq!(entity.physics.density, 7874.0);
        assert_eq!(entity.chemistry.elemental_composition.len(), 1);
        assert_eq!(entity.version, 0);
    }

    #[test]
    fn test_iron_water_interaction() {
        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let water = MetaEntity::water(Vec3::new(1.0, 0.0, 0.0), 0);
        assert!(iron.distance_to(&water) < 2.0);
    }

    #[test]
    fn test_damage() {
        let mut entity = MetaEntity::clone_organism(Vec3::ZERO, 0);
        entity.apply_damage(50.0);
        assert_eq!(entity.biology.health, 50.0);
        assert_eq!(entity.version, 1);
        entity.apply_damage(60.0);
        assert!(entity.is_destroyed());
    }

    #[test]
    fn test_version_increment() {
        let mut entity = MetaEntity::iron(Vec3::ZERO, 0);
        assert_eq!(entity.version, 0);
        entity.apply_force(Vec3::X);
        assert_eq!(entity.version, 1);
        entity.apply_heat(10.0);
        assert_eq!(entity.version, 2);
    }

    #[test]
    fn test_apply_changes() {
        let mut entity = MetaEntity::iron(Vec3::ZERO, 0);
        let changes = EntityChanges {
            entity_id: entity.id,
            expected_version: 0,
            position: Some(Vec3::new(1.0, 0.0, 0.0)),
            velocity: None,
            angular_velocity: None,
            physics_changes: {
                let mut m = HashMap::new();
                m.insert("hardness".to_string(), 10.0);
                m
            },
            chemistry_changes: HashMap::new(),
            biology_changes: HashMap::new(),
            state_change: None,
            extension_changes: HashMap::new(),
            spawn_children: vec![],
            despawn: false,
        };
        let result = entity.apply_changes(changes);
        assert!(result.is_ok());
        assert_eq!(entity.position.x, 1.0);
        assert_eq!(entity.physics.hardness, 10.0);
        assert_eq!(entity.version, 1);
    }

    #[test]
    fn test_apply_changes_version_mismatch() {
        let mut entity = MetaEntity::iron(Vec3::ZERO, 0);
        entity.apply_force(Vec3::X);
        let changes = EntityChanges {
            entity_id: entity.id,
            expected_version: 0,
            position: None,
            velocity: None,
            angular_velocity: None,
            physics_changes: HashMap::new(),
            chemistry_changes: HashMap::new(),
            biology_changes: HashMap::new(),
            state_change: None,
            extension_changes: HashMap::new(),
            spawn_children: vec![],
            despawn: false,
        };
        assert!(entity.apply_changes(changes).is_err());
    }

    #[test]
    fn test_attribute_hash() {
        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let water = MetaEntity::water(Vec3::new(1.0, 0.0, 0.0), 0);
        assert_ne!(iron.attribute_hash(), water.attribute_hash());
    }

    #[test]
    fn test_extensions() {
        let mut entity = MetaEntity::iron(Vec3::ZERO, 0);
        entity.set_extension("custom_label", ExtensionValue::String("测试".to_string()));
        assert!(entity.get_extension("custom_label").is_some());
    }

    #[test]
    fn test_quench_increases_hardness() {
        let mut iron = MetaEntity::iron(Vec3::ZERO, 0);
        let orig_hardness = iron.physics.hardness;
        iron.apply_quench(1100.0, ae_materials::manufacturing::QuenchMedium::Water);
        assert!(iron.physics.hardness > orig_hardness);
    }

    #[test]
    fn test_temper_reduces_hardness() {
        let mut iron = MetaEntity::iron(Vec3::ZERO, 0);
        iron.apply_quench(1100.0, ae_materials::manufacturing::QuenchMedium::Water);
        let quenched_hardness = iron.physics.hardness;
        iron.apply_temper(600.0, 3600.0);
        assert!(iron.physics.hardness < quenched_hardness);
    }

    #[test]
    fn test_anneal_softens() {
        let mut iron = MetaEntity::iron(Vec3::ZERO, 0);
        iron.apply_quench(1100.0, ae_materials::manufacturing::QuenchMedium::Water);
        let quenched_hardness = iron.physics.hardness;
        iron.apply_anneal(900.0, 1.0);
        assert!(iron.physics.hardness < quenched_hardness);
    }

    #[test]
    fn test_cold_work_increases_hardness() {
        let mut iron = MetaEntity::iron(Vec3::ZERO, 0);
        let orig_hardness = iron.physics.hardness;
        iron.apply_cold_work(0.3);
        assert!(iron.physics.hardness > orig_hardness);
    }

    #[test]
    fn test_forge_changes_properties() {
        let mut iron = MetaEntity::iron(Vec3::ZERO, 0);
        let orig_hardness = iron.physics.hardness;
        iron.apply_forge(1100.0, 0.5);
        assert!(iron.physics.hardness != orig_hardness);
    }
}
