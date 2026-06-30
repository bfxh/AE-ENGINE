use std::f64::consts::PI;

pub trait BusEffect: Send + Sync {
    fn process(&mut self, samples: &mut [f32], channels: usize, sample_rate: u32);
    fn reset(&mut self);
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
enum BiquadType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Peak,
    LowShelf,
    HighShelf,
}

#[derive(Clone, Copy, Debug)]
pub struct BiquadCoeffs {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl BiquadCoeffs {
    fn design(bq: BiquadType, freq: f32, gain_db: f32, q: f32, sample_rate: u32) -> Self {
        let w0 = 2.0 * PI * freq as f64 / sample_rate as f64;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f64.powf(gain_db as f64 / 40.0);
        let alpha = sin_w0 / (2.0 * q as f64);

        let (b0, b1, b2, a0, a1, a2): (f64, f64, f64, f64, f64, f64) = match bq {
            BiquadType::LowPass => {
                let b1 = 1.0 - cos_w0;
                (
                    0.5 * (1.0 - cos_w0),
                    b1,
                    0.5 * (1.0 - cos_w0),
                    1.0 + alpha,
                    -2.0 * cos_w0,
                    1.0 - alpha,
                )
            },
            BiquadType::HighPass => {
                let b1 = -(1.0 + cos_w0);
                (
                    0.5 * (1.0 + cos_w0),
                    b1,
                    0.5 * (1.0 + cos_w0),
                    1.0 + alpha,
                    -2.0 * cos_w0,
                    1.0 - alpha,
                )
            },
            BiquadType::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha),
            BiquadType::Notch => (1.0, -2.0 * cos_w0, 1.0, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha),
            BiquadType::Peak => (
                1.0 + alpha * a,
                -2.0 * cos_w0,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w0,
                1.0 - alpha / a,
            ),
            BiquadType::LowShelf => {
                let sq = 2.0 * (a * alpha).sqrt();
                (
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + sq),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - sq),
                    (a + 1.0) + (a - 1.0) * cos_w0 + sq,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - sq,
                )
            },
            BiquadType::HighShelf => {
                let sq = 2.0 * (a * alpha).sqrt();
                (
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + sq),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - sq),
                    (a + 1.0) - (a - 1.0) * cos_w0 + sq,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - sq,
                )
            },
        };

        BiquadCoeffs { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 }
    }
}

#[derive(Clone)]
pub struct BiquadFilter {
    coeffs: BiquadCoeffs,
    z1: f64,
    z2: f64,
}

impl BiquadFilter {
    pub fn low_pass(freq: f32, q: f32, sample_rate: u32) -> Self {
        Self {
            coeffs: BiquadCoeffs::design(BiquadType::LowPass, freq, 0.0, q, sample_rate),
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn high_pass(freq: f32, q: f32, sample_rate: u32) -> Self {
        Self {
            coeffs: BiquadCoeffs::design(BiquadType::HighPass, freq, 0.0, q, sample_rate),
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn peak(freq: f32, gain_db: f32, q: f32, sample_rate: u32) -> Self {
        Self {
            coeffs: BiquadCoeffs::design(BiquadType::Peak, freq, gain_db, q, sample_rate),
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn low_shelf(freq: f32, gain_db: f32, q: f32, sample_rate: u32) -> Self {
        Self {
            coeffs: BiquadCoeffs::design(BiquadType::LowShelf, freq, gain_db, q, sample_rate),
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn high_shelf(freq: f32, gain_db: f32, q: f32, sample_rate: u32) -> Self {
        Self {
            coeffs: BiquadCoeffs::design(BiquadType::HighShelf, freq, gain_db, q, sample_rate),
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[inline]
    fn process_sample(&mut self, x: f32) -> f32 {
        let x = x as f64;
        let c = self.coeffs;
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y as f32
    }
}

impl BusEffect for BiquadFilter {
    fn process(&mut self, samples: &mut [f32], channels: usize, _sample_rate: u32) {
        if channels == 0 {
            return;
        }
        for frame in samples.chunks_mut(channels) {
            for s in frame.iter_mut() {
                *s = self.process_sample(*s);
            }
        }
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[derive(Clone)]
pub struct Compressor {
    pub threshold: f32,
    pub ratio: f32,
    pub attack_coef: f32,
    pub release_coef: f32,
    pub makeup_gain: f32,
    envelope: f32,
}

impl Compressor {
    pub fn new(
        threshold: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_db: f32,
        sample_rate: u32,
    ) -> Self {
        let attack_coef = (-1.0 / (attack_ms * 0.001 * sample_rate as f32)).exp();
        let release_coef = (-1.0 / (release_ms * 0.001 * sample_rate as f32)).exp();
        Self {
            threshold,
            ratio,
            attack_coef,
            release_coef,
            makeup_gain: 10.0_f32.powf(makeup_db / 20.0),
            envelope: 0.0,
        }
    }

    #[inline]
    fn compute_gain(&mut self, sample: f32) -> f32 {
        let x = sample.abs();
        let coef = if x > self.envelope { self.attack_coef } else { self.release_coef };
        self.envelope = coef * self.envelope + (1.0 - coef) * x;

        if self.envelope <= self.threshold {
            return self.makeup_gain;
        }
        let over = self.envelope - self.threshold;
        let reduction_db = over * (1.0 - 1.0 / self.ratio);
        let linear_reduction = 10.0_f32.powf(-reduction_db / 20.0);
        linear_reduction * self.makeup_gain
    }
}

impl BusEffect for Compressor {
    fn process(&mut self, samples: &mut [f32], channels: usize, _sample_rate: u32) {
        if channels == 0 {
            return;
        }
        for frame in samples.chunks_mut(channels) {
            let mut peak = 0.0f32;
            for s in frame.iter() {
                peak = peak.max(s.abs());
            }
            let g = self.compute_gain(peak);
            for s in frame.iter_mut() {
                *s *= g;
            }
        }
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
    }
}

#[derive(Clone)]
pub struct SimpleDelay {
    buffer: Vec<f32>,
    index: usize,
    pub feedback: f32,
    pub mix: f32,
}

impl SimpleDelay {
    pub fn new(delay_ms: f32, feedback: f32, mix: f32, sample_rate: u32) -> Self {
        let len = ((delay_ms * 0.001 * sample_rate as f32).ceil() as usize).max(1);
        Self { buffer: vec![0.0; len], index: 0, feedback, mix }
    }
}

impl BusEffect for SimpleDelay {
    fn process(&mut self, samples: &mut [f32], channels: usize, _sample_rate: u32) {
        if channels == 0 || self.buffer.is_empty() {
            return;
        }
        for frame in samples.chunks_mut(channels) {
            let delayed = self.buffer[self.index];
            for s in frame.iter_mut() {
                let dry = *s;
                *s = dry * (1.0 - self.mix) + delayed * self.mix;
                self.buffer[self.index] = dry + delayed * self.feedback;
            }
            self.index = (self.index + 1) % self.buffer.len();
        }
    }

    fn reset(&mut self) {
        for s in self.buffer.iter_mut() {
            *s = 0.0;
        }
        self.index = 0;
    }
}

#[derive(Clone, Default)]
pub struct Gain {
    pub gain: f32,
}

impl Gain {
    pub fn new(gain: f32) -> Self {
        Self { gain }
    }
}

impl BusEffect for Gain {
    fn process(&mut self, samples: &mut [f32], _channels: usize, _sample_rate: u32) {
        for s in samples.iter_mut() {
            *s *= self.gain;
        }
    }

    fn reset(&mut self) {}
}

pub struct AudioBus {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub solo: bool,
    pub effects: Vec<Box<dyn BusEffect>>,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub buffer: Vec<f32>,
}

impl AudioBus {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            volume: 1.0,
            muted: false,
            solo: false,
            effects: Vec::new(),
            parent: None,
            children: Vec::new(),
            buffer: Vec::new(),
        }
    }

    pub fn push_effect<E: BusEffect + 'static>(&mut self, effect: E) {
        self.effects.push(Box::new(effect));
    }

    pub fn clear_effects(&mut self) {
        self.effects.clear();
    }
}

#[derive(Default)]
pub struct BusGraph {
    buses: Vec<AudioBus>,
    master: Option<usize>,
}

impl BusGraph {
    pub fn new() -> Self {
        let mut graph = Self::default();
        let master = graph.add_bus("master");
        graph.master = Some(master);
        graph
    }

    pub fn add_bus(&mut self, name: impl Into<String>) -> usize {
        let idx = self.buses.len();
        self.buses.push(AudioBus::new(name));
        idx
    }

    pub fn add_child(&mut self, parent: usize, child: usize) {
        if parent < self.buses.len() && child < self.buses.len() && parent != child {
            self.buses[child].parent = Some(parent);
            if !self.buses[parent].children.contains(&child) {
                self.buses[parent].children.push(child);
            }
        }
    }

    pub fn bus(&self, idx: usize) -> Option<&AudioBus> {
        self.buses.get(idx)
    }

    pub fn bus_mut(&mut self, idx: usize) -> Option<&mut AudioBus> {
        self.buses.get_mut(idx)
    }

    pub fn master(&self) -> Option<usize> {
        self.master
    }

    pub fn len(&self) -> usize {
        self.buses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buses.is_empty()
    }

    fn any_solo(&self) -> bool {
        self.buses.iter().any(|b| b.solo)
    }

    pub fn process(
        &mut self,
        input: &[(usize, &[f32])],
        output: &mut [f32],
        channels: usize,
        sample_rate: u32,
    ) {
        if channels == 0 || self.buses.is_empty() {
            return;
        }
        let _frames = output.len() / channels;
        for b in self.buses.iter_mut() {
            b.buffer.clear();
            b.buffer.resize(output.len(), 0.0);
        }

        for (bus_idx, samples) in input.iter() {
            if let Some(bus) = self.buses.get_mut(*bus_idx) {
                let n = samples.len().min(bus.buffer.len());
                for (dst, src) in bus.buffer[..n].iter_mut().zip(samples[..n].iter()) {
                    *dst += src;
                }
            }
        }

        let any_solo = self.any_solo();
        let order = self.topo_order();

        for &idx in &order {
            let parent = self.buses[idx].parent;
            let muted =
                self.buses[idx].muted || (any_solo && !self.buses[idx].solo && parent.is_some());
            let volume = self.buses[idx].volume;

            let mut effects = std::mem::take(&mut self.buses[idx].effects);
            for effect in effects.iter_mut() {
                effect.process(&mut self.buses[idx].buffer, channels, sample_rate);
            }

            if let Some(parent_idx) = parent {
                if !muted {
                    let n = self.buses[idx].buffer.len().min(self.buses[parent_idx].buffer.len());
                    let (min_i, max_i) =
                        if idx < parent_idx { (idx, parent_idx) } else { (parent_idx, idx) };
                    let (left, right) = self.buses.split_at_mut(max_i);
                    let (src_buf, dst_buf) = if idx < parent_idx {
                        (&left[min_i].buffer, &mut right[0].buffer)
                    } else {
                        (&right[0].buffer, &mut left[min_i].buffer)
                    };
                    for (dst, src) in dst_buf[..n].iter_mut().zip(src_buf[..n].iter()) {
                        *dst += src * volume;
                    }
                }
            } else {
                let n = self.buses[idx].buffer.len().min(output.len());
                if muted {
                    for o in output[..n].iter_mut() {
                        *o = 0.0;
                    }
                } else {
                    for (dst, src) in output[..n].iter_mut().zip(self.buses[idx].buffer[..n].iter())
                    {
                        *dst = src * volume;
                    }
                }
            }

            self.buses[idx].effects = effects;
        }
    }

    fn topo_order(&self) -> Vec<usize> {
        let n = self.buses.len();
        let mut visited = vec![false; n];
        let mut order = Vec::with_capacity(n);
        for i in 0..n {
            if !visited[i] {
                self.dfs_visit(i, &mut visited, &mut order);
            }
        }
        order
    }

    fn dfs_visit(&self, idx: usize, visited: &mut [bool], order: &mut Vec<usize>) {
        if visited[idx] {
            return;
        }
        visited[idx] = true;
        for &child in &self.buses[idx].children {
            self.dfs_visit(child, visited, order);
        }
        order.push(idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_lowpass_attenuates_highs() {
        let mut lp = BiquadFilter::low_pass(1000.0, 0.707, 48000);
        let sr = 48000.0_f32;
        let mut high = (0..1024)
            .map(|i| (2.0 * std::f32::consts::PI * 5000.0 * i as f32 / sr).sin() * 0.5)
            .collect::<Vec<_>>();
        let mut low = (0..1024)
            .map(|i| (2.0 * std::f32::consts::PI * 100.0 * i as f32 / sr).sin() * 0.5)
            .collect::<Vec<_>>();
        lp.process(&mut high, 1, 48000);
        lp.process(&mut low, 1, 48000);
        let high_energy: f32 = high[512..].iter().map(|s| s * s).sum();
        let low_energy: f32 = low[512..].iter().map(|s| s * s).sum();
        assert!(
            high_energy < low_energy,
            "lowpass should attenuate high freq: high={} low={}",
            high_energy,
            low_energy
        );
    }

    #[test]
    fn test_compressor_reduces_peak() {
        let mut comp = Compressor::new(0.1, 4.0, 1.0, 100.0, 0.0, 48000);
        let sr = 48000.0_f32;
        let mut samples = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 500.0 * i as f32 / sr).sin() * 0.9)
            .collect::<Vec<_>>();
        comp.process(&mut samples, 1, 48000);
        let peak = samples[2048..].iter().cloned().fold(0.0_f32, f32::max);
        assert!(peak < 0.85, "compressor should reduce peak above threshold, got {}", peak);
    }

    #[test]
    fn test_delay_produces_echo() {
        let mut delay = SimpleDelay::new(10.0, 0.5, 1.0, 48000);
        let delay_samples = delay.buffer.len();
        let mut samples = vec![0.0_f32; delay_samples + 480];
        samples[0] = 1.0;
        delay.process(&mut samples, 1, 48000);
        assert!(
            samples[delay_samples].abs() > 0.3,
            "delayed sample should be present at delay_samples={}, got {}",
            delay_samples,
            samples[delay_samples]
        );
    }

    #[test]
    fn test_bus_graph_routing() {
        let mut graph = BusGraph::new();
        let master = graph.master().unwrap();
        let sfx = graph.add_bus("sfx");
        let music = graph.add_bus("music");
        graph.add_child(master, sfx);
        graph.add_child(master, music);

        let sfx_input = vec![0.5_f32; 256];
        let music_input = vec![0.3_f32; 256];
        let mut output = vec![0.0_f32; 256];

        graph.process(&[(sfx, &sfx_input), (music, &music_input)], &mut output, 1, 48000);

        let peak = output.iter().cloned().fold(0.0_f32, f32::max);
        assert!(peak > 0.7, "master should sum sfx+music, got peak {}", peak);
    }

    #[test]
    fn test_bus_mute() {
        let mut graph = BusGraph::new();
        let master = graph.master().unwrap();
        let sfx = graph.add_bus("sfx");
        graph.add_child(master, sfx);
        graph.bus_mut(sfx).unwrap().muted = true;

        let sfx_input = vec![0.5_f32; 256];
        let mut output = vec![0.0_f32; 256];
        graph.process(&[(sfx, &sfx_input)], &mut output, 1, 48000);

        let peak = output.iter().cloned().fold(0.0_f32, f32::max);
        assert!(peak < 0.001, "muted bus should produce silence, got {}", peak);
    }

    #[test]
    fn test_bus_volume() {
        let mut graph = BusGraph::new();
        let master = graph.master().unwrap();
        let sfx = graph.add_bus("sfx");
        graph.add_child(master, sfx);
        graph.bus_mut(sfx).unwrap().volume = 0.5;

        let sfx_input = vec![1.0_f32; 256];
        let mut output = vec![0.0_f32; 256];
        graph.process(&[(sfx, &sfx_input)], &mut output, 1, 48000);

        let peak = output.iter().cloned().fold(0.0_f32, f32::max);
        assert!((peak - 0.5).abs() < 0.05, "volume 0.5 should scale input by half, got {}", peak);
    }

    #[test]
    fn test_bus_solo() {
        let mut graph = BusGraph::new();
        let master = graph.master().unwrap();
        let sfx = graph.add_bus("sfx");
        let music = graph.add_bus("music");
        graph.add_child(master, sfx);
        graph.add_child(master, music);
        graph.bus_mut(sfx).unwrap().solo = true;

        let sfx_input = vec![0.5_f32; 256];
        let music_input = vec![0.5_f32; 256];
        let mut output = vec![0.0_f32; 256];
        graph.process(&[(sfx, &sfx_input), (music, &music_input)], &mut output, 1, 48000);

        let peak = output.iter().cloned().fold(0.0_f32, f32::max);
        assert!((peak - 0.5).abs() < 0.1, "solo should mute non-solo buses, got peak {}", peak);
    }

    #[test]
    fn test_peak_eq_boosts_band() {
        let mut peak = BiquadFilter::peak(1000.0, 12.0, 1.0, 48000);
        let sr = 48000.0_f32;
        let mut samples = (0..2048)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr).sin() * 0.1)
            .collect::<Vec<_>>();
        peak.process(&mut samples, 1, 48000);
        let steady_state = samples[1024..].iter().cloned().fold(0.0_f32, f32::max);
        assert!(
            steady_state.abs() > 0.1,
            "peak EQ with positive gain should boost, got {}",
            steady_state
        );
    }

    #[test]
    fn test_gain_effect() {
        let mut gain = Gain::new(2.0);
        let mut samples = vec![0.5_f32; 64];
        gain.process(&mut samples, 1, 48000);
        assert!((samples[0] - 1.0).abs() < 1e-6);
    }
}
