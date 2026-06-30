use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::ecs::{ComponentType, ComponentValue, EcsWorld, SystemId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArbitrationPriority {
    PhysicsCorrection = 0,
    ChemicalCorrosion = 1,
    BiologicalMetabolism = 2,
    FieldInteraction = 3,
    ParticleInteraction = 4,
    PlayerAction = 5,
    External = 6,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeEvent {
    pub entity_id: Uuid,
    pub component_type: ComponentType,
    pub new_value: ComponentValue,
    pub priority: ArbitrationPriority,
    pub source_system: SystemId,
    pub tick: u64,
    pub blocked: bool,
    pub propagation_events: Vec<PropagationEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationEvent {
    pub target_entity_id: Uuid,
    pub target_component: ComponentType,
    pub effect_value: f32,
    pub effect_type: PropagationEffectType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropagationEffectType {
    DamageArmor,
    TransferHeat,
    SpreadCorrosion,
    InduceStress,
    DepleteResource,
    TriggerReaction,
}

#[derive(Debug, Clone)]
pub struct CrossSystemArbiter {
    pending_changes: Vec<StateChangeEvent>,
    resolved_changes: Vec<StateChangeEvent>,
    merge_rules: HashMap<ComponentType, MergeRule>,
    propagation_rules: HashMap<(ComponentType, SystemId), Vec<PropagationRule>>,
    blocked_effects: Vec<BlockedEffect>,
    tick: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum MergeRule {
    MaxAbsolute,
    Sum,
    Overwrite,
    PriorityBased,
    Custom(fn(&[f32]) -> f32),
}

#[derive(Debug, Clone)]
struct PropagationRule {
    pub target_component: ComponentType,
    pub effect_multiplier: f32,
    pub effect_type: PropagationEffectType,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BlockedEffect {
    pub entity_id: Uuid,
    pub component_type: ComponentType,
    pub original_value: ComponentValue,
    pub blocking_entity: Uuid,
    pub propagated: bool,
}

impl CrossSystemArbiter {
    pub fn new() -> Self {
        let mut merge_rules = HashMap::new();
        merge_rules.insert(ComponentType::Temperature, MergeRule::MaxAbsolute);
        merge_rules.insert(ComponentType::Health, MergeRule::MaxAbsolute);
        merge_rules.insert(ComponentType::Stress, MergeRule::MaxAbsolute);
        merge_rules.insert(ComponentType::RadiationLevel, MergeRule::Sum);
        merge_rules.insert(ComponentType::Velocity, MergeRule::Overwrite);
        merge_rules.insert(ComponentType::Position, MergeRule::Overwrite);
        merge_rules.insert(ComponentType::CorrosionDepth, MergeRule::Sum);

        let mut propagation_rules = HashMap::new();
        propagation_rules.insert(
            (ComponentType::Health, SystemId::Chemistry),
            vec![PropagationRule {
                target_component: ComponentType::CorrosionDepth,
                effect_multiplier: 0.3,
                effect_type: PropagationEffectType::DamageArmor,
            }],
        );
        propagation_rules.insert(
            (ComponentType::Temperature, SystemId::Chemistry),
            vec![PropagationRule {
                target_component: ComponentType::Stress,
                effect_multiplier: 0.1,
                effect_type: PropagationEffectType::TransferHeat,
            }],
        );

        Self {
            pending_changes: Vec::new(),
            resolved_changes: Vec::new(),
            merge_rules,
            propagation_rules,
            blocked_effects: Vec::new(),
            tick: 0,
        }
    }

    pub fn submit_change(
        &mut self,
        entity_id: Uuid,
        component_type: ComponentType,
        value: ComponentValue,
        priority: ArbitrationPriority,
        source_system: SystemId,
    ) {
        self.pending_changes.push(StateChangeEvent {
            entity_id,
            component_type,
            new_value: value,
            priority,
            source_system,
            tick: self.tick,
            blocked: false,
            propagation_events: Vec::new(),
        });
    }

    pub fn resolve(&mut self, ecs: &mut EcsWorld) {
        self.tick += 1;

        let mut merged: HashMap<(Uuid, ComponentType), Vec<StateChangeEvent>> = HashMap::new();
        for change in self.pending_changes.drain(..) {
            merged.entry((change.entity_id, change.component_type)).or_default().push(change);
        }

        for ((entity_id, component_type), mut changes) in merged {
            changes.sort_by_key(|c| c.priority);

            let rule = self.merge_rules.get(&component_type).unwrap_or(&MergeRule::Overwrite);
            let final_value = match rule {
                MergeRule::MaxAbsolute => {
                    let mut max_change = changes[0].clone();
                    for change in &changes[1..] {
                        if let ComponentValue::Float32(new_val) = change.new_value {
                            if let ComponentValue::Float32(curr_val) = max_change.new_value {
                                if new_val.abs() > curr_val.abs() {
                                    max_change = change.clone();
                                }
                            }
                        }
                    }
                    max_change.new_value
                },
                MergeRule::Sum => {
                    let mut total = 0.0f32;
                    for change in &changes {
                        if let ComponentValue::Float32(v) = change.new_value {
                            total += v;
                        }
                    }
                    ComponentValue::Float32(total)
                },
                MergeRule::Overwrite | MergeRule::PriorityBased => changes[0].new_value.clone(),
                MergeRule::Custom(f) => {
                    let values: Vec<f32> = changes
                        .iter()
                        .filter_map(|c| {
                            if let ComponentValue::Float32(v) = c.new_value {
                                Some(v)
                            } else {
                                None
                            }
                        })
                        .collect();
                    ComponentValue::Float32(f(&values))
                },
            };

            ecs.set_component(entity_id, component_type, final_value, SystemId::External(0));

            for change in &changes {
                if let Some(rules) =
                    self.propagation_rules.get(&(component_type, change.source_system))
                {
                    for rule in rules {
                        if let ComponentValue::Float32(v) = change.new_value {
                            let prop_value = v * rule.effect_multiplier;
                            let prop_event = PropagationEvent {
                                target_entity_id: entity_id,
                                target_component: rule.target_component,
                                effect_value: prop_value,
                                effect_type: rule.effect_type,
                            };
                            let mut resolved_change = change.clone();
                            resolved_change.propagation_events.push(prop_event);
                            self.resolved_changes.push(resolved_change);
                        }
                    }
                }
            }
        }

        let blocked_data: Vec<(Uuid, f32)> = self
            .blocked_effects
            .iter()
            .filter(|b| !b.propagated)
            .filter_map(|b| {
                if let ComponentValue::Float32(v) = b.original_value {
                    if v.abs() > 0.01 { Some((b.blocking_entity, v)) } else { None }
                } else {
                    None
                }
            })
            .collect();

        for (blocking_entity, v) in blocked_data {
            self.submit_change(
                blocking_entity,
                ComponentType::Health,
                ComponentValue::Float32(-v.abs() * 0.5),
                ArbitrationPriority::PhysicsCorrection,
                SystemId::Physics,
            );
        }
        self.blocked_effects.clear();
    }

    pub fn block_effect(
        &mut self,
        entity_id: Uuid,
        component_type: ComponentType,
        original_value: ComponentValue,
        blocking_entity: Uuid,
    ) {
        self.blocked_effects.push(BlockedEffect {
            entity_id,
            component_type,
            original_value,
            blocking_entity,
            propagated: false,
        });
    }

    pub fn resolved_changes(&self) -> &[StateChangeEvent] {
        &self.resolved_changes
    }

    pub fn drain_resolved(&mut self) -> Vec<StateChangeEvent> {
        std::mem::take(&mut self.resolved_changes)
    }

    pub fn pending_count(&self) -> usize {
        self.pending_changes.len()
    }

    pub fn blocked_count(&self) -> usize {
        self.blocked_effects.len()
    }

    pub fn clear(&mut self) {
        self.pending_changes.clear();
        self.resolved_changes.clear();
        self.blocked_effects.clear();
    }
}

impl Default for CrossSystemArbiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(ArbitrationPriority::PhysicsCorrection < ArbitrationPriority::ChemicalCorrosion);
        assert!(ArbitrationPriority::ChemicalCorrosion < ArbitrationPriority::BiologicalMetabolism);
    }

    #[test]
    fn test_merge_max_absolute() {
        let mut arbiter = CrossSystemArbiter::new();
        let mut ecs = EcsWorld::new();
        let entity_id = ecs.spawn();

        arbiter.submit_change(
            entity_id,
            ComponentType::Temperature,
            ComponentValue::Float32(100.0),
            ArbitrationPriority::PhysicsCorrection,
            SystemId::Physics,
        );
        arbiter.submit_change(
            entity_id,
            ComponentType::Temperature,
            ComponentValue::Float32(-200.0),
            ArbitrationPriority::ChemicalCorrosion,
            SystemId::Chemistry,
        );

        arbiter.resolve(&mut ecs);

        let temp = ecs.entity(entity_id).unwrap().get_f32(ComponentType::Temperature);
        assert_eq!(temp, Some(-200.0));
    }
}
