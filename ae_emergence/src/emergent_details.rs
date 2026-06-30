use glam::Vec3;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentDetailSystem {
    pub stress_analyzer: StressAnalyzer,
    pub corrosion_generator: CorrosionGenerator,
    pub growth_ring_generator: GrowthRingGenerator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressAnalyzer {
    pub crack_patterns: Vec<CrackPattern>,
    pub stress_threshold: f32,
    pub propagation_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrackPattern {
    pub start_point: Vec3,
    pub direction: Vec3,
    pub length: f32,
    pub branching_points: Vec<Vec3>,
    pub stress_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrosionGenerator {
    pub rust_spots: Vec<RustSpot>,
    pub pitting_patterns: Vec<PittingPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustSpot {
    pub center: Vec3,
    pub radius: f32,
    pub depth: f32,
    pub stage: RustStage,
    pub spread_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RustStage {
    Initial,
    Spreading,
    Pitting,
    Flaking,
    Perforation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PittingPattern {
    pub pits: Vec<(Vec3, f32)>,
    pub material_resistance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthRingGenerator {
    pub annual_growth: Vec<GrowthRing>,
    pub bark_fissures: Vec<BarkFissure>,
    pub knot_positions: Vec<Vec3>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthRing {
    pub year: u32,
    pub thickness: f32,
    pub color: [f32; 3],
    pub stress_anomalies: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarkFissure {
    pub position: Vec3,
    pub length: f32,
    pub width: f32,
    pub depth: f32,
    pub orientation: Vec3,
}

impl EmergentDetailSystem {
    pub fn new() -> Self {
        Self {
            stress_analyzer: StressAnalyzer {
                crack_patterns: Vec::new(),
                stress_threshold: 0.7,
                propagation_rate: 0.05,
            },
            corrosion_generator: CorrosionGenerator {
                rust_spots: Vec::new(),
                pitting_patterns: Vec::new(),
            },
            growth_ring_generator: GrowthRingGenerator {
                annual_growth: Vec::new(),
                bark_fissures: Vec::new(),
                knot_positions: Vec::new(),
            },
        }
    }

    pub fn generate_cracks(
        &mut self,
        surface_points: &[Vec3],
        stress_field: &[f32],
        material_brittleness: f32,
        seed: u64,
    ) -> Vec<CrackPattern> {
        let rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut patterns = Vec::new();

        for (point, stress) in surface_points.iter().zip(stress_field.iter()) {
            if *stress > self.stress_analyzer.stress_threshold * material_brittleness
                && rng.clone().gen::<f32>() < *stress * 0.3
            {
                let direction = Vec3::new(
                    rng.clone().gen::<f32>() - 0.5,
                    rng.clone().gen::<f32>() - 0.5,
                    rng.clone().gen::<f32>() - 0.5,
                )
                .normalize();

                let length = *stress * 5.0 * material_brittleness;
                let mut branching = Vec::new();

                let num_branches = (length / 2.0) as usize;
                for _ in 0..num_branches {
                    let t = rng.clone().gen::<f32>();
                    let branch_point = *point + direction * length * t;
                    branching.push(branch_point);
                }

                patterns.push(CrackPattern {
                    start_point: *point,
                    direction,
                    length,
                    branching_points: branching,
                    stress_level: *stress,
                });
            }
        }

        self.stress_analyzer.crack_patterns = patterns.clone();
        patterns
    }

    pub fn generate_rust_spots(
        &mut self,
        surface_points: &[Vec3],
        oxidation_depth: &[f32],
        humidity: f32,
        seed: u64,
    ) -> Vec<RustSpot> {
        let rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut spots = Vec::new();

        for (point, ox) in surface_points.iter().zip(oxidation_depth.iter()) {
            if *ox > 0.1 && rng.clone().gen::<f32>() < humidity * ox * 0.5 {
                let radius = *ox * 0.5 * (0.5 + rng.clone().gen::<f32>());
                let stage = if *ox < 0.3 {
                    RustStage::Initial
                } else if *ox < 0.5 {
                    RustStage::Spreading
                } else if *ox < 0.7 {
                    RustStage::Pitting
                } else if *ox < 0.9 {
                    RustStage::Flaking
                } else {
                    RustStage::Perforation
                };

                spots.push(RustSpot {
                    center: *point,
                    radius,
                    depth: *ox * 2.0,
                    stage,
                    spread_rate: humidity * 0.01,
                });
            }
        }

        self.corrosion_generator.rust_spots = spots.clone();
        spots
    }

    pub fn generate_growth_rings(
        &mut self,
        _trunk_center: Vec3,
        _trunk_radius: f32,
        years: u32,
        rainfall_history: &[f32],
    ) -> Vec<GrowthRing> {
        let mut rings = Vec::new();
        let mut _current_radius = 0.1f32;

        for year in 0..years {
            let rainfall = if (year as usize) < rainfall_history.len() {
                rainfall_history[year as usize]
            } else {
                1.0
            };
            let thickness = 0.5 + rainfall * 2.0;
            _current_radius += thickness;

            let color = if rainfall < 0.5 {
                [0.2, 0.1, 0.05]
            } else if rainfall < 1.0 {
                [0.3, 0.18, 0.08]
            } else {
                [0.35, 0.22, 0.1]
            };

            let anomalies = if rainfall < 0.3 { vec![0.2, 0.3, 0.1] } else { Vec::new() };

            rings.push(GrowthRing { year, thickness, color, stress_anomalies: anomalies });
        }

        self.growth_ring_generator.annual_growth = rings.clone();
        rings
    }

    pub fn generate_bark_fissures(
        &mut self,
        trunk_height: f32,
        trunk_radius: f32,
        age: f32,
        seed: u64,
    ) -> Vec<BarkFissure> {
        let rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut fissures = Vec::new();

        let count = (trunk_height * trunk_radius * 2.0) as usize;
        for _ in 0..count {
            let height = rng.clone().gen::<f32>() * trunk_height;
            let angle = rng.clone().gen::<f32>() * std::f32::consts::TAU;
            let pos = Vec3::new(angle.cos() * trunk_radius, height, angle.sin() * trunk_radius);

            let length = 0.5 + rng.clone().gen::<f32>() * 3.0 * (age / 50.0).min(1.0);
            let width = 0.02 + rng.clone().gen::<f32>() * 0.1;
            let depth = 0.01 + rng.clone().gen::<f32>() * 0.05 * (age / 50.0).min(1.0);
            let orientation = Vec3::new(0.0, 1.0, 0.0);

            fissures.push(BarkFissure { position: pos, length, width, depth, orientation });
        }

        self.growth_ring_generator.bark_fissures = fissures.clone();
        fissures
    }

    pub fn generate_erosion_pits(
        surface_points: &[Vec3],
        material_hardness: f32,
        exposure_time: f32,
        seed: u64,
    ) -> Vec<(Vec3, f32)> {
        let rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut pits = Vec::new();

        let count = (exposure_time * 0.5) as usize;
        for _ in 0..count.min(surface_points.len()) {
            let idx = rng.clone().gen_range(0..surface_points.len());
            let depth = rng.clone().gen::<f32>() * 0.5 / material_hardness;
            pits.push((surface_points[idx], depth));
        }

        pits
    }

    pub fn step(&mut self, dt: f32, humidity: f32) {
        for crack in &mut self.stress_analyzer.crack_patterns {
            crack.length += self.stress_analyzer.propagation_rate * crack.stress_level * dt;
            if crack.stress_level > 0.9 {
                let branch =
                    crack.start_point + crack.direction * crack.length * rand::random::<f32>();
                crack.branching_points.push(branch);
            }
        }

        for spot in &mut self.corrosion_generator.rust_spots {
            spot.radius += spot.spread_rate * humidity * dt;
            spot.depth += spot.spread_rate * humidity * dt * 0.5;

            if spot.depth > 8.0 && spot.stage == RustStage::Flaking {
                spot.stage = RustStage::Perforation;
            } else if spot.depth > 5.0 && spot.stage == RustStage::Pitting {
                spot.stage = RustStage::Flaking;
            } else if spot.depth > 2.0 && spot.stage == RustStage::Spreading {
                spot.stage = RustStage::Pitting;
            }
        }
    }
}

impl Default for EmergentDetailSystem {
    fn default() -> Self {
        Self::new()
    }
}
