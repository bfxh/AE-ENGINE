use godot::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;
use ae_chemistry::reactions::{ChemicalReaction, ReactionSystem};
use ae_metaentity::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandMeta {
    entities: Mutex<HashMap<Uuid, MetaEntity>>,
    tick: u64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandMeta {
    fn init(base: Base<Node>) -> Self {
        Self { entities: Mutex::new(HashMap::new()), tick: 0, base }
    }
}

#[godot_api]
impl WastelandMeta {
    #[func]
    fn spawn_entity(&mut self, material: GString, px: f32, py: f32, pz: f32) -> GString {
        let pos = glam::Vec3::new(px, py, pz);
        self.tick += 1;
        let entity = match material.to_string().as_str() {
            "iron" => MetaEntity::iron(pos, self.tick),
            "water" => MetaEntity::water(pos, self.tick),
            "concrete" => MetaEntity::concrete(pos, self.tick),
            "wood" => MetaEntity::wood(pos, self.tick),
            "organism" => MetaEntity::clone_organism(pos, self.tick),
            _ => MetaEntity::new(pos, PhysicsAttributes::default(), self.tick),
        };
        let id = entity.id;
        if let Ok(mut entities) = self.entities.lock() {
            entities.insert(id, entity);
        }
        let id_str = id.to_string();
        GString::from(id_str.as_str())
    }

    #[func]
    fn remove_entity(&mut self, id: GString) {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(mut entities) = self.entities.lock() {
                entities.remove(&parsed);
            }
        }
    }

    #[func]
    fn get_entity_position(&self, id: GString) -> Vector3 {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(entities) = self.entities.lock() {
                if let Some(e) = entities.get(&parsed) {
                    return Vector3::new(e.position.x, e.position.y, e.position.z);
                }
            }
        }
        Vector3::ZERO
    }

    #[func]
    fn get_entity_velocity(&self, id: GString) -> Vector3 {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(entities) = self.entities.lock() {
                if let Some(e) = entities.get(&parsed) {
                    return Vector3::new(e.velocity.x, e.velocity.y, e.velocity.z);
                }
            }
        }
        Vector3::ZERO
    }

    #[func]
    fn apply_force_to_entity(&mut self, id: GString, fx: f32, fy: f32, fz: f32) {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(mut entities) = self.entities.lock() {
                if let Some(e) = entities.get_mut(&parsed) {
                    e.apply_force(glam::Vec3::new(fx, fy, fz));
                }
            }
        }
    }

    #[func]
    fn apply_damage_to_entity(&mut self, id: GString, amount: f32) {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(mut entities) = self.entities.lock() {
                if let Some(e) = entities.get_mut(&parsed) {
                    e.apply_damage(amount);
                }
            }
        }
    }

    #[func]
    fn apply_heat_to_entity(&mut self, id: GString, delta: f32) {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(mut entities) = self.entities.lock() {
                if let Some(e) = entities.get_mut(&parsed) {
                    e.apply_heat(delta);
                }
            }
        }
    }

    #[func]
    fn get_entity_health(&self, id: GString) -> f32 {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(entities) = self.entities.lock() {
                if let Some(e) = entities.get(&parsed) {
                    return e.biology.health;
                }
            }
        }
        0.0
    }

    #[func]
    fn get_entity_temperature(&self, id: GString) -> f32 {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(entities) = self.entities.lock() {
                if let Some(e) = entities.get(&parsed) {
                    return e.physics.temperature;
                }
            }
        }
        0.0
    }

    #[func]
    fn get_entity_mass(&self, id: GString) -> f32 {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(entities) = self.entities.lock() {
                if let Some(e) = entities.get(&parsed) {
                    return e.physics.mass;
                }
            }
        }
        0.0
    }

    #[func]
    fn is_entity_destroyed(&self, id: GString) -> bool {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(entities) = self.entities.lock() {
                if let Some(e) = entities.get(&parsed) {
                    return e.is_destroyed();
                }
            }
        }
        true
    }

    #[func]
    fn distance_between(&self, id_a: GString, id_b: GString) -> f32 {
        if let (Ok(a), Ok(b)) =
            (Uuid::parse_str(&id_a.to_string()), Uuid::parse_str(&id_b.to_string()))
        {
            if let Ok(entities) = self.entities.lock() {
                if let (Some(ea), Some(eb)) = (entities.get(&a), entities.get(&b)) {
                    return ea.distance_to(eb);
                }
            }
        }
        0.0
    }

    #[func]
    fn get_entity_properties(&self, id: GString) -> Dictionary<Variant, Variant> {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(entities) = self.entities.lock() {
                if let Some(e) = entities.get(&parsed) {
                    return dict! {
                        "mass" => e.physics.mass,
                        "density" => e.physics.density,
                        "hardness" => e.physics.hardness,
                        "toughness" => e.physics.toughness,
                        "elastic_modulus" => e.physics.elastic_modulus,
                        "yield_strength" => e.physics.yield_strength,
                        "friction" => e.physics.friction_coefficient,
                        "temperature" => e.physics.temperature,
                        "reactivity" => e.chemistry.reactivity,
                        "ph" => e.chemistry.ph,
                        "flammability" => e.chemistry.flammability,
                        "health" => e.biology.health,
                        "max_health" => e.biology.max_health,
                        "metabolic_rate" => e.biology.metabolic_rate,
                    };
                }
            }
        }
        dict! {}
    }

    #[func]
    fn entity_count(&self) -> i64 {
        if let Ok(entities) = self.entities.lock() {
            return entities.len() as i64;
        }
        0
    }

    #[func]
    fn get_all_entity_ids(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if let Ok(entities) = self.entities.lock() {
            for id in entities.keys() {
                let id_str = id.to_string();
                arr.push(id_str.as_str());
            }
        }
        arr
    }

    #[func]
    fn get_entity_chemistry(&self, id: GString) -> Dictionary<Variant, Variant> {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(entities) = self.entities.lock() {
                if let Some(e) = entities.get(&parsed) {
                    return dict! {
                        "reactivity" => e.chemistry.reactivity,
                        "ph" => e.chemistry.ph,
                        "flammability" => e.chemistry.flammability,
                        "oxidation_state" => e.chemistry.oxidation_state,
                        "corrosiveness" => e.chemistry.corrosion_depth,
                    };
                }
            }
        }
        dict! {}
    }

    #[func]
    fn get_entity_biology(&self, id: GString) -> Dictionary<Variant, Variant> {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(entities) = self.entities.lock() {
                if let Some(e) = entities.get(&parsed) {
                    return dict! {
                        "health" => e.biology.health,
                        "max_health" => e.biology.max_health,
                        "metabolic_rate" => e.biology.metabolic_rate,
                        "age" => e.biology.growth_rate,
                        "radiation_dose" => e.biology.radiation_dose,
                        "toxin_level" => e.biology.toxin_level,
                        "mutation_count" => e.biology.gene_tokens.len() as i64,
                    };
                }
            }
        }
        dict! {}
    }

    #[func]
    fn trigger_reaction_between(
        &mut self,
        id_a: GString,
        id_b: GString,
    ) -> Dictionary<Variant, Variant> {
        if let (Ok(a), Ok(b)) =
            (Uuid::parse_str(&id_a.to_string()), Uuid::parse_str(&id_b.to_string()))
        {
            if let Ok(entities) = self.entities.lock() {
                if let (Some(ea), Some(eb)) = (entities.get(&a), entities.get(&b)) {
                    let reactivity = (ea.chemistry.reactivity + eb.chemistry.reactivity) * 0.5;
                    let energy = reactivity * ea.physics.mass * eb.physics.mass * 0.01;
                    let reaction_type =
                        if ea.chemistry.flammability > 0.5 && eb.chemistry.flammability > 0.5 {
                            GString::from("combustion")
                        } else if ea.chemistry.ph < 4.0 || eb.chemistry.ph < 4.0 {
                            GString::from("acid_reaction")
                        } else if ea.chemistry.reactivity > 0.7 || eb.chemistry.reactivity > 0.7 {
                            GString::from("catalytic")
                        } else {
                            GString::from("neutral")
                        };
                    let products = PackedStringArray::new();
                    return dict! {
                        "reaction_type" => &reaction_type,
                        "energy_released" => energy,
                        "products" => &products,
                    };
                }
            }
        }
        dict! {}
    }
}

#[derive(GodotClass)]
#[class(base=Node, rename=WastelandChemistrySystem)]
struct WastelandChemistrySystem {
    reaction_system: Mutex<ReactionSystem>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandChemistrySystem {
    fn init(base: Base<Node>) -> Self {
        Self { reaction_system: Mutex::new(ReactionSystem::new()), base }
    }
}

#[godot_api]
impl WastelandChemistrySystem {
    #[func]
    fn update(&mut self, dt: f32, time: f64) {
        if let Ok(mut system) = self.reaction_system.lock() {
            system.update(dt, time);
        }
    }

    #[func]
    fn set_environment(
        &mut self,
        temperature: f32,
        pressure: f32,
        oxygen: f32,
        humidity: f32,
        ph: f32,
        radiation: f32,
    ) {
        if let Ok(mut system) = self.reaction_system.lock() {
            system.temperature = temperature;
            system.pressure = pressure;
            system.oxygen_level = oxygen;
            system.humidity = humidity;
            system.ph = ph;
            system.radiation_level = radiation;
        }
    }

    #[func]
    fn trigger_reaction(
        &mut self,
        reaction_type: GString,
        px: f32,
        py: f32,
        pz: f32,
        intensity: f32,
    ) {
        let reaction = match reaction_type.to_string().as_str() {
            "combustion" => ChemicalReaction::combustion_organic(),
            "rust" => ChemicalReaction::rust_formation(),
            "acid_corrosion" => ChemicalReaction::acid_corrosion(),
            "explosion" => ChemicalReaction::explosion_tnt(),
            "radioactive_decay" => ChemicalReaction::radioactive_decay(),
            "photosynthesis" => ChemicalReaction::photosynthesis(),
            "fermentation" => ChemicalReaction::fermentation(),
            "thermite" => ChemicalReaction::thermite_reaction(),
            "mutagen" => ChemicalReaction::mutagen_synthesis(),
            "acid_rain" => ChemicalReaction::acid_rain_formation(),
            "biofuel" => ChemicalReaction::biofuel_combustion(),
            "cryo" => ChemicalReaction::cryo_agent_expansion(),
            "rust_inhibition" => ChemicalReaction::rust_inhibition(),
            "neurotoxin" => ChemicalReaction::neurotoxin_dispersion(),
            _ => return,
        };
        if let Ok(mut system) = self.reaction_system.lock() {
            system.trigger_reaction(reaction, glam::Vec3::new(px, py, pz), intensity);
        }
    }

    #[func]
    fn active_reaction_count(&self) -> i64 {
        if let Ok(system) = self.reaction_system.lock() {
            return system.active_reactions.len() as i64;
        }
        0
    }

    #[func]
    fn completed_reaction_count(&self) -> i64 {
        if let Ok(system) = self.reaction_system.lock() {
            return system.completed_reactions.len() as i64;
        }
        0
    }

    #[func]
    fn get_temperature(&self) -> f32 {
        if let Ok(system) = self.reaction_system.lock() {
            return system.temperature;
        }
        293.0
    }

    #[func]
    fn get_oxygen_level(&self) -> f32 {
        if let Ok(system) = self.reaction_system.lock() {
            return system.oxygen_level;
        }
        0.0
    }

    #[func]
    fn get_ph(&self) -> f32 {
        if let Ok(system) = self.reaction_system.lock() {
            return system.ph;
        }
        7.0
    }

    #[func]
    fn get_completed_reactions(&self) -> Array<Variant> {
        let mut arr = Array::new();
        if let Ok(system) = self.reaction_system.lock() {
            for result in &system.completed_reactions {
                let d: Dictionary<Variant, Variant> = dict! {
                    "type" => format!("{:?}", result.reaction.reaction_type).as_str(),
                    "pos_x" => result.position.x,
                    "pos_y" => result.position.y,
                    "pos_z" => result.position.z,
                    "energy_released" => result.energy_released,
                    "product_count" => result.products_generated.len() as i64,
                    "byproduct_count" => result.byproducts.len() as i64,
                    "timestamp" => result.timestamp,
                };
                arr.push(&d);
            }
        }
        arr
    }
}
