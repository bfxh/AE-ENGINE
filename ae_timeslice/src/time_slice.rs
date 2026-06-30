use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeSliceLayer {
    Layer0 = 0,
    Layer1 = 1,
    Layer2 = 2,
    Layer3 = 3,
    Layer4 = 4,
    Layer5 = 5,
}

impl TimeSliceLayer {
    pub fn divisor(&self) -> u64 {
        match self {
            TimeSliceLayer::Layer0 => 1,
            TimeSliceLayer::Layer1 => 1,
            TimeSliceLayer::Layer2 => 2,
            TimeSliceLayer::Layer3 => 5,
            TimeSliceLayer::Layer4 => 30,
            TimeSliceLayer::Layer5 => 0,
        }
    }

    pub fn should_update(&self, tick: u64) -> bool {
        if *self == TimeSliceLayer::Layer5 {
            return false;
        }
        tick.is_multiple_of(self.divisor())
    }
}

type SystemFn = Box<dyn FnMut(f32) + Send>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SystemId(u64);

impl SystemId {
    pub fn value(&self) -> u64 {
        self.0
    }
    pub fn from_raw(value: u64) -> Self {
        SystemId(value)
    }
}

#[derive(Debug)]
pub struct LayeredTimeSlicer {
    pub tick_count: u64,
    pub systems: HashMap<SystemId, SystemEntry>,
    pub(crate) layer_order: Vec<TimeSliceLayer>,
    next_id: u64,
}

pub struct SystemEntry {
    name: String,
    layer: TimeSliceLayer,
    enabled: bool,
    update_count: u64,
    total_time: f64,
    callback: SystemFn,
}

impl std::fmt::Debug for SystemEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemEntry")
            .field("name", &self.name)
            .field("layer", &self.layer)
            .field("enabled", &self.enabled)
            .field("update_count", &self.update_count)
            .field("total_time", &self.total_time)
            .finish()
    }
}

impl LayeredTimeSlicer {
    pub fn new() -> Self {
        Self {
            tick_count: 0,
            systems: HashMap::new(),
            layer_order: vec![
                TimeSliceLayer::Layer0,
                TimeSliceLayer::Layer1,
                TimeSliceLayer::Layer2,
                TimeSliceLayer::Layer3,
                TimeSliceLayer::Layer4,
            ],
            next_id: 0,
        }
    }

    pub fn register<F>(&mut self, name: &str, layer: TimeSliceLayer, callback: F) -> SystemId
    where
        F: FnMut(f32) + Send + 'static,
    {
        let id = SystemId(self.next_id);
        self.next_id += 1;
        self.systems.insert(
            id,
            SystemEntry {
                name: name.to_string(),
                layer,
                enabled: true,
                update_count: 0,
                total_time: 0.0,
                callback: Box::new(callback),
            },
        );
        id
    }

    pub fn set_enabled(&mut self, id: SystemId, enabled: bool) {
        if let Some(entry) = self.systems.get_mut(&id) {
            entry.enabled = enabled;
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.tick_count += 1;

        for layer in &self.layer_order {
            if !layer.should_update(self.tick_count) {
                continue;
            }

            let mut to_update: Vec<SystemId> = self
                .systems
                .iter()
                .filter(|(_, e)| e.layer == *layer && e.enabled)
                .map(|(id, _)| *id)
                .collect();
            to_update.sort_by_key(|id| id.0);

            for id in to_update {
                if let Some(entry) = self.systems.get_mut(&id) {
                    let start = std::time::Instant::now();
                    (entry.callback)(dt);
                    entry.total_time += start.elapsed().as_secs_f64();
                    entry.update_count += 1;
                }
            }
        }
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn system_stats(&self) -> Vec<SystemStats> {
        self.systems
            .iter()
            .map(|(id, entry)| {
                let avg_time = if entry.update_count > 0 {
                    entry.total_time / entry.update_count as f64
                } else {
                    0.0
                };
                SystemStats {
                    id: id.0,
                    name: entry.name.clone(),
                    layer: entry.layer,
                    update_count: entry.update_count,
                    avg_time_ms: avg_time * 1000.0,
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub id: u64,
    pub name: String,
    pub layer: TimeSliceLayer,
    pub update_count: u64,
    pub avg_time_ms: f64,
}

impl Default for LayeredTimeSlicer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_divisors() {
        assert_eq!(TimeSliceLayer::Layer0.divisor(), 1);
        assert_eq!(TimeSliceLayer::Layer1.divisor(), 1);
        assert_eq!(TimeSliceLayer::Layer2.divisor(), 2);
        assert_eq!(TimeSliceLayer::Layer3.divisor(), 5);
        assert_eq!(TimeSliceLayer::Layer4.divisor(), 30);
    }

    #[test]
    fn test_layer_update_pattern() {
        assert!(TimeSliceLayer::Layer0.should_update(0));
        assert!(TimeSliceLayer::Layer0.should_update(1));
        assert!(TimeSliceLayer::Layer2.should_update(0));
        assert!(!TimeSliceLayer::Layer2.should_update(1));
        assert!(TimeSliceLayer::Layer2.should_update(2));
        assert!(!TimeSliceLayer::Layer3.should_update(1));
        assert!(TimeSliceLayer::Layer3.should_update(5));
    }

    #[test]
    fn test_slicer_registration() {
        let mut slicer = LayeredTimeSlicer::new();
        let mut _counter = 0;
        let _id = slicer.register("test", TimeSliceLayer::Layer0, move |_dt| {
            _counter += 1;
        });
        slicer.step(0.016);
        assert_eq!(slicer.tick_count(), 1);
        assert!(!slicer.system_stats().is_empty());
    }
}
