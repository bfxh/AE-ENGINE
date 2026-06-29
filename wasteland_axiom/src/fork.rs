use super::axiom::{Axiom, AxiomDomain};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomFork {
    pub id: String,
    pub name: String,
    pub parent_axiom_ids: Vec<String>,
    pub base_domain: AxiomDomain,
    pub axioms: Vec<Axiom>,
    pub faction: String,
    pub dominance: f32,
    pub follower_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkManager {
    pub forks: Vec<AxiomFork>,
    pub active_fork_id: Option<String>,
}

impl ForkManager {
    pub fn new() -> Self {
        Self { forks: Vec::new(), active_fork_id: None }
    }

    pub fn create_fork(&mut self, name: &str, domain: AxiomDomain, faction: &str) -> &AxiomFork {
        let fork = AxiomFork {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            parent_axiom_ids: Vec::new(),
            base_domain: domain,
            axioms: Vec::new(),
            faction: faction.to_string(),
            dominance: 1.0,
            follower_count: 0,
        };
        self.forks.push(fork);
        self.forks.last().unwrap()
    }

    pub fn add_axiom_to_fork(&mut self, fork_id: &str, axiom: Axiom) {
        if let Some(fork) = self.forks.iter_mut().find(|f| f.id == fork_id) {
            fork.axioms.push(axiom);
        }
    }

    pub fn fork_from_axioms(
        &mut self,
        name: &str,
        axioms: &[Axiom],
        faction: &str,
    ) -> Option<&AxiomFork> {
        if axioms.is_empty() {
            return None;
        }
        let domain = axioms[0].domain;
        let fork = AxiomFork {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            parent_axiom_ids: axioms.iter().map(|a| a.id.clone()).collect(),
            base_domain: domain,
            axioms: axioms.to_vec(),
            faction: faction.to_string(),
            dominance: 1.0,
            follower_count: 0,
        };
        self.forks.push(fork);
        self.forks.last()
    }

    pub fn combat_effectiveness(&self, fork_id: &str, environment: &EnvironmentConditions) -> f32 {
        let fork = match self.forks.iter().find(|f| f.id == fork_id) {
            Some(f) => f,
            None => return 0.0,
        };

        let mut effectiveness = fork.dominance;

        for axiom in &fork.axioms {
            if let Some(&temp) = axiom.properties.get("optimal_temperature") {
                let temp_diff = (environment.temperature - temp).abs();
                effectiveness *= 1.0 / (1.0 + temp_diff * 0.01);
            }
            if let Some(&pressure) = axiom.properties.get("optimal_pressure") {
                let press_diff = (environment.pressure - pressure).abs();
                effectiveness *= 1.0 / (1.0 + press_diff * 0.0001);
            }
        }

        effectiveness * (1.0 + fork.follower_count as f32 * 0.001)
    }

    pub fn most_effective_fork(&self, environment: &EnvironmentConditions) -> Option<&AxiomFork> {
        self.forks.iter().max_by(|a, b| {
            let eff_a = self.combat_effectiveness(&a.id, environment);
            let eff_b = self.combat_effectiveness(&b.id, environment);
            eff_a.partial_cmp(&eff_b).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn update_dominance(&mut self) {
        let total_followers: u32 = self.forks.iter().map(|f| f.follower_count).sum();
        for fork in &mut self.forks {
            if total_followers > 0 {
                fork.dominance = fork.follower_count as f32 / total_followers as f32;
            }
        }
    }
}

impl Default for ForkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConditions {
    pub temperature: f32,
    pub pressure: f32,
    pub humidity: f32,
    pub radiation: f32,
    pub gravity: f32,
    pub magnetic_field: f32,
}
