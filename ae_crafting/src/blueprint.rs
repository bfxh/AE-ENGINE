use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::socket::{Constraint, FusionRule, Socket};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub sockets: Vec<Socket>,
    pub constraints: Vec<Constraint>,
    pub fusion_rules: Vec<FusionRule>,
    pub required_materials: Vec<(String, f32)>,
    pub tags: Vec<String>,
    pub license: BlueprintLicense,
    pub created_at: u64,
    pub usage_count: u64,
    pub rating: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlueprintLicense {
    MIT,
    CCBY,
    CCBYSA,
    CCBYNC,
    AllRightsReserved,
    Custom(String),
}

impl Blueprint {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: String::new(),
            author: String::new(),
            version: "1.0".to_string(),
            sockets: Vec::new(),
            constraints: Vec::new(),
            fusion_rules: Vec::new(),
            required_materials: Vec::new(),
            tags: Vec::new(),
            license: BlueprintLicense::CCBY,
            created_at: 0,
            usage_count: 0,
            rating: 0.0,
        }
    }

    pub fn add_socket(&mut self, socket: Socket) -> usize {
        let index = self.sockets.len();
        self.sockets.push(socket);
        index
    }

    pub fn required_sockets(&self) -> Vec<usize> {
        self.sockets.iter().enumerate().filter(|(_, s)| s.required).map(|(i, _)| i).collect()
    }

    pub fn validate(&self, filled_sockets: &[Option<usize>]) -> BlueprintValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if filled_sockets.len() != self.sockets.len() {
            errors.push(format!(
                "Socket count mismatch: expected {}, got {}",
                self.sockets.len(),
                filled_sockets.len()
            ));
        }

        for (i, socket) in self.sockets.iter().enumerate() {
            if socket.required && i < filled_sockets.len() && filled_sockets[i].is_none() {
                errors.push(format!("Required socket '{}' (index {}) is empty", socket.label, i));
            }
        }

        for constraint in &self.constraints {
            match constraint.constraint_type {
                crate::socket::ConstraintType::MutuallyExclusive => {
                    let a_filled =
                        filled_sockets.get(constraint.socket_a).and_then(|o| *o).is_some();
                    let b_filled =
                        filled_sockets.get(constraint.socket_b).and_then(|o| *o).is_some();
                    if a_filled && b_filled {
                        errors.push(format!(
                            "Sockets {} and {} are mutually exclusive",
                            constraint.socket_a, constraint.socket_b
                        ));
                    }
                },
                crate::socket::ConstraintType::RequiresB => {
                    let a_filled =
                        filled_sockets.get(constraint.socket_a).and_then(|o| *o).is_some();
                    let b_filled =
                        filled_sockets.get(constraint.socket_b).and_then(|o| *o).is_some();
                    if a_filled && !b_filled {
                        errors.push(format!(
                            "Socket {} requires socket {} to be filled",
                            constraint.socket_a, constraint.socket_b
                        ));
                    }
                },
                crate::socket::ConstraintType::Excludes => {
                    let a_filled =
                        filled_sockets.get(constraint.socket_a).and_then(|o| *o).is_some();
                    let b_filled =
                        filled_sockets.get(constraint.socket_b).and_then(|o| *o).is_some();
                    if a_filled && b_filled {
                        warnings.push(format!(
                            "Socket {} excludes socket {}",
                            constraint.socket_a, constraint.socket_b
                        ));
                    }
                },
                crate::socket::ConstraintType::Synergy => {
                    let a_filled =
                        filled_sockets.get(constraint.socket_a).and_then(|o| *o).is_some();
                    let b_filled =
                        filled_sockets.get(constraint.socket_b).and_then(|o| *o).is_some();
                    if a_filled && b_filled {
                        warnings.push(format!(
                            "Synergy bonus between sockets {} and {}",
                            constraint.socket_a, constraint.socket_b
                        ));
                    }
                },
                _ => {},
            }
        }

        BlueprintValidation { valid: errors.is_empty(), errors, warnings }
    }
}

#[derive(Debug, Clone)]
pub struct BlueprintValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintLibrary {
    pub blueprints: Vec<Blueprint>,
    pub shared_blueprints: Vec<SharedBlueprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedBlueprint {
    pub blueprint: Blueprint,
    pub shared_by: String,
    pub shared_at: u64,
    pub verified: bool,
    pub community_rating: f32,
    pub download_count: u64,
}

impl BlueprintLibrary {
    pub fn new() -> Self {
        Self { blueprints: Vec::new(), shared_blueprints: Vec::new() }
    }

    pub fn add(&mut self, blueprint: Blueprint) {
        self.blueprints.push(blueprint);
    }

    pub fn share(&mut self, blueprint: Blueprint, shared_by: &str, timestamp: u64) {
        self.shared_blueprints.push(SharedBlueprint {
            blueprint,
            shared_by: shared_by.to_string(),
            shared_at: timestamp,
            verified: false,
            community_rating: 0.0,
            download_count: 0,
        });
    }

    pub fn find_by_id(&self, id: Uuid) -> Option<&Blueprint> {
        self.blueprints.iter().find(|b| b.id == id)
    }

    pub fn search_by_tags(&self, tags: &[String]) -> Vec<&Blueprint> {
        self.blueprints.iter().filter(|b| tags.iter().any(|t| b.tags.contains(t))).collect()
    }
}

impl Default for BlueprintLibrary {
    fn default() -> Self {
        Self::new()
    }
}
