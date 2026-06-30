use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    pub package: PackageInfo,
    pub dependencies: Vec<Dependency>,
    pub modules: Vec<ModuleEntry>,
    pub conflicts: Vec<ConflictEntry>,
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub authors: Vec<String>,
    pub description: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleEntry {
    pub interface_type: String,
    pub module_type: ModuleType,
    pub entry: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleType {
    Native,
    Wasm,
    Lua,
    Data,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictEntry {
    pub target: String,
    pub reason: String,
}

impl ModManifest {
    pub fn from_toml(content: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct TomlManifest {
            package: TomlPackage,
            #[serde(default)]
            dependencies: Vec<TomlDependency>,
            #[serde(default)]
            modules: Vec<TomlModule>,
            #[serde(default)]
            conflicts: Vec<TomlConflict>,
            #[serde(default)]
            provides: Vec<String>,
        }

        #[derive(Deserialize)]
        struct TomlPackage {
            name: String,
            version: String,
            api_version: String,
            #[serde(default)]
            authors: Vec<String>,
            #[serde(default)]
            description: String,
            #[serde(default)]
            license: String,
            homepage: Option<String>,
            repository: Option<String>,
        }

        #[derive(Deserialize)]
        struct TomlDependency {
            name: String,
            #[serde(default = "default_version_req")]
            version_req: String,
            #[serde(default)]
            optional: bool,
        }

        fn default_version_req() -> String {
            "*".to_string()
        }

        #[derive(Deserialize)]
        struct TomlModule {
            interface_type: String,
            #[serde(default)]
            module_type: String,
            entry: String,
            #[serde(default)]
            permissions: Vec<String>,
        }

        #[derive(Deserialize)]
        struct TomlConflict {
            target: String,
            #[serde(default)]
            reason: String,
        }

        let tm: TomlManifest =
            toml::from_str(content).map_err(|e| format!("Failed to parse mod.toml: {}", e))?;

        Ok(ModManifest {
            package: PackageInfo {
                name: tm.package.name,
                version: tm.package.version,
                api_version: tm.package.api_version,
                authors: tm.package.authors,
                description: tm.package.description,
                license: tm.package.license,
                homepage: tm.package.homepage,
                repository: tm.package.repository,
            },
            dependencies: tm
                .dependencies
                .into_iter()
                .map(|d| Dependency {
                    name: d.name,
                    version_req: d.version_req,
                    optional: d.optional,
                })
                .collect(),
            modules: tm
                .modules
                .into_iter()
                .map(|m| {
                    let module_type = match m.module_type.as_str() {
                        "wasm" => ModuleType::Wasm,
                        "lua" => ModuleType::Lua,
                        "data" => ModuleType::Data,
                        _ => ModuleType::Native,
                    };
                    ModuleEntry {
                        interface_type: m.interface_type,
                        module_type,
                        entry: m.entry,
                        permissions: m.permissions,
                    }
                })
                .collect(),
            conflicts: tm
                .conflicts
                .into_iter()
                .map(|c| ConflictEntry { target: c.target, reason: c.reason })
                .collect(),
            provides: tm.provides,
        })
    }

    pub fn api_compatible(&self, target_api: &str) -> bool {
        let target_parts: Vec<u32> = target_api.split('.').filter_map(|s| s.parse().ok()).collect();
        let my_parts: Vec<u32> =
            self.package.api_version.split('.').filter_map(|s| s.parse().ok()).collect();

        if target_parts.is_empty() || my_parts.is_empty() {
            return false;
        }

        target_parts[0] == my_parts[0]
    }
}
