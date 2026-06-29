use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombFilter {
    pub delay_line: Vec<f32>,
    pub feedback: f32,
    pub index: usize,
}

impl CombFilter {
    pub fn new(delay_samples: usize, feedback: f32) -> Self {
        Self { delay_line: vec![0.0; delay_samples], feedback, index: 0 }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let delayed = self.delay_line[self.index];
        let output = input + delayed * self.feedback;
        self.delay_line[self.index] = output;
        self.index = (self.index + 1) % self.delay_line.len();
        output
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllPassFilter {
    pub delay_line: Vec<f32>,
    pub feedback: f32,
    pub index: usize,
}

impl AllPassFilter {
    pub fn new(delay_samples: usize, feedback: f32) -> Self {
        Self { delay_line: vec![0.0; delay_samples], feedback, index: 0 }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let delayed = self.delay_line[self.index];
        let feedback_signal = input + delayed * self.feedback;
        let output = delayed + feedback_signal * (-self.feedback);
        self.delay_line[self.index] = feedback_signal;
        self.index = (self.index + 1) % self.delay_line.len();
        output
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReverbPreset {
    Room,
    Hall,
    Chamber,
    Cathedral,
    Outdoors,
}

impl ReverbPreset {
    pub fn all() -> [ReverbPreset; 5] {
        [
            ReverbPreset::Room,
            ReverbPreset::Hall,
            ReverbPreset::Chamber,
            ReverbPreset::Cathedral,
            ReverbPreset::Outdoors,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverbParams {
    pub room_size: f32,
    pub damping: f32,
    pub wet_mix: f32,
    pub dry_mix: f32,
    pub width: f32,
    pub pre_delay: f32,
    pub decay_time: f32,
    pub early_reflections: usize,
    pub absorption: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            room_size: 0.5,
            damping: 0.5,
            wet_mix: 0.33,
            dry_mix: 0.67,
            width: 1.0,
            pre_delay: 0.02,
            decay_time: 1.5,
            early_reflections: 8,
            absorption: 0.3,
        }
    }
}

impl ReverbParams {
    pub fn from_preset(preset: ReverbPreset) -> Self {
        match preset {
            ReverbPreset::Room => Self {
                room_size: 0.3,
                damping: 0.6,
                wet_mix: 0.25,
                dry_mix: 0.75,
                width: 0.8,
                pre_delay: 0.01,
                decay_time: 0.5,
                early_reflections: 4,
                absorption: 0.4,
            },
            ReverbPreset::Hall => Self {
                room_size: 0.7,
                damping: 0.4,
                wet_mix: 0.35,
                dry_mix: 0.65,
                width: 1.0,
                pre_delay: 0.03,
                decay_time: 2.0,
                early_reflections: 12,
                absorption: 0.2,
            },
            ReverbPreset::Chamber => Self {
                room_size: 0.5,
                damping: 0.3,
                wet_mix: 0.4,
                dry_mix: 0.6,
                width: 0.9,
                pre_delay: 0.02,
                decay_time: 1.5,
                early_reflections: 8,
                absorption: 0.15,
            },
            ReverbPreset::Cathedral => Self {
                room_size: 0.95,
                damping: 0.2,
                wet_mix: 0.5,
                dry_mix: 0.5,
                width: 1.0,
                pre_delay: 0.06,
                decay_time: 4.5,
                early_reflections: 16,
                absorption: 0.1,
            },
            ReverbPreset::Outdoors => Self {
                room_size: 0.05,
                damping: 0.9,
                wet_mix: 0.1,
                dry_mix: 0.9,
                width: 0.5,
                pre_delay: 0.0,
                decay_time: 0.1,
                early_reflections: 0,
                absorption: 0.95,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reverb {
    pub params: ReverbParams,
    pub sample_rate: f32,
    pub combs: Vec<CombFilter>,
    pub allpasses: Vec<AllPassFilter>,
    pub early_delay_line: Vec<f32>,
    pub early_index: usize,
    pub pre_delay_buffer: Vec<f32>,
    pub pre_delay_index: usize,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let params = ReverbParams::default();
        let comb_delays = [1557.0, 1617.0, 1491.0, 1422.0];
        let comb_feedback: [f32; 4] = [0.773, 0.802, 0.753, 0.733];
        let allpass_delays = [556.0, 441.0];
        let allpass_feedback = [0.5, 0.5];

        let combs: Vec<CombFilter> = comb_delays
            .iter()
            .zip(comb_feedback.iter())
            .map(|(&d, &f)| CombFilter::new((d * sample_rate / 44100.0) as usize, f))
            .collect();

        let allpasses: Vec<AllPassFilter> = allpass_delays
            .iter()
            .zip(allpass_feedback.iter())
            .map(|(&d, &f)| AllPassFilter::new((d * sample_rate / 44100.0) as usize, f))
            .collect();

        Self {
            params,
            sample_rate,
            combs,
            allpasses,
            early_delay_line: vec![0.0; (sample_rate * 0.1) as usize],
            early_index: 0,
            pre_delay_buffer: vec![0.0; (sample_rate * 0.1) as usize],
            pre_delay_index: 0,
        }
    }

    pub fn from_preset(preset: ReverbPreset, sample_rate: f32) -> Self {
        let mut reverb = Self::new(sample_rate);
        reverb.set_params(ReverbParams::from_preset(preset));
        reverb
    }

    pub fn set_params(&mut self, params: ReverbParams) {
        let comb_feedback: [f32; 4] = [
            params.room_size * 0.773 + 0.1,
            params.room_size * 0.802 + 0.1,
            params.room_size * 0.753 + 0.1,
            params.room_size * 0.733 + 0.1,
        ];

        for (i, comb) in self.combs.iter_mut().enumerate() {
            comb.feedback = (comb_feedback[i] * (1.0 - params.damping * 0.5)).min(0.95);
        }

        for ap in &mut self.allpasses {
            ap.feedback = params.room_size * 0.5;
        }

        self.params = params;
    }

    pub fn process(&mut self, input: f32) -> (f32, f32) {
        let pre_delay_samples = (self.params.pre_delay * self.sample_rate) as usize;
        let pre_len = self.pre_delay_buffer.len();
        let pre_idx = self.pre_delay_index;
        let pre_out = self.pre_delay_buffer[pre_idx];
        self.pre_delay_buffer[pre_idx] = input;
        self.pre_delay_index = (pre_idx + 1) % pre_len;

        let delayed = if pre_delay_samples < pre_len {
            self.pre_delay_buffer[(pre_idx + pre_len - pre_delay_samples) % pre_len]
        } else {
            pre_out
        };

        let mut early_total = 0.0_f32;
        let early_len = self.early_delay_line.len();
        for i in 0..self.params.early_reflections {
            let tap = ((i + 1) as f32 * 0.007 * self.sample_rate) as usize % early_len;
            let idx = (self.early_index + early_len - tap) % early_len;
            early_total += self.early_delay_line[idx] * (1.0 / (i + 1) as f32);
        }

        let early = if self.params.early_reflections > 0 {
            early_total / self.params.early_reflections as f32
        } else {
            0.0
        };

        self.early_delay_line[self.early_index] = delayed;
        self.early_index = (self.early_index + 1) % early_len;

        let mut comb_output = 0.0_f32;
        for comb in &mut self.combs {
            comb_output += comb.process(delayed + early * 0.3);
        }
        comb_output /= self.combs.len() as f32;

        let mut ap_output = comb_output;
        for ap in &mut self.allpasses {
            ap_output = ap.process(ap_output);
        }

        let wet = ap_output * self.params.wet_mix;
        let dry = input * self.params.dry_mix;
        let width = self.params.width;

        let left = dry + wet * (1.0 + width) * 0.5;
        let right = dry + wet * (1.0 - width) * 0.5;

        (left.clamp(-1.0, 1.0), right.clamp(-1.0, 1.0))
    }

    pub fn process_mono(&mut self, input: f32) -> f32 {
        let (left, right) = self.process(input);
        (left + right) * 0.5
    }

    pub fn compute_rt60(
        volume: f32,
        surface_area: f32,
        absorption_coefficient: f32,
        speed_of_sound: f32,
    ) -> f32 {
        if absorption_coefficient < 0.001 {
            return volume * 0.161 / (surface_area * 0.001);
        }
        let sabine = 0.161 * volume / (surface_area * absorption_coefficient);
        let eyring = if absorption_coefficient > 0.99 {
            sabine
        } else {
            0.161 * volume / (-surface_area * (1.0 - absorption_coefficient).ln())
        };
        let air_absorption = 0.004 * volume / speed_of_sound;
        sabine.min(eyring) + air_absorption
    }

    pub fn reset(&mut self) {
        for comb in &mut self.combs {
            for v in &mut comb.delay_line {
                *v = 0.0;
            }
        }
        for ap in &mut self.allpasses {
            for v in &mut ap.delay_line {
                *v = 0.0;
            }
        }
        for v in &mut self.early_delay_line {
            *v = 0.0;
        }
        for v in &mut self.pre_delay_buffer {
            *v = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comb_filter() {
        let mut comb = CombFilter::new(10, 0.5);
        let out = comb.process(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_allpass_filter() {
        let mut ap = AllPassFilter::new(10, 0.5);
        let out = ap.process(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_reverb_creation() {
        let reverb = Reverb::new(44100.0);
        assert_eq!(reverb.combs.len(), 4);
        assert_eq!(reverb.allpasses.len(), 2);
    }

    #[test]
    fn test_reverb_presets() {
        let presets = ReverbPreset::all();
        assert_eq!(presets.len(), 5);
        for preset in presets {
            let reverb = Reverb::from_preset(preset, 44100.0);
            assert!(reverb.params.decay_time > 0.0);
        }
    }

    #[test]
    fn test_reverb_process() {
        let mut reverb = Reverb::from_preset(ReverbPreset::Room, 44100.0);
        let (left, right) = reverb.process(1.0);
        assert!(left.is_finite());
        assert!(right.is_finite());
    }

    #[test]
    fn test_reverb_process_mono() {
        let mut reverb = Reverb::from_preset(ReverbPreset::Hall, 44100.0);
        let out = reverb.process_mono(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_rt60() {
        let rt60 = Reverb::compute_rt60(100.0, 120.0, 0.3, 343.0);
        assert!(rt60 > 0.0);
        assert!(rt60 < 10.0);
    }

    #[test]
    fn test_rt60_high_absorption() {
        let rt60 = Reverb::compute_rt60(100.0, 120.0, 0.99, 343.0);
        assert!(rt60 > 0.0);
    }

    #[test]
    fn test_set_params() {
        let mut reverb = Reverb::new(44100.0);
        let params = ReverbParams::from_preset(ReverbPreset::Cathedral);
        reverb.set_params(params);
        assert!(reverb.params.decay_time > 3.0);
    }

    #[test]
    fn test_reset() {
        let mut reverb = Reverb::from_preset(ReverbPreset::Chamber, 44100.0);
        for _ in 0..100 {
            reverb.process(1.0);
        }
        reverb.reset();
        let (left, right) = reverb.process(0.0);
        assert!((left - 0.0).abs() < 0.01);
        assert!((right - 0.0).abs() < 0.01);
    }
}
