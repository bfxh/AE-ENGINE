use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcKnowledgeBase {
    pub npc_id: String,
    pub name: String,
    pub personality: PersonalityProfile,
    pub knowledge_graph: KnowledgeGraph,
    pub memories: Vec<Memory>,
    pub skills: HashMap<String, SkillLevel>,
    pub relationships: Vec<Relationship>,
    pub behavior_patterns: Vec<BehaviorPattern>,
    pub world_view: WorldView,
    pub metadata: NpcMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub openness: f32,
    pub conscientiousness: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
    pub traits: Vec<String>,
    pub quirks: Vec<String>,
    pub moral_compass: MoralCompass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralCompass {
    pub honesty: f32,
    pub compassion: f32,
    pub loyalty: f32,
    pub courage: f32,
    pub selfishness: f32,
    pub cruelty: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<KnowledgeNode>,
    pub edges: Vec<KnowledgeEdge>,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub concept: String,
    pub node_type: NodeType,
    pub confidence: f32,
    pub source: KnowledgeSource,
    pub acquired_at: u64,
    pub last_recalled: u64,
    pub decay_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Fact,
    Location,
    Person,
    Event,
    Skill,
    Belief,
    Rumor,
    Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeSource {
    DirectExperience,
    Witnessed,
    Taught,
    Overheard,
    Inferred,
    Innate,
    PlayerShared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub strength: f32,
    pub bidirectional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub memory_type: MemoryType,
    pub content: String,
    pub emotional_valence: f32,
    pub importance: f32,
    pub timestamp: u64,
    pub location: Option<[f32; 3]>,
    pub participants: Vec<String>,
    pub associated_emotions: Vec<EmotionTag>,
    pub recall_count: u32,
    pub last_recalled: u64,
    pub decay_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
    Emotional,
    Spatial,
    Social,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionTag {
    pub emotion: Emotion,
    pub intensity: f32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Emotion {
    Joy,
    Sadness,
    Anger,
    Fear,
    Surprise,
    Disgust,
    Trust,
    Anticipation,
    Shame,
    Pride,
    Guilt,
    Envy,
    Gratitude,
    Resentment,
    Hope,
    Despair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLevel {
    pub skill_name: String,
    pub level: f32,
    pub experience: f32,
    pub max_level: f32,
    pub specialization: Option<String>,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub target_id: String,
    pub target_name: String,
    pub relation_type: RelationType,
    pub affinity: f32,
    pub trust: f32,
    pub respect: f32,
    pub fear: f32,
    pub shared_secrets: u32,
    pub last_interaction: u64,
    pub interaction_count: u32,
    pub history: Vec<RelationEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    Stranger,
    Acquaintance,
    Friend,
    CloseFriend,
    Rival,
    Enemy,
    Ally,
    Family,
    Mentor,
    Student,
    Boss,
    Subordinate,
    Lover,
    ExLover,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationEvent {
    pub event_type: String,
    pub affinity_change: f32,
    pub timestamp: u64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    pub pattern_name: String,
    pub trigger: BehaviorTrigger,
    pub response: BehaviorResponse,
    pub frequency: f32,
    pub adaptability: f32,
    pub last_activated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorTrigger {
    pub condition_type: ConditionType,
    pub parameters: HashMap<String, f32>,
    pub threshold: f32,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    EmotionThreshold,
    NeedThreshold,
    TimeOfDay,
    Weather,
    ProximityToEntity,
    HealthThreshold,
    SocialContext,
    RecentEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorResponse {
    pub action: String,
    pub priority: u32,
    pub duration_ms: u64,
    pub interruptible: bool,
    pub emotional_effect: Vec<EmotionTag>,
    pub dialogue_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldView {
    pub beliefs: Vec<Belief>,
    pub goals: Vec<Goal>,
    pub fears: Vec<String>,
    pub desires: Vec<String>,
    pub faction_loyalty: HashMap<String, f32>,
    pub moral_stance: Vec<MoralStance>,
    pub knowledge_biases: Vec<KnowledgeBias>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub statement: String,
    pub confidence: f32,
    pub is_core: bool,
    pub mutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub description: String,
    pub priority: f32,
    pub progress: f32,
    pub deadline: Option<u64>,
    pub sub_goals: Vec<String>,
    pub status: GoalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalStatus {
    Active,
    Completed,
    Failed,
    Abandoned,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralStance {
    pub topic: String,
    pub stance: f32,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBias {
    pub bias_type: BiasType,
    pub strength: f32,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiasType {
    Confirmation,
    Recency,
    Familiarity,
    Authority,
    InGroup,
    OutGroup,
    Negativity,
    Optimism,
    Survivorship,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMetadata {
    pub age: u32,
    pub gender: String,
    pub occupation: String,
    pub faction: Option<String>,
    pub health: f32,
    pub max_health: f32,
    pub stamina: f32,
    pub intelligence: f32,
    pub charisma: f32,
    pub perception: f32,
    pub total_memories: u32,
    pub total_knowledge_nodes: u32,
    pub total_relationships: u32,
}

pub struct NpcKnowledgeInjector {
    pub config: KnowledgeInjectConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeInjectConfig {
    pub max_memories: usize,
    pub max_knowledge_nodes: usize,
    pub memory_decay_rate: f32,
    pub knowledge_decay_rate: f32,
    pub emotion_decay_rate: f32,
    pub learning_rate: f32,
    pub forgetting_threshold: f32,
}

impl Default for KnowledgeInjectConfig {
    fn default() -> Self {
        KnowledgeInjectConfig {
            max_memories: 1000,
            max_knowledge_nodes: 500,
            memory_decay_rate: 0.001,
            knowledge_decay_rate: 0.0005,
            emotion_decay_rate: 0.01,
            learning_rate: 0.1,
            forgetting_threshold: 0.05,
        }
    }
}

impl NpcKnowledgeInjector {
    pub fn new(config: KnowledgeInjectConfig) -> Self {
        NpcKnowledgeInjector { config }
    }

    pub fn create_npc(
        &self,
        npc_id: String,
        name: String,
        personality: PersonalityProfile,
        occupation: String,
        faction: Option<String>,
    ) -> NpcKnowledgeBase {
        NpcKnowledgeBase {
            npc_id,
            name,
            personality,
            knowledge_graph: KnowledgeGraph { nodes: Vec::new(), edges: Vec::new(), version: 1 },
            memories: Vec::new(),
            skills: HashMap::new(),
            relationships: Vec::new(),
            behavior_patterns: Vec::new(),
            world_view: WorldView {
                beliefs: Vec::new(),
                goals: Vec::new(),
                fears: Vec::new(),
                desires: Vec::new(),
                faction_loyalty: HashMap::new(),
                moral_stance: Vec::new(),
                knowledge_biases: Vec::new(),
            },
            metadata: NpcMetadata {
                age: 25,
                gender: "unknown".into(),
                occupation,
                faction,
                health: 100.0,
                max_health: 100.0,
                stamina: 100.0,
                intelligence: 50.0,
                charisma: 50.0,
                perception: 50.0,
                total_memories: 0,
                total_knowledge_nodes: 0,
                total_relationships: 0,
            },
        }
    }

    pub fn inject_knowledge(
        &self,
        npc: &mut NpcKnowledgeBase,
        concept: String,
        node_type: NodeType,
        confidence: f32,
        source: KnowledgeSource,
        timestamp: u64,
    ) {
        if npc.knowledge_graph.nodes.len() >= self.config.max_knowledge_nodes {
            self.prune_knowledge(npc);
        }
        let id = format!("kn_{}", npc.knowledge_graph.nodes.len());
        npc.knowledge_graph.nodes.push(KnowledgeNode {
            id,
            concept,
            node_type,
            confidence,
            source,
            acquired_at: timestamp,
            last_recalled: timestamp,
            decay_rate: self.config.knowledge_decay_rate,
        });
        npc.knowledge_graph.version += 1;
        npc.metadata.total_knowledge_nodes = npc.knowledge_graph.nodes.len() as u32;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn inject_memory(
        &self,
        npc: &mut NpcKnowledgeBase,
        content: String,
        memory_type: MemoryType,
        emotional_valence: f32,
        importance: f32,
        timestamp: u64,
        location: Option<[f32; 3]>,
        participants: Vec<String>,
        emotions: Vec<EmotionTag>,
    ) {
        if npc.memories.len() >= self.config.max_memories {
            self.consolidate_memories(npc);
        }
        let id = format!("mem_{}", npc.memories.len());
        npc.memories.push(Memory {
            id,
            memory_type,
            content,
            emotional_valence,
            importance,
            timestamp,
            location,
            participants,
            associated_emotions: emotions,
            recall_count: 0,
            last_recalled: timestamp,
            decay_rate: self.config.memory_decay_rate,
        });
        npc.metadata.total_memories = npc.memories.len() as u32;
    }

    pub fn update_relationship(
        &self,
        npc: &mut NpcKnowledgeBase,
        target_id: String,
        target_name: String,
        event: RelationEvent,
        timestamp: u64,
    ) {
        if let Some(rel) = npc.relationships.iter_mut().find(|r| r.target_id == target_id) {
            rel.affinity = (rel.affinity + event.affinity_change).clamp(-100.0, 100.0);
            rel.trust = (rel.trust + event.affinity_change * 0.5).clamp(-100.0, 100.0);
            rel.last_interaction = timestamp;
            rel.interaction_count += 1;
            rel.history.push(event);
        } else {
            npc.relationships.push(Relationship {
                target_id,
                target_name,
                relation_type: RelationType::Stranger,
                affinity: event.affinity_change.clamp(-100.0, 100.0),
                trust: (event.affinity_change * 0.5).clamp(-100.0, 100.0),
                respect: 0.0,
                fear: 0.0,
                shared_secrets: 0,
                last_interaction: timestamp,
                interaction_count: 1,
                history: vec![event],
            });
        }
        npc.metadata.total_relationships = npc.relationships.len() as u32;
    }

    pub fn add_behavior_pattern(&self, npc: &mut NpcKnowledgeBase, pattern: BehaviorPattern) {
        npc.behavior_patterns.push(pattern);
    }

    pub fn add_goal(&self, npc: &mut NpcKnowledgeBase, goal: Goal) {
        npc.world_view.goals.push(goal);
    }

    pub fn add_belief(&self, npc: &mut NpcKnowledgeBase, belief: Belief) {
        npc.world_view.beliefs.push(belief);
    }

    pub fn query_knowledge<'a>(
        &self,
        npc: &'a NpcKnowledgeBase,
        query: &str,
        min_confidence: f32,
    ) -> Vec<&'a KnowledgeNode> {
        let now = npc.memories.iter().map(|m| m.timestamp).max().unwrap_or(0);
        npc.knowledge_graph
            .nodes
            .iter()
            .filter(|n| {
                n.confidence >= min_confidence
                    && self.calculate_retention(n.acquired_at, n.decay_rate, now)
                        > self.config.forgetting_threshold
                    && n.concept.to_lowercase().contains(&query.to_lowercase())
            })
            .collect()
    }

    pub fn recall_memories<'a>(
        &self,
        npc: &'a NpcKnowledgeBase,
        query: &str,
        limit: usize,
    ) -> Vec<&'a Memory> {
        let now = npc.memories.iter().map(|m| m.timestamp).max().unwrap_or(0);
        let mut scored: Vec<(f32, &Memory)> = npc
            .memories
            .iter()
            .map(|m| {
                let relevance = if m.content.to_lowercase().contains(&query.to_lowercase()) {
                    1.0
                } else {
                    m.participants.iter().any(|p| p.to_lowercase().contains(&query.to_lowercase()))
                        as u8 as f32
                        * 0.5
                };
                let retention = self.calculate_retention(m.timestamp, m.decay_rate, now);
                let recency = 1.0 / (1.0 + (now.saturating_sub(m.timestamp)) as f32 / 1000.0);
                let score = relevance
                    * retention
                    * recency
                    * m.importance
                    * (1.0 + m.emotional_valence.abs());
                (score, m)
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();
        scored.sort_by(|(a, _), (b, _)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored.into_iter().map(|(_, m)| m).collect()
    }

    pub fn decay_all(&self, npc: &mut NpcKnowledgeBase, now: u64) {
        for memory in &mut npc.memories {
            let elapsed = now.saturating_sub(memory.last_recalled) as f32 / 1000.0;
            memory.importance *= (-self.config.memory_decay_rate * elapsed).exp();
            memory.emotional_valence *= (-self.config.emotion_decay_rate * elapsed).exp();
        }
        for node in &mut npc.knowledge_graph.nodes {
            let elapsed = now.saturating_sub(node.last_recalled) as f32 / 1000.0;
            node.confidence *= (-self.config.knowledge_decay_rate * elapsed).exp();
        }
    }

    fn calculate_retention(&self, timestamp: u64, decay_rate: f32, now: u64) -> f32 {
        let elapsed = now.saturating_sub(timestamp) as f32 / 1000.0;
        (-decay_rate * elapsed).exp()
    }

    fn prune_knowledge(&self, npc: &mut NpcKnowledgeBase) {
        let now = npc.memories.iter().map(|m| m.timestamp).max().unwrap_or(0);
        npc.knowledge_graph.nodes.sort_by(|a, b| {
            let ra = self.calculate_retention(a.acquired_at, a.decay_rate, now) * a.confidence;
            let rb = self.calculate_retention(b.acquired_at, b.decay_rate, now) * b.confidence;
            rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
        });
        let remove_count = npc.knowledge_graph.nodes.len() - self.config.max_knowledge_nodes + 50;
        npc.knowledge_graph
            .nodes
            .truncate(npc.knowledge_graph.nodes.len().saturating_sub(remove_count));
        npc.metadata.total_knowledge_nodes = npc.knowledge_graph.nodes.len() as u32;
    }

    fn consolidate_memories(&self, npc: &mut NpcKnowledgeBase) {
        let now = npc.memories.iter().map(|m| m.timestamp).max().unwrap_or(0);
        npc.memories.sort_by(|a, b| {
            let sa = self.calculate_retention(a.timestamp, a.decay_rate, now)
                * a.importance
                * (1.0 + a.emotional_valence.abs());
            let sb = self.calculate_retention(b.timestamp, b.decay_rate, now)
                * b.importance
                * (1.0 + b.emotional_valence.abs());
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        let remove_count = npc.memories.len() - self.config.max_memories + 100;
        npc.memories.truncate(npc.memories.len().saturating_sub(remove_count));
        npc.metadata.total_memories = npc.memories.len() as u32;
    }

    pub fn infer_knowledge(
        &self,
        npc: &mut NpcKnowledgeBase,
        from_concept: &str,
        to_concept: &str,
        relation: &str,
        strength: f32,
        timestamp: u64,
    ) {
        let from_node = npc.knowledge_graph.nodes.iter().find(|n| n.concept == from_concept);
        let to_node = npc.knowledge_graph.nodes.iter().find(|n| n.concept == to_concept);
        if let (Some(fn_), Some(tn_)) = (from_node, to_node) {
            let from_id = fn_.id.clone();
            let to_id = tn_.id.clone();
            let already_exists = npc
                .knowledge_graph
                .edges
                .iter()
                .any(|e| e.from == from_id && e.to == to_id && e.relation == relation);
            if !already_exists {
                npc.knowledge_graph.edges.push(KnowledgeEdge {
                    from: from_id,
                    to: to_id,
                    relation: relation.into(),
                    strength,
                    bidirectional: false,
                });
            }
        }
        let _ = timestamp;
    }

    pub fn emotional_response(
        &self,
        npc: &NpcKnowledgeBase,
        event_emotion: Emotion,
        event_intensity: f32,
    ) -> Vec<EmotionTag> {
        let mut responses = Vec::new();
        let personality = &npc.personality;
        let amplification = match event_emotion {
            Emotion::Joy => 1.0 + personality.extraversion * 0.5,
            Emotion::Sadness => 1.0 + personality.neuroticism * 0.5,
            Emotion::Anger => {
                1.0 + (1.0 - personality.agreeableness) * 0.5 + personality.neuroticism * 0.3
            },
            Emotion::Fear => 1.0 + personality.neuroticism * 0.7,
            Emotion::Surprise => 1.0 + personality.openness * 0.3,
            Emotion::Disgust => 1.0 + personality.conscientiousness * 0.3,
            Emotion::Trust => 1.0 + personality.agreeableness * 0.5,
            _ => 1.0,
        };
        responses.push(EmotionTag {
            emotion: event_emotion,
            intensity: event_intensity * amplification,
            duration_ms: (event_intensity * 5000.0) as u64,
        });
        if event_intensity > 0.7 {
            let secondary = match event_emotion {
                Emotion::Anger => Emotion::Resentment,
                Emotion::Joy => Emotion::Gratitude,
                Emotion::Sadness => Emotion::Despair,
                Emotion::Fear => Emotion::Shame,
                _ => Emotion::Anticipation,
            };
            responses.push(EmotionTag {
                emotion: secondary,
                intensity: event_intensity * 0.5,
                duration_ms: (event_intensity * 3000.0) as u64,
            });
        }
        responses
    }

    pub fn generate_dialogue_context(
        &self,
        npc: &NpcKnowledgeBase,
        player_name: &str,
        topic: &str,
    ) -> DialogueContext {
        let relevant_memories = self.recall_memories(npc, topic, 5);
        let relevant_knowledge = self.query_knowledge(npc, topic, 0.3);
        let memory_texts: Vec<String> =
            relevant_memories.iter().map(|m| m.content.clone()).collect();
        let knowledge_texts: Vec<String> =
            relevant_knowledge.iter().map(|k| k.concept.clone()).collect();
        let relationship = npc.relationships.iter().find(|r| r.target_name == player_name);
        let affinity = relationship.map(|r| r.affinity).unwrap_or(0.0);
        let trust = relationship.map(|r| r.trust).unwrap_or(0.0);
        let mood = self.calculate_mood(npc);
        DialogueContext {
            npc_name: npc.name.clone(),
            npc_personality: npc.personality.clone(),
            player_name: player_name.into(),
            topic: topic.into(),
            mood,
            affinity,
            trust,
            relevant_memories: memory_texts,
            relevant_knowledge: knowledge_texts,
            active_goals: npc
                .world_view
                .goals
                .iter()
                .filter(|g| g.status == GoalStatus::Active)
                .map(|g| g.description.clone())
                .collect(),
            recent_emotions: npc
                .memories
                .iter()
                .rev()
                .take(3)
                .flat_map(|m| m.associated_emotions.clone())
                .collect(),
        }
    }

    fn calculate_mood(&self, npc: &NpcKnowledgeBase) -> String {
        let recent_emotions: Vec<&EmotionTag> =
            npc.memories.iter().rev().take(5).flat_map(|m| &m.associated_emotions).collect();
        if recent_emotions.is_empty() {
            return "neutral".into();
        }
        let mut joy = 0.0f32;
        let mut sadness = 0.0f32;
        let mut anger = 0.0f32;
        let mut fear = 0.0f32;
        for tag in &recent_emotions {
            match tag.emotion {
                Emotion::Joy | Emotion::Gratitude | Emotion::Pride | Emotion::Hope => {
                    joy += tag.intensity
                },
                Emotion::Sadness | Emotion::Despair | Emotion::Guilt | Emotion::Shame => {
                    sadness += tag.intensity
                },
                Emotion::Anger | Emotion::Resentment | Emotion::Envy => anger += tag.intensity,
                Emotion::Fear => fear += tag.intensity,
                _ => {},
            }
        }
        let max_val = joy.max(sadness).max(anger).max(fear);
        if max_val < 0.2 {
            "neutral".into()
        } else if joy >= max_val {
            "happy".into()
        } else if sadness >= max_val {
            "sad".into()
        } else if anger >= max_val {
            "angry".into()
        } else {
            "fearful".into()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueContext {
    pub npc_name: String,
    pub npc_personality: PersonalityProfile,
    pub player_name: String,
    pub topic: String,
    pub mood: String,
    pub affinity: f32,
    pub trust: f32,
    pub relevant_memories: Vec<String>,
    pub relevant_knowledge: Vec<String>,
    pub active_goals: Vec<String>,
    pub recent_emotions: Vec<EmotionTag>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_personality() -> PersonalityProfile {
        PersonalityProfile {
            openness: 0.6,
            conscientiousness: 0.7,
            extraversion: 0.5,
            agreeableness: 0.6,
            neuroticism: 0.4,
            traits: vec!["curious".into(), "cautious".into()],
            quirks: vec!["whistles when nervous".into()],
            moral_compass: MoralCompass {
                honesty: 0.8,
                compassion: 0.7,
                loyalty: 0.9,
                courage: 0.5,
                selfishness: 0.3,
                cruelty: 0.1,
            },
        }
    }

    #[test]
    fn test_create_npc() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let npc = injector.create_npc(
            "npc_001".into(),
            "Wanderer".into(),
            make_personality(),
            "scavenger".into(),
            Some("wastelanders".into()),
        );
        assert_eq!(npc.name, "Wanderer");
        assert_eq!(npc.metadata.occupation, "scavenger");
        assert_eq!(npc.metadata.faction.as_deref(), Some("wastelanders"));
    }

    #[test]
    fn test_inject_knowledge() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        injector.inject_knowledge(
            &mut npc,
            "Old World tech is dangerous".into(),
            NodeType::Fact,
            0.9,
            KnowledgeSource::DirectExperience,
            1000,
        );
        assert_eq!(npc.knowledge_graph.nodes.len(), 1);
        assert_eq!(npc.knowledge_graph.version, 2);
    }

    #[test]
    fn test_inject_memory() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        injector.inject_memory(
            &mut npc,
            "Found a working radio in the ruins".into(),
            MemoryType::Episodic,
            0.7,
            0.8,
            1000,
            Some([10.0, 0.0, 20.0]),
            vec!["player".into()],
            vec![EmotionTag { emotion: Emotion::Joy, intensity: 0.8, duration_ms: 5000 }],
        );
        assert_eq!(npc.memories.len(), 1);
        assert_eq!(npc.memories[0].emotional_valence, 0.7);
    }

    #[test]
    fn test_update_relationship() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        injector.update_relationship(
            &mut npc,
            "player".into(),
            "Player".into(),
            RelationEvent {
                event_type: "helped".into(),
                affinity_change: 10.0,
                timestamp: 1000,
                description: "Player helped fix the water pump".into(),
            },
            1000,
        );
        assert_eq!(npc.relationships.len(), 1);
        assert_eq!(npc.relationships[0].affinity, 10.0);
    }

    #[test]
    fn test_recall_memories() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        injector.inject_memory(
            &mut npc,
            "Found a radio in the ruins".into(),
            MemoryType::Episodic,
            0.7,
            0.9,
            1000,
            None,
            vec!["player".into()],
            vec![],
        );
        injector.inject_memory(
            &mut npc,
            "The water is contaminated".into(),
            MemoryType::Semantic,
            -0.5,
            0.6,
            2000,
            None,
            vec![],
            vec![],
        );
        let results = injector.recall_memories(&npc, "radio", 5);
        assert!(!results.is_empty());
        assert!(results[0].content.contains("radio"));
    }

    #[test]
    fn test_query_knowledge() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        injector.inject_knowledge(
            &mut npc,
            "Radiation is dangerous".into(),
            NodeType::Fact,
            0.95,
            KnowledgeSource::DirectExperience,
            1000,
        );
        injector.inject_knowledge(
            &mut npc,
            "Radiation can be cured".into(),
            NodeType::Rumor,
            0.3,
            KnowledgeSource::Overheard,
            2000,
        );
        let results = injector.query_knowledge(&npc, "radiation", 0.5);
        assert_eq!(results.len(), 1);
        assert!(results[0].concept.contains("dangerous"));
    }

    #[test]
    fn test_infer_knowledge() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        injector.inject_knowledge(
            &mut npc,
            "Fire".into(),
            NodeType::Fact,
            1.0,
            KnowledgeSource::Innate,
            0,
        );
        injector.inject_knowledge(
            &mut npc,
            "Heat".into(),
            NodeType::Fact,
            1.0,
            KnowledgeSource::Innate,
            0,
        );
        injector.infer_knowledge(&mut npc, "Fire", "Heat", "produces", 0.9, 1000);
        assert_eq!(npc.knowledge_graph.edges.len(), 1);
        assert_eq!(npc.knowledge_graph.edges[0].relation, "produces");
    }

    #[test]
    fn test_emotional_response() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        let responses = injector.emotional_response(&npc, Emotion::Joy, 0.9);
        assert!(!responses.is_empty());
        assert_eq!(responses[0].emotion, Emotion::Joy);
        assert!(responses[0].intensity > 0.9);
    }

    #[test]
    fn test_generate_dialogue_context() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Grey".into(),
            make_personality(),
            "scavenger".into(),
            None,
        );
        injector.inject_memory(
            &mut npc,
            "Player helped find water".into(),
            MemoryType::Episodic,
            0.8,
            0.9,
            1000,
            None,
            vec!["Player".into()],
            vec![EmotionTag { emotion: Emotion::Gratitude, intensity: 0.7, duration_ms: 5000 }],
        );
        injector.update_relationship(
            &mut npc,
            "player".into(),
            "Player".into(),
            RelationEvent {
                event_type: "helped".into(),
                affinity_change: 15.0,
                timestamp: 1000,
                description: "Helped find water".into(),
            },
            1000,
        );
        let context = injector.generate_dialogue_context(&npc, "Player", "water");
        assert_eq!(context.npc_name, "Grey");
        assert_eq!(context.player_name, "Player");
        assert!(context.affinity > 0.0);
        assert!(!context.relevant_memories.is_empty() || !context.relevant_knowledge.is_empty());
    }

    #[test]
    fn test_decay() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig {
            memory_decay_rate: 0.1,
            knowledge_decay_rate: 0.1,
            ..Default::default()
        });
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        injector.inject_memory(
            &mut npc,
            "Old memory".into(),
            MemoryType::Episodic,
            0.5,
            0.5,
            0,
            None,
            vec![],
            vec![],
        );
        let original_importance = npc.memories[0].importance;
        injector.decay_all(&mut npc, 10000);
        assert!(npc.memories[0].importance < original_importance);
    }

    #[test]
    fn test_behavior_pattern() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        let pattern = BehaviorPattern {
            pattern_name: "flee_when_low_health".into(),
            trigger: BehaviorTrigger {
                condition_type: ConditionType::HealthThreshold,
                parameters: vec![("threshold".into(), 0.3)].into_iter().collect(),
                threshold: 0.3,
                cooldown_ms: 10000,
            },
            response: BehaviorResponse {
                action: "flee".into(),
                priority: 10,
                duration_ms: 5000,
                interruptible: false,
                emotional_effect: vec![EmotionTag {
                    emotion: Emotion::Fear,
                    intensity: 0.8,
                    duration_ms: 5000,
                }],
                dialogue_template: Some("I need to get out of here!".into()),
            },
            frequency: 0.5,
            adaptability: 0.3,
            last_activated: 0,
        };
        injector.add_behavior_pattern(&mut npc, pattern);
        assert_eq!(npc.behavior_patterns.len(), 1);
        assert_eq!(npc.behavior_patterns[0].pattern_name, "flee_when_low_health");
    }

    #[test]
    fn test_goals_and_beliefs() {
        let injector = NpcKnowledgeInjector::new(KnowledgeInjectConfig::default());
        let mut npc = injector.create_npc(
            "npc_001".into(),
            "Test".into(),
            make_personality(),
            "tester".into(),
            None,
        );
        injector.add_goal(
            &mut npc,
            Goal {
                description: "Find clean water".into(),
                priority: 0.9,
                progress: 0.3,
                deadline: None,
                sub_goals: vec!["Locate water source".into(), "Test water quality".into()],
                status: GoalStatus::Active,
            },
        );
        injector.add_belief(
            &mut npc,
            Belief {
                statement: "The wasteland is unforgiving".into(),
                confidence: 0.95,
                is_core: true,
                mutable: false,
            },
        );
        assert_eq!(npc.world_view.goals.len(), 1);
        assert_eq!(npc.world_view.beliefs.len(), 1);
    }
}
