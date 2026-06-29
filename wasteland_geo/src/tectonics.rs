use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TectonicPlate {
    pub id: u64,
    pub velocity: Vec3,
    pub boundary_type: PlateBoundary,
    pub stress_accumulated: f32,
    pub thickness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlateBoundary {
    Convergent,
    Divergent,
    Transform,
    Subduction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TectonicSolver {
    pub plates: Vec<TectonicPlate>,
    pub geothermal_gradient: f32,
    pub mantle_viscosity: f32,
}

impl TectonicSolver {
    pub fn new() -> Self {
        Self { plates: Vec::new(), geothermal_gradient: 25.0, mantle_viscosity: 1e21 }
    }

    pub fn add_plate(&mut self, velocity: Vec3, boundary: PlateBoundary, thickness: f32) -> u64 {
        let id = self.plates.len() as u64;
        self.plates.push(TectonicPlate {
            id,
            velocity,
            boundary_type: boundary,
            stress_accumulated: 0.0,
            thickness,
        });
        id
    }

    pub fn step(&mut self, dt: f32) {
        for i in 0..self.plates.len() {
            for j in i + 1..self.plates.len() {
                let va = self.plates[i].velocity;
                let vb = self.plates[j].velocity;
                let rel_vel = (va - vb).length();

                match (self.plates[i].boundary_type, self.plates[j].boundary_type) {
                    (PlateBoundary::Convergent, PlateBoundary::Convergent)
                    | (PlateBoundary::Subduction, _)
                    | (_, PlateBoundary::Subduction) => {
                        let stress_rate = rel_vel * 1e-6;
                        self.plates[i].stress_accumulated += stress_rate * dt;
                        self.plates[j].stress_accumulated += stress_rate * dt;
                    },
                    (PlateBoundary::Divergent, PlateBoundary::Divergent) => {
                        self.plates[i].stress_accumulated =
                            (self.plates[i].stress_accumulated - rel_vel * 1e-7 * dt).max(0.0);
                        self.plates[j].stress_accumulated =
                            (self.plates[j].stress_accumulated - rel_vel * 1e-7 * dt).max(0.0);
                    },
                    (PlateBoundary::Transform, _) | (_, PlateBoundary::Transform) => {
                        let stress_rate = rel_vel * 5e-7;
                        self.plates[i].stress_accumulated += stress_rate * dt;
                        self.plates[j].stress_accumulated += stress_rate * dt;
                    },
                    _ => {},
                }
            }
        }
    }

    pub fn check_earthquake(&mut self, threshold: f32) -> Vec<Earthquake> {
        let mut quakes = Vec::new();
        for plate in &mut self.plates {
            if plate.stress_accumulated > threshold {
                let magnitude = (plate.stress_accumulated / threshold).log10() * 2.0 + 4.0;
                quakes.push(Earthquake { magnitude: magnitude.min(9.5), plate_id: plate.id });
                plate.stress_accumulated *= 0.1;
            }
        }
        quakes
    }

    pub fn temperature_at_depth(&self, depth: f32) -> f32 {
        15.0 + depth * self.geothermal_gradient / 1000.0
    }
}

impl Default for TectonicSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Earthquake {
    pub magnitude: f32,
    pub plate_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_tectonic_plate_creation() {
        let mut solver = TectonicSolver::new();
        let id = solver.add_plate(Vec3::new(0.01, 0.0, 0.0), PlateBoundary::Convergent, 100.0);
        assert_eq!(id, 0);
        assert_eq!(solver.plates.len(), 1);
        assert_eq!(solver.plates[0].boundary_type, PlateBoundary::Convergent);
        assert_eq!(solver.plates[0].thickness, 100.0);
    }

    #[test]
    fn test_tectonic_step() {
        let mut solver = TectonicSolver::new();
        solver.add_plate(Vec3::new(0.01, 0.0, 0.0), PlateBoundary::Convergent, 100.0);
        solver.add_plate(Vec3::new(-0.01, 0.0, 0.0), PlateBoundary::Convergent, 100.0);
        solver.step(1.0);
        assert!(solver.plates[0].stress_accumulated > 0.0);
        assert!(solver.plates[1].stress_accumulated > 0.0);
    }

    #[test]
    fn test_earthquake_detection() {
        let mut solver = TectonicSolver::new();
        solver.add_plate(Vec3::new(0.01, 0.0, 0.0), PlateBoundary::Convergent, 100.0);
        solver.add_plate(Vec3::new(-0.01, 0.0, 0.0), PlateBoundary::Convergent, 100.0);
        solver.step(100000.0);
        let quakes = solver.check_earthquake(0.001);
        assert!(!quakes.is_empty());
        assert!(quakes[0].magnitude > 0.0);
    }
}
