use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecipitationType {
    None,
    Drizzle,
    Rain,
    HeavyRain,
    FreezingRain,
    Sleet,
    Snow,
    HeavySnow,
    Hail,
    Graupel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Precipitation {
    pub ptype: PrecipitationType,
    pub intensity: f32,
    pub drop_size: f32,
    pub liquid_equivalent: f32,
}

impl Default for Precipitation {
    fn default() -> Self {
        Self {
            ptype: PrecipitationType::None,
            intensity: 0.0,
            drop_size: 0.0,
            liquid_equivalent: 0.0,
        }
    }
}

impl Precipitation {
    pub fn determine_type(
        temperature: f32,
        humidity: f32,
        cloud_top_temp: f32,
    ) -> PrecipitationType {
        if humidity < 0.6 {
            return PrecipitationType::None;
        }
        if temperature > 0.0 && cloud_top_temp > -10.0 {
            match humidity {
                h if h > 0.95 => PrecipitationType::HeavyRain,
                h if h > 0.8 => PrecipitationType::Rain,
                _ => PrecipitationType::Drizzle,
            }
        } else if temperature < 0.0 && cloud_top_temp < -20.0 {
            match humidity {
                h if h > 0.9 => PrecipitationType::HeavySnow,
                _ => PrecipitationType::Snow,
            }
        } else if temperature > 0.0 && cloud_top_temp < -10.0 {
            PrecipitationType::Hail
        } else {
            PrecipitationType::Sleet
        }
    }

    pub fn rainfall_rate(&self) -> f32 {
        match self.ptype {
            PrecipitationType::None => 0.0,
            PrecipitationType::Drizzle => self.intensity * 0.5,
            PrecipitationType::Rain => self.intensity * 2.0,
            PrecipitationType::HeavyRain => self.intensity * 10.0,
            PrecipitationType::FreezingRain => self.intensity * 1.5,
            PrecipitationType::Sleet => self.intensity * 0.8,
            PrecipitationType::Snow => self.intensity * 0.1,
            PrecipitationType::HeavySnow => self.intensity * 0.5,
            PrecipitationType::Hail => self.intensity * 3.0,
            PrecipitationType::Graupel => self.intensity * 0.3,
        }
    }

    pub fn erosion_power(&self) -> f32 {
        self.rainfall_rate() * self.drop_size * self.drop_size * 0.5
    }

    pub fn visibility_reduction(&self) -> f32 {
        match self.ptype {
            PrecipitationType::None => 0.0,
            PrecipitationType::Drizzle => 0.1,
            PrecipitationType::Rain => 0.3,
            PrecipitationType::HeavyRain => 0.7,
            PrecipitationType::FreezingRain => 0.4,
            PrecipitationType::Sleet => 0.5,
            PrecipitationType::Snow => 0.6,
            PrecipitationType::HeavySnow => 0.9,
            PrecipitationType::Hail => 0.8,
            PrecipitationType::Graupel => 0.5,
        }
    }

    pub fn update(&mut self, temperature: f32, humidity: f32, cloud_top_temp: f32, dt: f32) {
        self.ptype = Self::determine_type(temperature, humidity, cloud_top_temp);
        if self.ptype != PrecipitationType::None {
            self.intensity = (self.intensity + humidity * dt * 0.1).min(1.0);
            self.drop_size = match self.ptype {
                PrecipitationType::Drizzle => 0.0005,
                PrecipitationType::Rain | PrecipitationType::FreezingRain => 0.002,
                PrecipitationType::HeavyRain => 0.004,
                PrecipitationType::Sleet => 0.001,
                PrecipitationType::Snow | PrecipitationType::Graupel => 0.003,
                PrecipitationType::HeavySnow => 0.005,
                PrecipitationType::Hail => 0.015,
                _ => 0.0,
            };
            self.liquid_equivalent += self.rainfall_rate() * dt / 3600.0;
        } else {
            self.intensity = (self.intensity - dt * 0.05).max(0.0);
        }
    }
}
