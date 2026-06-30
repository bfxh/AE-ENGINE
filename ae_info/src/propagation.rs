use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationModel {
    pub signals: Vec<Signal>,
    pub media: Vec<PropagationMedium>,
    pub speed_of_light: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub id: String,
    pub source_position: Vec3,
    pub content: String,
    pub strength: f32,
    pub frequency: f32,
    pub propagation_mode: SignalMode,
    pub timestamp: f32,
    pub priority: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalMode {
    Radio,
    Optical,
    Acoustic,
    Courier,
    Telegraph,
    Smoke,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationMedium {
    pub medium_type: MediumType,
    pub attenuation: f32,
    pub noise: f32,
    pub bandwidth: f32,
    pub range: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediumType {
    Air,
    Vacuum,
    Water,
    Solid,
    FiberOptic,
    CopperWire,
}

impl Signal {
    pub fn new(source_position: Vec3, content: &str, mode: SignalMode, strength: f32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_position,
            content: content.to_string(),
            strength,
            frequency: 1000.0,
            propagation_mode: mode,
            timestamp: 0.0,
            priority: 0,
        }
    }

    pub fn speed_in_medium(&self, medium: &PropagationMedium) -> f32 {
        match self.propagation_mode {
            SignalMode::Radio => match medium.medium_type {
                MediumType::Vacuum => 3.0e8,
                MediumType::Air => 2.99e8,
                _ => 2.0e8,
            },
            SignalMode::Optical => match medium.medium_type {
                MediumType::FiberOptic => 2.0e8,
                MediumType::Air => 3.0e8,
                _ => 1.0e8,
            },
            SignalMode::Acoustic => match medium.medium_type {
                MediumType::Air => 343.0,
                MediumType::Water => 1500.0,
                MediumType::Solid => 5000.0,
                _ => 343.0,
            },
            SignalMode::Courier => 5.0,
            SignalMode::Telegraph => 2.99e8,
            SignalMode::Smoke => 10.0,
        }
    }

    pub fn attenuation_over_distance(&self, distance: f32, medium: &PropagationMedium) -> f32 {
        match self.propagation_mode {
            SignalMode::Radio => {
                self.strength / (4.0 * std::f32::consts::PI * distance * distance).max(1.0)
            },
            SignalMode::Optical => self.strength * (-medium.attenuation * distance).exp(),
            SignalMode::Acoustic => self.strength / distance.max(1.0),
            SignalMode::Courier | SignalMode::Smoke => {
                self.strength * (1.0 - distance / medium.range.max(1.0)).max(0.0)
            },
            SignalMode::Telegraph => self.strength * (-medium.attenuation * distance * 0.001).exp(),
        }
    }

    pub fn can_reach(&self, distance: f32, medium: &PropagationMedium, min_strength: f32) -> bool {
        self.attenuation_over_distance(distance, medium) >= min_strength
    }
}

impl PropagationModel {
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
            media: vec![
                PropagationMedium {
                    medium_type: MediumType::Air,
                    attenuation: 0.01,
                    noise: 0.1,
                    bandwidth: 1000.0,
                    range: 10000.0,
                },
                PropagationMedium {
                    medium_type: MediumType::Vacuum,
                    attenuation: 0.0,
                    noise: 0.0,
                    bandwidth: 100000.0,
                    range: f32::MAX,
                },
                PropagationMedium {
                    medium_type: MediumType::Water,
                    attenuation: 0.5,
                    noise: 0.3,
                    bandwidth: 100.0,
                    range: 1000.0,
                },
                PropagationMedium {
                    medium_type: MediumType::Solid,
                    attenuation: 0.05,
                    noise: 0.2,
                    bandwidth: 500.0,
                    range: 5000.0,
                },
            ],
            speed_of_light: 3.0e8,
        }
    }

    pub fn broadcast(&mut self, signal: Signal) {
        self.signals.push(signal);
    }

    pub fn propagate(&mut self, dt: f32) -> Vec<ReceivedSignal> {
        let received = Vec::new();

        for signal in &mut self.signals {
            signal.timestamp += dt;
            let medium = self.media.iter().find(|m| m.medium_type == MediumType::Air).unwrap();

            let distance =
                signal.source_position.length() + signal.strength * signal.timestamp * 0.1;
            signal.strength = signal.attenuation_over_distance(distance, medium);
        }

        self.signals.retain(|s| s.strength > 0.01);
        received
    }
}

impl Default for PropagationModel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedSignal {
    pub signal_id: String,
    pub content: String,
    pub received_strength: f32,
    pub delay: f32,
    pub distortion: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_signal_creation() {
        let signal = Signal::new(Vec3::new(0.0, 0.0, 0.0), "测试信号", SignalMode::Radio, 1.0);
        assert_eq!(signal.content, "测试信号");
        assert_eq!(signal.propagation_mode, SignalMode::Radio);
        assert_eq!(signal.strength, 1.0);
    }

    #[test]
    fn test_signal_attenuation() {
        let signal = Signal::new(Vec3::ZERO, "测试", SignalMode::Acoustic, 100.0);
        let medium = PropagationMedium {
            medium_type: MediumType::Air,
            attenuation: 0.01,
            noise: 0.1,
            bandwidth: 1000.0,
            range: 10000.0,
        };
        let attenuation = signal.attenuation_over_distance(10.0, &medium);
        assert_eq!(attenuation, 10.0);
        assert!(signal.can_reach(10.0, &medium, 5.0));
        assert!(!signal.can_reach(100.0, &medium, 5.0));
    }

    #[test]
    fn test_propagation_model() {
        let mut model = PropagationModel::new();
        assert_eq!(model.media.len(), 4);
        let signal = Signal::new(Vec3::new(10.0, 0.0, 0.0), "广播", SignalMode::Radio, 1.0);
        model.broadcast(signal);
        assert_eq!(model.signals.len(), 1);
        model.propagate(0.1);
    }
}
