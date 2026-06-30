use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atmosphere {
    pub temperature: f32,
    pub humidity: f32,
    pub pressure: f32,
    pub density: f32,
    pub lapse_rate: f32,
    pub stability_class: StabilityClass,
    pub boundary_layer_height: f32,
    pub visibility: f32,
    pub pollution: f32,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            temperature: 288.15,
            humidity: 0.5,
            pressure: 101325.0,
            density: 1.225,
            lapse_rate: 0.0065,
            stability_class: StabilityClass::Neutral,
            boundary_layer_height: 1000.0,
            visibility: 10000.0,
            pollution: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StabilityClass {
    VeryUnstable,
    Unstable,
    SlightlyUnstable,
    Neutral,
    SlightlyStable,
    Stable,
    VeryStable,
}

impl StabilityClass {
    pub fn from_lapse_rate(lapse_rate: f32) -> Self {
        if lapse_rate > 0.02 {
            StabilityClass::VeryUnstable
        } else if lapse_rate > 0.015 {
            StabilityClass::Unstable
        } else if lapse_rate > 0.011 {
            StabilityClass::SlightlyUnstable
        } else if lapse_rate > 0.008 {
            StabilityClass::Neutral
        } else if lapse_rate > 0.005 {
            StabilityClass::SlightlyStable
        } else if lapse_rate > 0.001 {
            StabilityClass::Stable
        } else {
            StabilityClass::VeryStable
        }
    }

    pub fn dispersion_coefficient(&self) -> f32 {
        match self {
            StabilityClass::VeryUnstable => 0.5,
            StabilityClass::Unstable => 0.3,
            StabilityClass::SlightlyUnstable => 0.15,
            StabilityClass::Neutral => 0.08,
            StabilityClass::SlightlyStable => 0.04,
            StabilityClass::Stable => 0.02,
            StabilityClass::VeryStable => 0.01,
        }
    }
}

impl Atmosphere {
    const GAS_CONSTANT: f32 = 287.058;
    const SEA_LEVEL_PRESSURE: f32 = 101325.0;
    const SEA_LEVEL_TEMP: f32 = 288.15;
    const GRAVITY: f32 = 9.80665;
    const DRY_ADIABATIC_LAPSE: f32 = 0.0098;
    const MOIST_ADIABATIC_LAPSE: f32 = 0.0050;

    pub fn pressure_at_altitude(altitude: f32) -> f32 {
        Self::SEA_LEVEL_PRESSURE
            * (1.0 - Self::DRY_ADIABATIC_LAPSE * altitude / Self::SEA_LEVEL_TEMP)
                .powf(Self::GRAVITY / (Self::GAS_CONSTANT * Self::DRY_ADIABATIC_LAPSE))
    }

    pub fn temperature_at_altitude(altitude: f32) -> f32 {
        Self::SEA_LEVEL_TEMP - Self::DRY_ADIABATIC_LAPSE * altitude
    }

    pub fn density_from_pressure_temperature(pressure: f32, temperature: f32) -> f32 {
        pressure / (Self::GAS_CONSTANT * temperature)
    }

    pub fn saturation_vapor_pressure(temperature: f32) -> f32 {
        611.2 * ((17.67 * (temperature - 273.15)) / (temperature - 29.65)).exp()
    }

    pub fn relative_humidity(mixing_ratio: f32, pressure: f32, temperature: f32) -> f32 {
        let es = Self::saturation_vapor_pressure(temperature);
        let e = mixing_ratio * pressure / 0.622;
        (e / es).clamp(0.0, 1.0)
    }

    pub fn dew_point(temperature: f32, humidity: f32) -> f32 {
        let es = Self::saturation_vapor_pressure(temperature);
        let e = humidity * es;
        let ln_e = (e / 611.2).ln();
        (243.5 * ln_e) / (17.67 - ln_e) + 273.15
    }

    pub fn update(&mut self, solar_radiation: f32, surface_albedo: f32, dt: f32) {
        let net_heating = solar_radiation * (1.0 - surface_albedo) * 0.3;
        self.temperature += net_heating * dt;
        self.temperature -= (self.temperature - Self::SEA_LEVEL_TEMP) * 0.5 * dt; // Phase 6 fix: increased from 0.0001

        let evap_rate = solar_radiation * self.humidity * 0.01;
        self.humidity += evap_rate * dt * (1.0 - self.humidity);
        self.humidity = self.humidity.clamp(0.0, 1.0);

        self.density = Self::density_from_pressure_temperature(self.pressure, self.temperature);

        let actual_lapse = if self.humidity > 0.8 {
            Self::MOIST_ADIABATIC_LAPSE
        } else {
            Self::DRY_ADIABATIC_LAPSE
        };
        self.lapse_rate = actual_lapse;
        self.stability_class = StabilityClass::from_lapse_rate(self.lapse_rate);

        let visibility_decay = self.pollution * 0.001 + 0.0001;
        self.visibility = (self.visibility + 100.0 * dt).min(50000.0);
        self.visibility -= visibility_decay * self.visibility * dt;
        self.visibility = self.visibility.max(10.0);
    }

    pub fn wind_pressure_force(&self, velocity: Vec3, area: f32, drag_coefficient: f32) -> Vec3 {
        let dynamic_pressure = 0.5 * self.density * velocity.length_squared();
        let force_magnitude = dynamic_pressure * area * drag_coefficient;
        if velocity.length() < 0.01 { Vec3::ZERO } else { velocity.normalize() * force_magnitude }
    }

    pub fn heat_transfer_coefficient(&self, wind_speed: f32) -> f32 {
        5.7 + 3.8 * wind_speed
    }

    pub fn sound_speed(&self) -> f32 {
        const GAMMA: f32 = 1.4;
        (GAMMA * Self::GAS_CONSTANT * self.temperature).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atmosphere_creation() {
        let atm = Atmosphere::default();
        assert_eq!(atm.temperature, 288.15);
        assert_eq!(atm.pressure, 101325.0);
        assert_eq!(atm.humidity, 0.5);
        assert_eq!(atm.density, 1.225);
    }

    #[test]
    fn test_atmosphere_physics() {
        let p = Atmosphere::pressure_at_altitude(1000.0);
        assert!(p < 101325.0);
        let t = Atmosphere::temperature_at_altitude(1000.0);
        assert!(t < 288.15);
        let rho = Atmosphere::density_from_pressure_temperature(p, t);
        assert!(rho > 0.0);
    }

    #[test]
    fn test_atmosphere_update() {
        let mut atm = Atmosphere::default();
        atm.update(500.0, 0.3, 1.0);
        assert!(atm.temperature > 288.15);
        assert!(atm.sound_speed() > 300.0);
    }
}
