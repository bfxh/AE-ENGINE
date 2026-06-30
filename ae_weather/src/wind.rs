use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindField {
    pub velocity: Vec3,
    pub gust: f32,
    pub turbulence: f32,
    pub shear: Vec3,
}

impl Default for WindField {
    fn default() -> Self {
        Self { velocity: Vec3::ZERO, gust: 0.0, turbulence: 0.0, shear: Vec3::ZERO }
    }
}

impl WindField {
    pub fn wind_speed(&self) -> f32 {
        self.velocity.length()
    }

    pub fn effective_speed(&self) -> f32 {
        self.velocity.length() + self.gust * self.turbulence
    }

    pub fn wind_direction(&self) -> Vec3 {
        if self.velocity.length() < 0.01 { Vec3::ZERO } else { self.velocity.normalize() }
    }

    pub fn apply_coriolis(&mut self, latitude: f32, dt: f32) {
        const OMEGA: f32 = 7.2921e-5;
        let f = 2.0 * OMEGA * latitude.to_radians().sin();
        let coriolis = Vec3::new(f * self.velocity.y, -f * self.velocity.x, 0.0);
        self.velocity += coriolis * dt;
    }

    pub fn apply_pressure_gradient(&mut self, pressure_gradient: Vec3, air_density: f32, dt: f32) {
        let acc = -pressure_gradient / air_density.max(0.01);
        self.velocity += acc * dt;
    }

    pub fn apply_friction(&mut self, surface_roughness: f32, dt: f32) {
        let speed = self.velocity.length();
        if speed > 0.0 {
            let friction = -self.velocity.normalize() * surface_roughness * speed * speed;
            self.velocity += friction * dt;
        }
    }

    pub fn generate_gust(&mut self, max_gust: f32, rng: &mut impl rand::Rng) {
        self.gust = rng.gen_range(0.0..max_gust);
        let theta = rng.gen_range(0.0..std::f32::consts::TAU);
        let phi = rng.gen_range(0.0..std::f32::consts::PI);
        self.turbulence += rng.gen_range(-0.1..0.1);
        self.turbulence = self.turbulence.clamp(0.0, 1.0);
        self.shear =
            Vec3::new(theta.cos() * phi.sin(), theta.sin() * phi.sin(), phi.cos()) * self.gust;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeaufortScale {
    Calm,
    LightAir,
    LightBreeze,
    GentleBreeze,
    ModerateBreeze,
    FreshBreeze,
    StrongBreeze,
    NearGale,
    Gale,
    StrongGale,
    Storm,
    ViolentStorm,
    Hurricane,
}

impl BeaufortScale {
    pub fn from_speed(mps: f32) -> Self {
        match mps {
            s if s < 0.5 => BeaufortScale::Calm,
            s if s < 1.5 => BeaufortScale::LightAir,
            s if s < 3.3 => BeaufortScale::LightBreeze,
            s if s < 5.5 => BeaufortScale::GentleBreeze,
            s if s < 8.0 => BeaufortScale::ModerateBreeze,
            s if s < 10.8 => BeaufortScale::FreshBreeze,
            s if s < 13.9 => BeaufortScale::StrongBreeze,
            s if s < 17.2 => BeaufortScale::NearGale,
            s if s < 20.8 => BeaufortScale::Gale,
            s if s < 24.5 => BeaufortScale::StrongGale,
            s if s < 28.5 => BeaufortScale::Storm,
            s if s < 32.7 => BeaufortScale::ViolentStorm,
            _ => BeaufortScale::Hurricane,
        }
    }

    pub fn damage_multiplier(&self) -> f32 {
        match self {
            BeaufortScale::Calm => 0.0,
            BeaufortScale::LightAir | BeaufortScale::LightBreeze => 0.0,
            BeaufortScale::GentleBreeze => 0.01,
            BeaufortScale::ModerateBreeze => 0.05,
            BeaufortScale::FreshBreeze => 0.1,
            BeaufortScale::StrongBreeze => 0.2,
            BeaufortScale::NearGale => 0.35,
            BeaufortScale::Gale => 0.5,
            BeaufortScale::StrongGale => 0.7,
            BeaufortScale::Storm => 0.85,
            BeaufortScale::ViolentStorm => 0.95,
            BeaufortScale::Hurricane => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_wind_field_creation() {
        let wind = WindField::default();
        assert_eq!(wind.wind_speed(), 0.0);
        assert_eq!(wind.effective_speed(), 0.0);
        assert_eq!(wind.wind_direction(), Vec3::ZERO);
    }

    #[test]
    fn test_wind_field_with_velocity() {
        let mut wind = WindField {
            velocity: Vec3::new(3.0, 4.0, 0.0),
            gust: 0.0,
            turbulence: 0.0,
            shear: Vec3::ZERO,
        };
        assert_eq!(wind.wind_speed(), 5.0);
        let dir = wind.wind_direction();
        assert!((dir.length() - 1.0).abs() < 0.001);
        wind.apply_coriolis(45.0, 1.0);
        assert!(wind.velocity.x != 3.0 || wind.velocity.y != 4.0);
    }

    #[test]
    fn test_beaufort_scale() {
        assert_eq!(BeaufortScale::from_speed(0.0), BeaufortScale::Calm);
        assert_eq!(BeaufortScale::from_speed(5.0), BeaufortScale::GentleBreeze);
        assert_eq!(BeaufortScale::from_speed(15.0), BeaufortScale::NearGale);
        assert_eq!(BeaufortScale::from_speed(35.0), BeaufortScale::Hurricane);
        assert_eq!(BeaufortScale::Hurricane.damage_multiplier(), 1.0);
        assert_eq!(BeaufortScale::Calm.damage_multiplier(), 0.0);
    }
}
