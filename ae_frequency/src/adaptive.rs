use serde::{Deserialize, Serialize};

use crate::tier::FrequencyTier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMonitor {
    current_load: f32,
    load_history: Vec<f32>,
    history_idx: usize,
    overload_threshold: f32,
    underload_threshold: f32,
    overload_count: u32,
    underload_count: u32,
    scale_state: ScaleState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScaleState {
    Normal,
    ScalingUp,
    ScalingDown,
    Emergency,
}

impl Default for LoadMonitor {
    fn default() -> Self {
        Self {
            current_load: 0.0,
            load_history: vec![0.0; 60],
            history_idx: 0,
            overload_threshold: 0.85,
            underload_threshold: 0.3,
            overload_count: 0,
            underload_count: 0,
            scale_state: ScaleState::Normal,
        }
    }
}

impl LoadMonitor {
    pub fn update(&mut self, frame_time_ms: f32, target_frame_time_ms: f32) {
        let load = (frame_time_ms / target_frame_time_ms.max(0.001)).clamp(0.0, 2.0);
        self.current_load = load;
        self.load_history[self.history_idx] = load;
        self.history_idx = (self.history_idx + 1) % 60;

        if load > self.overload_threshold {
            self.overload_count += 1;
            self.underload_count = 0;
        } else if load < self.underload_threshold {
            self.underload_count += 1;
            self.overload_count = 0;
        } else {
            self.overload_count = self.overload_count.saturating_sub(1);
            self.underload_count = self.underload_count.saturating_sub(1);
        }

        self.scale_state = if self.overload_count > 10 {
            ScaleState::Emergency
        } else if self.overload_count > 5 {
            ScaleState::ScalingDown
        } else if self.underload_count > 5 {
            ScaleState::ScalingUp
        } else {
            ScaleState::Normal
        };
    }

    pub fn average_load(&self) -> f32 {
        let sum: f32 = self.load_history.iter().sum();
        sum / self.load_history.len() as f32
    }

    pub fn current_load(&self) -> f32 {
        self.current_load
    }

    pub fn scale_state(&self) -> ScaleState {
        self.scale_state
    }

    pub fn scale_factor(&self) -> f32 {
        match self.scale_state {
            ScaleState::Normal => 1.0,
            ScaleState::ScalingUp => 1.0 + (1.0 - self.current_load) * 0.5,
            ScaleState::ScalingDown => {
                let overload = self.current_load - self.overload_threshold;
                (1.0 - overload * 2.0).max(0.1)
            },
            ScaleState::Emergency => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveScaler {
    pub monitor: LoadMonitor,
    target_frame_time_ms: f32,
    base_intervals: [u64; 5],
    scaled_intervals: [u64; 5],
    min_intervals: [u64; 5],
}

impl AdaptiveScaler {
    pub fn new(target_frame_time_ms: f32) -> Self {
        let base = [
            FrequencyTier::Critical.interval_ticks(),
            FrequencyTier::High.interval_ticks(),
            FrequencyTier::Medium.interval_ticks(),
            FrequencyTier::Low.interval_ticks(),
            FrequencyTier::Background.interval_ticks(),
        ];
        let min = [1, 1, 2, 10, 100];
        Self {
            monitor: LoadMonitor::default(),
            target_frame_time_ms,
            base_intervals: base,
            scaled_intervals: base,
            min_intervals: min,
        }
    }

    pub fn update(&mut self, frame_time_ms: f32) {
        self.monitor.update(frame_time_ms, self.target_frame_time_ms);
        let factor = self.monitor.scale_factor();
        for i in 0..5 {
            let scaled = (self.base_intervals[i] as f32 / factor).round() as u64;
            self.scaled_intervals[i] = scaled.max(self.min_intervals[i]);
        }
    }

    pub fn get_scaled_interval(&self, tier: FrequencyTier) -> u64 {
        self.scaled_intervals[tier as usize]
    }

    pub fn should_skip_tier(&self, tier: FrequencyTier) -> bool {
        self.monitor.scale_state() == ScaleState::Emergency && tier >= FrequencyTier::Low
    }

    pub fn is_emergency(&self) -> bool {
        self.monitor.scale_state() == ScaleState::Emergency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_monitor_normal() {
        let mut m = LoadMonitor::default();
        m.update(8.0, 16.0);
        assert_eq!(m.scale_state(), ScaleState::Normal);
    }

    #[test]
    fn test_load_monitor_overload() {
        let mut m = LoadMonitor::default();
        for _ in 0..12 {
            m.update(16.0, 16.0);
        }
        assert_eq!(m.scale_state(), ScaleState::Emergency);
    }

    #[test]
    fn test_adaptive_scaler_scale_down() {
        let mut s = AdaptiveScaler::new(16.0);
        for _ in 0..12 {
            s.update(20.0);
        }
        let bg = s.get_scaled_interval(FrequencyTier::Background);
        assert!(bg >= s.min_intervals[FrequencyTier::Background as usize]);
    }

    #[test]
    fn test_adaptive_scaler_emergency_skip() {
        let mut s = AdaptiveScaler::new(16.0);
        for _ in 0..12 {
            s.update(20.0);
        }
        assert!(s.should_skip_tier(FrequencyTier::Low));
        assert!(!s.should_skip_tier(FrequencyTier::Critical));
    }
}
