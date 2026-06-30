use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    Mesh,
    Texture,
    Material,
    Skeleton,
    Animation,
    Sound,
    Shader,
    Script,
    Config,
    Prefab,
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetState {
    Unloaded,
    Loading,
    Loaded,
    Failed,
    Unloading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMeta {
    pub id: u64,
    pub path: PathBuf,
    pub asset_type: AssetType,
    pub size_bytes: u64,
    pub dependencies: Vec<u64>,
    pub checksum: u64,
    pub version: u32,
    pub compressed: bool,
}

#[derive(Debug, Clone)]
pub struct AssetEntry {
    pub meta: AssetMeta,
    pub state: AssetState,
    pub data: Option<Vec<u8>>,
    pub ref_count: u32,
    pub last_access: u64,
}

#[derive(Debug, Clone)]
pub struct AssetLoader {
    assets: HashMap<u64, AssetEntry>,
    path_index: HashMap<PathBuf, u64>,
    type_index: HashMap<AssetType, Vec<u64>>,
    load_queue: Vec<u64>,
    #[allow(dead_code)]
    max_concurrent: usize,
    active_loads: usize,
    next_id: u64,
}

impl AssetLoader {
    pub fn new() -> Self {
        AssetLoader {
            assets: HashMap::new(),
            path_index: HashMap::new(),
            type_index: HashMap::new(),
            load_queue: Vec::new(),
            max_concurrent: 4,
            active_loads: 0,
            next_id: 1,
        }
    }

    pub fn register(&mut self, mut meta: AssetMeta) -> u64 {
        if meta.id == 0 {
            meta.id = self.next_id;
            self.next_id += 1;
        }
        let id = meta.id;
        self.path_index.insert(meta.path.clone(), id);
        self.type_index.entry(meta.asset_type).or_default().push(id);
        self.assets.insert(
            id,
            AssetEntry {
                meta,
                state: AssetState::Unloaded,
                data: None,
                ref_count: 0,
                last_access: 0,
            },
        );
        id
    }

    pub fn request_load(&mut self, id: u64) -> bool {
        if let Some(entry) = self.assets.get_mut(&id) {
            if entry.state == AssetState::Unloaded || entry.state == AssetState::Failed {
                entry.state = AssetState::Loading;
                self.load_queue.push(id);
                return true;
            }
        }
        false
    }

    pub fn load_dependencies(&mut self, id: u64) {
        if let Some(entry) = self.assets.get(&id) {
            let deps: Vec<u64> = entry.meta.dependencies.clone();
            for dep_id in deps {
                self.request_load(dep_id);
            }
        }
    }

    pub fn complete_load(&mut self, id: u64, data: Vec<u8>) {
        if let Some(entry) = self.assets.get_mut(&id) {
            entry.data = Some(data);
            entry.state = AssetState::Loaded;
            entry.ref_count = 1;
        }
        self.active_loads = self.active_loads.saturating_sub(1);
    }

    pub fn fail_load(&mut self, id: u64) {
        if let Some(entry) = self.assets.get_mut(&id) {
            entry.state = AssetState::Failed;
        }
        self.active_loads = self.active_loads.saturating_sub(1);
    }

    pub fn get_data(&self, id: u64) -> Option<&[u8]> {
        self.assets.get(&id).and_then(|e| e.data.as_deref())
    }

    pub fn add_ref(&mut self, id: u64) {
        if let Some(entry) = self.assets.get_mut(&id) {
            entry.ref_count += 1;
        }
    }

    pub fn release(&mut self, id: u64) -> bool {
        if let Some(entry) = self.assets.get_mut(&id) {
            entry.ref_count = entry.ref_count.saturating_sub(1);
            if entry.ref_count == 0 {
                entry.state = AssetState::Unloading;
                entry.data = None;
                entry.state = AssetState::Unloaded;
                return true;
            }
        }
        false
    }

    pub fn query_by_type(&self, asset_type: AssetType) -> Vec<u64> {
        self.type_index.get(&asset_type).cloned().unwrap_or_default()
    }

    pub fn query_by_path(&self, path: &PathBuf) -> Option<u64> {
        self.path_index.get(path).copied()
    }

    pub fn state(&self, id: u64) -> Option<AssetState> {
        self.assets.get(&id).map(|e| e.state)
    }

    pub fn loaded_count(&self) -> usize {
        self.assets.values().filter(|e| e.state == AssetState::Loaded).count()
    }

    pub fn total_size_loaded(&self) -> u64 {
        self.assets
            .values()
            .filter(|e| e.state == AssetState::Loaded)
            .map(|e| e.meta.size_bytes)
            .sum()
    }
}

impl Default for AssetLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(id: u64, path: &str, atype: AssetType) -> AssetMeta {
        AssetMeta {
            id,
            path: PathBuf::from(path),
            asset_type: atype,
            size_bytes: 1024,
            dependencies: vec![],
            checksum: 0,
            version: 1,
            compressed: false,
        }
    }

    #[test]
    fn test_register_and_load() {
        let mut loader = AssetLoader::new();
        let meta = make_meta(1, "test.mesh", AssetType::Mesh);
        loader.register(meta);
        assert!(loader.request_load(1));
        loader.complete_load(1, vec![1, 2, 3]);
        assert!(loader.get_data(1).is_some());
        assert_eq!(loader.get_data(1).unwrap(), &[1, 2, 3]);
    }

    #[test]
    fn test_reference_counting() {
        let mut loader = AssetLoader::new();
        loader.register(make_meta(1, "a.mesh", AssetType::Mesh));
        loader.request_load(1);
        loader.complete_load(1, vec![]);
        loader.add_ref(1);
        loader.add_ref(1);
        let released = loader.release(1);
        assert!(!released);
        let released = loader.release(1);
        assert!(!released);
        let released = loader.release(1);
        assert!(released);
        assert!(loader.get_data(1).is_none());
    }

    #[test]
    fn test_query_by_type() {
        let mut loader = AssetLoader::new();
        loader.register(make_meta(1, "a.mesh", AssetType::Mesh));
        loader.register(make_meta(2, "b.mesh", AssetType::Mesh));
        loader.register(make_meta(3, "c.tex", AssetType::Texture));
        let meshes = loader.query_by_type(AssetType::Mesh);
        assert_eq!(meshes.len(), 2);
        let textures = loader.query_by_type(AssetType::Texture);
        assert_eq!(textures.len(), 1);
    }

    #[test]
    fn test_fail_load() {
        let mut loader = AssetLoader::new();
        loader.register(make_meta(1, "fail.mesh", AssetType::Mesh));
        loader.request_load(1);
        loader.fail_load(1);
        assert_eq!(loader.state(1), Some(AssetState::Failed));
        assert!(loader.request_load(1));
    }

    #[test]
    fn test_dependency_loading() {
        let mut loader = AssetLoader::new();
        let mut parent = make_meta(1, "parent.prefab", AssetType::Prefab);
        parent.dependencies = vec![2, 3];
        loader.register(parent);
        loader.register(make_meta(2, "child.mesh", AssetType::Mesh));
        loader.register(make_meta(3, "child.tex", AssetType::Texture));
        loader.load_dependencies(1);
        assert_eq!(loader.state(2), Some(AssetState::Loading));
        assert_eq!(loader.state(3), Some(AssetState::Loading));
    }
}
