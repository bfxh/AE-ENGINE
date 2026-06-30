use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceType {
    Point,
    Line,
    Plane,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropagationMode {
    Air,
    Solid,
    Liquid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DirectivityPattern {
    Omnidirectional,
    Cardioid,
    Hypercardioid,
    Figure8,
    Shotgun,
}

#[derive(Debug, Clone)]
pub struct SoundSource {
    pub position: Vec3,
    pub velocity: Vec3,
    pub volume_db: f32,
    pub frequency_range: (f32, f32),
    pub directivity: DirectivityPattern,
    pub propagation: PropagationMode,
    pub source_type: SourceType,
    pub radiation: [f32; 36],
    pub priority: SourcePriority,
    pub max_distance: f32,
    pub occlusion: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourcePriority {
    Critical = 0,
    High = 1,
    Medium = 2,
    Low = 3,
    Ambient = 4,
}

impl Default for SoundSource {
    fn default() -> Self {
        SoundSource {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            volume_db: 0.0,
            frequency_range: (20.0, 20000.0),
            directivity: DirectivityPattern::Omnidirectional,
            propagation: PropagationMode::Air,
            source_type: SourceType::Point,
            radiation: [1.0; 36],
            priority: SourcePriority::Medium,
            max_distance: 100.0,
            occlusion: 0.0,
        }
    }
}

impl SoundSource {
    pub fn new_point(position: Vec3, volume_db: f32) -> Self {
        SoundSource { position, volume_db, source_type: SourceType::Point, ..Default::default() }
    }

    pub fn with_directivity(mut self, pattern: DirectivityPattern) -> Self {
        self.directivity = pattern;
        self
    }

    pub fn with_propagation(mut self, mode: PropagationMode) -> Self {
        self.propagation = mode;
        self
    }

    pub fn with_frequency_range(mut self, low: f32, high: f32) -> Self {
        self.frequency_range = (low, high);
        self
    }

    pub fn with_priority(mut self, priority: SourcePriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn attenuation_at(&self, listener_pos: Vec3) -> f32 {
        let distance = self.position.distance(listener_pos);
        if distance < 0.01 {
            return 1.0;
        }
        if distance > self.max_distance {
            return 0.0;
        }

        let base = match self.source_type {
            SourceType::Point => 1.0 / (distance * distance),
            SourceType::Line => 1.0 / distance,
            SourceType::Plane => 1.0,
        };

        let directivity = self.directivity_factor(listener_pos);
        let occlusion = 1.0 - self.occlusion;
        let air_absorption = (-0.001 * distance).exp();

        base * directivity * occlusion * air_absorption
    }

    pub fn doppler_shift(
        &self,
        listener_pos: Vec3,
        listener_vel: Vec3,
        speed_of_sound: f32,
    ) -> f32 {
        let to_listener = (listener_pos - self.position).normalize_or_zero();
        if to_listener == Vec3::ZERO {
            return 1.0;
        }
        let source_radial = self.velocity.dot(to_listener);
        let listener_radial = listener_vel.dot(to_listener);
        (speed_of_sound + listener_radial) / (speed_of_sound - source_radial).max(0.01)
    }

    fn directivity_factor(&self, listener_pos: Vec3) -> f32 {
        let to_listener = (listener_pos - self.position).normalize_or_zero();
        if to_listener == Vec3::ZERO {
            return 1.0;
        }

        let cos_theta = to_listener.dot(Vec3::Z);
        match self.directivity {
            DirectivityPattern::Omnidirectional => 1.0,
            DirectivityPattern::Cardioid => 0.5 * (1.0 + cos_theta),
            DirectivityPattern::Hypercardioid => 0.25 * (1.0 + 3.0 * cos_theta),
            DirectivityPattern::Figure8 => cos_theta.abs(),
            DirectivityPattern::Shotgun => {
                let angle = cos_theta.acos();
                if angle < 0.3 {
                    1.0
                } else if angle < 0.6 {
                    0.5
                } else {
                    0.1
                }
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaterialAbsorption {
    pub name: String,
    pub absorption_125hz: f32,
    pub absorption_250hz: f32,
    pub absorption_500hz: f32,
    pub absorption_1000hz: f32,
    pub absorption_2000hz: f32,
    pub absorption_4000hz: f32,
    pub scattering: f32,
}

impl MaterialAbsorption {
    pub fn average(&self) -> f32 {
        (self.absorption_125hz
            + self.absorption_250hz
            + self.absorption_500hz
            + self.absorption_1000hz
            + self.absorption_2000hz
            + self.absorption_4000hz)
            / 6.0
    }

    pub fn at_frequency(&self, freq_hz: f32) -> f32 {
        if freq_hz < 250.0 {
            self.absorption_125hz
        } else if freq_hz < 500.0 {
            self.absorption_250hz
        } else if freq_hz < 1000.0 {
            self.absorption_500hz
        } else if freq_hz < 2000.0 {
            self.absorption_1000hz
        } else if freq_hz < 4000.0 {
            self.absorption_2000hz
        } else {
            self.absorption_4000hz
        }
    }

    pub fn concrete() -> Self {
        MaterialAbsorption {
            name: "concrete".to_string(),
            absorption_125hz: 0.01,
            absorption_250hz: 0.01,
            absorption_500hz: 0.02,
            absorption_1000hz: 0.02,
            absorption_2000hz: 0.02,
            absorption_4000hz: 0.03,
            scattering: 0.05,
        }
    }

    pub fn wood() -> Self {
        MaterialAbsorption {
            name: "wood".to_string(),
            absorption_125hz: 0.15,
            absorption_250hz: 0.11,
            absorption_500hz: 0.10,
            absorption_1000hz: 0.07,
            absorption_2000hz: 0.06,
            absorption_4000hz: 0.07,
            scattering: 0.20,
        }
    }

    pub fn metal() -> Self {
        MaterialAbsorption {
            name: "metal".to_string(),
            absorption_125hz: 0.05,
            absorption_250hz: 0.04,
            absorption_500hz: 0.03,
            absorption_1000hz: 0.03,
            absorption_2000hz: 0.02,
            absorption_4000hz: 0.02,
            scattering: 0.10,
        }
    }

    pub fn fabric() -> Self {
        MaterialAbsorption {
            name: "fabric".to_string(),
            absorption_125hz: 0.14,
            absorption_250hz: 0.35,
            absorption_500hz: 0.55,
            absorption_1000hz: 0.72,
            absorption_2000hz: 0.70,
            absorption_4000hz: 0.65,
            scattering: 0.30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_attenuation() {
        let source = SoundSource::new_point(Vec3::new(0.0, 0.0, 0.0), 0.0);
        let listener = Vec3::new(10.0, 0.0, 0.0);
        let att = source.attenuation_at(listener);
        assert!(att > 0.0);
        assert!(att < 1.0);
    }

    #[test]
    fn test_doppler_approaching() {
        let source = SoundSource {
            position: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 50.0),
            ..Default::default()
        };
        let listener_pos = Vec3::new(0.0, 0.0, 100.0);
        let listener_vel = Vec3::ZERO;
        let shift = source.doppler_shift(listener_pos, listener_vel, 343.0);
        assert!(shift > 1.0);
    }

    #[test]
    fn test_directivity_cardioid() {
        let source =
            SoundSource::new_point(Vec3::ZERO, 0.0).with_directivity(DirectivityPattern::Cardioid);
        let front = source.attenuation_at(Vec3::new(0.0, 0.0, 1.0));
        let back = source.attenuation_at(Vec3::new(0.0, 0.0, -1.0));
        assert!(front > back);
    }

    #[test]
    fn test_material_absorption() {
        let concrete = MaterialAbsorption::concrete();
        let fabric = MaterialAbsorption::fabric();
        assert!(fabric.average() > concrete.average());
    }
}
