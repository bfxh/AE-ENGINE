use glam::Vec3;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::tier::{FrequencyTier, Urgency};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub max_distance: f32,
    pub velocity_threshold: f32,
    pub max_scheduled_entities: usize,
    pub tier_budget: TierBudget,
    pub hysteresis: f32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_distance: 200.0,
            velocity_threshold: 50.0,
            max_scheduled_entities: 10000,
            tier_budget: TierBudget::default(),
            hysteresis: 0.15,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TierBudget {
    pub critical_max: usize,
    pub high_max: usize,
    pub medium_max: usize,
}

impl Default for TierBudget {
    fn default() -> Self {
        Self { critical_max: 50, high_max: 200, medium_max: 1000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySchedule {
    pub entity_id: Uuid,
    pub tier: FrequencyTier,
    pub urgency_score: f32,
    pub last_tick: u64,
    pub next_tick: u64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub is_player: bool,
    pub is_combat: bool,
}

#[derive(Debug)]
pub struct FrequencyScheduler {
    config: SchedulerConfig,
    schedules: HashMap<Uuid, EntitySchedule>,
    player_position: Vec3,
    tier_counts: [usize; 5],
    total_ticks: u64,
}

impl FrequencyScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            schedules: HashMap::new(),
            player_position: Vec3::ZERO,
            tier_counts: [0; 5],
            total_ticks: 0,
        }
    }

    pub fn register(&mut self, entity_id: Uuid, position: Vec3, velocity: Vec3, is_player: bool) {
        if self.schedules.len() >= self.config.max_scheduled_entities {
            return;
        }
        let tier = if is_player { FrequencyTier::Critical } else { FrequencyTier::Medium };
        let schedule = EntitySchedule {
            entity_id,
            tier,
            urgency_score: if is_player { 1.0 } else { 0.5 },
            last_tick: 0,
            next_tick: 0,
            position,
            velocity,
            is_player,
            is_combat: false,
        };
        self.tier_counts[tier as usize] += 1;
        self.schedules.insert(entity_id, schedule);
    }

    pub fn unregister(&mut self, entity_id: &Uuid) {
        if let Some(s) = self.schedules.remove(entity_id) {
            self.tier_counts[s.tier as usize] = self.tier_counts[s.tier as usize].saturating_sub(1);
        }
    }

    pub fn update_player_position(&mut self, position: Vec3) {
        self.player_position = position;
    }

    pub fn update_entity_state(
        &mut self,
        entity_id: &Uuid,
        position: Vec3,
        velocity: Vec3,
        is_combat: bool,
    ) {
        if let Some(s) = self.schedules.get_mut(entity_id) {
            s.position = position;
            s.velocity = velocity;
            s.is_combat = is_combat;
        }
    }

    pub fn tick(&mut self) -> Vec<Uuid> {
        self.total_ticks += 1;
        let mut active = Vec::new();

        if self.total_ticks.is_multiple_of(60) {
            self.rebalance();
        }

        let player_pos = self.player_position;
        let max_dist = self.config.max_distance;
        let vel_thresh = self.config.velocity_threshold;
        let hysteresis = self.config.hysteresis;

        let mut tier_updates: Vec<(Uuid, FrequencyTier)> = Vec::new();
        let mut to_remove = Vec::new();
        for schedule in self.schedules.values_mut() {
            if self.total_ticks >= schedule.next_tick {
                active.push(schedule.entity_id);
                schedule.last_tick = self.total_ticks;
                schedule.next_tick = self.total_ticks + schedule.tier.interval_ticks();
            }

            if self.total_ticks.is_multiple_of(10) {
                let new_tier =
                    evaluate_tier_static(schedule, player_pos, max_dist, vel_thresh, hysteresis);
                if new_tier != schedule.tier {
                    tier_updates.push((schedule.entity_id, new_tier));
                }
            }

            if schedule.last_tick > 0 && self.total_ticks - schedule.last_tick > 6000 {
                to_remove.push(schedule.entity_id);
            }
        }

        for (entity_id, new_tier) in tier_updates {
            if let Some(schedule) = self.schedules.get(&entity_id) {
                let old_tier = schedule.tier;
                if self.can_promote(new_tier, old_tier) {
                    if let Some(schedule) = self.schedules.get_mut(&entity_id) {
                        self.tier_counts[old_tier as usize] =
                            self.tier_counts[old_tier as usize].saturating_sub(1);
                        self.tier_counts[new_tier as usize] += 1;
                        schedule.tier = new_tier;
                    }
                }
            }
        }

        for id in to_remove {
            self.unregister(&id);
        }

        active
    }

    fn can_promote(&self, new_tier: FrequencyTier, old_tier: FrequencyTier) -> bool {
        if new_tier >= old_tier {
            return true;
        }
        let count = self.tier_counts[new_tier as usize];
        let max = match new_tier {
            FrequencyTier::Critical => self.config.tier_budget.critical_max,
            FrequencyTier::High => self.config.tier_budget.high_max,
            FrequencyTier::Medium => self.config.tier_budget.medium_max,
            FrequencyTier::Low => usize::MAX,
            FrequencyTier::Background => usize::MAX,
        };
        count < max
    }

    fn rebalance(&mut self) {
        let mut tier_counts: [usize; 5] = [0; 5];
        for schedule in self.schedules.values() {
            tier_counts[schedule.tier as usize] += 1;
        }

        if tier_counts[FrequencyTier::Critical as usize] > self.config.tier_budget.critical_max {
            let overflow = tier_counts[FrequencyTier::Critical as usize]
                - self.config.tier_budget.critical_max;
            self.demote_overflow(FrequencyTier::Critical, FrequencyTier::High, overflow);
        }
        if tier_counts[FrequencyTier::High as usize] > self.config.tier_budget.high_max {
            let overflow =
                tier_counts[FrequencyTier::High as usize] - self.config.tier_budget.high_max;
            self.demote_overflow(FrequencyTier::High, FrequencyTier::Medium, overflow);
        }
        if tier_counts[FrequencyTier::Medium as usize] > self.config.tier_budget.medium_max {
            let overflow =
                tier_counts[FrequencyTier::Medium as usize] - self.config.tier_budget.medium_max;
            self.demote_overflow(FrequencyTier::Medium, FrequencyTier::Low, overflow);
        }
    }

    fn demote_overflow(&mut self, from: FrequencyTier, to: FrequencyTier, count: usize) {
        let mut demoted = 0;
        for schedule in self.schedules.values_mut() {
            if demoted >= count {
                break;
            }
            if schedule.tier == from && !schedule.is_player && !schedule.is_combat {
                schedule.tier = to;
                demoted += 1;
            }
        }
        self.tier_counts[from as usize] = self.tier_counts[from as usize].saturating_sub(demoted);
        self.tier_counts[to as usize] += demoted;
    }

    pub fn get_tier(&self, entity_id: &Uuid) -> Option<FrequencyTier> {
        self.schedules.get(entity_id).map(|s| s.tier)
    }

    pub fn tier_counts(&self) -> [usize; 5] {
        self.tier_counts
    }

    pub fn total_entities(&self) -> usize {
        self.schedules.len()
    }

    pub fn should_update(&self, entity_id: &Uuid, current_tick: u64) -> bool {
        if let Some(s) = self.schedules.get(entity_id) { current_tick >= s.next_tick } else { true }
    }
}

fn evaluate_tier_static(
    schedule: &EntitySchedule,
    player_position: Vec3,
    max_distance: f32,
    velocity_threshold: f32,
    hysteresis: f32,
) -> FrequencyTier {
    if schedule.is_player || schedule.is_combat {
        return FrequencyTier::Critical;
    }

    let distance = (schedule.position - player_position).length();
    let urgency = Urgency::from_distance(distance, max_distance);

    let vel_urgency = Urgency::from_velocity(schedule.velocity.length(), velocity_threshold);

    let combined_tier = urgency.tier.min(vel_urgency.tier);

    if schedule.tier != combined_tier {
        let current_score = match schedule.tier {
            FrequencyTier::Critical => 1.0,
            FrequencyTier::High => 0.8,
            FrequencyTier::Medium => 0.5,
            FrequencyTier::Low => 0.2,
            FrequencyTier::Background => 0.0,
        };
        let new_score = match combined_tier {
            FrequencyTier::Critical => 1.0,
            FrequencyTier::High => 0.8,
            FrequencyTier::Medium => 0.5,
            FrequencyTier::Low => 0.2,
            FrequencyTier::Background => 0.0,
        };
        if new_score > current_score + hysteresis {
            return combined_tier;
        }
        if new_score < current_score - hysteresis {
            return combined_tier;
        }
        return schedule.tier;
    }
    combined_tier
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scheduler() -> FrequencyScheduler {
        FrequencyScheduler::new(SchedulerConfig::default())
    }

    #[test]
    fn test_register_and_should_update() {
        let mut s = make_scheduler();
        let id = Uuid::new_v4();
        s.register(id, Vec3::ZERO, Vec3::ZERO, false);
        let active = s.tick();
        assert!(active.contains(&id), "entity should be active on first tick");
    }

    #[test]
    fn test_player_always_critical() {
        let mut s = make_scheduler();
        let id = Uuid::new_v4();
        s.register(id, Vec3::ZERO, Vec3::ZERO, true);
        s.tick();
        assert_eq!(s.get_tier(&id), Some(FrequencyTier::Critical));
    }

    #[test]
    fn test_distant_entity_background() {
        let mut s = make_scheduler();
        let id = Uuid::new_v4();
        s.register(id, Vec3::new(150.0, 0.0, 0.0), Vec3::ZERO, false);
        s.update_player_position(Vec3::ZERO);
        for _ in 0..20 {
            s.tick();
        }
        let tier = s.get_tier(&id).unwrap();
        assert!(tier >= FrequencyTier::Medium);
    }

    #[test]
    fn test_unregister() {
        let mut s = make_scheduler();
        let id = Uuid::new_v4();
        s.register(id, Vec3::ZERO, Vec3::ZERO, false);
        assert_eq!(s.total_entities(), 1);
        s.unregister(&id);
        assert_eq!(s.total_entities(), 0);
    }

    #[test]
    fn test_tier_budget_enforcement() {
        let mut config = SchedulerConfig::default();
        config.tier_budget.critical_max = 2;
        let mut s = FrequencyScheduler::new(config);

        for i in 0..5 {
            let id = Uuid::new_v4();
            s.register(id, Vec3::new(i as f32, 0.0, 0.0), Vec3::ZERO, false);
        }
        s.update_player_position(Vec3::ZERO);

        for schedule in s.schedules.values_mut() {
            schedule.tier = FrequencyTier::Critical;
        }
        s.tier_counts = [5, 0, 0, 0, 0];

        s.rebalance();
        let counts = s.tier_counts();
        assert!(counts[FrequencyTier::Critical as usize] <= 2);
    }

    #[test]
    fn test_interval_spacing() {
        let mut s = make_scheduler();
        let id = Uuid::new_v4();
        s.register(id, Vec3::ZERO, Vec3::ZERO, false);

        let active_t0 = s.tick();
        assert!(active_t0.contains(&id));

        let active_t1 = s.tick();
        assert!(!active_t1.contains(&id));
    }

    #[test]
    fn test_hysteresis_prevents_flapping() {
        let mut s = make_scheduler();
        let id = Uuid::new_v4();
        s.register(id, Vec3::new(20.0, 0.0, 0.0), Vec3::ZERO, false);
        s.update_player_position(Vec3::ZERO);

        for _ in 0..5 {
            s.tick();
        }
        let tier1 = s.get_tier(&id).unwrap();

        for _ in 0..20 {
            s.tick();
        }
        let tier2 = s.get_tier(&id).unwrap();

        assert_eq!(tier1, tier2);
    }
}
