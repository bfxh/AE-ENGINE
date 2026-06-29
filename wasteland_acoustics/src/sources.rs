use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SourceType {
    Point,
    Directional,
    Planar,
    Line,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundSource {
    pub id: u64,
    pub source_type: SourceType,
    pub position: Vec3,
    pub direction: Vec3,
    pub frequency: f32,
    pub amplitude: f32,
    pub phase: f32,
    pub active: bool,
    pub time: f32,
    pub directivity: f32,
}

impl SoundSource {
    pub fn new_point(id: u64, position: Vec3, frequency: f32, amplitude: f32) -> Self {
        Self {
            id,
            source_type: SourceType::Point,
            position,
            direction: Vec3::Y,
            frequency,
            amplitude,
            phase: 0.0,
            active: true,
            time: 0.0,
            directivity: 0.0,
        }
    }

    pub fn new_directional(
        id: u64,
        position: Vec3,
        direction: Vec3,
        frequency: f32,
        amplitude: f32,
    ) -> Self {
        Self {
            id,
            source_type: SourceType::Directional,
            position,
            direction: direction.normalize(),
            frequency,
            amplitude,
            phase: 0.0,
            active: true,
            time: 0.0,
            directivity: 0.5,
        }
    }

    pub fn pressure_at(&self, listener_pos: Vec3, time: f32) -> f32 {
        let rel_pos = listener_pos - self.position;
        let distance = rel_pos.length();
        if distance < 0.01 {
            return 0.0;
        }
        let dir_factor = match self.source_type {
            SourceType::Point => 1.0,
            SourceType::Directional => {
                let to_listener = rel_pos.normalize();
                let cos_angle = to_listener.dot(self.direction).clamp(-1.0, 1.0);
                (1.0 - self.directivity) + self.directivity * cos_angle * cos_angle
            },
            SourceType::Planar => {
                let to_listener = rel_pos.normalize();
                let dot = to_listener.dot(self.direction).clamp(-1.0, 1.0);
                dot.max(0.0)
            },
            SourceType::Line => 1.0 / distance.sqrt(),
        };
        let distance_attenuation = 1.0 / distance;
        let angular_freq = 2.0 * std::f32::consts::PI * self.frequency;
        let wave_number = angular_freq / 343.0;
        let phase_delay = wave_number * distance;
        let total_phase = angular_freq * time - phase_delay + self.phase;
        self.amplitude * dir_factor * distance_attenuation * total_phase.sin()
    }

    pub fn step(&mut self, dt: f32) {
        self.time += dt;
    }
}
