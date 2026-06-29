use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use wasteland_ai::PersonalityTraits;
use wasteland_ai_bridge::character_bridge::{
    BehaviorInjection, CharacterBridge, CharacterBridgeConfig, DialogueContextParams,
    DialogueRequest, DialogueResponse, EmotionalState, FactInjection, GoalInjection,
    KnowledgeInjection, MoralCompassInjection, NpcRuntimeConfig, PersonalityInjection,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDefinition {
    pub id: Uuid,
    pub name: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub health: f32,
    pub max_health: f32,
    pub species: NpcSpecies,
    pub faction: String,
    pub personality: PersonalityTraits,
    pub behavior_config: BehaviorInjection,
    pub goals: Vec<GoalInjection>,
    pub initial_emotion: EmotionalState,
    pub initial_knowledge: KnowledgeInjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcSpecies {
    Human,
    Mutant,
    Ghoul,
    Robot,
    Animal,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcState {
    pub id: Uuid,
    pub name: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub health: f32,
    pub max_health: f32,
    pub alive: bool,
    pub species: NpcSpecies,
    pub faction: String,
    pub personality: PersonalityTraits,
    pub emotion: EmotionalState,
    pub current_goal: Option<String>,
    pub current_action: Option<String>,
    pub active: bool,
    pub spawn_tick: u64,
    pub last_update_tick: u64,
    pub dialogue_count: u32,
    pub stress_level: f32,
    pub radiation_dose: f32,
    pub toxin_level: f32,
    pub combat_state: NpcCombatState,
}

impl NpcState {
    pub fn apply_damage(&mut self, damage: f32, damage_type: &str) {
        if !self.alive {
            return;
        }
        self.health -= damage;
        self.health = self.health.clamp(0.0, self.max_health);

        if damage > 0.0 {
            self.emotion.dominant_emotion = "fear".into();
            self.emotion.intensity = (self.emotion.intensity + damage / self.max_health).min(1.0);

            if damage_type == "radiation" {
                self.radiation_dose += damage;
            } else if damage_type == "chemical" || damage_type == "toxin" {
                self.toxin_level += damage * 0.1;
            }

            if self.health <= 0.0 {
                self.alive = false;
                self.combat_state = NpcCombatState::Dead;
            } else if self.health < self.max_health * 0.3 {
                self.combat_state = NpcCombatState::Fleeing;
            } else {
                self.combat_state = NpcCombatState::Combat;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcCombatState {
    Idle,
    Alert,
    Combat,
    Fleeing,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcInteraction {
    pub npc_id: Uuid,
    pub target_id: Option<Uuid>,
    pub interaction_type: NpcInteractionType,
    pub timestamp: f64,
    pub result: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcInteractionType {
    Dialogue,
    Combat,
    Trade,
    Help,
    Flee,
    Observe,
}

#[derive(Debug, Clone)]
pub struct NpcSystem {
    pub npcs: Vec<NpcState>,
    pub definitions: Vec<NpcDefinition>,
    pub interactions: Vec<NpcInteraction>,
    pub bridge: CharacterBridge,
    pub spawn_queue: Vec<NpcDefinition>,
    pub despawn_queue: Vec<Uuid>,
    pub tick_count: u64,
    pub max_npcs: usize,
}

impl NpcSystem {
    pub fn new(max_npcs: usize) -> Self {
        let bridge_config = CharacterBridgeConfig {
            max_npcs,
            update_interval_ms: 100,
            memory_consolidation_interval_ms: 60000,
            emotional_decay_rate: 0.01,
            relationship_decay_rate: 0.001,
        };

        Self {
            npcs: Vec::new(),
            definitions: Vec::new(),
            interactions: Vec::new(),
            bridge: CharacterBridge::new(bridge_config),
            spawn_queue: Vec::new(),
            despawn_queue: Vec::new(),
            tick_count: 0,
            max_npcs,
        }
    }

    pub fn queue_spawn(&mut self, def: NpcDefinition) {
        self.spawn_queue.push(def);
    }

    pub fn queue_despawn(&mut self, npc_id: Uuid) {
        self.despawn_queue.push(npc_id);
    }

    pub fn update(&mut self, dt: f32, world_time: f64) {
        self.tick_count += 1;

        self.process_spawn_queue();
        self.process_despawn_queue();

        let _timestamp = world_time as u64;
        let mut behavior_update_indices: Vec<usize> = Vec::new();

        for (idx, npc) in self.npcs.iter_mut().enumerate() {
            if !npc.alive || !npc.active {
                continue;
            }

            npc.last_update_tick = self.tick_count;

            npc.emotion.intensity = (npc.emotion.intensity * (1.0 - 0.01 * dt)).max(0.0);
            npc.stress_level =
                (npc.stress_level * 0.99 + npc.emotion.intensity * 0.01).clamp(0.0, 1.0);

            npc.radiation_dose *= 0.999;
            if npc.radiation_dose > 50.0 {
                npc.health -= npc.radiation_dose * 0.001 * dt;
            }

            npc.toxin_level *= 0.995;
            if npc.toxin_level > 0.0 {
                npc.health -= npc.toxin_level * 0.01 * dt;
            }

            if npc.health <= 0.0 {
                npc.alive = false;
                npc.health = 0.0;
                npc.combat_state = NpcCombatState::Dead;
                continue;
            }

            if npc.health < npc.max_health * 0.1 {
                npc.combat_state = NpcCombatState::Fleeing;
            }

            if self.tick_count.is_multiple_of(10) {
                behavior_update_indices.push(idx);
            }
        }

        for idx in behavior_update_indices {
            if let Some(npc) = self.npcs.get_mut(idx) {
                let context = DialogueContextParams {
                    location: Some(format!(
                        "({:.1},{:.1},{:.1})",
                        npc.position.x, npc.position.y, npc.position.z
                    )),
                    time_of_day: 12.0,
                    weather: "clear".into(),
                    nearby_entities: vec![],
                    world_events: vec![],
                    player_reputation: 0.5,
                };

                if let Some(decision) = self.bridge.decide_behavior(&npc.id.to_string(), &context) {
                    npc.current_action = Some(decision.action.clone());

                    if npc.stress_level > 0.7 {
                        npc.combat_state = NpcCombatState::Fleeing;
                    } else if decision.action == "patrol" {
                        npc.combat_state = NpcCombatState::Idle;
                    } else if decision.action.contains("attack") {
                        npc.combat_state = NpcCombatState::Combat;
                    }

                    npc.current_goal = Some(decision.action);
                }
            }
        }

        self.npcs.retain(|n| n.alive);
    }

    fn process_spawn_queue(&mut self) {
        let to_spawn: Vec<_> = std::mem::take(&mut self.spawn_queue);
        for def in to_spawn {
            if self.npcs.len() >= self.max_npcs {
                break;
            }

            let runtime_config = NpcRuntimeConfig {
                npc_id: def.id.to_string(),
                personality: PersonalityInjection {
                    openness: def.personality.openness,
                    conscientiousness: def.personality.conscientiousness,
                    extraversion: def.personality.extraversion,
                    agreeableness: def.personality.agreeableness,
                    neuroticism: def.personality.neuroticism,
                    traits: vec![format!("{:?}", def.species)],
                    moral_compass: MoralCompassInjection {
                        honesty: def.personality.loyalty,
                        compassion: def.personality.agreeableness,
                        loyalty: def.personality.loyalty,
                        courage: 1.0 - def.personality.neuroticism,
                    },
                },
                knowledge: def.initial_knowledge.clone(),
                behavior: def.behavior_config.clone(),
                goals: def.goals.clone(),
                emotional_state: def.initial_emotion.clone(),
            };

            let _ = self.bridge.register_npc(runtime_config);

            let state = NpcState {
                id: def.id,
                name: def.name.clone(),
                position: def.position,
                velocity: def.velocity,
                health: def.health,
                max_health: def.max_health,
                alive: true,
                species: def.species,
                faction: def.faction.clone(),
                personality: def.personality.clone(),
                emotion: def.initial_emotion.clone(),
                current_goal: None,
                current_action: None,
                active: true,
                spawn_tick: self.tick_count,
                last_update_tick: self.tick_count,
                dialogue_count: 0,
                stress_level: 0.0,
                radiation_dose: 0.0,
                toxin_level: 0.0,
                combat_state: NpcCombatState::Idle,
            };

            self.npcs.push(state);
            self.definitions.push(def);
        }
    }

    fn process_despawn_queue(&mut self) {
        let to_despawn: Vec<_> = std::mem::take(&mut self.despawn_queue);
        for id in to_despawn {
            self.bridge.unregister_npc(&id.to_string());
            self.npcs.retain(|n| n.id != id);
            self.definitions.retain(|d| d.id != id);
        }
    }

    pub fn process_dialogue(
        &mut self,
        npc_id: Uuid,
        player_message: &str,
        time_of_day: f32,
        weather: &str,
        player_reputation: f32,
    ) -> Option<DialogueResponse> {
        let npc = self.npcs.iter_mut().find(|n| n.id == npc_id)?;
        if !npc.alive {
            return None;
        }

        let context = DialogueContextParams {
            location: Some(format!(
                "({:.1},{:.1},{:.1})",
                npc.position.x, npc.position.y, npc.position.z
            )),
            time_of_day,
            weather: weather.into(),
            nearby_entities: vec![],
            world_events: vec![],
            player_reputation,
        };

        let request = DialogueRequest {
            npc_id: npc_id.to_string(),
            player_message: player_message.into(),
            context,
            memory_query: None,
            emotion_trigger: None,
        };

        let response = self.bridge.process_dialogue(&request, self.tick_count);
        if let Some(ref resp) = response {
            npc.dialogue_count += 1;
            if resp.affinity_delta > 0.0 {
                npc.emotion.dominant_emotion = "joy".into();
                npc.emotion.intensity =
                    (npc.emotion.intensity + resp.affinity_delta * 0.1).min(1.0);
            } else if resp.affinity_delta < 0.0 {
                npc.emotion.dominant_emotion = "anger".into();
                npc.emotion.intensity =
                    (npc.emotion.intensity + (-resp.affinity_delta) * 0.1).min(1.0);
            }
        }
        response
    }

    pub fn apply_damage(&mut self, npc_id: Uuid, damage: f32, damage_type: &str) {
        if let Some(npc) = self.npcs.iter_mut().find(|n| n.id == npc_id) {
            npc.apply_damage(damage, damage_type);
        }
    }

    pub fn apply_force(&mut self, npc_id: Uuid, force: Vec3) {
        if let Some(npc) = self.npcs.iter_mut().find(|n| n.id == npc_id) {
            if !npc.alive {
                return;
            }
            npc.velocity += force * 0.1;
            npc.position += npc.velocity;
            npc.velocity *= 0.95;
        }
    }

    pub fn get_npc(&self, npc_id: Uuid) -> Option<&NpcState> {
        self.npcs.iter().find(|n| n.id == npc_id)
    }

    pub fn get_npc_mut(&mut self, npc_id: Uuid) -> Option<&mut NpcState> {
        self.npcs.iter_mut().find(|n| n.id == npc_id)
    }

    pub fn get_npcs_by_faction(&self, faction: &str) -> Vec<&NpcState> {
        self.npcs.iter().filter(|n| n.faction == faction).collect()
    }

    pub fn get_npcs_in_radius(&self, position: Vec3, radius: f32) -> Vec<&NpcState> {
        self.npcs.iter().filter(|n| n.alive && (n.position - position).length() <= radius).collect()
    }

    pub fn get_npc_positions(&self) -> Vec<[f32; 3]> {
        self.npcs
            .iter()
            .filter(|n| n.alive)
            .map(|n| [n.position.x, n.position.y, n.position.z])
            .collect()
    }

    pub fn get_npc_colors(&self) -> Vec<[f32; 4]> {
        self.npcs
            .iter()
            .filter(|n| n.alive)
            .map(|n| {
                let health_ratio = n.health / n.max_health;
                let base = match n.combat_state {
                    NpcCombatState::Combat => [1.0, 0.2, 0.2, 1.0],
                    NpcCombatState::Fleeing => [1.0, 0.8, 0.0, 1.0],
                    NpcCombatState::Alert => [1.0, 0.6, 0.0, 1.0],
                    _ => match n.species {
                        NpcSpecies::Human => [0.4, 0.7, 0.4, 1.0],
                        NpcSpecies::Mutant => [0.6, 0.3, 0.6, 1.0],
                        NpcSpecies::Ghoul => [0.5, 0.5, 0.3, 1.0],
                        NpcSpecies::Robot => [0.3, 0.3, 0.5, 1.0],
                        NpcSpecies::Animal => [0.7, 0.5, 0.3, 1.0],
                        NpcSpecies::Custom(_) => [0.5, 0.5, 0.5, 1.0],
                    },
                };
                [base[0], base[1], base[2], health_ratio]
            })
            .collect()
    }

    pub fn inject_knowledge(&mut self, npc_id: Uuid, facts: Vec<FactInjection>) -> bool {
        self.bridge.inject_knowledge(&npc_id.to_string(), facts)
    }

    pub fn update_emotion(&mut self, npc_id: Uuid, emotion: &str, intensity: f32) -> bool {
        self.bridge.update_emotional_state(&npc_id.to_string(), emotion, intensity)
    }

    pub fn npc_count(&self) -> usize {
        self.npcs.iter().filter(|n| n.alive).count()
    }

    pub fn total_npcs(&self) -> usize {
        self.npcs.len()
    }

    pub fn stats(&self) -> NpcSystemStats {
        let alive = self.npc_count();
        let dead = self.total_npcs() - alive;
        let factions: std::collections::HashSet<&str> =
            self.npcs.iter().map(|n| n.faction.as_str()).collect();

        NpcSystemStats {
            total: self.npcs.len(),
            alive,
            dead,
            faction_count: factions.len(),
            pending_spawn: self.spawn_queue.len(),
            pending_despawn: self.despawn_queue.len(),
            interactions: self.interactions.len(),
            bridge_npcs: self.bridge.total_npcs(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NpcSystemStats {
    pub total: usize,
    pub alive: usize,
    pub dead: usize,
    pub faction_count: usize,
    pub pending_spawn: usize,
    pub pending_despawn: usize,
    pub interactions: usize,
    pub bridge_npcs: usize,
}

pub fn create_default_npc_definition(
    name: &str,
    position: Vec3,
    species: NpcSpecies,
    faction: &str,
) -> NpcDefinition {
    let personality = match species {
        NpcSpecies::Human => PersonalityTraits {
            openness: 0.5,
            conscientiousness: 0.6,
            extraversion: 0.5,
            agreeableness: 0.6,
            neuroticism: 0.4,
            aggression: 0.3,
            curiosity: 0.6,
            loyalty: 0.7,
        },
        NpcSpecies::Mutant => PersonalityTraits {
            openness: 0.3,
            conscientiousness: 0.4,
            extraversion: 0.3,
            agreeableness: 0.4,
            neuroticism: 0.6,
            aggression: 0.7,
            curiosity: 0.3,
            loyalty: 0.5,
        },
        NpcSpecies::Ghoul => PersonalityTraits {
            openness: 0.2,
            conscientiousness: 0.3,
            extraversion: 0.2,
            agreeableness: 0.3,
            neuroticism: 0.7,
            aggression: 0.8,
            curiosity: 0.2,
            loyalty: 0.3,
        },
        NpcSpecies::Robot => PersonalityTraits {
            openness: 0.1,
            conscientiousness: 0.9,
            extraversion: 0.1,
            agreeableness: 0.5,
            neuroticism: 0.1,
            aggression: 0.5,
            curiosity: 0.1,
            loyalty: 1.0,
        },
        NpcSpecies::Animal => PersonalityTraits {
            openness: 0.2,
            conscientiousness: 0.3,
            extraversion: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
            aggression: 0.6,
            curiosity: 0.4,
            loyalty: 0.6,
        },
        NpcSpecies::Custom(_) => PersonalityTraits::default(),
    };

    let behavior = BehaviorInjection {
        patterns: vec!["patrol".into(), "idle".into(), "investigate".into()],
        idle_behaviors: vec!["patrol".into(), "rest".into()],
        combat_style: match species {
            NpcSpecies::Human => "tactical".into(),
            NpcSpecies::Mutant => "aggressive".into(),
            NpcSpecies::Ghoul => "feral".into(),
            NpcSpecies::Robot => "calculated".into(),
            NpcSpecies::Animal => "instinctual".into(),
            NpcSpecies::Custom(_) => "neutral".into(),
        },
        social_style: "neutral".into(),
        fear_responses: vec!["flee".into(), "hide".into()],
    };

    NpcDefinition {
        id: Uuid::new_v4(),
        name: name.into(),
        position,
        velocity: Vec3::ZERO,
        health: 100.0,
        max_health: 100.0,
        species,
        faction: faction.into(),
        personality,
        behavior_config: behavior,
        goals: vec![
            GoalInjection { description: "survive".into(), priority: 1.0, deadline: None },
            GoalInjection { description: "gather_resources".into(), priority: 0.5, deadline: None },
        ],
        initial_emotion: EmotionalState {
            dominant_emotion: "neutral".into(),
            intensity: 0.3,
            secondary_emotion: None,
            mood: "calm".into(),
            stress_level: 0.2,
        },
        initial_knowledge: KnowledgeInjection {
            facts: vec![],
            skills: vec![],
            relationships: vec![],
            world_lore: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_system_creation() {
        let system = NpcSystem::new(100);
        assert_eq!(system.total_npcs(), 0);
    }

    #[test]
    fn test_spawn_npc() {
        let mut system = NpcSystem::new(100);
        let def =
            create_default_npc_definition("TestNPC", Vec3::ZERO, NpcSpecies::Human, "settlers");
        let npc_id = def.id;
        system.queue_spawn(def);
        system.update(0.016, 0.0);
        assert_eq!(system.total_npcs(), 1);
        assert!(system.get_npc(npc_id).is_some());
    }

    #[test]
    fn test_despawn_npc() {
        let mut system = NpcSystem::new(100);
        let def =
            create_default_npc_definition("TestNPC", Vec3::ZERO, NpcSpecies::Human, "settlers");
        let npc_id = def.id;
        system.queue_spawn(def);
        system.update(0.016, 0.0);
        assert_eq!(system.total_npcs(), 1);

        system.queue_despawn(npc_id);
        system.update(0.016, 0.0);
        assert_eq!(system.total_npcs(), 0);
    }

    #[test]
    fn test_apply_damage() {
        let mut system = NpcSystem::new(100);
        let def =
            create_default_npc_definition("TestNPC", Vec3::ZERO, NpcSpecies::Human, "settlers");
        let npc_id = def.id;
        system.queue_spawn(def);
        system.update(0.016, 0.0);

        system.apply_damage(npc_id, 50.0, "physical");
        let npc = system.get_npc(npc_id).unwrap();
        assert_eq!(npc.health, 50.0);
        assert!(npc.alive);
    }

    #[test]
    fn test_apply_fatal_damage() {
        let mut system = NpcSystem::new(100);
        let def =
            create_default_npc_definition("TestNPC", Vec3::ZERO, NpcSpecies::Human, "settlers");
        let npc_id = def.id;
        system.queue_spawn(def);
        system.update(0.016, 0.0);

        system.apply_damage(npc_id, 200.0, "physical");
        system.update(0.016, 0.1);
        let npc = system.get_npc(npc_id);
        assert!(npc.is_none());
    }

    #[test]
    fn test_radiation_damage() {
        let mut system = NpcSystem::new(100);
        let def =
            create_default_npc_definition("TestNPC", Vec3::ZERO, NpcSpecies::Human, "settlers");
        let npc_id = def.id;
        system.queue_spawn(def);
        system.update(0.016, 0.0);

        system.apply_damage(npc_id, 100.0, "radiation");
        let npc = system.get_npc(npc_id).unwrap();
        assert!(npc.radiation_dose > 0.0);
    }

    #[test]
    fn test_get_npcs_by_faction() {
        let mut system = NpcSystem::new(100);
        let def1 =
            create_default_npc_definition("Settler1", Vec3::ZERO, NpcSpecies::Human, "settlers");
        let def2 = create_default_npc_definition(
            "Raider1",
            Vec3::new(10.0, 0.0, 0.0),
            NpcSpecies::Human,
            "raiders",
        );
        system.queue_spawn(def1);
        system.queue_spawn(def2);
        system.update(0.016, 0.0);

        let settlers = system.get_npcs_by_faction("settlers");
        assert_eq!(settlers.len(), 1);
        let raiders = system.get_npcs_by_faction("raiders");
        assert_eq!(raiders.len(), 1);
    }

    #[test]
    fn test_get_npcs_in_radius() {
        let mut system = NpcSystem::new(100);
        let def1 = create_default_npc_definition("Near", Vec3::ZERO, NpcSpecies::Human, "settlers");
        let def2 = create_default_npc_definition(
            "Far",
            Vec3::new(100.0, 0.0, 0.0),
            NpcSpecies::Human,
            "settlers",
        );
        system.queue_spawn(def1);
        system.queue_spawn(def2);
        system.update(0.016, 0.0);

        let nearby = system.get_npcs_in_radius(Vec3::ZERO, 10.0);
        assert_eq!(nearby.len(), 1);
    }

    #[test]
    fn test_max_npcs() {
        let mut system = NpcSystem::new(2);
        for i in 0..5 {
            let def = create_default_npc_definition(
                &format!("NPC_{}", i),
                Vec3::new(i as f32, 0.0, 0.0),
                NpcSpecies::Human,
                "settlers",
            );
            system.queue_spawn(def);
        }
        system.update(0.016, 0.0);
        assert_eq!(system.total_npcs(), 2);
    }

    #[test]
    fn test_npc_positions() {
        let mut system = NpcSystem::new(100);
        let def = create_default_npc_definition(
            "TestNPC",
            Vec3::new(1.0, 2.0, 3.0),
            NpcSpecies::Human,
            "settlers",
        );
        system.queue_spawn(def);
        system.update(0.016, 0.0);

        let positions = system.get_npc_positions();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_apply_force() {
        let mut system = NpcSystem::new(100);
        let def =
            create_default_npc_definition("TestNPC", Vec3::ZERO, NpcSpecies::Human, "settlers");
        let npc_id = def.id;
        system.queue_spawn(def);
        system.update(0.016, 0.0);

        system.apply_force(npc_id, Vec3::new(10.0, 0.0, 0.0));
        let npc = system.get_npc(npc_id).unwrap();
        assert!(npc.velocity.x > 0.0);
    }

    #[test]
    fn test_npc_stats() {
        let mut system = NpcSystem::new(100);
        let def1 =
            create_default_npc_definition("Settler", Vec3::ZERO, NpcSpecies::Human, "settlers");
        let def2 = create_default_npc_definition(
            "Mutant",
            Vec3::new(10.0, 0.0, 0.0),
            NpcSpecies::Mutant,
            "mutants",
        );
        system.queue_spawn(def1);
        system.queue_spawn(def2);
        system.update(0.016, 0.0);

        let stats = system.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.alive, 2);
        assert_eq!(stats.faction_count, 2);
    }
}
