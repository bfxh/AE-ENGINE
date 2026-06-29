use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticMaterial {
    pub name: String,
    pub density: f32,
    pub speed_of_sound: f32,
    pub absorption: f32,
    pub impedance: f32,
    pub reflection: f32,
    pub scattering: f32,
}

impl AcousticMaterial {
    pub fn characteristic_impedance(&self) -> f32 {
        self.density * self.speed_of_sound
    }

    pub fn air() -> Self {
        Self {
            name: "Air".to_string(),
            density: 1.21,
            speed_of_sound: 343.0,
            absorption: 0.01,
            impedance: 415.0,
            reflection: 0.1,
            scattering: 0.0,
        }
    }

    pub fn water() -> Self {
        Self {
            name: "Water".to_string(),
            density: 1000.0,
            speed_of_sound: 1480.0,
            absorption: 0.001,
            impedance: 1.48e6,
            reflection: 0.999,
            scattering: 0.0,
        }
    }

    pub fn steel() -> Self {
        Self {
            name: "Steel".to_string(),
            density: 7850.0,
            speed_of_sound: 5000.0,
            absorption: 0.0001,
            impedance: 3.925e7,
            reflection: 0.9999,
            scattering: 0.05,
        }
    }

    pub fn concrete() -> Self {
        Self {
            name: "Concrete".to_string(),
            density: 2400.0,
            speed_of_sound: 3500.0,
            absorption: 0.05,
            impedance: 8.4e6,
            reflection: 0.95,
            scattering: 0.2,
        }
    }

    pub fn wood() -> Self {
        Self {
            name: "Wood".to_string(),
            density: 600.0,
            speed_of_sound: 3500.0,
            absorption: 0.1,
            impedance: 2.1e6,
            reflection: 0.9,
            scattering: 0.15,
        }
    }

    pub fn fabric() -> Self {
        Self {
            name: "Fabric".to_string(),
            density: 100.0,
            speed_of_sound: 100.0,
            absorption: 0.8,
            impedance: 1.0e4,
            reflection: 0.2,
            scattering: 0.3,
        }
    }

    pub fn metal() -> Self {
        Self {
            name: "Metal".to_string(),
            density: 8000.0,
            speed_of_sound: 5100.0,
            absorption: 0.0005,
            impedance: 4.08e7,
            reflection: 0.9995,
            scattering: 0.08,
        }
    }

    pub fn glass() -> Self {
        Self {
            name: "Glass".to_string(),
            density: 2500.0,
            speed_of_sound: 5200.0,
            absorption: 0.03,
            impedance: 1.3e7,
            reflection: 0.97,
            scattering: 0.1,
        }
    }
}
