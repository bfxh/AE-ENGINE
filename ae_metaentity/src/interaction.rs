use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::interaction_cache::InteractionKey;
use crate::meta_entity::*;

/// 交互结果 — 统一响应函数的输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionResult {
    pub force_on_a: Vec3,
    pub force_on_b: Vec3,
    pub attribute_changes_a: Vec<AttributeChange>,
    pub attribute_changes_b: Vec<AttributeChange>,
    pub generated_entities: Vec<GeneratedEntityDesc>,
    pub heat_released: f32,
    pub light_emitted: [f32; 3],
    pub sound_intensity: f32,
    pub interaction_type: InteractionType,
    pub cache_key: InteractionKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeChange {
    pub field: AttributeField,
    pub delta: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeField {
    Temperature,
    Health,
    Mass,
    Density,
    CorrosionDepth,
    OxidationState,
    Ph,
    Hardness,
    Toughness,
    ElasticModulus,
    YieldStrength,
    Stress,
    Reactivity,
    Toxicity,
    Radioactivity,
    MetabolicRate,
    GrowthRate,
    NeuralSignal,
    Hydration,
    NutrientLevel,
    ToxinLevel,
    RadiationDose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedEntityDesc {
    pub entity_type: GeneratedEntityType,
    pub position: Vec3,
    pub velocity: Vec3,
    pub physics: PhysicsAttributes,
    pub chemistry: ChemistryAttributes,
    pub biology: BiologyAttributes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneratedEntityType {
    Gas,
    Debris,
    Particle,
    CorrosionFlake,
    Steam,
    Smoke,
    Spark,
    Spore,
    BioAerosol,
    LiquidDroplet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionType {
    MechanicalContact,
    ChemicalReaction,
    BiologicalInteraction,
    ThermalExchange,
    ElectricalConduction,
    MagneticInteraction,
    RadiationTransfer,
    PhaseTransition,
    Enzymatic,
    Osmotic,
    Hybrid,
}

/// 统一响应函数 — 根据两个元体的属性向量，计算所有交互结果
pub struct InteractionResponseFn;

impl InteractionResponseFn {
    /// 核心交互入口：给定两个元体和它们的距离，返回统一的交互结果
    pub fn compute(a: &MetaEntity, b: &MetaEntity, distance: f32, dt: f32) -> InteractionResult {
        let contact_normal =
            if distance > 0.001 { (b.position - a.position) / distance } else { Vec3::Y };

        let mut force_on_a = Vec3::ZERO;
        let mut force_on_b = Vec3::ZERO;
        let mut attr_changes_a = Vec::new();
        let mut attr_changes_b = Vec::new();
        let mut generated = Vec::new();
        let mut heat = 0.0f32;
        let mut light = [0.0f32; 3];
        let mut sound = 0.0f32;
        let mut interaction_type = InteractionType::Hybrid;

        let overlap = Self::compute_overlap(a, b, distance);

        if overlap > 0.0 {
            let (fa, fb, contact_heat) =
                Self::mechanical_response(a, b, overlap, contact_normal, dt);
            force_on_a += fa;
            force_on_b += fb;
            heat += contact_heat;
            sound += Self::compute_contact_sound(a, b, overlap);
            interaction_type = InteractionType::MechanicalContact;
        }

        if distance < 10.0 {
            let (ca, cb, chem_heat, chem_generated) = Self::chemical_response(a, b, distance, dt);
            for change in ca {
                attr_changes_a.push(change);
            }
            for change in cb {
                attr_changes_b.push(change);
            }
            heat += chem_heat;
            generated.extend(chem_generated);
            if !attr_changes_a.is_empty() || !attr_changes_b.is_empty() {
                interaction_type = InteractionType::ChemicalReaction;
            }
        }

        if distance < 5.0 {
            let thermal_heat = Self::thermal_response(a, b, distance, dt);
            heat += thermal_heat;
            if thermal_heat.abs() > 0.01 {
                attr_changes_a.push(AttributeChange {
                    field: AttributeField::Temperature,
                    delta: -thermal_heat / (a.physics.mass * a.physics.specific_heat_capacity),
                    reason: "thermal_exchange".into(),
                });
                attr_changes_b.push(AttributeChange {
                    field: AttributeField::Temperature,
                    delta: thermal_heat / (b.physics.mass * b.physics.specific_heat_capacity),
                    reason: "thermal_exchange".into(),
                });
            }
        }

        if a.biology.cell_type != CellType::Undefined
            && b.biology.cell_type != CellType::Undefined
            && distance < 2.0
        {
            let (ba, bb) = Self::biological_response(a, b, distance, dt);
            attr_changes_a.extend(ba);
            attr_changes_b.extend(bb);
            if !attr_changes_a.is_empty() || !attr_changes_b.is_empty() {
                interaction_type = InteractionType::BiologicalInteraction;
            }
        }

        if a.physics.electrical_conductivity > 0.0
            && b.physics.electrical_conductivity > 0.0
            && distance < 0.1
        {
            interaction_type = InteractionType::ElectricalConduction;
        }

        if !a.chemistry.elemental_composition.is_empty()
            && !b.chemistry.elemental_composition.is_empty()
        {
            if let Some((oxidation_changes, rust_heat)) =
                Self::oxidation_response(a, b, distance, dt)
            {
                attr_changes_a.extend(oxidation_changes);
                heat += rust_heat;
                light = [0.6, 0.3, 0.1];
            }
        }

        let cache_key = InteractionKey::new(a, b, distance);

        InteractionResult {
            force_on_a,
            force_on_b,
            attribute_changes_a: attr_changes_a,
            attribute_changes_b: attr_changes_b,
            generated_entities: generated,
            heat_released: heat,
            light_emitted: light,
            sound_intensity: sound,
            interaction_type,
            cache_key,
        }
    }

    fn compute_overlap(a: &MetaEntity, b: &MetaEntity, distance: f32) -> f32 {
        let radius_a = (a.physics.mass / a.physics.density / 0.75).cbrt();
        let radius_b = (b.physics.mass / b.physics.density / 0.75).cbrt();
        (radius_a + radius_b - distance).max(0.0)
    }

    fn mechanical_response(
        a: &MetaEntity,
        b: &MetaEntity,
        overlap: f32,
        normal: Vec3,
        _dt: f32,
    ) -> (Vec3, Vec3, f32) {
        let effective_stiffness = (a.physics.elastic_modulus * b.physics.elastic_modulus)
            / (a.physics.elastic_modulus + b.physics.elastic_modulus).max(1.0);
        let contact_force = effective_stiffness * overlap * 0.1;

        let relative_vel = a.velocity - b.velocity;
        let normal_vel = relative_vel.dot(normal);
        let damping = (a.physics.restitution * b.physics.restitution).sqrt() * normal_vel.abs();

        let total_force = contact_force + damping;
        let force_a = normal * total_force;
        let force_b = -force_a;

        let heat = (damping * overlap * 0.1).abs();

        (force_a, force_b, heat)
    }

    fn chemical_response(
        a: &MetaEntity,
        b: &MetaEntity,
        distance: f32,
        dt: f32,
    ) -> (Vec<AttributeChange>, Vec<AttributeChange>, f32, Vec<GeneratedEntityDesc>) {
        let mut changes_a = Vec::new();
        let mut changes_b = Vec::new();
        let mut heat = 0.0f32;
        let mut generated = Vec::new();

        if a.chemistry.reactivity < 0.01 && b.chemistry.reactivity < 0.01 {
            return (changes_a, changes_b, heat, generated);
        }

        let interaction_strength = (a.chemistry.reactivity * b.chemistry.reactivity).sqrt()
            * (1.0 - distance / 10.0).max(0.0);

        let has_acid = a.chemistry.ph < 4.0 || b.chemistry.ph < 4.0;
        let has_metal =
            a.chemistry.elemental_composition.iter().any(|e| {
                matches!(e.element, Element::Fe | Element::Cu | Element::Al | Element::Zn)
            }) || b.chemistry.elemental_composition.iter().any(|e| {
                matches!(e.element, Element::Fe | Element::Cu | Element::Al | Element::Zn)
            });

        if has_acid && has_metal {
            let corrosion_rate = interaction_strength * 0.01 * dt;
            let metal_idx = if a
                .chemistry
                .elemental_composition
                .iter()
                .any(|e| matches!(e.element, Element::Fe))
            {
                0
            } else {
                1
            };

            if metal_idx == 0 {
                changes_a.push(AttributeChange {
                    field: AttributeField::CorrosionDepth,
                    delta: corrosion_rate,
                    reason: "acid_corrosion".into(),
                });
                changes_a.push(AttributeChange {
                    field: AttributeField::Mass,
                    delta: -corrosion_rate * 0.001,
                    reason: "material_loss".into(),
                });
            } else {
                changes_b.push(AttributeChange {
                    field: AttributeField::CorrosionDepth,
                    delta: corrosion_rate,
                    reason: "acid_corrosion".into(),
                });
            }

            heat += interaction_strength * 100.0 * dt;

            if corrosion_rate > 0.001 {
                generated.push(GeneratedEntityDesc {
                    entity_type: GeneratedEntityType::Gas,
                    position: (a.position + b.position) * 0.5,
                    velocity: Vec3::new(0.0, 0.5, 0.0),
                    physics: PhysicsAttributes::default(),
                    chemistry: ChemistryAttributes {
                        elemental_composition: vec![ElementFraction {
                            element: Element::H,
                            fraction: 1.0,
                        }],
                        bond_types: vec![ChemicalBond::Covalent],
                        reactivity: 0.9,
                        ..Default::default()
                    },
                    biology: BiologyAttributes::default(),
                });
            }
        }

        let has_oxidizer = (a
            .chemistry
            .elemental_composition
            .iter()
            .any(|e| matches!(e.element, Element::O))
            && a.chemistry.reactivity > 0.3)
            || (b.chemistry.elemental_composition.iter().any(|e| matches!(e.element, Element::O))
                && b.chemistry.reactivity > 0.3);

        if has_metal && has_oxidizer {
            let oxidation_rate = interaction_strength * 0.005 * dt;
            changes_a.push(AttributeChange {
                field: AttributeField::OxidationState,
                delta: oxidation_rate,
                reason: "oxidation".into(),
            });
            heat += interaction_strength * 50.0 * dt;
        }

        let has_water = (a
            .chemistry
            .elemental_composition
            .iter()
            .any(|e| matches!(e.element, Element::H))
            && a.chemistry.elemental_composition.iter().any(|e| matches!(e.element, Element::O)))
            || (b.chemistry.elemental_composition.iter().any(|e| matches!(e.element, Element::H))
                && b.chemistry
                    .elemental_composition
                    .iter()
                    .any(|e| matches!(e.element, Element::O)));

        if has_metal && has_water {
            let rust_rate = interaction_strength * 0.002 * dt;
            changes_a.push(AttributeChange {
                field: AttributeField::OxidationState,
                delta: rust_rate,
                reason: "rusting".into(),
            });
            changes_a.push(AttributeChange {
                field: AttributeField::Toughness,
                delta: -rust_rate * 0.5,
                reason: "embrittlement".into(),
            });
        }

        (changes_a, changes_b, heat, generated)
    }

    fn oxidation_response(
        a: &MetaEntity,
        b: &MetaEntity,
        distance: f32,
        dt: f32,
    ) -> Option<(Vec<AttributeChange>, f32)> {
        let a_has_iron =
            a.chemistry.elemental_composition.iter().any(|e| matches!(e.element, Element::Fe));
        let b_has_oxygen =
            b.chemistry.elemental_composition.iter().any(|e| matches!(e.element, Element::O));
        let a_has_oxygen =
            a.chemistry.elemental_composition.iter().any(|e| matches!(e.element, Element::O));
        let b_has_iron =
            b.chemistry.elemental_composition.iter().any(|e| matches!(e.element, Element::Fe));

        let (_iron_entity, _oxygen_entity) = if a_has_iron && b_has_oxygen {
            (true, false)
        } else if a_has_oxygen && b_has_iron {
            (false, true)
        } else {
            return None;
        };

        let oxidation_rate = 0.001 * dt * (1.0 - distance / 10.0).max(0.0);
        let changes = vec![AttributeChange {
            field: AttributeField::OxidationState,
            delta: oxidation_rate,
            reason: "iron_oxidation".into(),
        }];

        Some((changes, oxidation_rate * 200.0))
    }

    fn thermal_response(a: &MetaEntity, b: &MetaEntity, distance: f32, dt: f32) -> f32 {
        let temp_diff = a.physics.temperature - b.physics.temperature;
        if temp_diff.abs() < 0.1 {
            return 0.0;
        }

        let avg_conductivity =
            (a.physics.thermal_conductivity * b.physics.thermal_conductivity).sqrt();
        let distance_factor = (1.0 - distance / 5.0).max(0.0);

        temp_diff * avg_conductivity * distance_factor * dt * 0.01
    }

    fn biological_response(
        a: &MetaEntity,
        b: &MetaEntity,
        distance: f32,
        dt: f32,
    ) -> (Vec<AttributeChange>, Vec<AttributeChange>) {
        let mut changes_a = Vec::new();
        let mut changes_b = Vec::new();

        let interaction_factor = (1.0 - distance / 2.0).max(0.0);
        let enzyme_strength =
            (a.biology.metabolic_rate * b.biology.metabolic_rate).sqrt() * interaction_factor * dt;

        if a.chemistry.toxicity > 0.0 && b.biology.neural_signal_strength > 0.0 {
            changes_b.push(AttributeChange {
                field: AttributeField::ToxinLevel,
                delta: a.chemistry.toxicity * enzyme_strength * 0.01,
                reason: "toxin_exposure".into(),
            });
        }
        if b.chemistry.toxicity > 0.0 && a.biology.neural_signal_strength > 0.0 {
            changes_a.push(AttributeChange {
                field: AttributeField::ToxinLevel,
                delta: b.chemistry.toxicity * enzyme_strength * 0.01,
                reason: "toxin_exposure".into(),
            });
        }

        if a.biology.cell_type == CellType::Mycelial && b.biology.cell_type != CellType::Undefined {
            changes_b.push(AttributeChange {
                field: AttributeField::Health,
                delta: -enzyme_strength * 0.5,
                reason: "mycelial_decomposition".into(),
            });
        }
        if b.biology.cell_type == CellType::Mycelial && a.biology.cell_type != CellType::Undefined {
            changes_a.push(AttributeChange {
                field: AttributeField::Health,
                delta: -enzyme_strength * 0.5,
                reason: "mycelial_decomposition".into(),
            });
        }

        (changes_a, changes_b)
    }

    fn compute_contact_sound(a: &MetaEntity, b: &MetaEntity, overlap: f32) -> f32 {
        let impulse = (a.velocity - b.velocity).length() * a.physics.mass.min(b.physics.mass);
        let material_factor = (a.physics.hardness * b.physics.hardness).sqrt() * 0.01;
        (impulse * material_factor * overlap).min(1.0)
    }
}

/// 应用交互结果到元体
pub fn apply_interaction_result(entity: &mut MetaEntity, result: &InteractionResult, is_a: bool) {
    let (force, changes) = if is_a {
        (result.force_on_a, &result.attribute_changes_a)
    } else {
        (result.force_on_b, &result.attribute_changes_b)
    };

    entity.apply_force(force);

    for change in changes {
        match change.field {
            AttributeField::Temperature => entity.apply_heat(change.delta),
            AttributeField::Health => entity.apply_damage(-change.delta),
            AttributeField::CorrosionDepth => entity.apply_corrosion(change.delta),
            AttributeField::OxidationState => {
                entity.chemistry.oxidation_state =
                    (entity.chemistry.oxidation_state + change.delta).clamp(0.0, 1.0);
            },
            AttributeField::Mass => {
                entity.physics.mass = (entity.physics.mass + change.delta).max(0.001);
            },
            AttributeField::Toughness => {
                entity.physics.toughness = (entity.physics.toughness + change.delta).max(0.0);
            },
            AttributeField::ToxinLevel => {
                entity.biology.toxin_level = (entity.biology.toxin_level + change.delta).max(0.0);
            },
            AttributeField::RadiationDose => {
                entity.biology.radiation_dose =
                    (entity.biology.radiation_dose + change.delta).max(0.0);
            },
            AttributeField::Ph => {
                entity.chemistry.ph = (entity.chemistry.ph + change.delta).clamp(0.0, 14.0);
            },
            AttributeField::Hardness => {
                entity.physics.hardness = (entity.physics.hardness + change.delta).max(0.0);
            },
            AttributeField::YieldStrength => {
                entity.physics.yield_strength =
                    (entity.physics.yield_strength + change.delta).max(0.0);
            },
            AttributeField::Stress => {
                if let Some(ref mut sf) = entity.structural_field {
                    sf.current_stress = (sf.current_stress + change.delta).max(0.0);
                }
            },
            _ => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iron_water_interaction() {
        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let water = MetaEntity::water(Vec3::new(0.5, 0.0, 0.0), 0);
        let result = InteractionResponseFn::compute(&iron, &water, 0.5, 1.0 / 60.0);
        assert!(result.force_on_a.length() > 0.0 || !result.attribute_changes_a.is_empty());
    }

    #[test]
    fn test_acid_metal_corrosion() {
        let metal = MetaEntity::iron(Vec3::ZERO, 0);
        let acid = MetaEntity::new(Vec3::new(0.5, 0.0, 0.0), PhysicsAttributes::default(), 0)
            .with_chemistry(ChemistryAttributes { ph: 1.0, reactivity: 0.9, ..Default::default() });

        let result = InteractionResponseFn::compute(&metal, &acid, 0.5, 1.0);
        let has_corrosion = result
            .attribute_changes_a
            .iter()
            .any(|c| matches!(c.field, AttributeField::CorrosionDepth));
        assert!(has_corrosion);
    }

    #[test]
    fn test_no_interaction_without_reactivity() {
        let a = MetaEntity::new(Vec3::ZERO, PhysicsAttributes::default(), 0);
        let b = MetaEntity::new(Vec3::new(5.0, 0.0, 0.0), PhysicsAttributes::default(), 0);
        let result = InteractionResponseFn::compute(&a, &b, 5.0, 1.0 / 60.0);
        assert!(result.attribute_changes_a.is_empty());
        assert!(result.attribute_changes_b.is_empty());
    }
}
