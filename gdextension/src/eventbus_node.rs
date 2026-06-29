use godot::prelude::*;

use wasteland_eventbus::bus::EventBus;
use wasteland_eventbus::event::{Event, EventData, EventType};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandEventBus {
    #[var]
    history_size: i64,
    #[var]
    batch_enabled: bool,

    bus: EventBus,
    event_count: i64,
    subscriber_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandEventBus {
    fn init(base: Base<Node>) -> Self {
        let mut bus = EventBus::new();
        bus.set_batch_mode(true);
        Self {
            history_size: 1000,
            batch_enabled: true,
            bus,
            event_count: 0,
            subscriber_count: 0,
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        self.bus.flush_batch();
    }
}

#[godot_api]
impl WastelandEventBus {
    #[func]
    fn emit(&mut self, event_type: GString, _source: GString, intensity: f32) -> i64 {
        let et = match event_type.to_string().as_str() {
            "collision_detected" => EventType::CollisionDetected,
            "force_applied" => EventType::ForceApplied,
            "destruction_started" => EventType::DestructionStarted,
            "destruction_complete" => EventType::DestructionComplete,
            "reaction_started" => EventType::ReactionStarted,
            "reaction_completed" => EventType::ReactionCompleted,
            "explosion_detected" => EventType::ExplosionDetected,
            "damage_received" => EventType::DamageReceived,
            "health_changed" => EventType::HealthChanged,
            "death_event" => EventType::DeathEvent,
            "structure_built" => EventType::StructureBuilt,
            "structure_destroyed" => EventType::StructureDestroyed,
            "npc_spoke" => EventType::NpcSpoke,
            "npc_attacked" => EventType::NpcAttacked,
            "mod_loaded" => EventType::ModLoaded,
            "config_changed" => EventType::ConfigChanged,
            "performance_warning" => EventType::PerformanceWarning,
            _ => EventType::CollisionDetected,
        };
        let event = Event::new(
            et,
            None,
            None,
            [0.0, 0.0, 0.0],
            intensity,
            EventData::None,
            self.event_count as u64,
        );
        self.bus.emit(event);
        self.event_count += 1;
        self.event_count
    }

    #[func]
    fn subscribe(&mut self, event_type: GString, priority: i64) -> i64 {
        let et = match event_type.to_string().as_str() {
            "collision_detected" => EventType::CollisionDetected,
            "force_applied" => EventType::ForceApplied,
            "destruction_started" => EventType::DestructionStarted,
            "destruction_complete" => EventType::DestructionComplete,
            "reaction_started" => EventType::ReactionStarted,
            "reaction_completed" => EventType::ReactionCompleted,
            "explosion_detected" => EventType::ExplosionDetected,
            "damage_received" => EventType::DamageReceived,
            "health_changed" => EventType::HealthChanged,
            "death_event" => EventType::DeathEvent,
            "structure_built" => EventType::StructureBuilt,
            "structure_destroyed" => EventType::StructureDestroyed,
            "npc_spoke" => EventType::NpcSpoke,
            "npc_attacked" => EventType::NpcAttacked,
            "mod_loaded" => EventType::ModLoaded,
            "config_changed" => EventType::ConfigChanged,
            "performance_warning" => EventType::PerformanceWarning,
            _ => EventType::CollisionDetected,
        };
        let id =
            self.bus.subscribe(vec![et], priority as i32, "gdextension", Box::new(|_event| vec![]));
        self.subscriber_count += 1;
        id as i64
    }

    #[func]
    fn unsubscribe(&mut self, subscription_id: i64) {
        self.bus.unsubscribe(subscription_id as u64);
        self.subscriber_count = (self.subscriber_count - 1).max(0);
    }

    #[func]
    fn get_event_type_name(&self, event_type: GString) -> GString {
        let et = match event_type.to_string().as_str() {
            "collision_detected" => EventType::CollisionDetected,
            "force_applied" => EventType::ForceApplied,
            "destruction_started" => EventType::DestructionStarted,
            "destruction_complete" => EventType::DestructionComplete,
            "reaction_started" => EventType::ReactionStarted,
            "reaction_completed" => EventType::ReactionCompleted,
            "explosion_detected" => EventType::ExplosionDetected,
            "damage_received" => EventType::DamageReceived,
            "health_changed" => EventType::HealthChanged,
            "death_event" => EventType::DeathEvent,
            "structure_built" => EventType::StructureBuilt,
            "structure_destroyed" => EventType::StructureDestroyed,
            "npc_spoke" => EventType::NpcSpoke,
            "npc_attacked" => EventType::NpcAttacked,
            "mod_loaded" => EventType::ModLoaded,
            "config_changed" => EventType::ConfigChanged,
            "performance_warning" => EventType::PerformanceWarning,
            _ => EventType::CollisionDetected,
        };
        GString::from(format!("{:?}", et).as_str())
    }

    #[func]
    fn get_history_count(&self) -> i64 {
        self.bus.history_count() as i64
    }

    #[func]
    fn get_pending_count(&self) -> i64 {
        self.bus.pending_count() as i64
    }

    #[func]
    fn clear(&mut self) {
        self.bus.clear();
        self.event_count = 0;
        self.subscriber_count = 0;
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "event_count" => self.event_count,
            "subscriber_count" => self.subscriber_count,
            "history_size" => self.history_size,
            "batch_enabled" => self.batch_enabled,
            "history_count" => self.bus.history_count() as i64,
            "pending_count" => self.bus.pending_count() as i64,
        }
    }
}
