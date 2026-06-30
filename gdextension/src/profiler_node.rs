use godot::prelude::*;

use ae_profiler::export::export_chrome_trace;
use ae_profiler::memory::MemoryTracker;
use ae_profiler::metrics::MetricsCollector;
use ae_profiler::timing::FrameTimer;

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandProfiler {
    #[var]
    max_history: i64,
    #[var]
    enabled: bool,
    #[var]
    export_path: GString,

    frame_timer: FrameTimer,
    memory_tracker: MemoryTracker,
    metrics: MetricsCollector,
    frame_count: i64,
    avg_fps: f32,
    avg_frame_time_ms: f32,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandProfiler {
    fn init(base: Base<Node>) -> Self {
        Self {
            max_history: 300,
            enabled: true,
            export_path: GString::from("profile_trace.json"),
            frame_timer: FrameTimer::new(300),
            memory_tracker: MemoryTracker::new(),
            metrics: MetricsCollector::new(),
            frame_count: 0,
            avg_fps: 0.0,
            avg_frame_time_ms: 0.0,
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        if !self.enabled {
            return;
        }
        self.frame_count += 1;
        self.frame_timer.begin_frame();
        self.frame_timer.end_section();
        let timing = self.frame_timer.end_frame();
        if let Some(t) = timing {
            self.avg_fps = t.fps();
            self.avg_frame_time_ms = t.total_ms();
        }
    }
}

#[godot_api]
impl WastelandProfiler {
    #[func]
    fn begin_section(&mut self, name: GString) {
        let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
        self.frame_timer.begin_section(leaked);
    }

    #[func]
    fn end_section(&mut self) {
        self.frame_timer.end_section();
    }

    #[func]
    fn get_frame_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "frame_count" => self.frame_count,
            "avg_fps" => self.avg_fps,
            "avg_frame_time_ms" => self.avg_frame_time_ms,
        }
    }

    #[func]
    fn get_memory_stats(&mut self) -> Dictionary<Variant, Variant> {
        let snap = self.memory_tracker.snapshot();
        dict! {
            "total_allocated_mb" => snap.alloc_bytes as f32 / 1048576.0,
            "total_used_mb" => snap.current_bytes as f32 / 1048576.0,
            "peak_allocated_mb" => snap.peak_bytes as f32 / 1048576.0,
            "allocation_count" => snap.alloc_count as i64,
        }
    }

    #[func]
    fn record_system_metrics(&mut self, system_name: GString, time_ms: f64) {
        self.metrics.record_system(&system_name.to_string(), time_ms);
    }

    #[func]
    fn record_frame_timing(
        &mut self,
        _physics_ms: f32,
        _render_ms: f32,
        _ai_ms: f32,
        _audio_ms: f32,
    ) {
    }

    #[func]
    fn export_chrome_trace(&self) -> GString {
        let path = self.export_path.to_string();
        let trace = export_chrome_trace(&[]);
        let _ = std::fs::write(&path, trace);
        GString::from(path.as_str())
    }

    #[func]
    fn reset(&mut self) {
        self.frame_timer = FrameTimer::new(self.max_history as usize);
        self.frame_count = 0;
        self.avg_fps = 0.0;
        self.avg_frame_time_ms = 0.0;
    }
}
