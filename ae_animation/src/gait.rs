use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub enum GaitType {
    Walk,
    Run,
    Sneak,
    Limp,
}

impl GaitType {
    pub fn stride_length(&self) -> f32 {
        match self {
            GaitType::Walk => 0.8,
            GaitType::Run => 1.5,
            GaitType::Sneak => 0.4,
            GaitType::Limp => 0.5,
        }
    }

    pub fn step_height(&self) -> f32 {
        match self {
            GaitType::Walk => 0.15,
            GaitType::Run => 0.3,
            GaitType::Sneak => 0.05,
            GaitType::Limp => 0.1,
        }
    }

    pub fn cycle_duration(&self) -> f32 {
        match self {
            GaitType::Walk => 1.0,
            GaitType::Run => 0.4,
            GaitType::Sneak => 2.0,
            GaitType::Limp => 1.5,
        }
    }

    pub fn duty_factor(&self) -> f32 {
        match self {
            GaitType::Walk => 0.6,
            GaitType::Run => 0.3,
            GaitType::Sneak => 0.7,
            GaitType::Limp => 0.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GaitController {
    pub gait: GaitType,
    pub phase: f32,
    pub speed: f32,
    pub leg_positions: Vec<Vec3>,
}

impl GaitController {
    pub fn new(gait: GaitType, leg_count: usize) -> Self {
        Self { gait, phase: 0.0, speed: 1.0, leg_positions: vec![Vec3::ZERO; leg_count] }
    }

    pub fn update(&mut self, dt: f32) {
        let duration = self.gait.cycle_duration();
        self.phase = (self.phase + dt * self.speed / duration) % 1.0;
    }

    pub fn leg_phase(&self, leg_index: usize, leg_count: usize) -> f32 {
        if leg_count <= 1 {
            return self.phase;
        }
        let offset = leg_index as f32 / leg_count as f32;
        (self.phase + offset) % 1.0
    }

    pub fn is_leg_stance(&self, leg_index: usize, leg_count: usize) -> bool {
        let p = self.leg_phase(leg_index, leg_count);
        p < self.gait.duty_factor()
    }

    pub fn foot_position(
        &self,
        leg_index: usize,
        leg_count: usize,
        rest_position: Vec3,
        direction: Vec3,
    ) -> Vec3 {
        let p = self.leg_phase(leg_index, leg_count);
        let dir = direction.normalize_or_zero();
        let stride = self.gait.stride_length();

        if self.is_leg_stance(leg_index, leg_count) {
            let stance_phase = p / self.gait.duty_factor();
            rest_position - dir * stride * (stance_phase - 0.5) * 2.0
        } else {
            let swing_phase = (p - self.gait.duty_factor()) / (1.0 - self.gait.duty_factor());
            let swing_forward = rest_position + dir * stride * 0.5 * (1.0 - swing_phase * 2.0);
            let step_h = self.gait.step_height() * (swing_phase * std::f32::consts::PI).sin();
            swing_forward + Vec3::Y * step_h
        }
    }

    pub fn spine_sway(&self) -> f32 {
        (self.phase * std::f32::consts::TAU).sin() * 0.05
    }

    pub fn hip_height_offset(&self) -> f32 {
        let stance_factor = if matches!(self.gait, GaitType::Run) { 0.05 } else { 0.02 };
        -(self.phase * std::f32::consts::TAU * 2.0).sin().abs() * stance_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gait_update() {
        let mut gait = GaitController::new(GaitType::Walk, 4);
        gait.speed = 1.0;
        let initial_phase = gait.phase;
        gait.update(0.25);
        assert!(gait.phase != initial_phase);
    }

    #[test]
    fn test_leg_phase_distribution() {
        let gait = GaitController::new(GaitType::Walk, 4);
        let p0 = gait.leg_phase(0, 4);
        let p1 = gait.leg_phase(1, 4);
        let p2 = gait.leg_phase(2, 4);
        let p3 = gait.leg_phase(3, 4);
        assert!(p0 < p1);
        assert!(p1 < p2);
        assert!(p2 < p3);
    }

    #[test]
    fn test_stance_swing() {
        let gait = GaitController::new(GaitType::Walk, 4);
        let stance = gait.is_leg_stance(0, 4);
        assert!(stance);
    }

    #[test]
    fn test_foot_position_movement() {
        let gait = GaitController::new(GaitType::Walk, 4);
        let pos = gait.foot_position(0, 4, Vec3::ZERO, Vec3::X);
        assert!(pos.x != 0.0 || pos.y != 0.0);
    }
}
