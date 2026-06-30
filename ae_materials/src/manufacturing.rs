use crate::microstructure::Microstructure;
use crate::phases::MaterialPhase;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuenchMedium {
    Water,
    Oil,
    Air,
    Brine,
}

impl QuenchMedium {
    pub fn cooling_rate(&self) -> f32 {
        match self {
            QuenchMedium::Water => 300.0,
            QuenchMedium::Oil => 100.0,
            QuenchMedium::Air => 20.0,
            QuenchMedium::Brine => 500.0,
        }
    }
}

pub fn quench(micro: &mut Microstructure, temp: f32, medium: QuenchMedium) {
    if temp < 1000.0 {
        return;
    }

    let cooling_rate = medium.cooling_rate();

    if cooling_rate > 200.0 {
        let martensite_fraction = ((cooling_rate - 200.0) / 300.0).clamp(0.0, 0.95);
        let austenite_remaining = 1.0 - martensite_fraction;

        micro.phase_fractions.clear();
        micro.add_phase(MaterialPhase::Martensite, martensite_fraction);
        if austenite_remaining > 0.01 {
            micro.add_phase(MaterialPhase::Austenite, austenite_remaining);
        }
        micro.crystal_structure = MaterialPhase::Martensite.crystal_structure();
        micro.dislocation_density *= 100.0;
        micro.grain_size *= 0.1;
        micro.update_grain_boundary();
    } else if cooling_rate > 50.0 {
        let bainite_fraction = ((cooling_rate - 50.0) / 150.0).clamp(0.0, 0.8);
        micro.phase_fractions.clear();
        micro.add_phase(MaterialPhase::Bainite, bainite_fraction);
        micro.add_phase(MaterialPhase::Ferrite, 1.0 - bainite_fraction);
        micro.dislocation_density *= 10.0;
    } else {
        micro.phase_fractions.clear();
        micro.add_phase(MaterialPhase::Pearlite, 0.7);
        micro.add_phase(MaterialPhase::Ferrite, 0.3);
        micro.dislocation_density *= 0.5;
    }
}

pub fn temper(micro: &mut Microstructure, temp: f32, duration: f32) {
    if !(400.0..=1000.0).contains(&temp) {
        return;
    }

    let martensite_present =
        micro.phase_fractions.iter().any(|(p, _)| *p == MaterialPhase::Martensite);

    if martensite_present {
        let temper_fraction = (duration / 3600.0).clamp(0.0, 1.0);
        let hardness_reduction = temper_fraction * (temp - 400.0) / 600.0;

        micro.phase_fractions.retain(|(p, _)| *p != MaterialPhase::Martensite);
        micro.add_phase(MaterialPhase::TemperedMartensite, 1.0 - hardness_reduction);
        micro.add_phase(MaterialPhase::Ferrite, hardness_reduction);

        micro.dislocation_density *= (1.0 - temper_fraction * 0.9).max(0.1);
        micro.grain_size *= 1.0 + temper_fraction * 0.5;
        micro.update_grain_boundary();
    }
}

pub fn anneal(micro: &mut Microstructure, temp: f32, cool_rate: f32) {
    if temp < 800.0 {
        return;
    }

    micro.phase_fractions.clear();
    micro.add_phase(MaterialPhase::Austenite, 1.0);

    if cool_rate < 5.0 {
        micro.phase_fractions.clear();
        micro.add_phase(MaterialPhase::Ferrite, 0.7);
        micro.add_phase(MaterialPhase::Pearlite, 0.3);
        micro.crystal_structure = MaterialPhase::Ferrite.crystal_structure();
    }

    micro.dislocation_density *= 0.1;
    micro.grain_size *= 3.0;
    micro.vacancy_concentration *= 0.5;
    micro.precipitate_density *= 0.1;
    micro.update_grain_boundary();
}

pub fn cold_work(micro: &mut Microstructure, strain: f32) {
    let strain_clamped = strain.clamp(0.0, 0.5);
    micro.dislocation_density *= 1.0 + strain_clamped * 100.0;
    micro.grain_size *= (1.0 - strain_clamped * 0.5).max(0.1);
    micro.texture_anisotropy = (micro.texture_anisotropy + strain_clamped).min(1.0);
    micro.update_grain_boundary();
}

pub fn carburize(micro: &mut Microstructure, temp: f32, duration: f32) {
    if temp < 1100.0 {
        return;
    }

    let carbon_increase = (duration / 3600.0) * 0.1;
    micro.carbon_content = (micro.carbon_content + carbon_increase).min(2.0);

    if micro.carbon_content > 0.8 {
        micro.phase_fractions.retain(|(p, _)| *p != MaterialPhase::Pearlite);
        micro.add_phase(MaterialPhase::Cementite, (micro.carbon_content - 0.8).min(0.3));
    }
}

pub fn forge(micro: &mut Microstructure, temp: f32, strain: f32) {
    if temp < 1000.0 {
        cold_work(micro, strain);
        return;
    }

    micro.grain_size *= (1.0 - strain * 0.5).max(0.1);
    micro.dislocation_density = (micro.dislocation_density * 0.3 + 1e9 * strain).max(1e9);
    micro.texture_anisotropy = (micro.texture_anisotropy + strain * 0.5).min(1.0);
    micro.vacancy_concentration *= 0.8;
    micro.update_grain_boundary();
}

pub fn age_harden(micro: &mut Microstructure, temp: f32, duration: f32) {
    if !(400.0..=600.0).contains(&temp) {
        return;
    }

    let precipitate_growth = (duration / 3600.0).clamp(0.0, 1.0);
    micro.precipitate_density = (micro.precipitate_density + precipitate_growth * 0.1).min(0.5);
    micro.dislocation_density *= 1.0 + precipitate_growth * 0.5;

    if precipitate_growth > 0.8 {
        micro.precipitate_density *= 0.9;
        micro.grain_size *= 1.2;
        micro.update_grain_boundary();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quench_produces_martensite() {
        let mut micro = Microstructure::default();
        quench(&mut micro, 1100.0, QuenchMedium::Water);
        let has_martensite =
            micro.phase_fractions.iter().any(|(p, _)| *p == MaterialPhase::Martensite);
        assert!(has_martensite);
    }

    #[test]
    fn test_quench_oil_produces_bainite() {
        let mut micro = Microstructure::default();
        quench(&mut micro, 1100.0, QuenchMedium::Oil);
        let has_bainite = micro.phase_fractions.iter().any(|(p, _)| *p == MaterialPhase::Bainite);
        assert!(has_bainite);
    }

    #[test]
    fn test_temper_reduces_hardness() {
        let mut micro = Microstructure::default();
        quench(&mut micro, 1100.0, QuenchMedium::Water);
        let hardness_before = micro.compute_hardness();
        temper(&mut micro, 600.0, 3600.0);
        let hardness_after = micro.compute_hardness();
        assert!(hardness_after < hardness_before);
    }

    #[test]
    fn test_anneal_softens() {
        let mut micro = Microstructure {
            dislocation_density: 1e12,
            ..Default::default()
        };
        anneal(&mut micro, 900.0, 1.0);
        assert!(micro.dislocation_density < 1e12);
    }

    #[test]
    fn test_cold_work_increases_dislocations() {
        let mut micro = Microstructure::default();
        let initial = micro.dislocation_density;
        cold_work(&mut micro, 0.3);
        assert!(micro.dislocation_density > initial);
    }

    #[test]
    fn test_carburize_increases_carbon() {
        let mut micro = Microstructure::default();
        let initial = micro.carbon_content;
        carburize(&mut micro, 1200.0, 3600.0);
        assert!(micro.carbon_content > initial);
    }

    #[test]
    fn test_forge_refines_grain() {
        let mut micro = Microstructure::default();
        let initial = micro.grain_size;
        forge(&mut micro, 1100.0, 0.5);
        assert!(micro.grain_size < initial);
    }
}
