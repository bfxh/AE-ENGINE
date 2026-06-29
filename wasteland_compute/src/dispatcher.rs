use crate::hardware::HardwareCaps;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone)]
pub struct TaskProfile {
    pub data_size: usize,
    pub operation_count: usize,
    pub is_embarrassingly_parallel: bool,
    pub memory_bound: bool,
    pub requires_double_precision: bool,
}

impl TaskProfile {
    pub fn new(data_size: usize) -> Self {
        TaskProfile {
            data_size,
            operation_count: data_size,
            is_embarrassingly_parallel: true,
            memory_bound: false,
            requires_double_precision: false,
        }
    }

    pub fn with_ops(mut self, ops: usize) -> Self {
        self.operation_count = ops;
        self
    }

    pub fn memory_bound(mut self) -> Self {
        self.memory_bound = true;
        self
    }

    pub fn double_precision(mut self) -> Self {
        self.requires_double_precision = true;
        self
    }
}

pub struct ComputeDispatcher {
    caps: &'static HardwareCaps,
    gpu_available: bool,
    gpu_min_size: usize,
}

impl Default for ComputeDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeDispatcher {
    pub fn new() -> Self {
        ComputeDispatcher {
            caps: crate::hardware::detect(),
            gpu_available: false,
            gpu_min_size: 65536,
        }
    }

    pub fn with_gpu(mut self, available: bool, min_size: usize) -> Self {
        self.gpu_available = available;
        self.gpu_min_size = min_size;
        self
    }

    pub fn select_device(&self, profile: &TaskProfile) -> Device {
        if !self.gpu_available {
            return Device::Cpu;
        }

        if profile.requires_double_precision {
            return Device::Cpu;
        }

        if profile.data_size < self.gpu_min_size {
            return Device::Cpu;
        }

        if profile.memory_bound && profile.data_size < self.caps.l3_cache {
            return Device::Cpu;
        }

        if profile.is_embarrassingly_parallel && profile.operation_count > 100_000 {
            return Device::Gpu;
        }

        Device::Cpu
    }

    pub fn optimal_chunk_size(&self, profile: &TaskProfile, device: Device) -> usize {
        match device {
            Device::Cpu => {
                let base = self.caps.recommended_parallel_chunk_size();
                if profile.memory_bound { base.min(self.caps.l1_cache / 4) } else { base }
            },
            Device::Gpu => {
                let wavefront = 64;
                let total = profile.data_size;
                let chunks = (total / wavefront).max(256);
                total / chunks
            },
        }
    }

    pub fn thread_count(&self, device: Device) -> usize {
        match device {
            Device::Cpu => self.caps.recommended_rayon_threads(),
            Device::Gpu => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_small_to_cpu() {
        let dispatcher = ComputeDispatcher::new().with_gpu(true, 1024);
        let profile = TaskProfile::new(100);
        assert_eq!(dispatcher.select_device(&profile), Device::Cpu);
    }

    #[test]
    fn test_dispatch_large_to_gpu() {
        let dispatcher = ComputeDispatcher::new().with_gpu(true, 1024);
        let profile = TaskProfile::new(100_000).with_ops(200_000);
        assert_eq!(dispatcher.select_device(&profile), Device::Gpu);
    }

    #[test]
    fn test_dispatch_double_precision_to_cpu() {
        let dispatcher = ComputeDispatcher::new().with_gpu(true, 1024);
        let profile = TaskProfile::new(100_000).double_precision();
        assert_eq!(dispatcher.select_device(&profile), Device::Cpu);
    }

    #[test]
    fn test_optimal_chunk_size() {
        let dispatcher = ComputeDispatcher::new();
        let profile = TaskProfile::new(1000);
        let chunk = dispatcher.optimal_chunk_size(&profile, Device::Cpu);
        assert!(chunk >= 256);
    }
}
