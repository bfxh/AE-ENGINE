use crate::cache::AssetCache;
use crate::hotreload::HotReloader;
use crate::loader::{AssetLoader, AssetMeta, AssetType};
use crate::streaming::{StreamPriority, StreamRequest, StreamScheduler};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AssetPipeline {
    pub loader: AssetLoader,
    pub cache: AssetCache,
    pub streamer: StreamScheduler,
    pub hotreload: HotReloader,
    pipeline_stages: Vec<PipelineStage>,
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub name: String,
    pub input_type: AssetType,
    pub output_type: AssetType,
    pub enabled: bool,
}

impl AssetPipeline {
    pub fn new(cache_size: usize, max_bandwidth: u64) -> Self {
        AssetPipeline {
            loader: AssetLoader::new(),
            cache: AssetCache::new(cache_size),
            streamer: StreamScheduler::new(max_bandwidth, 8),
            hotreload: HotReloader::new(),
            pipeline_stages: Vec::new(),
        }
    }

    pub fn add_stage(&mut self, stage: PipelineStage) {
        self.pipeline_stages.push(stage);
    }

    pub fn import_asset(&mut self, path: PathBuf, asset_type: AssetType, data: Vec<u8>) -> u64 {
        let meta = AssetMeta {
            id: 0,
            path: path.clone(),
            asset_type,
            size_bytes: data.len() as u64,
            dependencies: vec![],
            checksum: 0,
            version: 1,
            compressed: false,
        };
        let id = self.loader.register(meta);
        self.loader.complete_load(id, data);
        self.cache.insert(id, vec![]);
        self.hotreload.watch(path, id);
        id
    }

    pub fn request_asset(&mut self, id: u64, priority: StreamPriority) {
        if self.cache.contains(id) {
            return;
        }
        if let Some(data) = self.loader.get_data(id) {
            self.cache.insert(id, data.to_vec());
            return;
        }
        self.streamer.request(StreamRequest {
            asset_id: id,
            priority,
            offset: 0,
            size: 0,
            timestamp: 0,
        });
        self.loader.request_load(id);
        self.loader.load_dependencies(id);
    }

    pub fn get_asset(&mut self, id: u64) -> Option<Vec<u8>> {
        if let Some(data) = self.cache.get(id) {
            return Some(data.to_vec());
        }
        if let Some(data) = self.loader.get_data(id) {
            let vec = data.to_vec();
            self.cache.insert(id, vec.clone());
            return Some(vec);
        }
        None
    }

    pub fn update(&mut self, delta_secs: f32) {
        let dispatched = self.streamer.process_frame(delta_secs);
        for _req in dispatched {
            self.streamer.complete_stream();
        }
        let changed = self.hotreload.poll();
        for id in changed {
            self.loader.request_load(id);
        }
        let _ = self.hotreload.reload_changed();
    }

    pub fn stats(&self) -> PipelineStats {
        PipelineStats {
            loaded_assets: self.loader.loaded_count(),
            cached_assets: self.cache.len(),
            cache_size: self.cache.size(),
            pending_streams: self.streamer.pending_count(),
            watched_files: self.hotreload.watched_count(),
            bandwidth_usage: self.streamer.bandwidth_usage(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub loaded_assets: usize,
    pub cached_assets: usize,
    pub cache_size: usize,
    pub pending_streams: usize,
    pub watched_files: usize,
    pub bandwidth_usage: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_and_get() {
        let mut pipeline = AssetPipeline::new(1024 * 1024, 1024 * 1024);
        let id = pipeline.import_asset(PathBuf::from("test.mesh"), AssetType::Mesh, vec![1, 2, 3]);
        assert!(id > 0);
        let data = pipeline.get_asset(id);
        assert!(data.is_some());
    }

    #[test]
    fn test_stats() {
        let mut pipeline = AssetPipeline::new(1024 * 1024, 1024 * 1024);
        pipeline.import_asset(PathBuf::from("a.mesh"), AssetType::Mesh, vec![0; 100]);
        let stats = pipeline.stats();
        assert_eq!(stats.loaded_assets, 1);
        assert_eq!(stats.cached_assets, 1);
    }

    #[test]
    fn test_pipeline_stages() {
        let mut pipeline = AssetPipeline::new(1024 * 1024, 1024 * 1024);
        pipeline.add_stage(PipelineStage {
            name: "mesh_optimize".into(),
            input_type: AssetType::Mesh,
            output_type: AssetType::Mesh,
            enabled: true,
        });
    }
}
