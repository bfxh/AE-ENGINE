use crate::manifest::ModManifest;

#[derive(Debug, Clone)]
pub struct ConflictReport {
    pub conflicts: Vec<ModConflict>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModConflict {
    pub mod_a: String,
    pub mod_b: String,
    pub conflict_type: ConflictType,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    InterfaceConflict,
    ResourceConflict,
    EventHandlerConflict,
    ExplicitConflict,
}

#[derive(Debug)]
pub struct ConflictDetector;

impl ConflictDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect(&self, manifests: &[(String, ModManifest)]) -> ConflictReport {
        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();

        for i in 0..manifests.len() {
            for j in (i + 1)..manifests.len() {
                let (name_a, manifest_a) = &manifests[i];
                let (name_b, manifest_b) = &manifests[j];

                for conflict in &manifest_a.conflicts {
                    if conflict.target == *name_b {
                        conflicts.push(ModConflict {
                            mod_a: name_a.clone(),
                            mod_b: name_b.clone(),
                            conflict_type: ConflictType::ExplicitConflict,
                            description: conflict.reason.clone(),
                        });
                    }
                }

                for conflict in &manifest_b.conflicts {
                    if conflict.target == *name_a {
                        conflicts.push(ModConflict {
                            mod_a: name_b.clone(),
                            mod_b: name_a.clone(),
                            conflict_type: ConflictType::ExplicitConflict,
                            description: conflict.reason.clone(),
                        });
                    }
                }

                let modules_a: Vec<&str> =
                    manifest_a.modules.iter().map(|m| m.interface_type.as_str()).collect();
                let modules_b: Vec<&str> =
                    manifest_b.modules.iter().map(|m| m.interface_type.as_str()).collect();

                for iface in &modules_a {
                    if modules_b.contains(iface) {
                        conflicts.push(ModConflict {
                            mod_a: name_a.clone(),
                            mod_b: name_b.clone(),
                            conflict_type: ConflictType::InterfaceConflict,
                            description: format!("Both mods provide interface '{}'", iface),
                        });
                    }
                }

                let provides_a: Vec<&str> =
                    manifest_a.provides.iter().map(|s| s.as_str()).collect();
                let provides_b: Vec<&str> =
                    manifest_b.provides.iter().map(|s| s.as_str()).collect();

                for res in &provides_a {
                    if provides_b.contains(res) {
                        warnings.push(format!(
                            "Resource '{}' provided by both '{}' and '{}'",
                            res, name_a, name_b
                        ));
                    }
                }
            }
        }

        ConflictReport { conflicts, warnings }
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}
