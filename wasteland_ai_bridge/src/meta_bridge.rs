use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEntityBridge {
    pub property_mappings: Vec<PropertyMapping>,
    pub interaction_rules: Vec<InteractionRule>,
    pub generation_rules: Vec<GenerationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyMapping {
    pub ai_property: String,
    pub meta_property: String,
    pub conversion: ConversionType,
    pub scale: f32,
    pub offset: f32,
    pub clamp_range: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConversionType {
    Direct,
    Inverse,
    Squared,
    Sqrt,
    Log,
    Exp,
    Sigmoid,
    Threshold(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRule {
    pub rule_name: String,
    pub source_properties: Vec<String>,
    pub target_effects: Vec<PropertyEffect>,
    pub conditions: Vec<InteractionCondition>,
    pub priority: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyEffect {
    pub property_name: String,
    pub operation: EffectOperation,
    pub magnitude: f32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectOperation {
    Add,
    Multiply,
    Set,
    Lerp,
    Clamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionCondition {
    pub property: String,
    pub comparator: Comparator,
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Comparator {
    GreaterThan,
    LessThan,
    Equals,
    Between(f32, f32),
    NotEquals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRule {
    pub rule_name: String,
    pub input_properties: Vec<String>,
    pub output_entity_type: String,
    pub output_properties: Vec<PropertyMapping>,
    pub probability: f32,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEntityState {
    pub entity_id: u64,
    pub properties: Vec<MetaProperty>,
    pub active_effects: Vec<ActiveEffect>,
    pub generation_history: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaProperty {
    pub name: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub mutable: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEffect {
    pub effect_name: String,
    pub property_name: String,
    pub start_value: f32,
    pub target_value: f32,
    pub started_at: u64,
    pub duration_ms: u64,
}

impl MetaEntityBridge {
    pub fn new() -> Self {
        MetaEntityBridge {
            property_mappings: Self::default_mappings(),
            interaction_rules: Self::default_interactions(),
            generation_rules: Self::default_generations(),
        }
    }

    fn default_mappings() -> Vec<PropertyMapping> {
        vec![
            PropertyMapping {
                ai_property: "temperature".into(),
                meta_property: "thermal_energy".into(),
                conversion: ConversionType::Direct,
                scale: 1.0,
                offset: 0.0,
                clamp_range: Some([0.0, 1000.0]),
            },
            PropertyMapping {
                ai_property: "hardness".into(),
                meta_property: "yield_strength".into(),
                conversion: ConversionType::Direct,
                scale: 100.0,
                offset: 0.0,
                clamp_range: Some([0.0, 10000.0]),
            },
            PropertyMapping {
                ai_property: "density".into(),
                meta_property: "mass_density".into(),
                conversion: ConversionType::Direct,
                scale: 1.0,
                offset: 0.0,
                clamp_range: Some([0.01, 100.0]),
            },
            PropertyMapping {
                ai_property: "integrity".into(),
                meta_property: "structural_integrity".into(),
                conversion: ConversionType::Direct,
                scale: 1.0,
                offset: 0.0,
                clamp_range: Some([0.0, 1.0]),
            },
            PropertyMapping {
                ai_property: "radioactivity".into(),
                meta_property: "radiation_level".into(),
                conversion: ConversionType::Log,
                scale: 10.0,
                offset: 0.0,
                clamp_range: Some([0.0, 1000.0]),
            },
        ]
    }

    fn default_interactions() -> Vec<InteractionRule> {
        vec![
            InteractionRule {
                rule_name: "heat_transfer".into(),
                source_properties: vec!["thermal_energy".into()],
                target_effects: vec![PropertyEffect {
                    property_name: "thermal_energy".into(),
                    operation: EffectOperation::Lerp,
                    magnitude: 0.1,
                    duration_ms: 1000,
                }],
                conditions: vec![InteractionCondition {
                    property: "thermal_energy".into(),
                    comparator: Comparator::GreaterThan,
                    value: 300.0,
                }],
                priority: 5,
                enabled: true,
            },
            InteractionRule {
                rule_name: "radiation_damage".into(),
                source_properties: vec!["radiation_level".into()],
                target_effects: vec![PropertyEffect {
                    property_name: "structural_integrity".into(),
                    operation: EffectOperation::Add,
                    magnitude: -0.01,
                    duration_ms: 5000,
                }],
                conditions: vec![InteractionCondition {
                    property: "radiation_level".into(),
                    comparator: Comparator::GreaterThan,
                    value: 10.0,
                }],
                priority: 8,
                enabled: true,
            },
        ]
    }

    fn default_generations() -> Vec<GenerationRule> {
        vec![GenerationRule {
            rule_name: "rust_formation".into(),
            input_properties: vec!["thermal_energy".into(), "mass_density".into()],
            output_entity_type: "rust_particle".into(),
            output_properties: vec![PropertyMapping {
                ai_property: "size".into(),
                meta_property: "radius".into(),
                conversion: ConversionType::Direct,
                scale: 0.1,
                offset: 0.01,
                clamp_range: Some([0.01, 1.0]),
            }],
            probability: 0.3,
            cooldown_ms: 5000,
        }]
    }

    pub fn convert_property(&self, ai_property: &str, value: f32) -> Option<(String, f32)> {
        let mapping = self.property_mappings.iter().find(|m| m.ai_property == ai_property)?;
        let converted = match mapping.conversion {
            ConversionType::Direct => value * mapping.scale + mapping.offset,
            ConversionType::Inverse => mapping.scale / (value + 0.001) + mapping.offset,
            ConversionType::Squared => value.powi(2) * mapping.scale + mapping.offset,
            ConversionType::Sqrt => value.sqrt() * mapping.scale + mapping.offset,
            ConversionType::Log => (value + 0.001).ln() * mapping.scale + mapping.offset,
            ConversionType::Exp => value.exp() * mapping.scale + mapping.offset,
            ConversionType::Sigmoid => {
                1.0 / (1.0 + (-value * mapping.scale).exp()) + mapping.offset
            },
            ConversionType::Threshold(t) => {
                if value >= t {
                    1.0
                } else {
                    0.0
                }
            },
        };
        let clamped = if let Some([min, max]) = mapping.clamp_range {
            converted.clamp(min, max)
        } else {
            converted
        };
        Some((mapping.meta_property.clone(), clamped))
    }

    pub fn evaluate_interactions(
        &self,
        source: &MetaEntityState,
        targets: &[MetaEntityState],
        _now: u64,
    ) -> Vec<(u64, Vec<PropertyEffect>)> {
        let mut results = Vec::new();
        for rule in &self.interaction_rules {
            if !rule.enabled {
                continue;
            }
            let conditions_met = rule.conditions.iter().all(|cond| {
                source.properties.iter().any(|p| {
                    p.name == cond.property
                        && match cond.comparator {
                            Comparator::GreaterThan => p.value > cond.value,
                            Comparator::LessThan => p.value < cond.value,
                            Comparator::Equals => (p.value - cond.value).abs() < 0.001,
                            Comparator::Between(lo, hi) => p.value >= lo && p.value <= hi,
                            Comparator::NotEquals => (p.value - cond.value).abs() >= 0.001,
                        }
                })
            });
            if !conditions_met {
                continue;
            }
            for target in targets {
                let effects: Vec<PropertyEffect> = rule
                    .target_effects
                    .iter()
                    .filter(|e| {
                        target.properties.iter().any(|p| p.name == e.property_name && p.mutable)
                    })
                    .cloned()
                    .collect();
                if !effects.is_empty() {
                    results.push((target.entity_id, effects));
                }
            }
        }
        results
    }

    pub fn check_generation(&self, state: &MetaEntityState, now: u64) -> Vec<&GenerationRule> {
        self.generation_rules
            .iter()
            .filter(|rule| {
                let inputs_available = rule
                    .input_properties
                    .iter()
                    .all(|p| state.properties.iter().any(|sp| sp.name == *p));
                let not_on_cooldown = state
                    .generation_history
                    .iter()
                    .all(|t| now.saturating_sub(*t) > rule.cooldown_ms);
                inputs_available && not_on_cooldown && rand::random::<f32>() < rule.probability
            })
            .collect()
    }

    pub fn apply_effect(&self, state: &mut MetaEntityState, effect: &PropertyEffect, now: u64) {
        if let Some(prop) = state.properties.iter_mut().find(|p| p.name == effect.property_name) {
            match effect.operation {
                EffectOperation::Add => prop.value += effect.magnitude,
                EffectOperation::Multiply => prop.value *= effect.magnitude,
                EffectOperation::Set => prop.value = effect.magnitude,
                EffectOperation::Lerp => {
                    let target = effect.magnitude;
                    prop.value += (target - prop.value) * 0.1;
                },
                EffectOperation::Clamp => {
                    prop.value = prop.value.clamp(prop.min, prop.max);
                },
            }
            prop.value = prop.value.clamp(prop.min, prop.max);
        }
        state.active_effects.push(ActiveEffect {
            effect_name: "".into(),
            property_name: effect.property_name.clone(),
            start_value: state
                .properties
                .iter()
                .find(|p| p.name == effect.property_name)
                .map(|p| p.value)
                .unwrap_or(0.0),
            target_value: effect.magnitude,
            started_at: now,
            duration_ms: effect.duration_ms,
        });
    }
}

impl Default for MetaEntityBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(id: u64, thermal: f32, integrity: f32, radiation: f32) -> MetaEntityState {
        MetaEntityState {
            entity_id: id,
            properties: vec![
                MetaProperty {
                    name: "thermal_energy".into(),
                    value: thermal,
                    min: 0.0,
                    max: 1000.0,
                    mutable: true,
                    tags: vec![],
                },
                MetaProperty {
                    name: "structural_integrity".into(),
                    value: integrity,
                    min: 0.0,
                    max: 1.0,
                    mutable: true,
                    tags: vec![],
                },
                MetaProperty {
                    name: "radiation_level".into(),
                    value: radiation,
                    min: 0.0,
                    max: 1000.0,
                    mutable: true,
                    tags: vec![],
                },
                MetaProperty {
                    name: "mass_density".into(),
                    value: 2.7,
                    min: 0.0,
                    max: 100.0,
                    mutable: false,
                    tags: vec![],
                },
            ],
            active_effects: vec![],
            generation_history: vec![],
        }
    }

    #[test]
    fn test_property_conversion_direct() {
        let bridge = MetaEntityBridge::new();
        let result = bridge.convert_property("temperature", 25.0);
        assert!(result.is_some());
        let (name, value) = result.unwrap();
        assert_eq!(name, "thermal_energy");
        assert_eq!(value, 25.0);
    }

    #[test]
    fn test_property_conversion_log() {
        let bridge = MetaEntityBridge::new();
        let result = bridge.convert_property("radioactivity", 100.0);
        assert!(result.is_some());
        let (name, value) = result.unwrap();
        assert_eq!(name, "radiation_level");
        assert!(value > 0.0);
    }

    #[test]
    fn test_property_clamping() {
        let bridge = MetaEntityBridge::new();
        let result = bridge.convert_property("density", -5.0);
        assert!(result.is_some());
        let (_, value) = result.unwrap();
        assert!(value >= 0.01);
    }

    #[test]
    fn test_interaction_heat_transfer() {
        let bridge = MetaEntityBridge::new();
        let source = make_state(1, 500.0, 1.0, 0.0);
        let targets = vec![make_state(2, 100.0, 1.0, 0.0)];
        let effects = bridge.evaluate_interactions(&source, &targets, 1000);
        assert!(!effects.is_empty());
        assert_eq!(effects[0].0, 2);
    }

    #[test]
    fn test_interaction_no_conditions_met() {
        let bridge = MetaEntityBridge::new();
        let source = make_state(1, 200.0, 1.0, 0.0);
        let targets = vec![make_state(2, 100.0, 1.0, 0.0)];
        let effects = bridge.evaluate_interactions(&source, &targets, 1000);
        assert!(effects.is_empty());
    }

    #[test]
    fn test_apply_effect_add() {
        let bridge = MetaEntityBridge::new();
        let mut state = make_state(1, 200.0, 1.0, 0.0);
        let effect = PropertyEffect {
            property_name: "thermal_energy".into(),
            operation: EffectOperation::Add,
            magnitude: 100.0,
            duration_ms: 1000,
        };
        bridge.apply_effect(&mut state, &effect, 1000);
        let thermal = state.properties.iter().find(|p| p.name == "thermal_energy").unwrap();
        assert_eq!(thermal.value, 300.0);
    }

    #[test]
    fn test_unknown_property() {
        let bridge = MetaEntityBridge::new();
        let result = bridge.convert_property("nonexistent", 10.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_conversion_sigmoid() {
        let mapping = PropertyMapping {
            ai_property: "test".into(),
            meta_property: "output".into(),
            conversion: ConversionType::Sigmoid,
            scale: 1.0,
            offset: 0.0,
            clamp_range: None,
        };
        assert_eq!(mapping.conversion, ConversionType::Sigmoid);
    }
}
