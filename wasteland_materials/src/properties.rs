use crate::microstructure::Microstructure;

#[derive(Debug, Clone)]
pub struct DerivedProperties {
    pub hardness: f32,
    pub toughness: f32,
    pub yield_strength: f32,
    pub elastic_modulus: f32,
    pub fatigue_limit: f32,
    pub creep_resistance: f32,
    pub corrosion_rate: f32,
    pub thermal_conductivity: f32,
    pub density: f32,
}

impl DerivedProperties {
    pub fn from_microstructure(micro: &Microstructure) -> Self {
        let hardness = micro.compute_hardness();
        let toughness = micro.compute_toughness();
        let yield_strength = micro.compute_yield_strength();
        let elastic_modulus = micro.compute_elastic_modulus();

        let fatigue_limit = yield_strength * 0.4;
        let creep_resistance = 1.0 / (micro.dislocation_density * 1e-13).max(0.01);
        let corrosion_rate =
            (1.0 + micro.carbon_content * 5.0) / (micro.grain_size * 0.1).max(0.01);

        let thermal_conductivity =
            50.0 - micro.dislocation_density * 1e-10 + micro.grain_size * 0.1;
        let density = 7.8 - micro.carbon_content * 0.1 - micro.vacancy_concentration * 100.0;

        Self {
            hardness,
            toughness,
            yield_strength,
            elastic_modulus,
            fatigue_limit,
            creep_resistance,
            corrosion_rate,
            thermal_conductivity: thermal_conductivity.max(5.0),
            density: density.max(7.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phases::MaterialPhase;

    #[test]
    fn test_derived_properties_sanity() {
        let micro = Microstructure::default();
        let props = DerivedProperties::from_microstructure(&micro);
        assert!(props.hardness > 0.0);
        assert!(props.toughness > 0.0);
        assert!(props.yield_strength > 0.0);
        assert!(props.elastic_modulus > 100.0);
    }

    #[test]
    fn test_martensite_hardness() {
        let micro = Microstructure {
            phase_fractions: vec![(MaterialPhase::Martensite, 1.0)],
            ..Default::default()
        };
        let props = DerivedProperties::from_microstructure(&micro);
        assert!(props.hardness > 500.0);
    }
}
