use crate::fixed_point::FixedPoint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicsTier {
    CCD,
    Discrete,
    Simplified,
    Sleeping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChemistryTier {
    FullDerivation,
    CachedLookup,
    Simplified,
    Dormant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiologyTier {
    FullSimulation,
    MacroUpdate,
    Frozen,
}

#[derive(Debug, Clone)]
pub struct PerformanceTierManager {
    pub physics_tiers: Vec<PhysicsTier>,
    pub chemistry_tier: ChemistryTier,
    pub biology_tier: BiologyTier,
    pub frame_budget_us: u64,
    pub current_frame_us: u64,
    pub auto_scale: bool,
    pub target_frame_time_us: u64,
    pub thermal_throttle: bool,
    pub gpu_budget_percent: f32,
}

impl PerformanceTierManager {
    pub fn new(target_fps: u32) -> Self {
        let target_frame_time_us = 1_000_000 / target_fps as u64;
        Self {
            physics_tiers: Vec::new(),
            chemistry_tier: ChemistryTier::FullDerivation,
            biology_tier: BiologyTier::FullSimulation,
            frame_budget_us: target_frame_time_us,
            current_frame_us: 0,
            auto_scale: true,
            target_frame_time_us,
            thermal_throttle: false,
            gpu_budget_percent: 0.8,
        }
    }

    pub fn assign_physics_tier(
        &mut self,
        priority: f32,
        distance_to_camera: FixedPoint,
    ) -> PhysicsTier {
        let tier = if priority > 0.9 {
            PhysicsTier::CCD
        } else if priority > 0.5 || distance_to_camera < FixedPoint::from_f32(10.0) {
            PhysicsTier::Discrete
        } else if distance_to_camera < FixedPoint::from_f32(50.0) {
            PhysicsTier::Simplified
        } else {
            PhysicsTier::Sleeping
        };
        self.physics_tiers.push(tier);
        tier
    }

    pub fn update_chemistry_tier(&mut self, active_reactions: usize) {
        self.chemistry_tier = if active_reactions > 100 {
            ChemistryTier::Simplified
        } else if active_reactions > 10 {
            ChemistryTier::CachedLookup
        } else {
            ChemistryTier::FullDerivation
        };
    }

    pub fn update_biology_tier(&mut self, organism_count: usize, distance_to_player: FixedPoint) {
        self.biology_tier =
            if organism_count > 100 || distance_to_player > FixedPoint::from_f32(200.0) {
                BiologyTier::MacroUpdate
            } else {
                BiologyTier::FullSimulation
            };
    }

    pub fn should_step_physics(&self, tier: PhysicsTier, frame: u64) -> bool {
        match tier {
            PhysicsTier::CCD => true,
            PhysicsTier::Discrete => true,
            PhysicsTier::Simplified => frame.is_multiple_of(2),
            PhysicsTier::Sleeping => frame.is_multiple_of(30),
        }
    }

    pub fn should_step_chemistry(&self, frame: u64) -> bool {
        match self.chemistry_tier {
            ChemistryTier::FullDerivation => true,
            ChemistryTier::CachedLookup => true,
            ChemistryTier::Simplified => frame.is_multiple_of(5),
            ChemistryTier::Dormant => frame.is_multiple_of(60),
        }
    }

    pub fn should_step_biology(&self, frame: u64) -> bool {
        match self.biology_tier {
            BiologyTier::FullSimulation => true,
            BiologyTier::MacroUpdate => frame.is_multiple_of(30),
            BiologyTier::Frozen => false,
        }
    }

    pub fn scale_down_if_needed(&mut self) {
        if !self.auto_scale {
            return;
        }
        if self.current_frame_us > self.target_frame_time_us {
            match self.chemistry_tier {
                ChemistryTier::FullDerivation => self.chemistry_tier = ChemistryTier::CachedLookup,
                ChemistryTier::CachedLookup => self.chemistry_tier = ChemistryTier::Simplified,
                _ => {},
            }
        }
    }

    pub fn report_frame_time(&mut self, us: u64) {
        self.current_frame_us = us;
        self.scale_down_if_needed();
    }

    pub fn budget_remaining_us(&self) -> u64 {
        self.frame_budget_us.saturating_sub(self.current_frame_us)
    }

    pub fn physics_performance_ratio(&self) -> f32 {
        let active = self.physics_tiers.iter().filter(|t| **t != PhysicsTier::Sleeping).count();
        let total = self.physics_tiers.len().max(1);
        active as f32 / total as f32
    }
}

impl Default for PerformanceTierManager {
    fn default() -> Self {
        Self::new(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_assignment() {
        let mut mgr = PerformanceTierManager::new(60);
        let tier = mgr.assign_physics_tier(1.0, FixedPoint::from_f32(5.0));
        assert_eq!(tier, PhysicsTier::CCD);

        let tier = mgr.assign_physics_tier(0.3, FixedPoint::from_f32(100.0));
        assert_eq!(tier, PhysicsTier::Sleeping);
    }

    #[test]
    fn test_auto_scale() {
        let mut mgr = PerformanceTierManager::new(60);
        assert_eq!(mgr.chemistry_tier, ChemistryTier::FullDerivation);
        mgr.report_frame_time(20_000);
        assert_eq!(mgr.chemistry_tier, ChemistryTier::CachedLookup);
    }
}
