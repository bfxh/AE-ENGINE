//! 数据层管理器
//!
//! 管理资源加载、时间切片、事件存储和性能监控。

use std::sync::Arc;

use ae_asset::streaming::StreamScheduler;
use ae_profiler::timing::FrameTimer;
use ae_timeslice::prelude::*;

use crate::systems::{MemorySystem, SupervisionSystem, WorkflowSystem};

/// 数据层管理器
pub struct DataManager {
    // 时间切片系统
    pub time_slicer: LayeredTimeSlicer,
    pub event_store: EventStore,
    pub diff_graph: DiffUpdateGraph,

    // 资源管理
    pub stream_scheduler: StreamScheduler,

    // AI 基础设施
    pub memory: Arc<MemorySystem>,
    pub supervision: Arc<SupervisionSystem>,
    pub workflow: Arc<WorkflowSystem>,

    // 性能监控
    pub frame_timer: FrameTimer,
}

impl std::fmt::Debug for DataManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataManager")
            .field("time_slicer", &self.time_slicer)
            .field("event_store", &self.event_store)
            .finish()
    }
}

impl DataManager {
    /// 创建新的数据层管理器
    pub fn new() -> Self {
        let memory = Arc::new(MemorySystem::new(10000));
        let supervision = Arc::new(SupervisionSystem::new());
        let workflow = Arc::new(WorkflowSystem::new(4));
        workflow.start();

        Self {
            time_slicer: LayeredTimeSlicer::new(),
            event_store: EventStore::new(100000),
            diff_graph: DiffUpdateGraph::new(),
            stream_scheduler: StreamScheduler::new(50 * 1024 * 1024, 8),
            memory,
            supervision,
            workflow,
            frame_timer: FrameTimer::new(60),
        }
    }

    /// 更新数据层
    pub fn update(&mut self, dt: f32) {
        self.stream_scheduler.process_frame(dt);
    }

    /// 帧开始
    pub fn begin_frame(&mut self) {
        self.frame_timer.begin_frame();
    }

    /// 帧结束
    pub fn end_frame(&mut self) {
        let _ = self.frame_timer.end_frame();
    }
}

impl Default for DataManager {
    fn default() -> Self {
        Self::new()
    }
}
