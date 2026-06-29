use godot::prelude::*;

use wasteland_ai_bridge::character_bridge::{
    BehaviorInjection, CharacterBridge, CharacterBridgeConfig, DialogueContextParams,
    DialogueRequest, EmotionalState, KnowledgeInjection, MoralCompassInjection, NpcRuntimeConfig,
    PersonalityInjection,
};
use wasteland_ai_bridge::meta_bridge::{
    Comparator, ConversionType, EffectOperation, GenerationRule, InteractionCondition,
    InteractionRule, MetaEntityBridge, PropertyEffect, PropertyMapping,
};
use wasteland_ai_bridge::physics_bridge::{
    PhysicsAction, PhysicsActionType, PhysicsBridge, PhysicsBridgeConfig,
};
use wasteland_ai_bridge::world_bridge::{WorldBridge, WorldBridgeConfig, WorldSpawnRequest};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandAIBridge {
    #[var]
    max_force: f32,
    #[var]
    max_torque: f32,
    #[var]
    safety_checks_enabled: bool,

    physics_bridge: PhysicsBridge,
    world_bridge: WorldBridge,
    character_bridge: CharacterBridge,
    meta_bridge: MetaEntityBridge,
    action_count: i64,
    rejected_count: i64,
    spawn_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandAIBridge {
    fn init(base: Base<Node>) -> Self {
        let p_config =
            PhysicsBridgeConfig { max_force: 10000.0, max_torque: 5000.0, ..Default::default() };
        let w_config = WorldBridgeConfig::default();
        let c_config = CharacterBridgeConfig::default();
        Self {
            max_force: 10000.0,
            max_torque: 5000.0,
            safety_checks_enabled: true,
            physics_bridge: PhysicsBridge::new(p_config),
            world_bridge: WorldBridge::new(w_config),
            character_bridge: CharacterBridge::new(c_config),
            meta_bridge: MetaEntityBridge::new(),
            action_count: 0,
            rejected_count: 0,
            spawn_count: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandAIBridge {
    #[func]
    fn apply_force(
        &mut self,
        target_entity: GString,
        fx: f32,
        fy: f32,
        fz: f32,
        duration_ms: i64,
    ) -> Dictionary<Variant, Variant> {
        self.action_count += 1;
        let action = PhysicsAction {
            action_type: PhysicsActionType::ApplyForce,
            target_entity: Some(target_entity.to_string()),
            force: [fx, fy, fz],
            torque: [0.0, 0.0, 0.0],
            impulse: [0.0, 0.0, 0.0],
            constraint_params: None,
            duration_ms: duration_ms as u64,
            priority: 0,
        };
        let translated = self.physics_bridge.translate_action(&action);
        let response = self.physics_bridge.simulate_response(&translated);
        if !response.success {
            self.rejected_count += 1;
        }
        dict! {
            "success" => response.success,
            "energy_consumed" => response.energy_consumed,
            "velocity_x" => response.resulting_velocity[0],
            "velocity_y" => response.resulting_velocity[1],
            "velocity_z" => response.resulting_velocity[2],
        }
    }

    #[func]
    fn apply_impulse(
        &mut self,
        target_entity: GString,
        ix: f32,
        iy: f32,
        iz: f32,
    ) -> Dictionary<Variant, Variant> {
        self.action_count += 1;
        let action = PhysicsAction {
            action_type: PhysicsActionType::ApplyImpulse,
            target_entity: Some(target_entity.to_string()),
            force: [0.0, 0.0, 0.0],
            torque: [0.0, 0.0, 0.0],
            impulse: [ix, iy, iz],
            constraint_params: None,
            duration_ms: 0,
            priority: 0,
        };
        let translated = self.physics_bridge.translate_action(&action);
        let response = self.physics_bridge.simulate_response(&translated);
        if !response.success {
            self.rejected_count += 1;
        }
        dict! {
            "success" => response.success,
            "energy_consumed" => response.energy_consumed,
        }
    }

    #[func]
    fn request_spawn(
        &mut self,
        entity_type: GString,
        x: f32,
        y: f32,
        z: f32,
    ) -> Dictionary<Variant, Variant> {
        self.spawn_count += 1;
        let request = WorldSpawnRequest {
            entity_type: entity_type.to_string(),
            position: [x, y, z],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            properties: vec![],
            components: vec![],
            spawn_priority: 0,
            chunk_id: None,
        };
        let result = self.world_bridge.spawn_entity(&request);
        dict! {
            "success" => result.success,
            "entity_id" => result.entity_id as i64,
            "error" => &GString::from(result.error.unwrap_or_default().as_str()),
        }
    }

    #[func]
    fn query_nearby_entities(&self, x: f32, y: f32, z: f32, radius: f32) -> GString {
        let entities = self.world_bridge.query_nearby([x, y, z], radius, None);
        let names: Vec<String> = entities.iter().map(|e| e.entity_type.clone()).collect();
        GString::from(names.join(",").as_str())
    }

    #[func]
    fn register_npc(&mut self, npc_id: GString, personality_json: GString) -> bool {
        let _ = personality_json;
        let config = NpcRuntimeConfig {
            npc_id: npc_id.to_string(),
            personality: PersonalityInjection {
                openness: 0.5,
                conscientiousness: 0.5,
                extraversion: 0.5,
                agreeableness: 0.5,
                neuroticism: 0.5,
                traits: vec![],
                moral_compass: MoralCompassInjection {
                    honesty: 0.5,
                    compassion: 0.5,
                    loyalty: 0.5,
                    courage: 0.5,
                },
            },
            knowledge: KnowledgeInjection {
                facts: vec![],
                skills: vec![],
                relationships: vec![],
                world_lore: vec![],
            },
            behavior: BehaviorInjection {
                patterns: vec![],
                idle_behaviors: vec![],
                combat_style: "neutral".into(),
                social_style: "neutral".into(),
                fear_responses: vec![],
            },
            goals: vec![],
            emotional_state: EmotionalState {
                dominant_emotion: "neutral".into(),
                intensity: 0.3,
                secondary_emotion: None,
                mood: "calm".into(),
                stress_level: 0.2,
            },
        };
        self.character_bridge.register_npc(config).is_ok()
    }

    #[func]
    fn process_npc_dialogue(
        &mut self,
        npc_id: GString,
        player_message: GString,
        topic: GString,
    ) -> Dictionary<Variant, Variant> {
        let request = DialogueRequest {
            npc_id: npc_id.to_string(),
            player_message: player_message.to_string(),
            context: DialogueContextParams {
                location: None,
                time_of_day: 12.0,
                weather: "clear".into(),
                nearby_entities: vec![],
                world_events: vec![topic.to_string()],
                player_reputation: 0.5,
            },
            memory_query: None,
            emotion_trigger: None,
        };
        let response = self.character_bridge.process_dialogue(&request, 0);
        match response {
            Some(r) => dict! {
                "npc_id" => &npc_id,
                "text" => &GString::from(r.text.as_str()),
                "emotion" => &GString::from(r.emotion.as_str()),
                "affinity_delta" => r.affinity_delta,
            },
            None => dict! {
                "npc_id" => &npc_id,
                "text" => &GString::from(""),
                "emotion" => &GString::from(""),
                "affinity_delta" => 0.0f32,
            },
        }
    }

    #[func]
    fn add_property_mapping(
        &mut self,
        ai_prop: GString,
        meta_prop: GString,
        scale: f32,
        offset: f32,
    ) {
        let mapping = PropertyMapping {
            ai_property: ai_prop.to_string(),
            meta_property: meta_prop.to_string(),
            conversion: ConversionType::Direct,
            scale,
            offset,
            clamp_range: None,
        };
        self.meta_bridge.property_mappings.push(mapping);
    }

    #[func]
    fn map_property(&self, ai_prop: GString, value: f32) -> Dictionary<Variant, Variant> {
        match self.meta_bridge.convert_property(&ai_prop.to_string(), value) {
            Some((name, converted)) => dict! {
                "found" => true,
                "meta_property" => &GString::from(name.as_str()),
                "converted_value" => converted,
            },
            None => dict! {
                "found" => false,
                "meta_property" => &GString::from(""),
                "converted_value" => value,
            },
        }
    }

    #[func]
    fn add_generation_rule(
        &mut self,
        rule_name: GString,
        output_entity_type: GString,
        probability: f32,
        cooldown_ms: i64,
    ) {
        let rule = GenerationRule {
            rule_name: rule_name.to_string(),
            input_properties: vec![],
            output_entity_type: output_entity_type.to_string(),
            output_properties: vec![],
            probability,
            cooldown_ms: cooldown_ms as u64,
        };
        self.meta_bridge.generation_rules.push(rule);
    }

    #[func]
    fn add_interaction_rule(
        &mut self,
        rule_name: GString,
        source_prop: GString,
        target_prop: GString,
        comparator: GString,
        threshold: f32,
    ) {
        let comp = match comparator.to_string().as_str() {
            "greater_than" => Comparator::GreaterThan,
            "less_than" => Comparator::LessThan,
            "equals" => Comparator::Equals,
            "between" => Comparator::Between(0.0, threshold),
            _ => Comparator::GreaterThan,
        };
        let rule = InteractionRule {
            rule_name: rule_name.to_string(),
            source_properties: vec![source_prop.to_string()],
            target_effects: vec![PropertyEffect {
                property_name: target_prop.to_string(),
                operation: EffectOperation::Add,
                magnitude: threshold,
                duration_ms: 1000,
            }],
            conditions: vec![InteractionCondition {
                property: source_prop.to_string(),
                comparator: comp,
                value: threshold,
            }],
            priority: 0,
            enabled: true,
        };
        self.meta_bridge.interaction_rules.push(rule);
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "action_count" => self.action_count,
            "rejected_count" => self.rejected_count,
            "spawn_count" => self.spawn_count,
            "max_force" => self.max_force,
            "max_torque" => self.max_torque,
            "safety_checks_enabled" => self.safety_checks_enabled,
            "total_npcs" => self.character_bridge.total_npcs() as i64,
            "total_entities" => self.world_bridge.total_entities() as i64,
        }
    }
}
