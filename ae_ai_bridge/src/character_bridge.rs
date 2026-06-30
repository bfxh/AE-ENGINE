use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRuntimeConfig {
    pub npc_id: String,
    pub personality: PersonalityInjection,
    pub knowledge: KnowledgeInjection,
    pub behavior: BehaviorInjection,
    pub goals: Vec<GoalInjection>,
    pub emotional_state: EmotionalState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityInjection {
    pub openness: f32,
    pub conscientiousness: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
    pub traits: Vec<String>,
    pub moral_compass: MoralCompassInjection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralCompassInjection {
    pub honesty: f32,
    pub compassion: f32,
    pub loyalty: f32,
    pub courage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeInjection {
    pub facts: Vec<FactInjection>,
    pub skills: Vec<SkillInjection>,
    pub relationships: Vec<RelationshipInjection>,
    pub world_lore: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactInjection {
    pub topic: String,
    pub content: String,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInjection {
    pub skill_name: String,
    pub level: f32,
    pub experience: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipInjection {
    pub target_name: String,
    pub relation_type: String,
    pub affinity: f32,
    pub trust: f32,
    pub history: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorInjection {
    pub patterns: Vec<String>,
    pub idle_behaviors: Vec<String>,
    pub combat_style: String,
    pub social_style: String,
    pub fear_responses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalInjection {
    pub description: String,
    pub priority: f32,
    pub deadline: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub dominant_emotion: String,
    pub intensity: f32,
    pub secondary_emotion: Option<String>,
    pub mood: String,
    pub stress_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueRequest {
    pub npc_id: String,
    pub player_message: String,
    pub context: DialogueContextParams,
    pub memory_query: Option<String>,
    pub emotion_trigger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueContextParams {
    pub location: Option<String>,
    pub time_of_day: f32,
    pub weather: String,
    pub nearby_entities: Vec<String>,
    pub world_events: Vec<String>,
    pub player_reputation: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueResponse {
    pub npc_id: String,
    pub text: String,
    pub emotion: String,
    pub action: Option<String>,
    pub memory_updated: bool,
    pub relationship_changed: bool,
    pub affinity_delta: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorDecision {
    pub npc_id: String,
    pub action: String,
    pub target: Option<String>,
    pub priority: u32,
    pub duration_ms: u64,
    pub interruptible: bool,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub struct CharacterBridge {
    pub config: CharacterBridgeConfig,
    npcs: Vec<NpcRuntimeState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBridgeConfig {
    pub max_npcs: usize,
    pub update_interval_ms: u64,
    pub memory_consolidation_interval_ms: u64,
    pub emotional_decay_rate: f32,
    pub relationship_decay_rate: f32,
}

impl Default for CharacterBridgeConfig {
    fn default() -> Self {
        CharacterBridgeConfig {
            max_npcs: 1000,
            update_interval_ms: 100,
            memory_consolidation_interval_ms: 60000,
            emotional_decay_rate: 0.01,
            relationship_decay_rate: 0.001,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NpcRuntimeState {
    pub id: String,
    pub config: NpcRuntimeConfig,
    pub last_update: u64,
    pub dialogue_count: u32,
    pub active_behaviors: Vec<String>,
}

impl CharacterBridge {
    pub fn new(config: CharacterBridgeConfig) -> Self {
        CharacterBridge { config, npcs: Vec::new() }
    }

    pub fn register_npc(&mut self, config: NpcRuntimeConfig) -> Result<(), String> {
        if self.npcs.len() >= self.config.max_npcs {
            return Err("max npcs reached".into());
        }
        if self.npcs.iter().any(|n| n.id == config.npc_id) {
            return Err(format!("npc {} already registered", config.npc_id));
        }
        self.npcs.push(NpcRuntimeState {
            id: config.npc_id.clone(),
            config,
            last_update: 0,
            dialogue_count: 0,
            active_behaviors: Vec::new(),
        });
        Ok(())
    }

    pub fn unregister_npc(&mut self, npc_id: &str) -> bool {
        if let Some(pos) = self.npcs.iter().position(|n| n.id == npc_id) {
            self.npcs.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_npc(&self, npc_id: &str) -> Option<&NpcRuntimeConfig> {
        self.npcs.iter().find(|n| n.id == npc_id).map(|n| &n.config)
    }

    pub fn process_dialogue(
        &mut self,
        request: &DialogueRequest,
        timestamp: u64,
    ) -> Option<DialogueResponse> {
        let npc = self.npcs.iter_mut().find(|n| n.id == request.npc_id)?;
        npc.dialogue_count += 1;
        npc.last_update = timestamp;
        let affinity_delta: f32 = if request.player_message.to_lowercase().contains("help") {
            1.0
        } else if request.player_message.to_lowercase().contains("threat") {
            -2.0
        } else {
            0.1
        };
        Some(DialogueResponse {
            npc_id: request.npc_id.clone(),
            text: format!(
                "[{}]#{}",
                npc.config.personality.traits.first().unwrap_or(&"neutral".into()),
                npc.dialogue_count
            ),
            emotion: npc.config.emotional_state.dominant_emotion.clone(),
            action: None,
            memory_updated: true,
            relationship_changed: affinity_delta.abs() > 0.5,
            affinity_delta,
        })
    }

    pub fn decide_behavior(
        &self,
        npc_id: &str,
        _context: &DialogueContextParams,
    ) -> Option<BehaviorDecision> {
        let npc = self.npcs.iter().find(|n| n.id == npc_id)?;
        let action = if npc.config.emotional_state.stress_level > 0.7 {
            npc.config.behavior.fear_responses.first().cloned().unwrap_or_else(|| "flee".into())
        } else {
            npc.config.behavior.idle_behaviors.first().cloned().unwrap_or_else(|| "idle".into())
        };
        Some(BehaviorDecision {
            npc_id: npc_id.into(),
            action,
            target: None,
            priority: 1,
            duration_ms: 5000,
            interruptible: true,
            reasoning: "default behavior".into(),
        })
    }

    pub fn update_emotional_state(&mut self, npc_id: &str, emotion: &str, intensity: f32) -> bool {
        if let Some(npc) = self.npcs.iter_mut().find(|n| n.id == npc_id) {
            npc.config.emotional_state.dominant_emotion = emotion.into();
            npc.config.emotional_state.intensity =
                (npc.config.emotional_state.intensity + intensity).clamp(0.0, 1.0);
            npc.config.emotional_state.stress_level += intensity * 0.1;
            true
        } else {
            false
        }
    }

    pub fn inject_knowledge(&mut self, npc_id: &str, facts: Vec<FactInjection>) -> bool {
        if let Some(npc) = self.npcs.iter_mut().find(|n| n.id == npc_id) {
            npc.config.knowledge.facts.extend(facts);
            true
        } else {
            false
        }
    }

    pub fn total_npcs(&self) -> usize {
        self.npcs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_npc_config(id: &str) -> NpcRuntimeConfig {
        NpcRuntimeConfig {
            npc_id: id.into(),
            personality: PersonalityInjection {
                openness: 0.5,
                conscientiousness: 0.6,
                extraversion: 0.4,
                agreeableness: 0.7,
                neuroticism: 0.3,
                traits: vec!["friendly".into()],
                moral_compass: MoralCompassInjection {
                    honesty: 0.8,
                    compassion: 0.7,
                    loyalty: 0.9,
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
                idle_behaviors: vec!["patrol".into()],
                combat_style: "defensive".into(),
                social_style: "cooperative".into(),
                fear_responses: vec!["flee".into()],
            },
            goals: vec![],
            emotional_state: EmotionalState {
                dominant_emotion: "neutral".into(),
                intensity: 0.3,
                secondary_emotion: None,
                mood: "calm".into(),
                stress_level: 0.2,
            },
        }
    }

    #[test]
    fn test_register_npc() {
        let mut bridge = CharacterBridge::new(CharacterBridgeConfig::default());
        assert!(bridge.register_npc(make_npc_config("npc_1")).is_ok());
        assert_eq!(bridge.total_npcs(), 1);
    }

    #[test]
    fn test_duplicate_npc() {
        let mut bridge = CharacterBridge::new(CharacterBridgeConfig::default());
        bridge.register_npc(make_npc_config("npc_1")).unwrap();
        assert!(bridge.register_npc(make_npc_config("npc_1")).is_err());
    }

    #[test]
    fn test_unregister_npc() {
        let mut bridge = CharacterBridge::new(CharacterBridgeConfig::default());
        bridge.register_npc(make_npc_config("npc_1")).unwrap();
        assert!(bridge.unregister_npc("npc_1"));
        assert_eq!(bridge.total_npcs(), 0);
    }

    #[test]
    fn test_process_dialogue() {
        let mut bridge = CharacterBridge::new(CharacterBridgeConfig::default());
        bridge.register_npc(make_npc_config("npc_1")).unwrap();
        let request = DialogueRequest {
            npc_id: "npc_1".into(),
            player_message: "Hello, can you help me?".into(),
            context: DialogueContextParams {
                location: None,
                time_of_day: 12.0,
                weather: "clear".into(),
                nearby_entities: vec![],
                world_events: vec![],
                player_reputation: 0.5,
            },
            memory_query: None,
            emotion_trigger: None,
        };
        let response = bridge.process_dialogue(&request, 1000);
        assert!(response.is_some());
        let resp = response.unwrap();
        assert_eq!(resp.npc_id, "npc_1");
        assert!(resp.affinity_delta > 0.0);
    }

    #[test]
    fn test_decide_behavior() {
        let bridge = CharacterBridge {
            config: CharacterBridgeConfig::default(),
            npcs: vec![NpcRuntimeState {
                id: "npc_1".into(),
                config: make_npc_config("npc_1"),
                last_update: 0,
                dialogue_count: 0,
                active_behaviors: vec![],
            }],
        };
        let context = DialogueContextParams {
            location: None,
            time_of_day: 12.0,
            weather: "clear".into(),
            nearby_entities: vec![],
            world_events: vec![],
            player_reputation: 0.5,
        };
        let decision = bridge.decide_behavior("npc_1", &context);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().action, "patrol");
    }

    #[test]
    fn test_update_emotional_state() {
        let mut bridge = CharacterBridge::new(CharacterBridgeConfig::default());
        bridge.register_npc(make_npc_config("npc_1")).unwrap();
        assert!(bridge.update_emotional_state("npc_1", "angry", 0.8));
        let npc = bridge.get_npc("npc_1").unwrap();
        assert_eq!(npc.emotional_state.dominant_emotion, "angry");
    }

    #[test]
    fn test_inject_knowledge() {
        let mut bridge = CharacterBridge::new(CharacterBridgeConfig::default());
        bridge.register_npc(make_npc_config("npc_1")).unwrap();
        let facts = vec![FactInjection {
            topic: "water".into(),
            content: "The well is dry".into(),
            confidence: 0.9,
            source: "personal_observation".into(),
        }];
        assert!(bridge.inject_knowledge("npc_1", facts));
        let npc = bridge.get_npc("npc_1").unwrap();
        assert_eq!(npc.knowledge.facts.len(), 1);
    }

    #[test]
    fn test_max_npcs() {
        let mut bridge =
            CharacterBridge::new(CharacterBridgeConfig { max_npcs: 2, ..Default::default() });
        assert!(bridge.register_npc(make_npc_config("a")).is_ok());
        assert!(bridge.register_npc(make_npc_config("b")).is_ok());
        assert!(bridge.register_npc(make_npc_config("c")).is_err());
    }
}
