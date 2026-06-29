//! Mod 实例：管理一个已加载 Mod 的生命周期

use crate::manifest::{ModManifest, ModuleEntry, ModuleType};
use crate::runtime::lua_runtime::{LuaError, LuaRuntime};
use crate::sandbox::SandboxConfig;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

/// Mod 加载状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModState {
    /// 已创建但未加载脚本
    Created,
    /// 脚本已加载
    Loaded,
    /// 已初始化（on_init 已调用）
    Initialized,
    /// 已启用（正在运行）
    Enabled,
    /// 已禁用
    Disabled,
    /// 加载失败
    Error,
}

/// Mod 实例
pub struct ModInstance {
    pub manifest: ModManifest,
    pub base_path: PathBuf,
    pub state: RwLock<ModState>,
    pub lua_runtime: Option<Arc<LuaRuntime>>,
    pub error: RwLock<Option<String>>,
}

impl ModInstance {
    /// 创建新的 Mod 实例
    pub fn new(manifest: ModManifest, base_path: PathBuf) -> Self {
        Self {
            manifest,
            base_path,
            state: RwLock::new(ModState::Created),
            lua_runtime: None,
            error: RwLock::new(None),
        }
    }

    /// 加载 Mod 的所有 Lua 模块
    pub fn load(&mut self, sandbox_config: &SandboxConfig) -> Result<(), ModLoadError> {
        let mod_name = &self.manifest.package.name;

        // 创建 Lua 运行时
        #[allow(clippy::arc_with_non_send_sync)]
        let runtime = Arc::new(LuaRuntime::new(mod_name, sandbox_config.clone())?);

        // 加载所有 Lua 模块
        for module in &self.manifest.modules {
            if module.module_type == ModuleType::Lua {
                self.load_lua_module(&runtime, module)?;
            }
            // Wasm 和 Native 模块暂不支持
        }

        self.lua_runtime = Some(runtime);
        *self.state.write() = ModState::Loaded;
        Ok(())
    }

    /// 加载单个 Lua 模块
    fn load_lua_module(
        &self,
        runtime: &Arc<LuaRuntime>,
        module: &ModuleEntry,
    ) -> Result<(), ModLoadError> {
        let script_path = self.base_path.join(&module.entry);
        let source = std::fs::read_to_string(&script_path).map_err(|e| {
            ModLoadError::Io(format!("failed to read {}: {}", script_path.display(), e))
        })?;

        let chunk_name = format!("{}/{}", self.manifest.package.name, module.entry);
        runtime.load_script(&source, &chunk_name)?;
        log::info!("Loaded Lua module: {}", chunk_name);
        Ok(())
    }

    /// 初始化 Mod（调用 on_init）
    pub fn init(&self) -> Result<(), ModLoadError> {
        if let Some(rt) = &self.lua_runtime {
            rt.call_on_init()?;
            *self.state.write() = ModState::Initialized;
        }
        Ok(())
    }

    /// 启用 Mod
    pub fn enable(&self) {
        *self.state.write() = ModState::Enabled;
        log::info!("Mod enabled: {}", self.manifest.package.name);
    }

    /// 禁用 Mod
    pub fn disable(&self) {
        *self.state.write() = ModState::Disabled;
        log::info!("Mod disabled: {}", self.manifest.package.name);
    }

    /// 每帧更新
    pub fn update(&self, dt: f32) -> Result<(), ModLoadError> {
        if *self.state.read() != ModState::Enabled {
            return Ok(());
        }
        if let Some(rt) = &self.lua_runtime {
            rt.call_on_update(dt)?;
        }
        Ok(())
    }

    /// 卸载 Mod
    pub fn unload(&self) {
        if let Some(rt) = &self.lua_runtime {
            let _ = rt.call_on_unload();
        }
        *self.state.write() = ModState::Disabled;
        log::info!("Mod unloaded: {}", self.manifest.package.name);
    }

    /// 触发事件
    pub fn fire_event(&self, event: &str, args: mlua::MultiValue) -> Result<(), ModLoadError> {
        if let Some(rt) = &self.lua_runtime {
            rt.fire_event(event, args)?;
        }
        Ok(())
    }

    /// 获取当前状态
    pub fn state(&self) -> ModState {
        *self.state.read()
    }

    /// 获取错误信息
    pub fn error(&self) -> Option<String> {
        self.error.read().clone()
    }

    /// 是否有 Lua 模块
    pub fn has_lua(&self) -> bool {
        self.manifest.modules.iter().any(|m| m.module_type == ModuleType::Lua)
    }
}

/// Mod 加载错误
#[derive(Debug)]
pub enum ModLoadError {
    Lua(LuaError),
    Io(String),
}

impl std::fmt::Display for ModLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lua(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ModLoadError {}

impl From<LuaError> for ModLoadError {
    fn from(e: LuaError) -> Self {
        Self::Lua(e)
    }
}

/// Mod 实例管理器
pub struct ModManager {
    instances: Vec<Arc<ModInstance>>,
    sandbox_config: SandboxConfig,
}

impl ModManager {
    pub fn new(sandbox_config: SandboxConfig) -> Self {
        Self { instances: Vec::new(), sandbox_config }
    }

    /// 加载一个 Mod
    pub fn load_mod(
        &mut self,
        manifest: ModManifest,
        base_path: PathBuf,
    ) -> Result<Arc<ModInstance>, ModLoadError> {
        let mut instance = ModInstance::new(manifest, base_path);
        instance.load(&self.sandbox_config)?;
        #[allow(clippy::arc_with_non_send_sync)]
        let instance = Arc::new(instance);
        self.instances.push(instance.clone());
        Ok(instance)
    }

    /// 初始化所有已加载的 Mod
    pub fn init_all(&self) -> Result<(), ModLoadError> {
        for inst in &self.instances {
            inst.init()?;
        }
        Ok(())
    }

    /// 启用所有 Mod
    pub fn enable_all(&self) {
        for inst in &self.instances {
            inst.enable();
        }
    }

    /// 更新所有 Mod
    pub fn update_all(&self, dt: f32) -> Result<(), ModLoadError> {
        for inst in &self.instances {
            inst.update(dt)?;
        }
        Ok(())
    }

    /// 向所有 Mod 广播事件
    pub fn broadcast_event(&self, event: &str, args: mlua::MultiValue) {
        for inst in &self.instances {
            let _ = inst.fire_event(event, args.clone());
        }
    }

    /// 卸载所有 Mod
    pub fn unload_all(&self) {
        for inst in &self.instances {
            inst.unload();
        }
    }

    /// 获取所有实例
    pub fn instances(&self) -> &[Arc<ModInstance>] {
        &self.instances
    }

    /// 获取实例数量
    pub fn len(&self) -> usize {
        self.instances.len()
    }
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ModuleEntry, ModuleType, PackageInfo};

    fn test_manifest() -> ModManifest {
        ModManifest {
            package: PackageInfo {
                name: "test_mod".to_string(),
                version: "0.1.0".to_string(),
                api_version: "1.0".to_string(),
                authors: vec!["test".to_string()],
                description: "test".to_string(),
                license: "MIT".to_string(),
                homepage: None,
                repository: None,
            },
            dependencies: vec![],
            modules: vec![],
            conflicts: vec![],
            provides: vec![],
        }
    }

    fn test_config() -> SandboxConfig {
        SandboxConfig::default()
    }

    #[test]
    fn mod_instance_creation() {
        let inst = ModInstance::new(test_manifest(), PathBuf::from("/tmp"));
        assert_eq!(inst.state(), ModState::Created);
        assert!(!inst.has_lua());
    }

    #[test]
    fn mod_instance_with_lua_module() {
        let mut manifest = test_manifest();
        manifest.modules.push(ModuleEntry {
            interface_type: "test".to_string(),
            module_type: ModuleType::Lua,
            entry: "init.lua".to_string(),
            permissions: vec![],
        });
        let inst = ModInstance::new(manifest, PathBuf::from("/tmp"));
        assert!(inst.has_lua());
    }

    #[test]
    fn mod_manager_creation() {
        let mgr = ModManager::new(test_config());
        assert!(mgr.is_empty());
    }

    #[test]
    fn mod_state_transitions() {
        let inst = ModInstance::new(test_manifest(), PathBuf::from("/tmp"));
        assert_eq!(inst.state(), ModState::Created);
        inst.enable();
        assert_eq!(inst.state(), ModState::Enabled);
        inst.disable();
        assert_eq!(inst.state(), ModState::Disabled);
    }

    #[test]
    fn mod_load_error_display() {
        let e = ModLoadError::Io("file not found".to_string());
        assert!(format!("{e}").contains("file not found"));
    }
}
