use serde::{Deserialize, Serialize};
use glam::Vec3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thunderstorm {
    pub position: Vec3,
    pub radius: f32,
    pub updraft_speed: f32,
    pub downdraft_speed: f32,
    pub cloud_top: f32,
    pub cloud_base: f32,
    pub precipitation_rate: f32,
    pub lightning_rate: f32,
    pub stage: StormStage,
    pub lifetime: f32,
    pub age: f32,
    pub cape: f32,
    pub shear: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StormStage {
    Cumulus,
    Mature,
    Dissipating,
}

impl Thunderstorm {
    pub fn new(position: Vec3, cape: f32, shear: f32) -> Self {
        let updraft = (cape * 2.0).sqrt().min(50.0);
        Self {
            position,
            radius: 5000.0,
            updraft_speed: updraft,
            downdraft_speed: 0.0,
            cloud_top: 8000.0,
            cloud_base: 1500.0,
            precipitation_rate: 0.0,
            lightning_rate: 0.0,
            stage: StormStage::Cumulus,
            lifetime: 3600.0 + cape * 2.0,
            age: 0.0,
            cape,
            shear,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.age += dt;

        match self.stage {
            StormStage::Cumulus => {
                self.cloud_top += self.updraft_speed * dt * 0.5;
                self.cloud_top = self.cloud_top.min(15000.0);
                if self.cloud_top > 10000.0 {
                    self.stage = StormStage::Mature;
                }
            }
            StormStage::Mature => {
                self.updraft_speed = (self.updraft_speed * 0.99).max(5.0);
                self.downdraft_speed = (self.downdraft_speed + dt * 0.5).min(20.0);
                self.precipitation_rate = (self.downdraft_speed * 0.2).min(50.0);
                self.lightning_rate = (self.updraft_speed * self.cloud_top * 1e-5).min(10.0);
                if self.age > self.lifetime * 0.6 {
                    self.stage = StormStage::Dissipating;
                }
            }
            StormStage::Dissipating => {
                self.updraft_speed = (self.updraft_speed * 0.95).max(0.0);
                self.downdraft_speed *= 0.97;
                self.precipitation_rate *= 0.95;
                self.lightning_rate *= 0.9;
            }
        }
    }

    pub fn wind_at(&self, pos: Vec3) -> Vec3 {
        let delta = pos - self.position;
        let dist = delta.length();
        if dist > self.radius {
            return Vec3::ZERO;
        }
        let factor = 1.0 - dist / self.radius;
        let radial = if dist > 0.01 { delta.normalize() } else { Vec3::Y };
        let tangential = Vec3::new(-radial.z, 0.0, radial.x);
        let inflow = -radial * self.updraft_speed * factor * 0.3;
        let rotation = tangential * self.shear * factor * 0.5;
        inflow + rotation
    }

    pub fn tornado_risk(&self) -> f32 {
        if self.stage != StormStage::Mature {
            return 0.0;
        }
        let shear_factor = (self.shear / 20.0).min(1.0);
        let cape_factor = (self.cape / 3000.0).min(1.0);
        shear_factor * cape_factor
    }

    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime && self.updraft_speed > 0.5
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tornado {
    pub position: Vec3,
    pub path: Vec<Vec3>,
    pub intensity: TornadoIntensity,
    pub max_wind_speed: f32,
    pub width: f32,
    pub length: f32,
    pub age: f32,
    pub lifetime: f32,
    pub damage_radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TornadoIntensity {
    EF0,
    EF1,
    EF2,
    EF3,
    EF4,
    EF5,
}

impl TornadoIntensity {
    pub fn from_wind_speed(mps: f32) -> Self {
        match mps {
            s if s < 38.0 => TornadoIntensity::EF0,
            s if s < 49.0 => TornadoIntensity::EF1,
            s if s < 60.0 => TornadoIntensity::EF2,
            s if s < 74.0 => TornadoIntensity::EF3,
            s if s < 89.0 => TornadoIntensity::EF4,
            _ => TornadoIntensity::EF5,
        }
    }

    pub fn damage_multiplier(&self) -> f32 {
        match self {
            TornadoIntensity::EF0 => 0.05,
            TornadoIntensity::EF1 => 0.15,
            TornadoIntensity::EF2 => 0.35,
            TornadoIntensity::EF3 => 0.6,
            TornadoIntensity::EF4 => 0.8,
            TornadoIntensity::EF5 => 1.0,
        }
    }

    pub fn wind_range(&self) -> (f32, f32) {
        match self {
            TornadoIntensity::EF0 => (29.0, 38.0),
            TornadoIntensity::EF1 => (38.0, 49.0),
            TornadoIntensity::EF2 => (49.0, 60.0),
            TornadoIntensity::EF3 => (60.0, 74.0),
            TornadoIntensity::EF4 => (74.0, 89.0),
            TornadoIntensity::EF5 => (89.0, 140.0),
        }
    }
}

impl Tornado {
    pub fn new(position: Vec3, max_wind_speed: f32) -> Self {
        let intensity = TornadoIntensity::from_wind_speed(max_wind_speed);
        Self {
            position,
            path: vec![position],
            intensity,
            max_wind_speed,
            width: 50.0 + max_wind_speed * 2.0,
            length: 0.0,
            age: 0.0,
            lifetime: 300.0 + max_wind_speed * 5.0,
            damage_radius: 50.0 + max_wind_speed * 3.0,
        }
    }

    pub fn update(&mut self, movement: Vec3, dt: f32) {
        self.age += dt;
        self.position += movement * dt;
        self.length += movement.length() * dt;
        self.path.push(self.position);
        if self.path.len() > 1000 {
            self.path.remove(0);
        }
        self.max_wind_speed *= 0.999;
        self.width *= 0.998;
    }

    pub fn wind_at(&self, pos: Vec3) -> Vec3 {
        let delta = pos - self.position;
        let dist = delta.length();
        if dist > self.width * 2.0 || dist < 0.01 {
            return Vec3::ZERO;
        }
        let factor = (1.0 - dist / (self.width * 2.0)).max(0.0);
        let radial = delta.normalize();
        let tangential = Vec3::new(-radial.z, 0.0, radial.x);
        let vortex_speed = self.max_wind_speed * factor * (1.0 - dist / self.width).max(0.0);
        tangential * vortex_speed - radial * vortex_speed * 0.3
    }

    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime && self.max_wind_speed > 20.0
    }

    pub fn damage_at(&self, pos: Vec3) -> f32 {
        let dist = (pos - self.position).length();
        if dist > self.damage_radius {
            return 0.0;
        }
        let factor = 1.0 - dist / self.damage_radius;
        self.intensity.damage_multiplier() * factor
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hurricane {
    pub position: Vec3,
    pub eye_position: Vec3,
    pub eye_radius: f32,
    pub max_wind_radius: f32,
    pub max_wind_speed: f32,
    pub central_pressure: f32,
    pub outer_pressure: f32,
    pub category: HurricaneCategory,
    pub translation_speed: f32,
    pub translation_direction: Vec3,
    pub age: f32,
    pub sea_surface_temp: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HurricaneCategory {
    TropicalDepression,
    TropicalStorm,
    Cat1,
    Cat2,
    Cat3,
    Cat4,
    Cat5,
}

impl HurricaneCategory {
    pub fn from_wind_speed(mps: f32) -> Self {
        match mps {
            s if s < 17.0 => HurricaneCategory::TropicalDepression,
            s if s < 33.0 => HurricaneCategory::TropicalStorm,
            s if s < 43.0 => HurricaneCategory::Cat1,
            s if s < 50.0 => HurricaneCategory::Cat2,
            s if s < 58.0 => HurricaneCategory::Cat3,
            s if s < 70.0 => HurricaneCategory::Cat4,
            _ => HurricaneCategory::Cat5,
        }
    }

    pub fn storm_surge_height(&self) -> f32 {
        match self {
            HurricaneCategory::TropicalDepression => 0.5,
            HurricaneCategory::TropicalStorm => 1.5,
            HurricaneCategory::Cat1 => 2.0,
            HurricaneCategory::Cat2 => 3.5,
            HurricaneCategory::Cat3 => 5.0,
            HurricaneCategory::Cat4 => 7.0,
            HurricaneCategory::Cat5 => 9.0,
        }
    }
}

impl Hurricane {
    pub fn new(position: Vec3, max_wind_speed: f32, sea_surface_temp: f32) -> Self {
        let category = HurricaneCategory::from_wind_speed(max_wind_speed);
        Self {
            position,
            eye_position: position,
            eye_radius: 15000.0,
            max_wind_radius: 50000.0,
            max_wind_speed,
            central_pressure: 101325.0 - max_wind_speed * 50.0,
            outer_pressure: 101325.0,
            category,
            translation_speed: 5.0,
            translation_direction: Vec3::X,
            age: 0.0,
            sea_surface_temp,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.age += dt;
        self.position += self.translation_direction * self.translation_speed * dt;
        self.eye_position = self.position;

        let sst_factor = (self.sea_surface_temp - 26.0).max(0.0) / 5.0;
        if sst_factor > 0.0 {
            self.max_wind_speed += sst_factor * dt * 0.1;
            self.central_pressure -= sst_factor * dt * 5.0;
        } else {
            self.max_wind_speed *= 0.999;
            self.central_pressure += dt * 10.0;
        }
        self.max_wind_speed = self.max_wind_speed.clamp(0.0, 90.0);
        self.central_pressure = self.central_pressure.clamp(87000.0, self.outer_pressure);
        self.category = HurricaneCategory::from_wind_speed(self.max_wind_speed);
    }

    pub fn wind_at(&self, pos: Vec3) -> Vec3 {
        let delta = pos - self.eye_position;
        let r = delta.length();
        if r < self.eye_radius {
            let factor = r / self.eye_radius;
            let tangential = Vec3::new(-delta.z, 0.0, delta.x);
            if tangential.length() < 0.01 {
                return Vec3::ZERO;
            }
            return tangential.normalize() * self.max_wind_speed * factor;
        }
        if r > self.max_wind_radius * 3.0 {
            return Vec3::ZERO;
        }
        let factor = (self.max_wind_radius / r).min(1.0);
        let tangential = Vec3::new(-delta.z, 0.0, delta.x);
        if tangential.length() < 0.01 {
            return Vec3::ZERO;
        }
        let inflow = -delta.normalize() * self.max_wind_speed * 0.1 * factor;
        tangential.normalize() * self.max_wind_speed * factor + inflow
    }

    pub fn pressure_at(&self, pos: Vec3) -> f32 {
        let r = (pos - self.eye_position).length();
        if r < self.eye_radius {
            return self.central_pressure;
        }
        let dp = self.outer_pressure - self.central_pressure;
        let b = 1.5;
        self.central_pressure + dp * (1.0 - (-(self.max_wind_radius / r).powf(b)).exp())
    }

    pub fn storm_surge(&self) -> f32 {
        self.category.storm_surge_height()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thunderstorm_lifecycle() {
        let mut storm = Thunderstorm::new(Vec3::ZERO, 2000.0, 15.0);
        assert_eq!(storm.stage, StormStage::Cumulus);
        for _ in 0..100 {
            storm.update(100.0);
        }
        assert!(storm.updraft_speed > 0.0);
        assert!(storm.cloud_top > 8000.0);
    }

    #[test]
    fn test_thunderstorm_wind_field() {
        let mut storm = Thunderstorm::new(Vec3::ZERO, 2000.0, 20.0);
        storm.stage = StormStage::Mature;
        storm.downdraft_speed = 10.0;
        let wind = storm.wind_at(Vec3::new(1000.0, 0.0, 0.0));
        assert!(wind.length() > 0.0);
    }

    #[test]
    fn test_tornado_intensity() {
        assert_eq!(TornadoIntensity::from_wind_speed(30.0), TornadoIntensity::EF0);
        assert_eq!(TornadoIntensity::from_wind_speed(50.0), TornadoIntensity::EF2);
        assert_eq!(TornadoIntensity::from_wind_speed(90.0), TornadoIntensity::EF5);
        assert!(TornadoIntensity::EF5.damage_multiplier() > TornadoIntensity::EF0.damage_multiplier());
    }

    #[test]
    fn test_tornado_lifecycle() {
        let mut tornado = Tornado::new(Vec3::ZERO, 60.0);
        assert!(tornado.is_alive());
        tornado.update(Vec3::new(5.0, 0.0, 0.0), 10.0);
        assert!(tornado.path.len() > 1);
        assert!(tornado.length > 0.0);
    }

    #[test]
    fn test_hurricane_categories() {
        assert_eq!(HurricaneCategory::from_wind_speed(10.0), HurricaneCategory::TropicalDepression);
        assert_eq!(HurricaneCategory::from_wind_speed(25.0), HurricaneCategory::TropicalStorm);
        assert_eq!(HurricaneCategory::from_wind_speed(45.0), HurricaneCategory::Cat2);
        assert_eq!(HurricaneCategory::from_wind_speed(65.0), HurricaneCategory::Cat4);
        assert_eq!(HurricaneCategory::from_wind_speed(80.0), HurricaneCategory::Cat5);
    }

    #[test]
    fn test_hurricane_wind_field() {
        let hurricane = Hurricane::new(Vec3::ZERO, 50.0, 28.0);
        let wind_near = hurricane.wind_at(Vec3::new(40000.0, 0.0, 0.0));
        assert!(wind_near.length() > 0.0);
        let wind_far = hurricane.wind_at(Vec3::new(200000.0, 0.0, 0.0));
        assert!(wind_far.length() < 1.0);
    }

    #[test]
    fn test_hurricane_pressure_field() {
        let hurricane = Hurricane::new(Vec3::ZERO, 50.0, 28.0);
        let p_eye = hurricane.pressure_at(hurricane.eye_position);
        let p_far = hurricane.pressure_at(Vec3::new(200000.0, 0.0, 0.0));
        assert!(p_eye < p_far);
    }
}