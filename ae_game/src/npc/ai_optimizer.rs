use serde::{Deserialize, Serialize};
use super::NpcId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcAiConfig {
    pub combat_think_interval: f32,
    pub idle_think_interval: f32,
    pub patrol_think_interval: f32,
    pub far_combat_think_interval: f32,
    pub perception_update_interval: f32,
    pub far_perception_interval: f32,
    pub max_perception_targets: usize,
    pub far_npc_simplified: bool,
    pub script_driven_idle: bool,
    pub combat_accuracy_bonus: f32,
    pub path_recalc_distance: f32,
    pub far_path_recalc_distance: f32,
}

impl Default for NpcAiConfig {
    fn default() -> Self {
        NpcAiConfig {
            combat_think_interval: 0.05,
            idle_think_interval: 0.5,
            patrol_think_interval: 0.3,
            far_combat_think_interval: 0.15,
            perception_update_interval: 0.1,
            far_perception_interval: 0.5,
            max_perception_targets: 10,
            far_npc_simplified: true,
            script_driven_idle: true,
            combat_accuracy_bonus: 0.2,
            path_recalc_distance: 5.0,
            far_path_recalc_distance: 20.0,
        }
    }
}

/// NPC AI 节流器：基于距离的 LOD 思考/感知频率控制。
///
/// 接入点：NpcManager::step() 每帧调用 should_perceive / should_think。
/// 计时器用 dt（秒）累积，与帧率解耦。
pub struct NpcAiOptimizer {
    pub config: NpcAiConfig,
    pub npc_think_timers: std::collections::HashMap<NpcId, f32>,
    pub npc_perception_timers: std::collections::HashMap<NpcId, f32>,
}

impl NpcAiOptimizer {
    pub fn new(config: NpcAiConfig) -> Self {
        NpcAiOptimizer {
            config,
            npc_think_timers: std::collections::HashMap::new(),
            npc_perception_timers: std::collections::HashMap::new(),
        }
    }

    /// 是否本帧执行思考（状态机决策）。
    /// `dt` 为帧时间（秒），`in_combat` 是否战斗中，`distance_to_player` 用于 LOD 分档。
    pub fn should_think(&mut self, npc_id: NpcId, dt: f32, in_combat: bool, distance_to_player: f32) -> bool {
        let timer = self.npc_think_timers.entry(npc_id).or_insert(0.0);
        *timer -= dt;

        let interval = if in_combat {
            if distance_to_player > 50.0 {
                self.config.far_combat_think_interval
            } else {
                self.config.combat_think_interval
            }
        } else if distance_to_player > 100.0 {
            self.config.patrol_think_interval * 2.0
        } else {
            self.config.idle_think_interval
        };

        if *timer <= 0.0 {
            *timer = interval;
            true
        } else {
            false
        }
    }

    /// 是否本帧执行感知扫描。
    pub fn should_perceive(&mut self, npc_id: NpcId, dt: f32, distance_to_player: f32) -> bool {
        let timer = self.npc_perception_timers.entry(npc_id).or_insert(0.0);
        *timer -= dt;

        let interval = if distance_to_player > 80.0 {
            self.config.far_perception_interval
        } else {
            self.config.perception_update_interval
        };

        if *timer <= 0.0 {
            *timer = interval;
            true
        } else {
            false
        }
    }

    pub fn get_combat_quality(&self, distance_to_player: f32) -> CombatQuality {
        if distance_to_player < 30.0 {
            CombatQuality::Full
        } else if distance_to_player < 80.0 {
            CombatQuality::High
        } else if distance_to_player < 150.0 {
            CombatQuality::Medium
        } else {
            CombatQuality::Low
        }
    }

    pub fn remove_npc(&mut self, npc_id: NpcId) {
        self.npc_think_timers.remove(&npc_id);
        self.npc_perception_timers.remove(&npc_id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CombatQuality {
    Full,
    High,
    Medium,
    Low,
}

impl CombatQuality {
    pub fn perception_range_multiplier(&self) -> f32 {
        match self {
            CombatQuality::Full => 1.0,
            CombatQuality::High => 0.9,
            CombatQuality::Medium => 0.7,
            CombatQuality::Low => 0.5,
        }
    }

    pub fn reaction_time_multiplier(&self) -> f32 {
        match self {
            CombatQuality::Full => 1.0,
            CombatQuality::High => 1.0,
            CombatQuality::Medium => 1.5,
            CombatQuality::Low => 2.0,
        }
    }

    pub fn accuracy_multiplier(&self) -> f32 {
        match self {
            CombatQuality::Full => 1.0,
            CombatQuality::High => 0.95,
            CombatQuality::Medium => 0.8,
            CombatQuality::Low => 0.6,
        }
    }

    pub fn should_use_script(&self) -> bool {
        matches!(self, CombatQuality::Medium | CombatQuality::Low)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::manager::NpcManager;
    use glam::Vec3;

    #[test]
    fn think_throttle_near_player() {
        let mut opt = NpcAiOptimizer::new(NpcAiConfig::default());
        let mut mgr = NpcManager::new(10);
        let id = mgr.spawn_combatant(Vec3::ZERO, 1);
        // 战斗中近距离应频繁思考（0.05s 间隔）
        assert!(opt.should_think(id, 0.016, true, 10.0));
        // 紧接第二帧不应再思考（timer 刚重置）
        assert!(!opt.should_think(id, 0.016, true, 10.0));
    }

    #[test]
    fn perceive_throttle_far_from_player() {
        let mut opt = NpcAiOptimizer::new(NpcAiConfig::default());
        let mut mgr = NpcManager::new(10);
        let id = mgr.spawn_combatant(Vec3::new(200.0, 0.0, 0.0), 1);
        // 远距离感知间隔 0.5s
        assert!(opt.should_perceive(id, 0.016, 200.0));
        assert!(!opt.should_perceive(id, 0.016, 200.0));
    }

    #[test]
    fn combat_quality_lod() {
        let opt = NpcAiOptimizer::new(NpcAiConfig::default());
        assert_eq!(opt.get_combat_quality(10.0), CombatQuality::Full);
        assert_eq!(opt.get_combat_quality(50.0), CombatQuality::High);
        assert_eq!(opt.get_combat_quality(100.0), CombatQuality::Medium);
        assert_eq!(opt.get_combat_quality(200.0), CombatQuality::Low);
    }

    #[test]
    fn remove_npc_clears_timers() {
        let mut opt = NpcAiOptimizer::new(NpcAiConfig::default());
        let mut mgr = NpcManager::new(10);
        let id = mgr.spawn_combatant(Vec3::ZERO, 1);
        opt.should_think(id, 0.016, false, 10.0);
        opt.should_perceive(id, 0.016, 10.0);
        opt.remove_npc(id);
        assert!(opt.npc_think_timers.is_empty());
        assert!(opt.npc_perception_timers.is_empty());
    }
}
