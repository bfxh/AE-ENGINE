use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::event::{Event, EventType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub event: Event,
    pub frame: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayBuffer {
    records: Vec<EventRecord>,
    max_frames: u64,
    next_sequence: u64,
    current_frame: u64,
    recording: bool,
    auto_deduplicate: bool,
}

impl ReplayBuffer {
    pub fn new(max_frames: u64) -> Self {
        Self {
            records: Vec::with_capacity(1024),
            max_frames,
            next_sequence: 0,
            current_frame: 0,
            recording: false,
            auto_deduplicate: true,
        }
    }

    pub fn start_recording(&mut self) {
        self.recording = true;
        self.records.clear();
    }

    pub fn stop_recording(&mut self) {
        self.recording = false;
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    pub fn record(&mut self, event: &Event) {
        if !self.recording {
            return;
        }

        if self.auto_deduplicate {
            if let Some(last) = self.records.last() {
                if last.event.event_type == event.event_type
                    && last.event.source_entity == event.source_entity
                    && last.event.target_entity == event.target_entity
                    && last.frame == self.current_frame
                {
                    return;
                }
            }
        }

        self.records.push(EventRecord {
            event: event.clone(),
            frame: self.current_frame,
            sequence: self.next_sequence,
        });
        self.next_sequence += 1;

        while self.records.len() > 1 {
            let first_frame = self.records.first().map(|r| r.frame).unwrap_or(0);
            if self.current_frame - first_frame > self.max_frames {
                self.records.remove(0);
            } else {
                break;
            }
        }
    }

    pub fn advance_frame(&mut self) {
        self.current_frame += 1;
    }

    pub fn replay_frame(&self, target_frame: u64) -> Vec<&Event> {
        self.records.iter().filter(|r| r.frame == target_frame).map(|r| &r.event).collect()
    }

    pub fn replay_range(&self, start_frame: u64, end_frame: u64) -> Vec<&Event> {
        self.records
            .iter()
            .filter(|r| r.frame >= start_frame && r.frame <= end_frame)
            .map(|r| &r.event)
            .collect()
    }

    pub fn rollback_to(&mut self, target_frame: u64) -> Vec<Event> {
        self.records.retain(|r| r.frame <= target_frame);
        self.current_frame = target_frame;
        self.records.iter().filter(|r| r.frame == target_frame).map(|r| r.event.clone()).collect()
    }

    pub fn find_events_by_type(&self, event_type: EventType) -> Vec<&Event> {
        self.records.iter().filter(|r| r.event.event_type == event_type).map(|r| &r.event).collect()
    }

    pub fn find_events_by_entity(&self, entity_id: Uuid) -> Vec<&Event> {
        self.records
            .iter()
            .filter(|r| {
                r.event.source_entity == Some(entity_id) || r.event.target_entity == Some(entity_id)
            })
            .map(|r| &r.event)
            .collect()
    }

    pub fn frame_count(&self) -> u64 {
        self.current_frame
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.next_sequence = 0;
        self.current_frame = 0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSnapshot {
    pub frame: u64,
    pub events: Vec<Event>,
    pub timestamp: f64,
}

impl EventSnapshot {
    pub fn new(frame: u64, events: Vec<Event>) -> Self {
        Self { frame, events, timestamp: 0.0 }
    }
}

#[derive(Debug)]
pub struct EventJournal {
    snapshots: Vec<EventSnapshot>,
    max_snapshots: usize,
    checkpoint_interval: u64,
    last_checkpoint: u64,
}

impl EventJournal {
    pub fn new(max_snapshots: usize, checkpoint_interval: u64) -> Self {
        Self {
            snapshots: Vec::with_capacity(max_snapshots),
            max_snapshots,
            checkpoint_interval,
            last_checkpoint: 0,
        }
    }

    pub fn should_checkpoint(&self, frame: u64) -> bool {
        frame - self.last_checkpoint >= self.checkpoint_interval
    }

    pub fn create_checkpoint(&mut self, frame: u64, events: Vec<Event>) {
        self.snapshots.push(EventSnapshot::new(frame, events));
        self.last_checkpoint = frame;
        while self.snapshots.len() > self.max_snapshots {
            self.snapshots.remove(0);
        }
    }

    pub fn latest_checkpoint(&self) -> Option<&EventSnapshot> {
        self.snapshots.last()
    }

    pub fn checkpoint_at(&self, frame: u64) -> Option<&EventSnapshot> {
        self.snapshots.iter().find(|s| s.frame == frame)
    }

    pub fn checkpoint_before(&self, frame: u64) -> Option<&EventSnapshot> {
        self.snapshots.iter().rev().find(|s| s.frame <= frame)
    }

    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventData;

    fn make_event(frame: u64) -> Event {
        Event::new(
            EventType::CollisionDetected,
            Some(Uuid::new_v4()),
            None,
            [0.0, 0.0, 0.0],
            1.0,
            EventData::None,
            frame,
        )
    }

    #[test]
    fn test_replay_buffer_record() {
        let mut buf = ReplayBuffer::new(1000);
        buf.start_recording();
        buf.record(&make_event(0));
        buf.advance_frame();
        assert_eq!(buf.record_count(), 1);
        assert_eq!(buf.frame_count(), 1);
    }

    #[test]
    fn test_replay_buffer_deduplicate() {
        let mut buf = ReplayBuffer::new(1000);
        buf.start_recording();
        let event = make_event(0);
        buf.record(&event);
        buf.record(&event);
        assert_eq!(buf.record_count(), 1);
    }

    #[test]
    fn test_replay_buffer_rollback() {
        let mut buf = ReplayBuffer::new(1000);
        buf.start_recording();
        buf.record(&make_event(0));
        buf.advance_frame();
        buf.record(&make_event(1));
        buf.advance_frame();
        buf.record(&make_event(2));
        buf.advance_frame();
        assert_eq!(buf.record_count(), 3);
        buf.rollback_to(1);
        assert_eq!(buf.record_count(), 2);
    }

    #[test]
    fn test_replay_buffer_range() {
        let mut buf = ReplayBuffer::new(1000);
        buf.start_recording();
        for i in 0..10 {
            buf.record(&make_event(i));
            buf.advance_frame();
        }
        let range = buf.replay_range(3, 6);
        assert_eq!(range.len(), 4);
    }

    #[test]
    fn test_journal_checkpoint() {
        let mut journal = EventJournal::new(10, 5);
        assert!(journal.should_checkpoint(5));
        journal.create_checkpoint(5, vec![make_event(5)]);
        assert!(!journal.should_checkpoint(6));
        assert!(journal.checkpoint_at(5).is_some());
    }
}
