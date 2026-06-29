use super::axiom::Axiom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetector {
    pub conflicts: Vec<AxiomConflict>,
    pub resolution_history: Vec<ConflictResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomConflict {
    pub id: String,
    pub axiom_a_id: String,
    pub axiom_b_id: String,
    pub conflicting_property: String,
    pub value_a: f32,
    pub value_b: f32,
    pub conflict_type: ConflictType,
    pub status: ConflictStatus,
    pub votes: HashMap<String, Vote>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    DirectContradiction,
    IncompatibleImplications,
    DomainOverlap,
    ExperimentalDisagreement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictStatus {
    Detected,
    UnderReview,
    CommunityVoting,
    Resolved,
    Escalated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Vote {
    SupportA,
    SupportB,
    Synthesize,
    RejectBoth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub conflict_id: String,
    pub resolution_type: ResolutionType,
    pub winning_axiom_id: Option<String>,
    pub synthesized_axiom_id: Option<String>,
    pub rationale: String,
    pub timestamp: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionType {
    AWins,
    BWins,
    Synthesized,
    BothRejected,
    SplitDomain,
    Unresolved,
}

impl ConflictDetector {
    pub fn new() -> Self {
        Self { conflicts: Vec::new(), resolution_history: Vec::new() }
    }

    pub fn detect_conflicts(&mut self, axioms: &[Axiom]) -> Vec<AxiomConflict> {
        let mut new_conflicts = Vec::new();

        for i in 0..axioms.len() {
            for j in (i + 1)..axioms.len() {
                let a = &axioms[i];
                let b = &axioms[j];

                if a.domain != b.domain {
                    continue;
                }

                for (prop, &val_a) in &a.properties {
                    if let Some(&val_b) = b.properties.get(prop) {
                        if (val_a - val_b).abs() > 0.01 {
                            let conflict = AxiomConflict {
                                id: uuid::Uuid::new_v4().to_string(),
                                axiom_a_id: a.id.clone(),
                                axiom_b_id: b.id.clone(),
                                conflicting_property: prop.clone(),
                                value_a: val_a,
                                value_b: val_b,
                                conflict_type: ConflictType::DirectContradiction,
                                status: ConflictStatus::Detected,
                                votes: HashMap::new(),
                            };
                            new_conflicts.push(conflict);
                        }
                    }
                }
            }
        }

        self.conflicts.extend(new_conflicts.clone());
        new_conflicts
    }

    pub fn cast_vote(&mut self, conflict_id: &str, voter: &str, vote: Vote) {
        if let Some(conflict) = self.conflicts.iter_mut().find(|c| c.id == conflict_id) {
            conflict.votes.insert(voter.to_string(), vote);
        }
    }

    pub fn tally_votes(&self, conflict_id: &str) -> Option<ResolutionType> {
        let conflict = self.conflicts.iter().find(|c| c.id == conflict_id)?;
        if conflict.votes.is_empty() {
            return None;
        }

        let mut counts: HashMap<Vote, u32> = HashMap::new();
        for vote in conflict.votes.values() {
            *counts.entry(*vote).or_insert(0) += 1;
        }

        let total = conflict.votes.len() as f32;
        let support_a = *counts.get(&Vote::SupportA).unwrap_or(&0) as f32 / total;
        let support_b = *counts.get(&Vote::SupportB).unwrap_or(&0) as f32 / total;
        let synthesize = *counts.get(&Vote::Synthesize).unwrap_or(&0) as f32 / total;
        let reject = *counts.get(&Vote::RejectBoth).unwrap_or(&0) as f32 / total;

        if support_a >= 0.67 {
            Some(ResolutionType::AWins)
        } else if support_b >= 0.67 {
            Some(ResolutionType::BWins)
        } else if synthesize >= 0.67 {
            Some(ResolutionType::Synthesized)
        } else if reject >= 0.67 {
            Some(ResolutionType::BothRejected)
        } else if support_a + synthesize >= 0.67 {
            Some(ResolutionType::SplitDomain)
        } else {
            None
        }
    }

    pub fn resolve(&mut self, conflict_id: &str) -> Option<ConflictResolution> {
        let resolution_type = self.tally_votes(conflict_id)?;
        let conflict = self.conflicts.iter().find(|c| c.id == conflict_id)?;

        let resolution = ConflictResolution {
            conflict_id: conflict_id.to_string(),
            resolution_type,
            winning_axiom_id: match resolution_type {
                ResolutionType::AWins => Some(conflict.axiom_a_id.clone()),
                ResolutionType::BWins => Some(conflict.axiom_b_id.clone()),
                _ => None,
            },
            synthesized_axiom_id: None,
            rationale: format!("community vote: {:?}", resolution_type),
            timestamp: 0.0,
        };

        self.resolution_history.push(resolution.clone());
        if let Some(conflict) = self.conflicts.iter_mut().find(|c| c.id == conflict_id) {
            conflict.status = ConflictStatus::Resolved;
        }
        Some(resolution)
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axiom::AxiomDomain;

    #[test]
    fn test_conflict_detector_creation() {
        let detector = ConflictDetector::new();
        assert!(detector.conflicts.is_empty());
        assert!(detector.resolution_history.is_empty());
    }

    #[test]
    fn test_conflict_detection() {
        let mut detector = ConflictDetector::new();
        let mut a = Axiom::new("公理A", AxiomDomain::Physics, "爱");
        let mut b = Axiom::new("公理B", AxiomDomain::Physics, "玻");
        a.add_property("speed", 3.0);
        b.add_property("speed", 5.0);
        let conflicts = detector.detect_conflicts(&[a, b]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].value_a, 3.0);
        assert_eq!(conflicts[0].value_b, 5.0);
    }

    #[test]
    fn test_vote_tally() {
        let mut detector = ConflictDetector::new();
        let mut a = Axiom::new("A", AxiomDomain::Physics, "甲");
        let mut b = Axiom::new("B", AxiomDomain::Physics, "乙");
        a.add_property("mass", 1.0);
        b.add_property("mass", 2.0);
        let conflicts = detector.detect_conflicts(&[a, b]);
        let cid = conflicts[0].id.clone();

        detector.cast_vote(&cid, "评委1", Vote::SupportA);
        detector.cast_vote(&cid, "评委2", Vote::SupportA);
        detector.cast_vote(&cid, "评委3", Vote::SupportA);

        let result = detector.tally_votes(&cid);
        assert_eq!(result, Some(ResolutionType::AWins));

        let resolution = detector.resolve(&cid);
        assert!(resolution.is_some());
    }
}
