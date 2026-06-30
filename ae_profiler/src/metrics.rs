use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub name: String,
    pub total_calls: u64,
    pub total_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub avg_time_ms: f64,
}

impl SystemMetrics {
    pub fn new(name: &str) -> Self {
        SystemMetrics {
            name: name.to_string(),
            total_calls: 0,
            total_time_ms: 0.0,
            min_time_ms: f64::MAX,
            max_time_ms: 0.0,
            avg_time_ms: 0.0,
        }
    }

    pub fn record(&mut self, time_ms: f64) {
        self.total_calls += 1;
        self.total_time_ms += time_ms;
        if time_ms < self.min_time_ms {
            self.min_time_ms = time_ms;
        }
        if time_ms > self.max_time_ms {
            self.max_time_ms = time_ms;
        }
        self.avg_time_ms = self.total_time_ms / self.total_calls as f64;
    }
}

pub struct MetricsCollector {
    systems: HashMap<String, SystemMetrics>,
    global_frame_count: u64,
    global_total_time_ms: f64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        MetricsCollector {
            systems: HashMap::new(),
            global_frame_count: 0,
            global_total_time_ms: 0.0,
        }
    }

    pub fn record_system(&mut self, name: &str, time_ms: f64) {
        self.systems
            .entry(name.to_string())
            .or_insert_with(|| SystemMetrics::new(name))
            .record(time_ms);
    }

    pub fn record_frame(&mut self, total_time_ms: f64) {
        self.global_frame_count += 1;
        self.global_total_time_ms += total_time_ms;
    }

    pub fn get_system(&self, name: &str) -> Option<&SystemMetrics> {
        self.systems.get(name)
    }

    pub fn all_systems(&self) -> Vec<&SystemMetrics> {
        let mut systems: Vec<&SystemMetrics> = self.systems.values().collect();
        systems.sort_by(|a, b| {
            b.total_time_ms.partial_cmp(&a.total_time_ms).unwrap_or(std::cmp::Ordering::Equal)
        });
        systems
    }

    pub fn top_n(&self, n: usize) -> Vec<&SystemMetrics> {
        let mut all = self.all_systems();
        all.truncate(n);
        all
    }

    pub fn global_avg_ms(&self) -> f64 {
        if self.global_frame_count == 0 {
            return 0.0;
        }
        self.global_total_time_ms / self.global_frame_count as f64
    }

    pub fn total_recorded_time_ms(&self) -> f64 {
        self.systems.values().map(|s| s.total_time_ms).sum()
    }

    pub fn reset(&mut self) {
        self.systems.clear();
        self.global_frame_count = 0;
        self.global_total_time_ms = 0.0;
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TimingScope {
    name: String,
    start: std::time::Instant,
    recorded: bool,
}

impl TimingScope {
    pub fn new(name: &str) -> Self {
        TimingScope { name: name.to_string(), start: std::time::Instant::now(), recorded: false }
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    pub fn finish(&mut self, collector: &mut MetricsCollector) {
        if !self.recorded {
            collector.record_system(&self.name, self.elapsed_ms());
            self.recorded = true;
        }
    }
}

impl Drop for TimingScope {
    fn drop(&mut self) {
        if !self.recorded {
            let elapsed = self.start.elapsed().as_secs_f64() * 1000.0;
            eprintln!(
                "WARNING: TimingScope '{}' dropped without finish(), elapsed={:.2}ms",
                self.name, elapsed
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_metrics_record() {
        let mut sm = SystemMetrics::new("physics");
        sm.record(10.0);
        sm.record(20.0);
        sm.record(5.0);
        assert_eq!(sm.total_calls, 3);
        assert_eq!(sm.total_time_ms, 35.0);
        assert_eq!(sm.min_time_ms, 5.0);
        assert_eq!(sm.max_time_ms, 20.0);
        assert!((sm.avg_time_ms - 35.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_metrics_collector() {
        let mut mc = MetricsCollector::new();
        mc.record_system("physics", 10.0);
        mc.record_system("physics", 15.0);
        mc.record_system("render", 8.0);
        mc.record_frame(33.0);
        let phys = mc.get_system("physics").unwrap();
        assert_eq!(phys.total_calls, 2);
        assert_eq!(phys.total_time_ms, 25.0);
        assert!((mc.global_avg_ms() - 33.0).abs() < 0.01);
    }

    #[test]
    fn test_top_n() {
        let mut mc = MetricsCollector::new();
        mc.record_system("a", 5.0);
        mc.record_system("b", 20.0);
        mc.record_system("c", 10.0);
        let top = mc.top_n(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "b");
        assert_eq!(top[1].name, "c");
    }

    #[test]
    fn test_timing_scope() {
        let mut mc = MetricsCollector::new();
        let mut scope = TimingScope::new("test_system");
        scope.finish(&mut mc);
        let metrics = mc.get_system("test_system").unwrap();
        assert_eq!(metrics.total_calls, 1);
        assert!(metrics.total_time_ms >= 0.0);
    }

    #[test]
    fn test_reset() {
        let mut mc = MetricsCollector::new();
        mc.record_system("a", 10.0);
        mc.record_frame(10.0);
        mc.reset();
        assert_eq!(mc.global_frame_count, 0);
        assert!(mc.get_system("a").is_none());
    }
}
