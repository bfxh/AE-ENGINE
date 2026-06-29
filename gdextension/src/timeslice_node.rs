use godot::prelude::*;

use wasteland_timeslice::diff_graph::DiffUpdateGraph;
use wasteland_timeslice::time_slice::{LayeredTimeSlicer, SystemId, TimeSliceLayer};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandTimeslice {
    #[var]
    tick_rate: i64,
    #[var]
    auto_advance: bool,

    slicer: LayeredTimeSlicer,
    diff_graph: DiffUpdateGraph,
    tick_count: i64,
    system_count: i64,
    total_updates: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandTimeslice {
    fn init(base: Base<Node>) -> Self {
        Self {
            tick_rate: 60,
            auto_advance: true,
            slicer: LayeredTimeSlicer::new(),
            diff_graph: DiffUpdateGraph::new(),
            tick_count: 0,
            system_count: 0,
            total_updates: 0,
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        if !self.auto_advance {
            return;
        }
        self.tick_count += 1;
        self.slicer.step(_delta as f32);
        let stats = self.slicer.system_stats();
        self.total_updates = stats.iter().map(|s| s.update_count as i64).sum();
    }
}

#[godot_api]
impl WastelandTimeslice {
    #[func]
    fn register_system(&mut self, name: GString, layer: i64) -> i64 {
        let l = match layer {
            0 => TimeSliceLayer::Layer0,
            1 => TimeSliceLayer::Layer1,
            2 => TimeSliceLayer::Layer2,
            3 => TimeSliceLayer::Layer3,
            4 => TimeSliceLayer::Layer4,
            _ => TimeSliceLayer::Layer5,
        };
        let n = name.to_string();
        let id = self.slicer.register(&n, l, |_dt| {});
        self.system_count += 1;
        id.value() as i64
    }

    #[func]
    fn should_update(&self, layer: i64) -> bool {
        let l = match layer {
            0 => TimeSliceLayer::Layer0,
            1 => TimeSliceLayer::Layer1,
            2 => TimeSliceLayer::Layer2,
            3 => TimeSliceLayer::Layer3,
            4 => TimeSliceLayer::Layer4,
            _ => TimeSliceLayer::Layer5,
        };
        l.should_update(self.tick_count as u64)
    }

    #[func]
    fn get_layer_frequency(&self, layer: i64) -> f32 {
        let l = match layer {
            0 => TimeSliceLayer::Layer0,
            1 => TimeSliceLayer::Layer1,
            2 => TimeSliceLayer::Layer2,
            3 => TimeSliceLayer::Layer3,
            4 => TimeSliceLayer::Layer4,
            _ => TimeSliceLayer::Layer5,
        };
        let divisor = l.divisor();
        if divisor == 0 { 0.0 } else { self.tick_rate as f32 / divisor as f32 }
    }

    #[func]
    fn enable_system(&mut self, system_id: i64, enabled: bool) {
        self.slicer.set_enabled(SystemId::from_raw(system_id as u64), enabled);
    }

    #[func]
    fn get_diff_count(&self) -> i64 {
        self.diff_graph.stats().total_nodes as i64
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "tick_count" => self.tick_count,
            "system_count" => self.system_count,
            "total_updates" => self.total_updates,
            "tick_rate" => self.tick_rate,
            "diff_count" => self.diff_graph.stats().total_nodes as i64,
        }
    }
}
