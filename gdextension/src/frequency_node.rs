use godot::prelude::*;

use wasteland_frequency::scheduler::{FrequencyScheduler, SchedulerConfig};
use wasteland_frequency::tier::{FrequencyTier, Urgency};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandFrequency {
    #[var]
    base_tick_rate: f32,
    #[var]
    max_distance: f32,

    #[allow(dead_code)]
    scheduler: FrequencyScheduler,
    tick_count: i64,
    critical_count: i64,
    high_count: i64,
    medium_count: i64,
    low_count: i64,
    background_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandFrequency {
    fn init(base: Base<Node>) -> Self {
        Self {
            base_tick_rate: 60.0,
            max_distance: 1000.0,
            scheduler: FrequencyScheduler::new(SchedulerConfig::default()),
            tick_count: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            background_count: 0,
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        self.tick_count += 1;
    }
}

#[godot_api]
impl WastelandFrequency {
    #[func]
    fn get_tier_hz(&self, tier: i64) -> f32 {
        let t = match tier {
            0 => FrequencyTier::Critical,
            1 => FrequencyTier::High,
            2 => FrequencyTier::Medium,
            3 => FrequencyTier::Low,
            _ => FrequencyTier::Background,
        };
        t.hz()
    }

    #[func]
    fn get_tier_interval_ticks(&self, tier: i64) -> i64 {
        let t = match tier {
            0 => FrequencyTier::Critical,
            1 => FrequencyTier::High,
            2 => FrequencyTier::Medium,
            3 => FrequencyTier::Low,
            _ => FrequencyTier::Background,
        };
        t.interval_ticks() as i64
    }

    #[func]
    fn get_tier_label(&self, tier: i64) -> GString {
        let t = match tier {
            0 => FrequencyTier::Critical,
            1 => FrequencyTier::High,
            2 => FrequencyTier::Medium,
            3 => FrequencyTier::Low,
            _ => FrequencyTier::Background,
        };
        GString::from(t.label())
    }

    #[func]
    fn compute_urgency_from_distance(&self, distance: f32) -> Dictionary<Variant, Variant> {
        let urgency = Urgency::from_distance(distance, self.max_distance);
        dict! {
            "tier" => urgency.tier as i64,
            "score" => urgency.score,
            "label" => &GString::from(urgency.tier.label()),
        }
    }

    #[func]
    fn compute_urgency_from_velocity(
        &self,
        velocity: f32,
        threshold: f32,
    ) -> Dictionary<Variant, Variant> {
        let urgency = Urgency::from_velocity(velocity, threshold);
        dict! {
            "tier" => urgency.tier as i64,
            "score" => urgency.score,
            "label" => &GString::from(urgency.tier.label()),
        }
    }

    #[func]
    fn should_update_this_tick(&self, tier: i64) -> bool {
        let t = match tier {
            0 => FrequencyTier::Critical,
            1 => FrequencyTier::High,
            2 => FrequencyTier::Medium,
            3 => FrequencyTier::Low,
            _ => FrequencyTier::Background,
        };
        let interval = t.interval_ticks();
        if interval == 0 {
            return false;
        }
        (self.tick_count as u64).is_multiple_of(interval)
    }

    #[func]
    fn register_task(&mut self, tier: i64) {
        match tier {
            0 => self.critical_count += 1,
            1 => self.high_count += 1,
            2 => self.medium_count += 1,
            3 => self.low_count += 1,
            _ => self.background_count += 1,
        }
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "tick_count" => self.tick_count,
            "critical_count" => self.critical_count,
            "high_count" => self.high_count,
            "medium_count" => self.medium_count,
            "low_count" => self.low_count,
            "background_count" => self.background_count,
            "total_tasks" => self.critical_count + self.high_count + self.medium_count + self.low_count + self.background_count,
        }
    }
}
