use std::collections::VecDeque;
use std::time::Instant;

#[cfg(test)]
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct FrameTiming {
    pub frame_id: u64,
    pub delta_secs: f32,
    pub cpu_time_ms: f32,
    pub gpu_time_ms: f32,
    pub physics_time_ms: f32,
    pub render_time_ms: f32,
    pub ai_time_ms: f32,
    pub audio_time_ms: f32,
    pub other_time_ms: f32,
}

impl FrameTiming {
    pub fn total_ms(&self) -> f32 {
        self.cpu_time_ms + self.gpu_time_ms
    }

    pub fn fps(&self) -> f32 {
        if self.delta_secs > 0.0 { 1.0 / self.delta_secs } else { 0.0 }
    }
}

#[derive(Debug, Clone)]
pub struct FrameTimer {
    history: VecDeque<FrameTiming>,
    max_history: usize,
    frame_id: u64,
    frame_start: Option<Instant>,
    section_start: Option<Instant>,
    current_section: Option<&'static str>,
    current_physics: f32,
    current_render: f32,
    current_ai: f32,
    current_audio: f32,
    current_other: f32,
}

impl FrameTimer {
    pub fn new(max_history: usize) -> Self {
        FrameTimer {
            history: VecDeque::with_capacity(max_history),
            max_history,
            frame_id: 0,
            frame_start: None,
            section_start: None,
            current_section: None,
            current_physics: 0.0,
            current_render: 0.0,
            current_ai: 0.0,
            current_audio: 0.0,
            current_other: 0.0,
        }
    }

    pub fn begin_frame(&mut self) {
        self.frame_start = Some(Instant::now());
        self.current_physics = 0.0;
        self.current_render = 0.0;
        self.current_ai = 0.0;
        self.current_audio = 0.0;
        self.current_other = 0.0;
    }

    pub fn begin_section(&mut self, name: &'static str) {
        self.section_start = Some(Instant::now());
        self.current_section = Some(name);
    }

    pub fn end_section(&mut self) {
        if let (Some(start), Some(name)) = (self.section_start, self.current_section) {
            let elapsed = start.elapsed().as_secs_f32() * 1000.0;
            match name {
                "physics" => self.current_physics += elapsed,
                "render" => self.current_render += elapsed,
                "ai" => self.current_ai += elapsed,
                "audio" => self.current_audio += elapsed,
                _ => self.current_other += elapsed,
            }
        }
        self.section_start = None;
        self.current_section = None;
    }

    pub fn end_frame(&mut self) -> Option<FrameTiming> {
        if let Some(start) = self.frame_start {
            let delta = start.elapsed();
            let delta_secs = delta.as_secs_f32();
            let cpu_time = delta_secs * 1000.0;
            let timing = FrameTiming {
                frame_id: self.frame_id,
                delta_secs,
                cpu_time_ms: cpu_time,
                gpu_time_ms: 0.0,
                physics_time_ms: self.current_physics,
                render_time_ms: self.current_render,
                ai_time_ms: self.current_ai,
                audio_time_ms: self.current_audio,
                other_time_ms: self.current_other,
            };
            if self.history.len() >= self.max_history {
                self.history.pop_front();
            }
            self.history.push_back(timing.clone());
            self.frame_id += 1;
            self.frame_start = None;
            Some(timing)
        } else {
            None
        }
    }

    pub fn average_fps(&self, window: usize) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let count = window.min(self.history.len());
        let total: f32 = self.history.iter().rev().take(count).map(|f| f.delta_secs).sum();
        if total > 0.0 { count as f32 / total } else { 0.0 }
    }

    pub fn average_frame_time_ms(&self, window: usize) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let count = window.min(self.history.len());
        let total: f32 = self.history.iter().rev().take(count).map(|f| f.cpu_time_ms).sum();
        total / count as f32
    }

    pub fn frame_time_percentile(&self, percentile: f32) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let mut times: Vec<f32> = self.history.iter().map(|f| f.cpu_time_ms).collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((percentile / 100.0) * (times.len() - 1) as f32) as usize;
        times[idx.min(times.len() - 1)]
    }

    pub fn section_breakdown(&self, window: usize) -> Vec<(&str, f32)> {
        if self.history.is_empty() {
            return vec![];
        }
        let count = window.min(self.history.len());
        let mut phys = 0.0f32;
        let mut rend = 0.0f32;
        let mut ai = 0.0f32;
        let mut aud = 0.0f32;
        let mut oth = 0.0f32;
        for f in self.history.iter().rev().take(count) {
            phys += f.physics_time_ms;
            rend += f.render_time_ms;
            ai += f.ai_time_ms;
            aud += f.audio_time_ms;
            oth += f.other_time_ms;
        }
        let n = count as f32;
        vec![
            ("physics", phys / n),
            ("render", rend / n),
            ("ai", ai / n),
            ("audio", aud / n),
            ("other", oth / n),
        ]
    }

    pub fn history(&self) -> &VecDeque<FrameTiming> {
        &self.history
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new(3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_timing_creation() {
        let timing = FrameTiming {
            frame_id: 0,
            delta_secs: 0.016,
            cpu_time_ms: 10.0,
            gpu_time_ms: 5.0,
            physics_time_ms: 3.0,
            render_time_ms: 4.0,
            ai_time_ms: 1.0,
            audio_time_ms: 0.5,
            other_time_ms: 1.5,
        };
        assert!((timing.fps() - 62.5).abs() < 1.0);
        assert_eq!(timing.total_ms(), 15.0);
    }

    #[test]
    fn test_frame_timer_begin_end() {
        let mut timer = FrameTimer::new(100);
        timer.begin_frame();
        timer.begin_section("physics");
        std::thread::sleep(Duration::from_millis(1));
        timer.end_section();
        let timing = timer.end_frame().unwrap();
        assert!(timing.physics_time_ms > 0.0);
    }

    #[test]
    fn test_average_fps() {
        let mut timer = FrameTimer::new(100);
        for _ in 0..10 {
            timer.begin_frame();
            timer.end_frame();
        }
        assert!(timer.average_fps(10) > 0.0);
    }

    #[test]
    fn test_frame_time_percentile() {
        let mut timer = FrameTimer::new(100);
        for _ in 0..10 {
            timer.begin_frame();
            timer.end_frame();
        }
        let p50 = timer.frame_time_percentile(50.0);
        let p99 = timer.frame_time_percentile(99.0);
        assert!(p99 >= p50);
    }

    #[test]
    fn test_section_breakdown() {
        let mut timer = FrameTimer::new(100);
        timer.begin_frame();
        timer.begin_section("physics");
        std::thread::sleep(std::time::Duration::from_micros(10));
        timer.end_section();
        timer.begin_section("ai");
        std::thread::sleep(std::time::Duration::from_micros(10));
        timer.end_section();
        timer.end_frame();
        let breakdown = timer.section_breakdown(1);
        assert_eq!(breakdown.len(), 5);
        assert!(breakdown[0].1 > 0.0, "physics time should be > 0, got {}", breakdown[0].1);
    }
}
