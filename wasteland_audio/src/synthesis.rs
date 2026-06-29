use rand::Rng;

#[derive(Debug, Clone)]
pub struct ImpactSound {
    pub amplitude: f32,
    pub frequency: f32,
    pub decay: f32,
    pub material_modes: Vec<ModalFrequency>,
}

#[derive(Debug, Clone, Copy)]
pub struct ModalFrequency {
    pub freq: f32,
    pub amplitude: f32,
    pub decay: f32,
}

impl ImpactSound {
    pub fn from_materials(
        source_material: &str,
        target_material: &str,
        impact_velocity: f32,
        mass: f32,
        rng: &mut impl Rng,
    ) -> Self {
        let (base_freq, base_amp, base_decay) = material_impact_params(source_material);
        let (target_freq, target_amp, target_decay) = material_impact_params(target_material);

        let energy = 0.5 * mass * impact_velocity * impact_velocity;
        let amplitude = (energy * base_amp * target_amp * 0.01).min(1.0);
        let frequency = (base_freq + target_freq) * 0.5;
        let decay = (base_decay + target_decay) * 0.5;

        let mode_count = 5 + (mass * 10.0) as usize;
        let modes: Vec<ModalFrequency> = (0..mode_count)
            .map(|i| {
                let harmonic = (i + 1) as f32;
                let freq_jitter = rng.gen_range(-0.05..0.05);
                let decay_jitter = rng.gen_range(-0.1..0.1);
                ModalFrequency {
                    freq: frequency * harmonic * (1.0 + freq_jitter),
                    amplitude: amplitude / (harmonic * 1.5),
                    decay: decay * (1.0 + decay_jitter) * harmonic.sqrt(),
                }
            })
            .collect();

        ImpactSound { amplitude, frequency, decay, material_modes: modes }
    }

    pub fn synthesize(&self, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
        let total_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut output = vec![0.0f32; total_samples];

        for mode in &self.material_modes {
            let omega = 2.0 * std::f32::consts::PI * mode.freq;
            let decay_per_sample = (-mode.decay / sample_rate as f32).exp();

            for (i, sample) in output.iter_mut().enumerate() {
                let t = i as f32 / sample_rate as f32;
                let envelope = (decay_per_sample.powi(i as i32)).min(1.0);
                *sample += mode.amplitude * (omega * t).sin() * envelope;
            }
        }

        let peak = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if peak > 0.0 {
            let scale = self.amplitude / peak;
            for sample in &mut output {
                *sample *= scale;
            }
        }

        output
    }
}

pub struct FrictionSound {
    pub base_frequency: f32,
    pub roughness: f32,
    pub pressure: f32,
    pub velocity: f32,
}

impl FrictionSound {
    pub fn new(roughness: f32, pressure: f32, velocity: f32) -> Self {
        FrictionSound { base_frequency: 200.0 + roughness * 2000.0, roughness, pressure, velocity }
    }

    pub fn synthesize(&self, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
        let total_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut output = vec![0.0f32; total_samples];
        let mut rng = rand::thread_rng();

        let amplitude = (self.pressure * self.velocity * 0.1).min(1.0);

        for (i, sample) in output.iter_mut().enumerate() {
            let t = i as f32 / sample_rate as f32;
            let noise = rng.gen_range(-1.0..1.0);
            let micro_vibration = (2.0 * std::f32::consts::PI * self.base_frequency * t).sin();

            let stick_slip = if (t * self.velocity * 10.0).fract() < 0.1 { 1.0 } else { 0.0 };

            *sample = amplitude
                * (micro_vibration * 0.5 + noise * self.roughness * 0.3 + stick_slip * 0.2);
        }

        output
    }
}

pub struct FractureSound {
    pub crack_count: usize,
    pub material_density: f32,
    pub stress_level: f32,
}

impl FractureSound {
    pub fn new(crack_count: usize, material_density: f32, stress_level: f32) -> Self {
        FractureSound { crack_count, material_density, stress_level }
    }

    pub fn synthesize(&self, sample_rate: u32) -> Vec<f32> {
        let duration = 0.1 + self.crack_count as f32 * 0.05;
        let total_samples = (sample_rate as f32 * duration) as usize;
        let mut output = vec![0.0f32; total_samples];
        let mut rng = rand::thread_rng();

        for crack in 0..self.crack_count {
            let crack_time = crack as f32 / self.crack_count as f32 * duration;
            let crack_start = (crack_time * sample_rate as f32) as usize;
            let crack_length = (0.005 * sample_rate as f32) as usize;

            let freq = 500.0 + self.material_density * 2000.0 + rng.gen_range(-200.0..200.0);
            let amp = (self.stress_level * 0.5).min(1.0);

            for i in 0..crack_length {
                let idx = crack_start + i;
                if idx >= total_samples {
                    break;
                }
                let t = i as f32 / sample_rate as f32;
                let envelope = (-t * 500.0).exp();
                let noise = rng.gen_range(-1.0..1.0);
                output[idx] +=
                    amp * noise * envelope * (2.0 * std::f32::consts::PI * freq * t).sin();
            }
        }

        output
    }
}

fn material_impact_params(material: &str) -> (f32, f32, f32) {
    match material.to_lowercase().as_str() {
        "metal" | "iron" | "steel" => (800.0, 0.8, 3.0),
        "wood" | "oak" => (400.0, 0.5, 5.0),
        "stone" | "rock" | "concrete" => (300.0, 0.6, 4.0),
        "glass" | "crystal" => (2000.0, 0.7, 1.5),
        "flesh" | "skin" => (100.0, 0.3, 8.0),
        "bone" => (600.0, 0.4, 6.0),
        "cloth" | "fabric" | "leather" => (80.0, 0.2, 10.0),
        "water" | "liquid" => (50.0, 0.1, 12.0),
        _ => (400.0, 0.5, 5.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impact_synthesis() {
        let mut rng = rand::thread_rng();
        let impact = ImpactSound::from_materials("metal", "wood", 5.0, 2.0, &mut rng);
        let samples = impact.synthesize(44100, 0.5);
        assert!(!samples.is_empty());
        let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(peak > 0.0);
    }

    #[test]
    fn test_impact_modes() {
        let mut rng = rand::thread_rng();
        let impact = ImpactSound::from_materials("stone", "stone", 10.0, 3.0, &mut rng);
        assert!(!impact.material_modes.is_empty());
        assert!(impact.material_modes.len() > 5);
    }

    #[test]
    fn test_friction_synthesis() {
        let friction = FrictionSound::new(0.5, 0.8, 1.0);
        let samples = friction.synthesize(44100, 0.2);
        assert!(!samples.is_empty());
    }

    #[test]
    fn test_fracture_synthesis() {
        let fracture = FractureSound::new(5, 2.0, 0.9);
        let samples = fracture.synthesize(44100);
        assert!(!samples.is_empty());
    }

    #[test]
    fn test_material_params() {
        let (freq, amp, decay) = material_impact_params("glass");
        assert!(freq > 1000.0);
        assert!(amp > 0.0);
        assert!(decay > 0.0);
    }
}
