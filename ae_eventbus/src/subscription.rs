use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: u64,
    pub event_types: Vec<super::event::EventType>,
    pub priority: i32,
    pub source_mod: String,
}

impl Subscription {
    pub fn new(
        id: u64,
        event_types: Vec<super::event::EventType>,
        priority: i32,
        source_mod: &str,
    ) -> Self {
        Self { id, event_types, priority, source_mod: source_mod.to_string() }
    }
}

pub struct SubscriptionHandle {
    pub id: u64,
    pub event_types: Vec<super::event::EventType>,
    pub priority: i32,
    pub source_mod: String,
    pub callback: Box<dyn Fn(&super::event::Event) -> Vec<super::event::Event> + Send + Sync>,
}

impl std::fmt::Debug for SubscriptionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubscriptionHandle")
            .field("id", &self.id)
            .field("event_types", &self.event_types)
            .field("priority", &self.priority)
            .field("source_mod", &self.source_mod)
            .field("callback", &"<closure>")
            .finish()
    }
}

impl SubscriptionHandle {
    pub fn matches(&self, event_type: &super::event::EventType) -> bool {
        self.event_types.is_empty() || self.event_types.contains(event_type)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    pub source_entity: Option<Uuid>,
    pub target_entity: Option<Uuid>,
    pub min_intensity: Option<f32>,
    pub max_intensity: Option<f32>,
    pub position_radius: Option<(f32, [f32; 3])>,
}

impl SubscriptionFilter {
    pub fn matches(&self, event: &super::event::Event) -> bool {
        if let Some(source) = self.source_entity {
            if event.source_entity != Some(source) {
                return false;
            }
        }
        if let Some(target) = self.target_entity {
            if event.target_entity != Some(target) {
                return false;
            }
        }
        if let Some(min) = self.min_intensity {
            if event.intensity < min {
                return false;
            }
        }
        if let Some(max) = self.max_intensity {
            if event.intensity > max {
                return false;
            }
        }
        if let Some((radius, center)) = self.position_radius {
            let dx = event.position[0] - center[0];
            let dy = event.position[1] - center[1];
            let dz = event.position[2] - center[2];
            if (dx * dx + dy * dy + dz * dz) > radius * radius {
                return false;
            }
        }
        true
    }
}
