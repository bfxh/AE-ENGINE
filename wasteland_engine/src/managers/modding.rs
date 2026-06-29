use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;

use wasteland_modding::prelude::*;

pub struct ModdingManager {
    pub loader: Arc<Mutex<ModLoader>>,
    pub mod_manager: Arc<Mutex<ModManager>>,
    pub enabled: bool,
}

impl std::fmt::Debug for ModdingManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModdingManager")
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

impl ModdingManager {
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new() -> Self {
        Self {
            loader: Arc::new(Mutex::new(ModLoader::new())),
            mod_manager: Arc::new(Mutex::new(ModManager::new(SandboxConfig::default()))),
            enabled: true,
        }
    }

    pub fn scan_mods(&self, dir: &Path) -> Vec<String> {
        if !self.enabled || !dir.exists() {
            return Vec::new();
        }
        let mut loader = self.loader.lock();
        loader.scan_directory(&dir.to_string_lossy())
    }

    pub fn resolve_dependencies(&self) -> Result<Vec<String>, Vec<(String, String)>> {
        let mut loader = self.loader.lock();
        loader.resolve_dependencies()
    }

    pub fn enable_mod(&self, name: &str) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let mut loader = self.loader.lock();
        loader.enable_mod(name)
    }

    pub fn load_enabled_mods(&self) -> Result<usize, ModLoadError> {
        if !self.enabled {
            return Ok(0);
        }
        let mods_to_load: Vec<(ModManifest, std::path::PathBuf)> = {
            let loader = self.loader.lock();
            loader
                .get_enabled()
                .into_iter()
                .map(|m| (m.manifest.clone(), m.path.clone().into()))
                .collect()
        };

        let mut count = 0usize;
        for (manifest, base_path) in mods_to_load {
            let mut mgr = self.mod_manager.lock();
            mgr.load_mod(manifest, base_path)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn init_all(&self) -> Result<(), ModLoadError> {
        if !self.enabled {
            return Ok(());
        }
        let mgr = self.mod_manager.lock();
        mgr.init_all()
    }

    pub fn enable_all(&self) {
        if !self.enabled {
            return;
        }
        let mgr = self.mod_manager.lock();
        mgr.enable_all();
    }

    pub fn update(&self, dt: f32) {
        if !self.enabled {
            return;
        }
        let mgr = self.mod_manager.lock();
        let _ = mgr.update_all(dt);
    }

    pub fn unload_all(&self) {
        let mgr = self.mod_manager.lock();
        mgr.unload_all();
    }

    pub fn mod_count(&self) -> usize {
        self.mod_manager.lock().len()
    }

    pub fn discovered_count(&self) -> usize {
        self.loader.lock().mods.len()
    }
}

impl Default for ModdingManager {
    fn default() -> Self {
        Self::new()
    }
}
