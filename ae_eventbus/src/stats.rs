use std::cmp::Reverse;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::event::EventType;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventStats {
    pub total_emitted: u64,
    pub total_dispatched: u64,
    pub total_deferred: u64,
    pub peak_pending: usize,
    pub peak_history: usize,
    pub type_counts: HashMap<EventType, u64>,
    pub frame_counts: Vec<u64>,
    pub max_frame_count: usize,
    pub avg_events_per_frame: f32,
    pub burst_count: u64,
    pub throttled_count: u64,
}

impl EventStats {
    pub fn new(max_frame_count: usize) -> Self {
        Self { max_frame_count, frame_counts: vec![0; max_frame_count], ..Default::default() }
    }

    pub fn record_emit(&mut self, event_type: EventType) {
        self.total_emitted += 1;
        *self.type_counts.entry(event_type).or_insert(0) += 1;
    }

    pub fn record_dispatch(&mut self) {
        self.total_dispatched += 1;
    }

    pub fn record_deferred(&mut self) {
        self.total_deferred += 1;
    }

    pub fn record_frame(&mut self, frame: u64, event_count: usize, pending: usize, history: usize) {
        let idx = frame as usize % self.max_frame_count;
        self.frame_counts[idx] = event_count as u64;
        self.peak_pending = self.peak_pending.max(pending);
        self.peak_history = self.peak_history.max(history);

        let total: u64 = self.frame_counts.iter().sum();
        let filled = (frame as usize + 1).min(self.max_frame_count);
        self.avg_events_per_frame = total as f32 / filled as f32;
    }

    pub fn record_burst(&mut self, count: usize, threshold: usize) {
        if count > threshold {
            self.burst_count += 1;
        }
    }

    pub fn record_throttled(&mut self) {
        self.throttled_count += 1;
    }

    pub fn top_types(&self, n: usize) -> Vec<(EventType, u64)> {
        let mut pairs: Vec<_> = self.type_counts.iter().map(|(k, v)| (*k, *v)).collect();
        pairs.sort_by_key(|x| Reverse(x.1));
        pairs.truncate(n);
        pairs
    }

    pub fn throttled_ratio(&self) -> f32 {
        if self.total_emitted == 0 {
            0.0
        } else {
            self.throttled_count as f32 / self.total_emitted as f32
        }
    }

    pub fn burst_rate(&self) -> f32 {
        if self.total_emitted == 0 {
            0.0
        } else {
            self.burst_count as f32 / self.total_emitted as f32
        }
    }

    pub fn reset(&mut self) {
        self.total_emitted = 0;
        self.total_dispatched = 0;
        self.total_deferred = 0;
        self.peak_pending = 0;
        self.peak_history = 0;
        self.type_counts.clear();
        self.frame_counts.fill(0);
        self.burst_count = 0;
        self.throttled_count = 0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottleConfig {
    pub max_events_per_type_per_frame: usize,
    pub burst_threshold: usize,
    pub burst_window: usize,
    pub cooldown_frames: u64,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            max_events_per_type_per_frame: 100,
            burst_threshold: 50,
            burst_window: 5,
            cooldown_frames: 10,
        }
    }
}

#[derive(Debug)]
pub struct EventThrottle {
    config: ThrottleConfig,
    type_frame_counts: HashMap<EventType, usize>,
    burst_history: Vec<usize>,
    cooldown_until: HashMap<EventType, u64>,
    current_frame: u64,
}

impl EventThrottle {
    pub fn new(config: ThrottleConfig) -> Self {
        Self {
            config,
            type_frame_counts: HashMap::new(),
            burst_history: Vec::with_capacity(64),
            cooldown_until: HashMap::new(),
            current_frame: 0,
        }
    }

    pub fn should_allow(&mut self, event_type: EventType, frame: u64) -> bool {
        if frame != self.current_frame {
            self.type_frame_counts.clear();
            self.current_frame = frame;
        }

        if let Some(&cooldown_end) = self.cooldown_until.get(&event_type) {
            if frame < cooldown_end {
                return false;
            } else {
                self.cooldown_until.remove(&event_type);
            }
        }

        let count = self.type_frame_counts.entry(event_type).or_insert(0);
        if *count >= self.config.max_events_per_type_per_frame {
            return false;
        }

        *count += 1;
        self.burst_history.push(*count);
        if self.burst_history.len() > self.config.burst_window {
            self.burst_history.remove(0);
        }

        let burst_sum: usize = self.burst_history.iter().sum();
        if burst_sum > self.config.burst_threshold * self.config.burst_window {
            self.cooldown_until.insert(event_type, frame + self.config.cooldown_frames);
            return false;
        }

        true
    }

    pub fn reset(&mut self) {
        self.type_frame_counts.clear();
        self.burst_history.clear();
        self.cooldown_until.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_stats_recording() {
        let mut stats = EventStats::new(100);
        stats.record_emit(EventType::CollisionDetected);
        stats.record_emit(EventType::CollisionDetected);
        stats.record_emit(EventType::DamageReceived);
        stats.record_frame(0, 3, 0, 3);
        assert_eq!(stats.total_emitted, 3);
        assert_eq!(stats.type_counts[&EventType::CollisionDetected], 2);
    }

    #[test]
    fn test_event_stats_top_types() {
        let mut stats = EventStats::new(100);
        for _ in 0..5 {
            stats.record_emit(EventType::CollisionDetected);
        }
        for _ in 0..3 {
            stats.record_emit(EventType::DamageReceived);
        }
        for _ in 0..8 {
            stats.record_emit(EventType::ForceApplied);
        }
        let top = stats.top_types(2);
        assert_eq!(top[0].0, EventType::ForceApplied);
        assert_eq!(top[1].0, EventType::CollisionDetected);
    }

    #[test]
    fn test_throttle_allow() {
        let mut throttle = EventThrottle::new(ThrottleConfig::default());
        assert!(throttle.should_allow(EventType::CollisionDetected, 0));
        assert!(throttle.should_allow(EventType::CollisionDetected, 0));
    }

    #[test]
    fn test_throttle_limit() {
        let config = ThrottleConfig { max_events_per_type_per_frame: 2, ..Default::default() };
        let mut throttle = EventThrottle::new(config);
        assert!(throttle.should_allow(EventType::CollisionDetected, 0));
        assert!(throttle.should_allow(EventType::CollisionDetected, 0));
        assert!(!throttle.should_allow(EventType::CollisionDetected, 0));
    }

    #[test]
    fn test_throttle_frame_reset() {
        let config = ThrottleConfig { max_events_per_type_per_frame: 2, ..Default::default() };
        let mut throttle = EventThrottle::new(config);
        assert!(throttle.should_allow(EventType::CollisionDetected, 0));
        assert!(throttle.should_allow(EventType::CollisionDetected, 0));
        assert!(!throttle.should_allow(EventType::CollisionDetected, 0));
        assert!(throttle.should_allow(EventType::CollisionDetected, 1));
    }
}
