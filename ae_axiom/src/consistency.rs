use super::axiom::Axiom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyChecker {
    pub checks: Vec<ConsistencyCheck>,
    pub tolerance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyCheck {
    pub name: String,
    pub property_a: String,
    pub property_b: String,
    pub expected_relation: ExpectedRelation,
    pub check_fn_id: CheckFunction,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExpectedRelation {
    Positive,
    Negative,
    WithinRange(f32, f32),
    EqualWithin(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckFunction {
    ElectronegativityRadius,
    DensityAtomicMass,
    MeltingBondEnergy,
    ReactivityElectronegativity,
    ThermalExpansionSpecificHeat,
    HardnessBondEnergy,
    Custom,
}

impl Default for ConsistencyChecker {
    fn default() -> Self {
        let mut checker = Self { checks: Vec::new(), tolerance: 0.1 };
        checker.register_builtin_checks();
        checker
    }
}

impl ConsistencyChecker {
    pub fn register_builtin_checks(&mut self) {
        self.checks.push(ConsistencyCheck {
            name: "electronegativity_vs_atomic_radius".to_string(),
            property_a: "electronegativity".to_string(),
            property_b: "atomic_radius".to_string(),
            expected_relation: ExpectedRelation::Negative,
            check_fn_id: CheckFunction::ElectronegativityRadius,
        });

        self.checks.push(ConsistencyCheck {
            name: "density_vs_atomic_mass".to_string(),
            property_a: "density".to_string(),
            property_b: "atomic_mass".to_string(),
            expected_relation: ExpectedRelation::Positive,
            check_fn_id: CheckFunction::DensityAtomicMass,
        });

        self.checks.push(ConsistencyCheck {
            name: "melting_point_vs_bond_energy".to_string(),
            property_a: "melting_point".to_string(),
            property_b: "bond_energy".to_string(),
            expected_relation: ExpectedRelation::Positive,
            check_fn_id: CheckFunction::MeltingBondEnergy,
        });

        self.checks.push(ConsistencyCheck {
            name: "reactivity_vs_electronegativity".to_string(),
            property_a: "reactivity".to_string(),
            property_b: "electronegativity".to_string(),
            expected_relation: ExpectedRelation::Positive,
            check_fn_id: CheckFunction::ReactivityElectronegativity,
        });

        self.checks.push(ConsistencyCheck {
            name: "thermal_expansion_vs_specific_heat".to_string(),
            property_a: "thermal_expansion".to_string(),
            property_b: "specific_heat".to_string(),
            expected_relation: ExpectedRelation::Positive,
            check_fn_id: CheckFunction::ThermalExpansionSpecificHeat,
        });

        self.checks.push(ConsistencyCheck {
            name: "hardness_vs_bond_energy".to_string(),
            property_a: "hardness".to_string(),
            property_b: "bond_energy".to_string(),
            expected_relation: ExpectedRelation::Positive,
            check_fn_id: CheckFunction::HardnessBondEnergy,
        });
    }

    pub fn add_custom_check(&mut self, check: ConsistencyCheck) {
        self.checks.push(check);
    }

    pub fn verify(&self, axiom: &Axiom) -> Vec<ConsistencyViolation> {
        let mut violations = Vec::new();

        for check in &self.checks {
            let value_a = match axiom.properties.get(&check.property_a) {
                Some(v) => *v,
                None => continue,
            };
            let value_b = match axiom.properties.get(&check.property_b) {
                Some(v) => *v,
                None => continue,
            };

            let violation = match &check.expected_relation {
                ExpectedRelation::Positive => {
                    if value_a * value_b < 0.0 {
                        Some(ConsistencyViolation {
                            check_name: check.name.clone(),
                            property_a: check.property_a.clone(),
                            value_a,
                            property_b: check.property_b.clone(),
                            value_b,
                            expected: "positive correlation".to_string(),
                            actual: format!("a={}, b={}", value_a, value_b),
                            severity: ViolationSeverity::Warning,
                        })
                    } else {
                        None
                    }
                },
                ExpectedRelation::Negative => {
                    if value_a * value_b > 0.0 {
                        Some(ConsistencyViolation {
                            check_name: check.name.clone(),
                            property_a: check.property_a.clone(),
                            value_a,
                            property_b: check.property_b.clone(),
                            value_b,
                            expected: "negative correlation".to_string(),
                            actual: format!("a={}, b={}", value_a, value_b),
                            severity: ViolationSeverity::Warning,
                        })
                    } else {
                        None
                    }
                },
                ExpectedRelation::WithinRange(min, max) => {
                    let ratio = if value_b != 0.0 { value_a / value_b } else { f32::MAX };
                    if ratio < *min || ratio > *max {
                        Some(ConsistencyViolation {
                            check_name: check.name.clone(),
                            property_a: check.property_a.clone(),
                            value_a,
                            property_b: check.property_b.clone(),
                            value_b,
                            expected: format!("ratio in [{}, {}]", min, max),
                            actual: format!("ratio = {}", ratio),
                            severity: ViolationSeverity::Error,
                        })
                    } else {
                        None
                    }
                },
                ExpectedRelation::EqualWithin(epsilon) => {
                    if (value_a - value_b).abs() > *epsilon {
                        Some(ConsistencyViolation {
                            check_name: check.name.clone(),
                            property_a: check.property_a.clone(),
                            value_a,
                            property_b: check.property_b.clone(),
                            value_b,
                            expected: format!("equal within ±{}", epsilon),
                            actual: format!("diff = {}", (value_a - value_b).abs()),
                            severity: ViolationSeverity::Error,
                        })
                    } else {
                        None
                    }
                },
            };

            if let Some(v) = violation {
                violations.push(v);
            }
        }

        violations
    }

    pub fn is_self_consistent(&self, axiom: &Axiom) -> bool {
        self.verify(axiom).iter().all(|v| v.severity != ViolationSeverity::Error)
    }

    pub fn consistency_score(&self, axiom: &Axiom) -> f32 {
        let violations = self.verify(axiom);
        if violations.is_empty() {
            return 1.0;
        }
        let errors = violations.iter().filter(|v| v.severity == ViolationSeverity::Error).count();
        let warnings =
            violations.iter().filter(|v| v.severity == ViolationSeverity::Warning).count();
        (1.0 - errors as f32 * 0.3 - warnings as f32 * 0.1).max(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyViolation {
    pub check_name: String,
    pub property_a: String,
    pub value_a: f32,
    pub property_b: String,
    pub value_b: f32,
    pub expected: String,
    pub actual: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Warning,
    Error,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axiom::AxiomDomain;

    #[test]
    fn test_consistency_checker_creation() {
        let checker = ConsistencyChecker::default();
        assert!(checker.checks.len() >= 6);
        assert_eq!(checker.tolerance, 0.1);
    }

    #[test]
    fn test_self_consistent_axiom() {
        let checker = ConsistencyChecker::default();
        let mut axiom = Axiom::new("测试", AxiomDomain::Chemistry, "测试者");
        axiom.add_property("electronegativity", 1.0);
        axiom.add_property("atomic_radius", -1.0);
        axiom.add_property("density", 5.0);
        axiom.add_property("atomic_mass", 10.0);
        assert!(checker.is_self_consistent(&axiom));
        assert_eq!(checker.consistency_score(&axiom), 1.0);
    }

    #[test]
    fn test_consistency_violation_detection() {
        let checker = ConsistencyChecker::default();
        let mut axiom = Axiom::new("测试", AxiomDomain::Chemistry, "测试者");
        axiom.add_property("electronegativity", 1.0);
        axiom.add_property("atomic_radius", 1.0);
        let violations = checker.verify(&axiom);
        assert!(!violations.is_empty());
        let score = checker.consistency_score(&axiom);
        assert!(score < 1.0);
    }
}
