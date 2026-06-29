use serde::{Deserialize, Serialize};

use crate::phases::{CrystalStructure, MaterialPhase};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Microstructure {
    pub grain_size: f32,
    pub dislocation_density: f32,
    pub phase_fractions: Vec<(MaterialPhase, f32)>,
    pub crystal_structure: CrystalStructure,
    pub carbon_content: f32,
    pub vacancy_concentration: f32,
    pub precipitate_density: f32,
    pub texture_anisotropy: f32,
    pub grain_boundary_area: f32,
}

impl Default for Microstructure {
    fn default() -> Self {
        Self {
            grain_size: 50.0,
            dislocation_density: 1e10,
            phase_fractions: vec![(MaterialPhase::Ferrite, 1.0)],
            crystal_structure: CrystalStructure::BCC,
            carbon_content: 0.2,
            vacancy_concentration: 1e-6,
            precipitate_density: 0.0,
            texture_anisotropy: 0.0,
            grain_boundary_area: 0.02,
        }
    }
}

impl Microstructure {
    pub fn new(carbon_content: f32) -> Self {
        let phase = if carbon_content < 0.008 {
            MaterialPhase::Ferrite
        } else if carbon_content < 0.8 {
            MaterialPhase::Pearlite
        } else if carbon_content < 2.0 {
            MaterialPhase::Martensite
        } else {
            MaterialPhase::Cementite
        };
        Self { carbon_content, phase_fractions: vec![(phase, 1.0)], ..Default::default() }
    }

    pub fn compute_hardness(&self) -> f32 {
        let phase_hardness: f32 =
            self.phase_fractions.iter().map(|(p, frac)| p.base_hardness() * frac).sum();
        let grain_contribution = 20.0 / self.grain_size.sqrt().max(0.01);
        let dislocation_contribution = 2.0e-8 * self.dislocation_density.sqrt();
        let precipitate_contribution = 50.0 * self.precipitate_density;

        phase_hardness + grain_contribution + dislocation_contribution + precipitate_contribution
    }

    pub fn compute_toughness(&self) -> f32 {
        let phase_toughness: f32 =
            self.phase_fractions.iter().map(|(p, frac)| p.base_toughness() * frac).sum();
        let grain_contribution = 5.0 * self.grain_size.sqrt();
        let dislocation_penalty = 1e-8 * self.dislocation_density;
        let carbon_penalty = self.carbon_content * 20.0;

        phase_toughness + grain_contribution - dislocation_penalty - carbon_penalty
    }

    pub fn compute_yield_strength(&self) -> f32 {
        let hall_petch = 100.0 / self.grain_size.sqrt().max(0.01);
        let dislocation = 0.5e-8 * self.dislocation_density.sqrt();
        let solid_solution = 50.0 * self.carbon_content;
        let precipitate = 100.0 * self.precipitate_density;

        let base = self
            .phase_fractions
            .iter()
            .map(|(p, frac)| {
                (match p {
                    MaterialPhase::Ferrite => 200.0,
                    MaterialPhase::Austenite => 300.0,
                    MaterialPhase::Martensite => 1500.0,
                    MaterialPhase::Pearlite => 500.0,
                    MaterialPhase::Bainite => 800.0,
                    MaterialPhase::Cementite => 2000.0,
                    MaterialPhase::Graphite => 50.0,
                    MaterialPhase::Ledeburite => 1200.0,
                    MaterialPhase::Spheroidite => 400.0,
                    MaterialPhase::TemperedMartensite => 1000.0,
                }) * frac
            })
            .sum::<f32>();

        base + hall_petch + dislocation + solid_solution + precipitate
    }

    pub fn compute_elastic_modulus(&self) -> f32 {
        let base = 210.0;
        let porosity_penalty = self.vacancy_concentration * 1e5;
        let carbon_effect = self.carbon_content * 5.0;
        base - porosity_penalty + carbon_effect
    }

    pub fn add_phase(&mut self, phase: MaterialPhase, fraction: f32) {
        if let Some((_, existing)) = self.phase_fractions.iter_mut().find(|(p, _)| *p == phase) {
            *existing = (*existing + fraction).min(1.0);
        } else {
            self.phase_fractions.push((phase, fraction));
        }
        self.normalize_phases();
    }

    fn normalize_phases(&mut self) {
        let total: f32 = self.phase_fractions.iter().map(|(_, f)| *f).sum();
        if total > 0.0 {
            for (_, frac) in &mut self.phase_fractions {
                *frac /= total;
            }
        }
    }

    pub fn dominant_phase(&self) -> Option<MaterialPhase> {
        self.phase_fractions.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).map(|(p, _)| *p)
    }

    pub fn update_grain_boundary(&mut self) {
        self.grain_boundary_area = 2.0 / self.grain_size.max(0.01);
    }
}
