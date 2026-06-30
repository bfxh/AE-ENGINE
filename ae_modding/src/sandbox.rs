use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub memory_limit_mb: u64,
    pub fuel_limit: u64,
    pub allowed_dirs: Vec<String>,
    pub network_allowed: bool,
    pub file_system_allowed: bool,
    pub max_execution_time_ms: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: 64,
            fuel_limit: 100_000,
            allowed_dirs: Vec::new(),
            network_allowed: false,
            file_system_allowed: false,
            max_execution_time_ms: 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxType {
    Wasm,
    Lua,
    Native,
}

#[derive(Debug, Clone)]
pub struct SandboxManager {
    pub wasm_config: SandboxConfig,
    pub lua_config: SandboxConfig,
    pub native_allowed: bool,
}

impl SandboxManager {
    pub fn new() -> Self {
        Self {
            wasm_config: SandboxConfig {
                memory_limit_mb: 64,
                fuel_limit: 100_000,
                ..Default::default()
            },
            lua_config: SandboxConfig {
                memory_limit_mb: 16,
                fuel_limit: 10_000,
                ..Default::default()
            },
            native_allowed: false,
        }
    }

    pub fn is_sandbox_type_allowed(&self, sandbox_type: SandboxType) -> bool {
        match sandbox_type {
            SandboxType::Native => self.native_allowed,
            _ => true,
        }
    }

    pub fn get_config(&self, sandbox_type: SandboxType) -> SandboxConfig {
        match sandbox_type {
            SandboxType::Wasm => self.wasm_config.clone(),
            SandboxType::Lua => self.lua_config.clone(),
            SandboxType::Native => SandboxConfig::default(),
        }
    }

    pub fn set_memory_limit(&mut self, sandbox_type: SandboxType, limit_mb: u64) {
        match sandbox_type {
            SandboxType::Wasm => self.wasm_config.memory_limit_mb = limit_mb,
            SandboxType::Lua => self.lua_config.memory_limit_mb = limit_mb,
            SandboxType::Native => {},
        }
    }

    pub fn set_fuel_limit(&mut self, sandbox_type: SandboxType, limit: u64) {
        match sandbox_type {
            SandboxType::Wasm => self.wasm_config.fuel_limit = limit,
            SandboxType::Lua => self.lua_config.fuel_limit = limit,
            SandboxType::Native => {},
        }
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_defaults() {
        let manager = SandboxManager::new();

        assert!(!manager.is_sandbox_type_allowed(SandboxType::Native));
        assert!(manager.is_sandbox_type_allowed(SandboxType::Wasm));
        assert!(manager.is_sandbox_type_allowed(SandboxType::Lua));

        assert_eq!(manager.wasm_config.memory_limit_mb, 64);
        assert_eq!(manager.lua_config.memory_limit_mb, 16);
    }

    #[test]
    fn test_sandbox_config_update() {
        let mut manager = SandboxManager::new();
        manager.set_memory_limit(SandboxType::Wasm, 128);
        manager.set_fuel_limit(SandboxType::Lua, 50_000);

        assert_eq!(manager.wasm_config.memory_limit_mb, 128);
        assert_eq!(manager.lua_config.fuel_limit, 50_000);
    }
}
