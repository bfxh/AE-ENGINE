use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ModuleRegistry {
    pub modules: HashMap<String, RegisteredModule>,
}

#[derive(Debug, Clone)]
pub struct RegisteredModule {
    pub id: Uuid,
    pub interface_type: String,
    pub mod_name: String,
    pub version: String,
    pub authors: Vec<String>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self { modules: HashMap::new() }
    }

    pub fn register(
        &mut self,
        interface_type: &str,
        mod_name: &str,
        version: &str,
        authors: Vec<String>,
    ) {
        let id = Uuid::new_v4();
        self.modules.insert(
            interface_type.to_string(),
            RegisteredModule {
                id,
                interface_type: interface_type.to_string(),
                mod_name: mod_name.to_string(),
                version: version.to_string(),
                authors,
            },
        );
    }

    pub fn unregister(&mut self, interface_type: &str) -> Option<RegisteredModule> {
        self.modules.remove(interface_type)
    }

    pub fn get_provider(&self, interface_type: &str) -> Option<&RegisteredModule> {
        self.modules.get(interface_type)
    }

    pub fn list_interfaces(&self) -> Vec<&String> {
        self.modules.keys().collect()
    }

    pub fn is_registered(&self, interface_type: &str) -> bool {
        self.modules.contains_key(interface_type)
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
