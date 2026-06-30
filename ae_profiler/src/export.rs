use crate::metrics::SystemMetrics;
use crate::timing::FrameTiming;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct ChromeTraceEvent {
    name: String,
    cat: String,
    ph: String,
    pid: u32,
    tid: u32,
    ts: f64,
    dur: f64,
    args: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
struct ChromeTrace {
    #[serde(rename = "traceEvents")]
    trace_events: Vec<ChromeTraceEvent>,
    #[serde(rename = "displayTimeUnit")]
    display_time_unit: String,
}

pub fn export_chrome_trace(timings: &[FrameTiming]) -> String {
    let mut events = Vec::new();
    let mut ts = 0.0f64;
    for timing in timings {
        let sections = [
            ("physics", timing.physics_time_ms),
            ("render", timing.render_time_ms),
            ("ai", timing.ai_time_ms),
            ("audio", timing.audio_time_ms),
            ("other", timing.other_time_ms),
        ];
        for (name, dur) in &sections {
            if *dur > 0.0 {
                events.push(ChromeTraceEvent {
                    name: format!("Frame {}", timing.frame_id),
                    cat: name.to_string(),
                    ph: "X".to_string(),
                    pid: 1,
                    tid: 0,
                    ts: ts * 1000.0,
                    dur: *dur as f64 * 1000.0,
                    args: None,
                });
                ts += *dur as f64;
            }
        }
    }
    let trace = ChromeTrace { trace_events: events, display_time_unit: "ns".to_string() };
    serde_json::to_string_pretty(&trace).unwrap_or_default()
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfilerReport {
    pub average_fps: f32,
    pub frame_time_ms_avg: f32,
    pub frame_time_ms_p99: f32,
    pub systems: Vec<SystemReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemReport {
    pub name: String,
    pub calls: u64,
    pub total_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
    pub percent_of_frame: f64,
}

pub fn generate_report(
    avg_fps: f32,
    avg_frame_ms: f32,
    p99_ms: f32,
    systems: &[&SystemMetrics],
    total_frame_ms: f64,
) -> ProfilerReport {
    let system_reports: Vec<SystemReport> = systems
        .iter()
        .map(|s| SystemReport {
            name: s.name.clone(),
            calls: s.total_calls,
            total_ms: s.total_time_ms,
            avg_ms: s.avg_time_ms,
            max_ms: s.max_time_ms,
            percent_of_frame: if total_frame_ms > 0.0 {
                s.total_time_ms / total_frame_ms * 100.0
            } else {
                0.0
            },
        })
        .collect();
    ProfilerReport {
        average_fps: avg_fps,
        frame_time_ms_avg: avg_frame_ms,
        frame_time_ms_p99: p99_ms,
        systems: system_reports,
    }
}

pub fn export_json_report(report: &ProfilerReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricsCollector;

    fn make_timings() -> Vec<FrameTiming> {
        vec![
            FrameTiming {
                frame_id: 0,
                delta_secs: 0.016,
                cpu_time_ms: 10.0,
                gpu_time_ms: 5.0,
                physics_time_ms: 3.0,
                render_time_ms: 4.0,
                ai_time_ms: 1.0,
                audio_time_ms: 0.5,
                other_time_ms: 1.5,
            },
            FrameTiming {
                frame_id: 1,
                delta_secs: 0.016,
                cpu_time_ms: 12.0,
                gpu_time_ms: 5.0,
                physics_time_ms: 4.0,
                render_time_ms: 4.0,
                ai_time_ms: 2.0,
                audio_time_ms: 0.5,
                other_time_ms: 1.5,
            },
        ]
    }

    #[test]
    fn test_chrome_trace_export() {
        let timings = make_timings();
        let trace = export_chrome_trace(&timings);
        assert!(trace.contains("traceEvents"));
        assert!(trace.contains("physics"));
        assert!(trace.contains("render"));
    }

    #[test]
    fn test_generate_report() {
        let mut mc = MetricsCollector::new();
        mc.record_system("physics", 10.0);
        mc.record_system("render", 8.0);
        let systems = mc.all_systems();
        let report = generate_report(60.0, 16.0, 20.0, &systems, 18.0);
        assert_eq!(report.average_fps, 60.0);
        assert_eq!(report.frame_time_ms_p99, 20.0);
        assert_eq!(report.systems.len(), 2);
        let phys = &report.systems[0];
        assert!((phys.percent_of_frame - 55.5).abs() < 1.0);
    }

    #[test]
    fn test_json_report_export() {
        let mut mc = MetricsCollector::new();
        mc.record_system("test", 5.0);
        let systems = mc.all_systems();
        let report = generate_report(30.0, 33.0, 50.0, &systems, 5.0);
        let json = export_json_report(&report);
        assert!(json.contains("average_fps"));
        assert!(json.contains("test"));
    }
}
