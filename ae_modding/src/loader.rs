use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

use crate::manifest::ModManifest;

#[derive(Debug, Clone)]
pub struct ModMetadata {
    pub id: Uuid,
    pub manifest: ModManifest,
    pub path: String,
    pub enabled: bool,
    pub loaded: bool,
    pub load_order: u32,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct ModLoader {
    pub mods: HashMap<String, ModMetadata>,
    pub load_order: Vec<String>,
    pub enabled_order: Vec<String>,
}

impl ModLoader {
    pub fn new() -> Self {
        Self { mods: HashMap::new(), load_order: Vec::new(), enabled_order: Vec::new() }
    }

    pub fn scan_directory(&mut self, base_path: &str) -> Vec<String> {
        let mut discovered = Vec::new();

        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let mod_toml = path.join("mod.toml");
                    if mod_toml.exists() {
                        if let Ok(content) = std::fs::read_to_string(&mod_toml) {
                            match ModManifest::from_toml(&content) {
                                Ok(manifest) => {
                                    let name = manifest.package.name.clone();
                                    let path_str = path.to_string_lossy().to_string();

                                    self.mods.insert(
                                        name.clone(),
                                        ModMetadata {
                                            id: Uuid::new_v4(),
                                            manifest,
                                            path: path_str,
                                            enabled: false,
                                            loaded: false,
                                            load_order: 0,
                                            error: None,
                                        },
                                    );

                                    discovered.push(name);
                                },
                                Err(e) => {
                                    log::warn!("Failed to parse mod.toml in {:?}: {}", path, e);
                                },
                            }
                        }
                    }
                }
            }
        }

        discovered
    }

    pub fn resolve_dependencies(&mut self) -> Result<Vec<String>, Vec<(String, String)>> {
        let mut errors = Vec::new();
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();

        for (name, meta) in &self.mods {
            in_degree.entry(name.as_str()).or_insert(0);
            let deps: Vec<&str> = meta
                .manifest
                .dependencies
                .iter()
                .filter(|d| !d.optional)
                .map(|d| d.name.as_str())
                .collect();

            for dep in &deps {
                if !self.mods.contains_key(*dep) {
                    errors.push((name.clone(), dep.to_string()));
                }
            }

            graph.insert(name.as_str(), deps);
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        for deps in graph.values() {
            for dep in deps {
                *in_degree.entry(dep).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> =
            in_degree.iter().filter(|(_, &deg)| deg == 0).map(|(&name, _)| name).collect();

        let mut sorted = Vec::new();

        while let Some(name) = queue.pop_front() {
            sorted.push(name.to_string());

            if let Some(deps) = graph.get(name) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        if sorted.len() != self.mods.len() {
            let cycle_nodes: Vec<String> = in_degree
                .iter()
                .filter(|(_, &deg)| deg > 0)
                .map(|(&name, _)| name.to_string())
                .collect();
            return Err(cycle_nodes
                .into_iter()
                .map(|n| (n, "dependency cycle detected".to_string()))
                .collect());
        }

        for (order, name) in sorted.iter().enumerate() {
            if let Some(meta) = self.mods.get_mut(name.as_str()) {
                meta.load_order = order as u32;
            }
        }

        self.load_order = sorted.clone();
        Ok(sorted)
    }

    pub fn enable_mod(&mut self, name: &str) -> Result<(), String> {
        let meta = self.mods.get_mut(name).ok_or_else(|| format!("Mod '{}' not found", name))?;

        if !meta.enabled {
            meta.enabled = true;
            self.enabled_order.push(name.to_string());
        }

        Ok(())
    }

    pub fn disable_mod(&mut self, name: &str) -> Result<(), String> {
        let meta = self.mods.get_mut(name).ok_or_else(|| format!("Mod '{}' not found", name))?;

        meta.enabled = false;
        meta.loaded = false;
        self.enabled_order.retain(|n| n != name);

        Ok(())
    }

    pub fn get_enabled(&self) -> Vec<&ModMetadata> {
        self.enabled_order.iter().filter_map(|name| self.mods.get(name)).collect()
    }

    pub fn get_all(&self) -> Vec<&ModMetadata> {
        self.load_order.iter().filter_map(|name| self.mods.get(name)).collect()
    }
}

impl Default for ModLoader {
    fn default() -> Self {
        Self::new()
    }
}
