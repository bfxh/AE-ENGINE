use hashbrown::HashMap;
use std::collections::VecDeque;

#[cfg(test)]
use crate::event::EventData;
use crate::event::{Event, EventType};
use crate::subscription::{SubscriptionFilter, SubscriptionHandle};

const HISTORY_SIZE: usize = 1000;

#[derive(Debug)]
pub struct EventBus {
    subscribers: HashMap<usize, Vec<SubscriptionHandle>>,
    pending: VecDeque<Event>,
    history: VecDeque<Event>,
    next_id: u64,
    frame_count: u64,
    batch_enabled: bool,
    batch_buffer: Vec<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
            pending: VecDeque::with_capacity(256),
            history: VecDeque::with_capacity(HISTORY_SIZE),
            next_id: 1,
            frame_count: 0,
            batch_enabled: true,
            batch_buffer: Vec::with_capacity(128),
        }
    }

    pub fn subscribe(
        &mut self,
        event_types: Vec<EventType>,
        priority: i32,
        source_mod: &str,
        callback: Box<dyn Fn(&Event) -> Vec<Event> + Send + Sync>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let handle = SubscriptionHandle {
            id,
            event_types: event_types.clone(),
            priority,
            source_mod: source_mod.to_string(),
            callback,
        };

        let bucket = priority_to_bucket(priority);
        self.subscribers.entry(bucket).or_default().push(handle);
        self.subscribers.entry(bucket).or_default().sort_by_key(|s| s.priority);

        id
    }

    pub fn unsubscribe(&mut self, id: u64) {
        for bucket in self.subscribers.values_mut() {
            bucket.retain(|s| s.id != id);
        }
    }

    pub fn emit(&mut self, event: Event) {
        if self.batch_enabled {
            self.batch_buffer.push(event);
        } else {
            self.dispatch(event);
        }
    }

    pub fn emit_immediate(&mut self, event: Event) {
        self.dispatch(event);
    }

    pub fn flush_batch(&mut self) {
        let batch: Vec<Event> = std::mem::take(&mut self.batch_buffer);
        for event in batch {
            self.dispatch(event);
        }
    }

    pub fn tick(&mut self) {
        self.frame_count += 1;
        self.flush_batch();

        while let Some(event) = self.pending.pop_front() {
            self.dispatch(event);
        }
    }

    pub fn emit_deferred(&mut self, event: Event) {
        self.pending.push_back(event);
    }

    fn dispatch(&mut self, event: Event) {
        for bucket in 0..=PRIORITY_BUCKETS {
            if let Some(subscribers) = self.subscribers.get(&bucket) {
                for sub in subscribers {
                    if sub.matches(&event.event_type) {
                        let new_events = (sub.callback)(&event);
                        for new_event in new_events {
                            self.pending.push_back(new_event);
                        }
                    }
                }
            }
        }

        if self.history.len() >= HISTORY_SIZE {
            self.history.pop_front();
        }
        self.history.push_back(event);
    }

    pub fn query_history(
        &self,
        event_type: Option<EventType>,
        filter: Option<&SubscriptionFilter>,
        limit: usize,
    ) -> Vec<&Event> {
        self.history
            .iter()
            .rev()
            .filter(|e| {
                if let Some(t) = &event_type {
                    if e.event_type != *t {
                        return false;
                    }
                }
                if let Some(f) = filter {
                    if !f.matches(e) {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .collect()
    }

    pub fn query_by_entity(&self, entity_id: uuid::Uuid, limit: usize) -> Vec<&Event> {
        self.history
            .iter()
            .rev()
            .filter(|e| e.source_entity == Some(entity_id) || e.target_entity == Some(entity_id))
            .take(limit)
            .collect()
    }

    pub fn set_batch_mode(&mut self, enabled: bool) {
        if !enabled {
            self.flush_batch();
        }
        self.batch_enabled = enabled;
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn subscription_count(&self) -> usize {
        self.subscribers.values().map(|v| v.len()).sum()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    pub fn clear(&mut self) {
        self.subscribers.clear();
        self.pending.clear();
        self.history.clear();
        self.batch_buffer.clear();
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

const PRIORITY_BUCKETS: usize = 4;

fn priority_to_bucket(priority: i32) -> usize {
    match priority {
        ..=-1 => 0,
        0..=9 => 1,
        10..=49 => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_and_dispatch() {
        let mut bus = EventBus::new();
        bus.set_batch_mode(false);

        let event = Event::new(
            EventType::CollisionDetected,
            None,
            None,
            [0.0, 0.0, 0.0],
            1.0,
            EventData::None,
            0,
        );

        bus.emit(event);
        assert_eq!(bus.history_count(), 1);
    }

    #[test]
    fn test_subscribe_and_receive() {
        let mut bus = EventBus::new();
        bus.set_batch_mode(false);

        bus.subscribe(vec![EventType::DamageReceived], 0, "test", Box::new(|_event| vec![]));

        let event = Event::new(
            EventType::DamageReceived,
            None,
            None,
            [0.0, 0.0, 0.0],
            1.0,
            EventData::None,
            0,
        );

        bus.emit(event);
        assert_eq!(bus.history_count(), 1);
    }

    #[test]
    fn test_batch_flush() {
        let mut bus = EventBus::new();

        for i in 0..10 {
            bus.emit(Event::new(
                EventType::CollisionDetected,
                None,
                None,
                [0.0, 0.0, 0.0],
                1.0,
                EventData::None,
                i,
            ));
        }

        assert_eq!(bus.history_count(), 0);
        bus.flush_batch();
        assert_eq!(bus.history_count(), 10);
    }

    #[test]
    fn test_history_limit() {
        let mut bus = EventBus::new();
        bus.set_batch_mode(false);

        for i in 0..1200 {
            bus.emit(Event::new(
                EventType::CollisionDetected,
                None,
                None,
                [0.0, 0.0, 0.0],
                1.0,
                EventData::None,
                i,
            ));
        }

        assert!(bus.history_count() <= HISTORY_SIZE);
    }
}
