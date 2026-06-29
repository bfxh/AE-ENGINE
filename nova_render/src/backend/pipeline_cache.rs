//! PipelineCache（借鉴 bevy）
//!
//! 异步渲染管线编译：
//! - 提交 PipelineDescriptor 后立即返回 PipelineId
//! - 后台线程编译，完成后状态变为 Ready
//! - 渲染线程使用时检查状态，未 Ready 的 fallback

use std::sync::Arc;
use parking_lot::Mutex;
use hashbrown::HashMap;
use std::any::Any;

/// Pipeline ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineId(pub u64);

/// Pipeline 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    /// 排队中
    Queued,
    /// 编译中
    Compiling,
    /// 就绪
    Ready,
    /// 编译失败
    Failed,
}

/// Pipeline 条目
struct PipelineEntry {
    state: PipelineState,
    pipeline: Option<Arc<dyn Any + Send + Sync>>,
}

/// PipelineCache
pub struct PipelineCache {
    entries: Mutex<HashMap<PipelineId, PipelineEntry>>,
    next_id: Mutex<u64>,
}

impl PipelineCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            next_id: Mutex::new(0),
        }
    }

    /// 分配新 PipelineId
    pub fn allocate(&self) -> PipelineId {
        let mut next = self.next_id.lock();
        let id = PipelineId(*next);
        *next += 1;
        self.entries.lock().insert(id, PipelineEntry { state: PipelineState::Queued, pipeline: None });
        id
    }

    /// 获取状态
    pub fn state(&self, id: PipelineId) -> PipelineState {
        self.entries.lock().get(&id).map(|e| e.state).unwrap_or(PipelineState::Failed)
    }

    /// 设置 pipeline（编译完成时调用）
    pub fn set_ready(&self, id: PipelineId, pipeline: Arc<dyn Any + Send + Sync>) {
        if let Some(entry) = self.entries.lock().get_mut(&id) {
            entry.state = PipelineState::Ready;
            entry.pipeline = Some(pipeline);
        }
    }

    /// 获取 pipeline
    pub fn get(&self, id: PipelineId) -> Option<Arc<dyn Any + Send + Sync>> {
        self.entries.lock().get(&id).and_then(|e| e.pipeline.clone())
    }
}

impl Default for PipelineCache {
    fn default() -> Self { Self::new() }
}
