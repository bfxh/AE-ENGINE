use godot::prelude::*;

use wasteland_compute::hardware;
use wasteland_compute::parallel;

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandCompute {
    #[var]
    max_threads: i64,
    #[var]
    use_simd: bool,

    task_count: i64,
    completed_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandCompute {
    fn init(base: Base<Node>) -> Self {
        let caps = hardware::detect();
        let max_threads = caps.logical_cores as i64;
        Self {
            max_threads,
            use_simd: caps.simd_level != hardware::SimdLevel::None,
            task_count: 0,
            completed_count: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandCompute {
    #[func]
    fn detect_hardware(&self) -> Dictionary<Variant, Variant> {
        let caps = hardware::detect();
        dict! {
            "cpu_name" => &GString::from(caps.cpu_name.as_str()),
            "physical_cores" => caps.physical_cores as i64,
            "logical_cores" => caps.logical_cores as i64,
            "simd_level" => &GString::from(format!("{:?}", caps.simd_level).as_str()),
            "l1_cache_kb" => (caps.l1_cache / 1024) as i64,
            "l2_cache_kb" => (caps.l2_cache / 1024) as i64,
            "l3_cache_mb" => (caps.l3_cache / 1048576) as i64,
            "total_ram_gb" => (caps.total_ram / 1073741824) as i64,
            "page_size_kb" => (caps.page_size / 1024) as i64,
        }
    }

    #[func]
    fn get_simd_level(&self) -> GString {
        let caps = hardware::detect();
        GString::from(format!("{:?}", caps.simd_level).as_str())
    }

    #[func]
    fn get_core_count(&self) -> i64 {
        let caps = hardware::detect();
        caps.logical_cores as i64
    }

    #[func]
    fn has_avx2(&self) -> bool {
        let caps = hardware::detect();
        matches!(caps.simd_level, hardware::SimdLevel::Avx2 | hardware::SimdLevel::Avx512)
    }

    #[func]
    fn has_avx512(&self) -> bool {
        let caps = hardware::detect();
        caps.simd_level == hardware::SimdLevel::Avx512
    }

    #[func]
    fn dispatch_parallel(&mut self, task_count: i64) -> i64 {
        self.task_count += task_count;
        let mut data: Vec<f32> = vec![0.0; task_count as usize];
        parallel::parallel_for(&mut data, |_i, item| {
            *item = 1.0;
        });
        self.completed_count += task_count;
        task_count
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        let caps = hardware::detect();
        dict! {
            "task_count" => self.task_count,
            "completed_count" => self.completed_count,
            "logical_cores" => caps.logical_cores as i64,
            "physical_cores" => caps.physical_cores as i64,
            "max_threads" => self.max_threads,
            "use_simd" => self.use_simd,
        }
    }
}
