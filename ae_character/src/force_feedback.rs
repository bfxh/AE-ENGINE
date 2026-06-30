use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackEventType {
    SurfaceSlide,
    MicroVibration,
    BreakThrough,
    Stuck,
    ElasticRebound,
    ConstraintBreak,
    TextureContact,
    EdgeContact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceFeedbackEvent {
    pub timestamp: f32,
    pub position: Vec3,
    pub force_vector: Vec3,
    pub event_type: FeedbackEventType,
    pub intensity: f32,
    pub frequency: f32,
    pub bone_id: Uuid,
    pub duration: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceFeedbackBus {
    pub events: Vec<ForceFeedbackEvent>,
    pub max_events: usize,
    pub active_sliding: bool,
    pub sliding_force: f32,
    pub sliding_frequency: f32,
    pub last_breakthrough: Option<f32>,
    pub stuck_force: f32,
}

impl Default for ForceFeedbackBus {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            max_events: 64,
            active_sliding: false,
            sliding_force: 0.0,
            sliding_frequency: 0.0,
            last_breakthrough: None,
            stuck_force: 0.0,
        }
    }
}

impl ForceFeedbackBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit_surface_slide(
        &mut self,
        bone_id: Uuid,
        position: Vec3,
        velocity: Vec3,
        friction: f32,
        roughness: f32,
        force_magnitude: f32,
        time: f32,
    ) {
        let speed = velocity.length();
        let frequency = roughness * 200.0 + speed * 50.0;
        let intensity = (friction * force_magnitude * 0.1).min(1.0);

        self.active_sliding = intensity > 0.01;
        self.sliding_force = intensity;
        self.sliding_frequency = frequency;

        self.events.push(ForceFeedbackEvent {
            timestamp: time,
            position,
            force_vector: velocity.normalize_or_zero() * force_magnitude * friction,
            event_type: FeedbackEventType::SurfaceSlide,
            intensity,
            frequency,
            bone_id,
            duration: 0.016,
        });
        self.prune();
    }

    pub fn emit_break_through(
        &mut self,
        bone_id: Uuid,
        position: Vec3,
        penetration_normal: Vec3,
        force_before: f32,
        force_after: f32,
        time: f32,
    ) {
        let force_drop = force_before - force_after;
        let intensity = (force_drop / force_before.max(0.01)).clamp(0.1, 1.0);

        self.last_breakthrough = Some(time);
        self.active_sliding = false;

        self.events.push(ForceFeedbackEvent {
            timestamp: time,
            position,
            force_vector: penetration_normal * force_drop,
            event_type: FeedbackEventType::BreakThrough,
            intensity,
            frequency: 30.0,
            bone_id,
            duration: 0.05,
        });
        self.prune();
    }

    pub fn emit_stuck(
        &mut self,
        bone_id: Uuid,
        position: Vec3,
        constraint_normal: Vec3,
        force_magnitude: f32,
        time: f32,
    ) {
        let intensity = (force_magnitude * 0.01).min(1.0);

        self.stuck_force = force_magnitude;

        self.events.push(ForceFeedbackEvent {
            timestamp: time,
            position,
            force_vector: constraint_normal * force_magnitude,
            event_type: FeedbackEventType::Stuck,
            intensity,
            frequency: 10.0,
            bone_id,
            duration: 0.1,
        });
        self.prune();
    }

    pub fn emit_constraint_break(
        &mut self,
        bone_id: Uuid,
        position: Vec3,
        break_direction: Vec3,
        stored_energy: f32,
        time: f32,
    ) {
        let intensity = (stored_energy * 0.001).clamp(0.1, 1.0);

        self.events.push(ForceFeedbackEvent {
            timestamp: time,
            position,
            force_vector: break_direction * stored_energy,
            event_type: FeedbackEventType::ConstraintBreak,
            intensity,
            frequency: 60.0,
            bone_id,
            duration: 0.03,
        });
        self.prune();
    }

    pub fn emit_micro_vibration(
        &mut self,
        bone_id: Uuid,
        position: Vec3,
        roughness: f32,
        speed: f32,
        time: f32,
    ) {
        let frequency = roughness * 300.0 + speed * 100.0;
        let intensity = (roughness * speed * 0.5).min(1.0);

        self.events.push(ForceFeedbackEvent {
            timestamp: time,
            position,
            force_vector: Vec3::ZERO,
            event_type: FeedbackEventType::MicroVibration,
            intensity,
            frequency,
            bone_id,
            duration: 0.008,
        });
        self.prune();
    }

    pub fn emit_texture_contact(
        &mut self,
        bone_id: Uuid,
        position: Vec3,
        texture_direction: Vec3,
        bump_density: f32,
        bump_height: f32,
        time: f32,
    ) {
        let frequency = bump_density * 50.0;
        let intensity = (bump_height * 100.0).min(1.0);

        self.events.push(ForceFeedbackEvent {
            timestamp: time,
            position,
            force_vector: texture_direction * intensity,
            event_type: FeedbackEventType::TextureContact,
            intensity,
            frequency,
            bone_id,
            duration: 0.016,
        });
        self.prune();
    }

    pub fn get_active_events(&self, current_time: f32) -> Vec<&ForceFeedbackEvent> {
        self.events.iter().filter(|e| current_time - e.timestamp < e.duration + 0.01).collect()
    }

    pub fn get_combined_force(&self, current_time: f32) -> Vec3 {
        self.get_active_events(current_time)
            .iter()
            .map(|e| e.force_vector * e.intensity)
            .fold(Vec3::ZERO, |acc, f| acc + f)
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.active_sliding = false;
        self.sliding_force = 0.0;
        self.sliding_frequency = 0.0;
        self.last_breakthrough = None;
        self.stuck_force = 0.0;
    }

    fn prune(&mut self) {
        if self.events.len() > self.max_events {
            let remove = self.events.len() - self.max_events;
            self.events.drain(0..remove);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_surface_slide() {
        let mut bus = ForceFeedbackBus::new();
        bus.emit_surface_slide(Uuid::new_v4(), Vec3::ZERO, Vec3::X * 5.0, 0.5, 0.3, 100.0, 0.0);
        assert!(bus.active_sliding);
        assert!(bus.sliding_force > 0.0);
    }

    #[test]
    fn test_break_through() {
        let mut bus = ForceFeedbackBus::new();
        bus.emit_break_through(Uuid::new_v4(), Vec3::ZERO, Vec3::Y, 1000.0, 200.0, 0.0);
        assert!(bus.last_breakthrough.is_some());
        let active = bus.get_active_events(0.01);
        assert!(!active.is_empty());
    }

    #[test]
    fn test_combined_force() {
        let mut bus = ForceFeedbackBus::new();
        bus.emit_surface_slide(Uuid::new_v4(), Vec3::ZERO, Vec3::X * 5.0, 0.5, 0.3, 100.0, 0.0);
        bus.emit_break_through(
            Uuid::new_v4(),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::Y,
            1000.0,
            200.0,
            0.0,
        );
        let force = bus.get_combined_force(0.01);
        assert!(force.length() > 0.0);
    }
}
