use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimdLevel {
    None,
    Sse2,
    Sse42,
    Avx,
    Avx2,
    Avx512,
    Neon,
}

#[derive(Debug, Clone)]
pub struct HardwareCaps {
    pub cpu_name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub simd_level: SimdLevel,
    pub l1_cache: usize,
    pub l2_cache: usize,
    pub l3_cache: usize,
    pub total_ram: usize,
    pub page_size: usize,
}

static CAPS: OnceLock<HardwareCaps> = OnceLock::new();

pub fn detect() -> &'static HardwareCaps {
    CAPS.get_or_init(|| {
        let cpu_name = detect_cpu_name();
        let simd = detect_simd();
        let logical = std::thread::available_parallelism().map(|p| p.get()).unwrap_or(1);
        let physical = logical;
        let (l1, l2, l3) = detect_cache_sizes();
        let total_ram = detect_total_ram();
        let page_size = detect_page_size();

        HardwareCaps {
            cpu_name,
            physical_cores: physical,
            logical_cores: logical,
            simd_level: simd,
            l1_cache: l1,
            l2_cache: l2,
            l3_cache: l3,
            total_ram,
            page_size,
        }
    })
}

fn detect_cpu_name() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        let mut brand = [0u8; 48];
        unsafe {
            let _func = 0x8000_0002u32;
            core::arch::x86_64::__cpuid(_func);
            let mut ptr = brand.as_mut_ptr();
            for leaf in 0x8000_0002u32..=0x8000_0004u32 {
                let res = core::arch::x86_64::__cpuid(leaf);
                core::ptr::copy_nonoverlapping(res.eax.to_ne_bytes().as_ptr(), ptr, 4);
                core::ptr::copy_nonoverlapping(res.ebx.to_ne_bytes().as_ptr(), ptr.add(4), 4);
                core::ptr::copy_nonoverlapping(res.ecx.to_ne_bytes().as_ptr(), ptr.add(8), 4);
                core::ptr::copy_nonoverlapping(res.edx.to_ne_bytes().as_ptr(), ptr.add(12), 4);
                ptr = ptr.add(16);
            }
        }
        String::from_utf8_lossy(&brand).trim_end_matches('\0').trim().to_string()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        "unknown".to_string()
    }
}

fn detect_simd() -> SimdLevel {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") {
            return SimdLevel::Avx512;
        }
        if is_x86_feature_detected!("avx2") {
            return SimdLevel::Avx2;
        }
        if is_x86_feature_detected!("avx") {
            return SimdLevel::Avx;
        }
        if is_x86_feature_detected!("sse4.2") {
            return SimdLevel::Sse42;
        }
        if is_x86_feature_detected!("sse2") {
            return SimdLevel::Sse2;
        }
        SimdLevel::None
    }
    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            return SimdLevel::Neon;
        }
        SimdLevel::None
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        SimdLevel::None
    }
}

fn detect_cache_sizes() -> (usize, usize, usize) {
    #[cfg(target_os = "windows")]
    {
        use std::mem;
        #[repr(C)]
        struct Slpi {
            processor_mask: usize,
            relationship: u32,
            _padding: u32,
            _union: [u64; 2],
        }
        extern "system" {
            fn GetLogicalProcessorInformation(buffer: *mut Slpi, returned_length: *mut u32) -> i32;
        }

        let mut len: u32 = 0;
        unsafe { GetLogicalProcessorInformation(std::ptr::null_mut(), &mut len) };
        if len == 0 {
            return (0, 0, 0);
        }

        let count = len as usize / mem::size_of::<Slpi>();
        let mut buf: Vec<Slpi> = Vec::with_capacity(count);
        let ret = unsafe { GetLogicalProcessorInformation(buf.as_mut_ptr(), &mut len) };
        if ret == 0 {
            return (0, 0, 0);
        }
        unsafe { buf.set_len(len as usize / mem::size_of::<Slpi>()) };

        let mut l1 = 0;
        let mut l2 = 0;
        let mut l3 = 0;
        for info in &buf {
            match info.relationship {
                2 => {
                    let cache_size = info._union[0] as u32;
                    l1 += cache_size as usize;
                },
                3 => {
                    let cache_size = info._union[0] as u32;
                    l2 += cache_size as usize;
                },
                4 => {
                    let cache_size = info._union[0] as u32;
                    l3 += cache_size as usize;
                },
                _ => {},
            }
        }
        (l1, l2, l3)
    }
    #[cfg(not(target_os = "windows"))]
    {
        (0, 0, 0)
    }
}

fn detect_total_ram() -> usize {
    #[cfg(target_os = "windows")]
    {
        use std::mem;
        #[repr(C)]
        struct MemoryStatusEx {
            length: u32,
            memory_load: u32,
            total_phys: u64,
            avail_phys: u64,
            total_pagefile: u64,
            avail_pagefile: u64,
            total_virtual: u64,
            avail_virtual: u64,
            avail_extended_virtual: u64,
        }
        extern "system" {
            fn GlobalMemoryStatusEx(lp_buffer: *mut MemoryStatusEx) -> i32;
        }
        let mut status = MemoryStatusEx {
            length: mem::size_of::<MemoryStatusEx>() as u32,
            memory_load: 0,
            total_phys: 0,
            avail_phys: 0,
            total_pagefile: 0,
            avail_pagefile: 0,
            total_virtual: 0,
            avail_virtual: 0,
            avail_extended_virtual: 0,
        };
        unsafe { GlobalMemoryStatusEx(&mut status) };
        status.total_phys as usize
    }
    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

fn detect_page_size() -> usize {
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct SystemInfo {
            _processor_arch: u16,
            _reserved: u16,
            page_size: u32,
            _min_app_addr: usize,
            _max_app_addr: usize,
            _active_proc_mask: usize,
            _num_procs: u32,
            _proc_type: u32,
            _alloc_gran: u32,
            _proc_level: u16,
            _proc_rev: u16,
        }
        extern "system" {
            fn GetSystemInfo(lp_system_info: *mut SystemInfo);
        }
        let mut info: SystemInfo = unsafe { std::mem::zeroed() };
        unsafe { GetSystemInfo(&mut info) };
        info.page_size as usize
    }
    #[cfg(not(target_os = "windows"))]
    {
        4096
    }
}

impl HardwareCaps {
    pub fn simd_width(&self) -> usize {
        match self.simd_level {
            SimdLevel::Avx512 => 64,
            SimdLevel::Avx | SimdLevel::Avx2 => 32,
            SimdLevel::Sse2 | SimdLevel::Sse42 | SimdLevel::Neon => 16,
            SimdLevel::None => 8,
        }
    }

    pub fn recommended_parallel_chunk_size(&self) -> usize {
        (self.l1_cache / 2).max(1024)
    }

    pub fn recommended_rayon_threads(&self) -> usize {
        self.physical_cores.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_hardware() {
        let caps = detect();
        assert!(caps.logical_cores > 0);
        assert!(caps.physical_cores > 0);
        assert!(caps.total_ram > 0);
        assert!(!caps.cpu_name.is_empty());
    }

    #[test]
    fn test_simd_width() {
        let caps = detect();
        let width = caps.simd_width();
        assert!(width >= 8);
    }

    #[test]
    fn test_recommended_params() {
        let caps = detect();
        assert!(caps.recommended_parallel_chunk_size() >= 1024);
        assert!(caps.recommended_rayon_threads() > 0);
    }
}
