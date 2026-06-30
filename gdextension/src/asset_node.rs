use godot::prelude::*;

use ae_asset::loader::{AssetState, AssetType};
use ae_asset::pipeline::AssetPipeline;

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandAsset {
    #[var]
    base_path: GString,
    #[var]
    cache_size_mb: i64,
    #[var]
    max_concurrent_loads: i64,
    #[var]
    hot_reload_enabled: bool,

    pipeline: AssetPipeline,
    asset_count: i64,
    loaded_count: i64,
    failed_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandAsset {
    fn init(base: Base<Node>) -> Self {
        let cache_mb = 256;
        let cache_bytes = cache_mb * 1024 * 1024;
        let bandwidth = 1024 * 1024 * 1024u64;
        Self {
            base_path: GString::from("assets/"),
            cache_size_mb: cache_mb as i64,
            max_concurrent_loads: 4,
            hot_reload_enabled: false,
            pipeline: AssetPipeline::new(cache_bytes, bandwidth),
            asset_count: 0,
            loaded_count: 0,
            failed_count: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandAsset {
    #[func]
    fn import_asset(&mut self, path: GString, asset_type: GString, data: PackedByteArray) -> i64 {
        let at = match asset_type.to_string().as_str() {
            "mesh" => AssetType::Mesh,
            "texture" => AssetType::Texture,
            "material" => AssetType::Material,
            "skeleton" => AssetType::Skeleton,
            "animation" => AssetType::Animation,
            "sound" => AssetType::Sound,
            "shader" => AssetType::Shader,
            "script" => AssetType::Script,
            "config" => AssetType::Config,
            "prefab" => AssetType::Prefab,
            _ => AssetType::Custom(0),
        };
        let id = self.pipeline.import_asset(
            std::path::PathBuf::from(path.to_string()),
            at,
            data.as_slice().to_vec(),
        );
        self.asset_count += 1;
        id as i64
    }

    #[func]
    fn load_asset(&mut self, asset_id: i64) -> bool {
        let result = self.pipeline.loader.request_load(asset_id as u64);
        if result {
            self.pipeline.loader.load_dependencies(asset_id as u64);
            self.loaded_count += 1;
        } else {
            self.failed_count += 1;
        }
        result
    }

    #[func]
    fn complete_load(&mut self, asset_id: i64, data: PackedByteArray) {
        self.pipeline.loader.complete_load(asset_id as u64, data.as_slice().to_vec());
    }

    #[func]
    fn unload_asset(&mut self, asset_id: i64) {
        self.pipeline.loader.release(asset_id as u64);
        self.loaded_count = (self.loaded_count - 1).max(0);
    }

    #[func]
    fn get_asset_state(&self, asset_id: i64) -> GString {
        match self.pipeline.loader.state(asset_id as u64) {
            Some(AssetState::Unloaded) => GString::from("unloaded"),
            Some(AssetState::Loading) => GString::from("loading"),
            Some(AssetState::Loaded) => GString::from("loaded"),
            Some(AssetState::Failed) => GString::from("failed"),
            Some(AssetState::Unloading) => GString::from("unloading"),
            None => GString::from("unknown"),
        }
    }

    #[func]
    fn get_asset_data(&self, asset_id: i64) -> PackedByteArray {
        let data = self.pipeline.loader.get_data(asset_id as u64);
        match data {
            Some(d) => {
                let mut arr = PackedByteArray::new();
                for &b in d.iter() {
                    arr.push(b);
                }
                arr
            },
            None => PackedByteArray::new(),
        }
    }

    #[func]
    fn add_ref(&mut self, asset_id: i64) {
        self.pipeline.loader.add_ref(asset_id as u64);
    }

    #[func]
    fn get_assets_by_type(&self, asset_type: GString) -> PackedInt64Array {
        let at = match asset_type.to_string().as_str() {
            "mesh" => AssetType::Mesh,
            "texture" => AssetType::Texture,
            "material" => AssetType::Material,
            "skeleton" => AssetType::Skeleton,
            "animation" => AssetType::Animation,
            "sound" => AssetType::Sound,
            "shader" => AssetType::Shader,
            "script" => AssetType::Script,
            "config" => AssetType::Config,
            "prefab" => AssetType::Prefab,
            _ => return PackedInt64Array::new(),
        };
        let ids = self.pipeline.loader.query_by_type(at);
        let mut arr = PackedInt64Array::new();
        for &id in ids.iter() {
            arr.push(id as i64);
        }
        arr
    }

    #[func]
    fn watch_asset(&mut self, path: GString, asset_id: i64) {
        self.pipeline.hotreload.watch(std::path::PathBuf::from(path.to_string()), asset_id as u64);
        self.hot_reload_enabled = true;
    }

    #[func]
    fn poll_hot_reload(&mut self) -> PackedInt64Array {
        let changed = self.pipeline.hotreload.poll();
        let mut arr = PackedInt64Array::new();
        for &id in changed.iter() {
            arr.push(id as i64);
        }
        arr
    }

    #[func]
    fn update(&mut self, delta: f32) {
        self.pipeline.update(delta);
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        let stats = self.pipeline.stats();
        dict! {
            "asset_count" => self.asset_count,
            "loaded_count" => stats.loaded_assets as i64,
            "failed_count" => self.failed_count,
            "cached_assets" => stats.cached_assets as i64,
            "cache_size" => stats.cache_size as i64,
            "pending_streams" => stats.pending_streams as i64,
            "watched_files" => stats.watched_files as i64,
            "bandwidth_usage" => stats.bandwidth_usage,
            "hot_reload_enabled" => self.hot_reload_enabled,
        }
    }
}
