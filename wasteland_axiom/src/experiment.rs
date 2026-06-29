use super::axiom::Axiom;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub id: String,
    pub name: String,
    pub instrument_type: InstrumentType,
    pub precision: f32,
    pub accuracy: f32,
    pub calibration: f32,
    pub trust_level: f32,
    pub underlying_axioms: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstrumentType {
    Spectrometer,
    Thermometer,
    Barometer,
    HardnessTester,
    ChemicalAnalyzer,
    Microscope,
    Electroscope,
    Magnetometer,
    Custom,
}

impl Instrument {
    pub fn new(name: &str, instrument_type: InstrumentType, precision: f32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            instrument_type,
            precision,
            accuracy: precision,
            calibration: 1.0,
            trust_level: 0.5,
            underlying_axioms: Vec::new(),
        }
    }

    pub fn measure(&self, true_value: f32) -> f32 {
        let calibrated = true_value * self.calibration;
        let noise = (1.0 - self.precision) * true_value * 0.1;
        let mut rng = rand::thread_rng();
        let sign = if rng.gen::<bool>() { 1.0 } else { -1.0 };
        (calibrated + noise * sign).max(0.0)
    }

    pub fn measure_with_confidence(&self, true_value: f32) -> (f32, f32) {
        let measured = self.measure(true_value);
        let confidence = self.precision * self.trust_level;
        (measured, confidence)
    }

    pub fn validate_against(&self, other: &Instrument, test_value: f32) -> f32 {
        let a = self.measure(test_value);
        let b = other.measure(test_value);
        1.0 / (1.0 + (a - b).abs())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentRunner {
    pub instruments: Vec<Instrument>,
    pub results: Vec<ExperimentResult>,
    pub accepted_standard: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    pub axiom_id: String,
    pub property_tested: String,
    pub predicted_value: f32,
    pub measured_value: f32,
    pub instrument_id: String,
    pub confidence: f32,
    pub passed: bool,
}

impl ExperimentRunner {
    pub fn new() -> Self {
        Self { instruments: Vec::new(), results: Vec::new(), accepted_standard: None }
    }

    pub fn add_instrument(&mut self, instrument: Instrument) {
        self.instruments.push(instrument);
    }

    pub fn set_standard(&mut self, instrument_id: &str) {
        self.accepted_standard = Some(instrument_id.to_string());
    }

    pub fn run_experiment(
        &mut self,
        axiom: &Axiom,
        property: &str,
        true_value: f32,
    ) -> ExperimentResult {
        let instrument = self.instruments.first().unwrap();
        let (measured, confidence) = instrument.measure_with_confidence(true_value);

        let tolerance = (1.0 - instrument.precision) * true_value.abs() + 0.01;
        let passed = (measured - true_value).abs() <= tolerance;

        let result = ExperimentResult {
            axiom_id: axiom.id.clone(),
            property_tested: property.to_string(),
            predicted_value: true_value,
            measured_value: measured,
            instrument_id: instrument.id.clone(),
            confidence,
            passed,
        };

        self.results.push(result.clone());
        result
    }

    pub fn verify_axiom(&mut self, axiom: &Axiom) -> f32 {
        if axiom.properties.is_empty() {
            return 0.0;
        }

        let mut passed = 0;
        let total = axiom.properties.len();

        for (property, &value) in &axiom.properties {
            let result = self.run_experiment(axiom, property, value);
            if result.passed {
                passed += 1;
            }
        }

        if total > 0 { passed as f32 / total as f32 } else { 0.0 }
    }

    pub fn cross_validate(
        &mut self,
        _axiom: &Axiom,
        instrument_a_idx: usize,
        instrument_b_idx: usize,
        _property: &str,
        value: f32,
    ) -> f32 {
        if instrument_a_idx >= self.instruments.len() || instrument_b_idx >= self.instruments.len()
        {
            return 0.0;
        }
        let a = &self.instruments[instrument_a_idx];
        let b = &self.instruments[instrument_b_idx];
        a.validate_against(b, value)
    }
}

impl Default for ExperimentRunner {
    fn default() -> Self {
        Self::new()
    }
}
