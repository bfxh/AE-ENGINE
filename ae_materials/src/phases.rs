use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrystalStructure {
    BCC,
    FCC,
    HCP,
    BCT,
    Diamond,
    Hexagonal,
    Rhombohedral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialPhase {
    Ferrite,
    Austenite,
    Martensite,
    Pearlite,
    Bainite,
    Cementite,
    Graphite,
    Ledeburite,
    Spheroidite,
    TemperedMartensite,
}

impl MaterialPhase {
    pub fn crystal_structure(&self) -> CrystalStructure {
        match self {
            MaterialPhase::Ferrite => CrystalStructure::BCC,
            MaterialPhase::Austenite => CrystalStructure::FCC,
            MaterialPhase::Martensite => CrystalStructure::BCT,
            MaterialPhase::Pearlite => CrystalStructure::BCC,
            MaterialPhase::Bainite => CrystalStructure::BCC,
            MaterialPhase::Cementite => CrystalStructure::Rhombohedral,
            MaterialPhase::Graphite => CrystalStructure::Hexagonal,
            MaterialPhase::Ledeburite => CrystalStructure::BCC,
            MaterialPhase::Spheroidite => CrystalStructure::BCC,
            MaterialPhase::TemperedMartensite => CrystalStructure::BCT,
        }
    }

    pub fn base_hardness(&self) -> f32 {
        match self {
            MaterialPhase::Ferrite => 80.0,
            MaterialPhase::Austenite => 200.0,
            MaterialPhase::Martensite => 800.0,
            MaterialPhase::Pearlite => 250.0,
            MaterialPhase::Bainite => 400.0,
            MaterialPhase::Cementite => 900.0,
            MaterialPhase::Graphite => 10.0,
            MaterialPhase::Ledeburite => 600.0,
            MaterialPhase::Spheroidite => 200.0,
            MaterialPhase::TemperedMartensite => 500.0,
        }
    }

    pub fn base_toughness(&self) -> f32 {
        match self {
            MaterialPhase::Ferrite => 200.0,
            MaterialPhase::Austenite => 180.0,
            MaterialPhase::Martensite => 15.0,
            MaterialPhase::Pearlite => 50.0,
            MaterialPhase::Bainite => 80.0,
            MaterialPhase::Cementite => 5.0,
            MaterialPhase::Graphite => 1.0,
            MaterialPhase::Ledeburite => 10.0,
            MaterialPhase::Spheroidite => 100.0,
            MaterialPhase::TemperedMartensite => 40.0,
        }
    }

    pub fn formation_temp(&self) -> f32 {
        match self {
            MaterialPhase::Ferrite => 300.0,
            MaterialPhase::Austenite => 1000.0,
            MaterialPhase::Martensite => 500.0,
            MaterialPhase::Pearlite => 800.0,
            MaterialPhase::Bainite => 600.0,
            MaterialPhase::Cementite => 1400.0,
            MaterialPhase::Graphite => 1200.0,
            MaterialPhase::Ledeburite => 1300.0,
            MaterialPhase::Spheroidite => 800.0,
            MaterialPhase::TemperedMartensite => 600.0,
        }
    }
}
