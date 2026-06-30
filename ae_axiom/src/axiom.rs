use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Axiom {
    pub id: String,
    pub name: String,
    pub domain: AxiomDomain,
    pub properties: HashMap<String, f32>,
    pub relations: Vec<AxiomRelation>,
    pub declared_by: String,
    pub version: u32,
    pub status: AxiomStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AxiomDomain {
    Physics,
    Chemistry,
    Biology,
    Geology,
    Meteorology,
    MaterialScience,
    Electromagnetism,
    Optics,
    Acoustics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomRelation {
    pub relation_type: RelationType,
    pub target_property: String,
    pub formula: RelationFormula,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    Proportional,
    InverseProportional,
    Exponential,
    Logarithmic,
    Threshold,
    Periodic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationFormula {
    Linear { slope: f32, intercept: f32 },
    Power { coefficient: f32, exponent: f32 },
    Logistic { midpoint: f32, steepness: f32, maximum: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AxiomStatus {
    Proposed,
    Verified,
    Accepted,
    Contested,
    Rejected,
    Deprecated,
}

impl Axiom {
    pub fn new(name: &str, domain: AxiomDomain, declared_by: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            domain,
            properties: HashMap::new(),
            relations: Vec::new(),
            declared_by: declared_by.to_string(),
            version: 1,
            status: AxiomStatus::Proposed,
        }
    }

    pub fn add_property(&mut self, key: &str, value: f32) {
        self.properties.insert(key.to_string(), value);
    }

    pub fn add_relation(&mut self, relation: AxiomRelation) {
        self.relations.push(relation);
    }

    pub fn evaluate_relation(&self, target_property: &str, input_value: f32) -> Option<f32> {
        let relation = self.relations.iter().find(|r| r.target_property == target_property)?;
        match &relation.formula {
            RelationFormula::Linear { slope, intercept } => Some(slope * input_value + intercept),
            RelationFormula::Power { coefficient, exponent } => {
                Some(coefficient * input_value.powf(*exponent))
            },
            RelationFormula::Logistic { midpoint, steepness, maximum } => {
                let exp = (-steepness * (input_value - midpoint)).exp();
                Some(maximum / (1.0 + exp))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axiom_creation() {
        let axiom = Axiom::new("重力常数", AxiomDomain::Physics, "牛顿");
        assert_eq!(axiom.name, "重力常数");
        assert_eq!(axiom.domain, AxiomDomain::Physics);
        assert_eq!(axiom.declared_by, "牛顿");
        assert_eq!(axiom.version, 1);
        assert_eq!(axiom.status, AxiomStatus::Proposed);
    }

    #[test]
    fn test_axiom_properties_and_relations() {
        let mut axiom = Axiom::new("密度", AxiomDomain::MaterialScience, "阿基米德");
        axiom.add_property("mass", 10.0);
        axiom.add_property("volume", 2.0);
        assert_eq!(axiom.properties.get("mass"), Some(&10.0));
        assert_eq!(axiom.properties.get("volume"), Some(&2.0));

        axiom.add_relation(AxiomRelation {
            relation_type: RelationType::Proportional,
            target_property: "density".to_string(),
            formula: RelationFormula::Linear { slope: 2.0, intercept: 0.0 },
        });
        let result = axiom.evaluate_relation("density", 5.0);
        assert_eq!(result, Some(10.0));
    }

    #[test]
    fn test_axiom_formula_evaluation() {
        let mut axiom = Axiom::new("测试", AxiomDomain::Physics, "测试者");
        axiom.add_relation(AxiomRelation {
            relation_type: RelationType::Exponential,
            target_property: "power".to_string(),
            formula: RelationFormula::Power { coefficient: 3.0, exponent: 2.0 },
        });
        assert_eq!(axiom.evaluate_relation("power", 4.0), Some(48.0));
        assert_eq!(axiom.evaluate_relation("nonexistent", 1.0), None);
    }
}
