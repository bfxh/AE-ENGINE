use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: EventType,
    pub source_entity: Option<Uuid>,
    pub target_entity: Option<Uuid>,
    pub position: [f32; 3],
    pub intensity: f32,
    pub data: EventData,
    pub frame: u64,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    // 物理事件
    CollisionDetected,
    ForceApplied,
    DestructionStarted,
    DestructionComplete,
    FragmentGenerated,
    MaterialDeformed,
    StressExceeded,

    // 化学事件
    ReactionStarted,
    ReactionProgressed,
    ReactionCompleted,
    ExplosionDetected,
    CorrosionApplied,
    ToxinReleased,
    StateChanged,

    // 生物事件
    DamageReceived,
    ToxinApplied,
    DrugApplied,
    RadiationApplied,
    MutationOccurred,
    HealthChanged,
    DeathEvent,
    ReproductionEvent,
    GrowthEvent,

    // 世界事件
    StructureBuilt,
    StructureDestroyed,
    ItemCrafted,
    ItemDisassembled,
    BlueprintDiscovered,
    KnowledgeGained,

    // AI/NPC事件
    NpcPerceived,
    NpcDecided,
    NpcSpoke,
    NpcTraded,
    NpcAttacked,
    NpcFled,

    // 系统事件
    ModLoaded,
    ModUnloaded,
    ConfigChanged,
    PerformanceWarning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventData {
    None,
    Collision {
        contact_normal: [f32; 3],
        penetration_depth: f32,
        relative_velocity: [f32; 3],
    },
    Force {
        force_vector: [f32; 3],
        torque: [f32; 3],
        application_point: [f32; 3],
    },
    ChemicalReaction {
        reaction_id: u64,
        reactants: Vec<String>,
        products: Vec<String>,
        energy_released: f32,
        temperature_change: f32,
        explosion_pressure: Option<f32>,
        hazard_flags: u32,
    },
    Damage {
        amount: f32,
        damage_type: String,
        source_position: [f32; 3],
    },
    Health {
        previous: f32,
        current: f32,
        max: f32,
        cause: String,
    },
    Toxin {
        toxin_id: String,
        dose: f32,
        binding_affinity: f32,
        effect_description: String,
    },
    Mutation {
        gene_layer: u8,
        previous_value: u8,
        new_value: u8,
        mutation_type: String,
    },
    Crafting {
        blueprint_id: Uuid,
        ingredients: Vec<String>,
        result: String,
        quality: f32,
    },
    Structure {
        structure_id: Uuid,
        entity_count: u32,
        critical_nodes: u32,
    },
    NpcDialogue {
        npc_id: Uuid,
        text: String,
        sentiment: f32,
        topics: Vec<String>,
    },
    ModEvent {
        mod_name: String,
        mod_version: String,
        action: String,
    },
}

impl Event {
    pub fn new(
        event_type: EventType,
        source: Option<Uuid>,
        target: Option<Uuid>,
        position: [f32; 3],
        intensity: f32,
        data: EventData,
        frame: u64,
    ) -> Self {
        Self {
            event_type,
            source_entity: source,
            target_entity: target,
            position,
            intensity,
            data,
            frame,
            timestamp: 0.0,
        }
    }

    pub fn is_physics(&self) -> bool {
        matches!(
            self.event_type,
            EventType::CollisionDetected
                | EventType::ForceApplied
                | EventType::DestructionStarted
                | EventType::DestructionComplete
                | EventType::FragmentGenerated
                | EventType::MaterialDeformed
                | EventType::StressExceeded
        )
    }

    pub fn is_chemistry(&self) -> bool {
        matches!(
            self.event_type,
            EventType::ReactionStarted
                | EventType::ReactionProgressed
                | EventType::ReactionCompleted
                | EventType::ExplosionDetected
                | EventType::CorrosionApplied
                | EventType::ToxinReleased
                | EventType::StateChanged
        )
    }

    pub fn is_biology(&self) -> bool {
        matches!(
            self.event_type,
            EventType::DamageReceived
                | EventType::ToxinApplied
                | EventType::DrugApplied
                | EventType::RadiationApplied
                | EventType::MutationOccurred
                | EventType::HealthChanged
                | EventType::DeathEvent
                | EventType::ReproductionEvent
                | EventType::GrowthEvent
        )
    }

    pub fn is_high_priority(&self) -> bool {
        matches!(
            self.event_type,
            EventType::ExplosionDetected | EventType::DeathEvent | EventType::DestructionComplete
        )
    }
}
