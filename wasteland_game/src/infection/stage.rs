//! 感染阶段

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfectionStage {
    Latent,
    Initial,
    Progressive,
    Terminal,
}

impl InfectionStage {
    pub fn from_infection_level(level: f32) -> Self {
        if level < 0.25 { Self::Latent }
        else if level < 0.5 { Self::Initial }
        else if level < 0.75 { Self::Progressive }
        else { Self::Terminal }
    }

    pub fn hp_penalty(&self) -> f32 {
        match self {
            Self::Latent => 0.0,
            Self::Initial => 0.05,
            Self::Progressive => 0.20,
            Self::Terminal => 0.50,
        }
    }

    pub fn speed_penalty(&self) -> f32 {
        match self {
            Self::Latent => 0.0,
            Self::Initial => 0.10,
            Self::Progressive => 0.30,
            Self::Terminal => 0.60,
        }
    }

    pub fn is_reversible(&self) -> bool {
        !matches!(self, Self::Terminal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_from_level() {
        assert_eq!(InfectionStage::from_infection_level(0.1), InfectionStage::Latent);
        assert_eq!(InfectionStage::from_infection_level(0.3), InfectionStage::Initial);
        assert_eq!(InfectionStage::from_infection_level(0.6), InfectionStage::Progressive);
        assert_eq!(InfectionStage::from_infection_level(0.9), InfectionStage::Terminal);
    }

    #[test]
    fn test_stage_penalties() {
        assert_eq!(InfectionStage::Latent.hp_penalty(), 0.0);
        assert!(InfectionStage::Terminal.hp_penalty() > InfectionStage::Initial.hp_penalty());
        assert!(InfectionStage::Latent.is_reversible());
        assert!(!InfectionStage::Terminal.is_reversible());
    }
}
