use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub event_id: Uuid,
    pub tick: u64,
    pub timestamp: f64,
    pub event_type: EventType,
    pub entity_id: Option<Uuid>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    EntitySpawned,
    EntityDestroyed,
    EntityMoved,
    Collision,
    ChemicalReaction,
    PhaseTransition,
    FieldChanged,
    MorphogenesisStep,
    StateSnapshot,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStore {
    pub events: Vec<GameEvent>,
    pub snapshots: Vec<SnapshotEntry>,
    pub max_events: usize,
    pub total_events_ever: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub tick: u64,
    pub timestamp: f64,
    pub data: Vec<u8>,
    pub event_index: usize,
}

impl EventStore {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_events.min(10000)),
            snapshots: Vec::new(),
            max_events,
            total_events_ever: 0,
        }
    }

    pub fn record(
        &mut self,
        tick: u64,
        timestamp: f64,
        event_type: EventType,
        entity_id: Option<Uuid>,
        data: Vec<u8>,
    ) -> Uuid {
        let event_id = Uuid::new_v4();
        let event = GameEvent { event_id, tick, timestamp, event_type, entity_id, data };
        self.events.push(event);
        self.total_events_ever += 1;

        while self.events.len() > self.max_events {
            self.events.remove(0);
        }

        event_id
    }

    pub fn create_snapshot(&mut self, tick: u64, timestamp: f64, data: Vec<u8>) {
        let event_index = self.events.len();
        let entry = SnapshotEntry { tick, timestamp, data, event_index };
        self.snapshots.push(entry);
    }

    pub fn events_since(&self, tick: u64) -> Vec<&GameEvent> {
        self.events.iter().filter(|e| e.tick >= tick).collect()
    }

    pub fn events_between(&self, start_tick: u64, end_tick: u64) -> Vec<&GameEvent> {
        self.events.iter().filter(|e| e.tick >= start_tick && e.tick < end_tick).collect()
    }

    pub fn events_by_type(&self, event_type: EventType) -> Vec<&GameEvent> {
        self.events.iter().filter(|e| e.event_type == event_type).collect()
    }

    pub fn events_by_entity(&self, entity_id: Uuid) -> Vec<&GameEvent> {
        self.events.iter().filter(|e| e.entity_id == Some(entity_id)).collect()
    }

    pub fn latest_snapshot_before(&self, tick: u64) -> Option<&SnapshotEntry> {
        self.snapshots.iter().filter(|s| s.tick <= tick).max_by_key(|s| s.tick)
    }

    pub fn replay_events(&self, from_tick: u64, to_tick: u64, mut handler: impl FnMut(&GameEvent)) {
        for event in self.events_between(from_tick, to_tick) {
            handler(event);
        }
    }

    pub fn replay_all(&self, mut handler: impl FnMut(&GameEvent)) {
        for event in &self.events {
            handler(event);
        }
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.snapshots.clear();
    }
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new(100000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_recording() {
        let mut store = EventStore::new(1000);
        let _id =
            store.record(0, 0.0, EventType::EntitySpawned, Some(Uuid::new_v4()), vec![1, 2, 3]);
        assert_eq!(store.event_count(), 1);
        assert_eq!(store.total_events_ever, 1);
    }

    #[test]
    fn test_snapshot_and_replay() {
        let mut store = EventStore::new(1000);
        let entity_id = Uuid::new_v4();

        for i in 0..10 {
            store.record(i, i as f64, EventType::EntityMoved, Some(entity_id), vec![i as u8]);
        }

        store.create_snapshot(5, 5.0, vec![5]);

        let events: Vec<_> = store.events_between(5, 10).iter().map(|e| e.tick).collect();
        assert_eq!(events, vec![5, 6, 7, 8, 9]);

        let snapshot = store.latest_snapshot_before(6);
        assert!(snapshot.is_some());
    }
}
