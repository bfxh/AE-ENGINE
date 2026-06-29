use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MaterialParams {
    pub hardness: f32,
    pub density: f32,
    pub roughness: f32,
    pub toughness: f32,
    pub youngs_modulus: f32,
    pub poisson_ratio: f32,
}

impl Default for MaterialParams {
    fn default() -> Self {
        Self {
            hardness: 0.5,
            density: 1000.0,
            roughness: 0.3,
            toughness: 0.5,
            youngs_modulus: 2.0e9,
            poisson_ratio: 0.3,
        }
    }
}

impl MaterialParams {
    pub fn metal() -> Self {
        Self {
            hardness: 0.8,
            density: 7800.0,
            roughness: 0.1,
            toughness: 0.9,
            youngs_modulus: 2.0e11,
            poisson_ratio: 0.3,
        }
    }

    pub fn wood() -> Self {
        Self {
            hardness: 0.3,
            density: 600.0,
            roughness: 0.5,
            toughness: 0.4,
            youngs_modulus: 1.0e10,
            poisson_ratio: 0.35,
        }
    }

    pub fn glass() -> Self {
        Self {
            hardness: 0.9,
            density: 2500.0,
            roughness: 0.05,
            toughness: 0.1,
            youngs_modulus: 7.0e10,
            poisson_ratio: 0.2,
        }
    }

    pub fn stone() -> Self {
        Self {
            hardness: 0.7,
            density: 2700.0,
            roughness: 0.6,
            toughness: 0.3,
            youngs_modulus: 5.0e10,
            poisson_ratio: 0.2,
        }
    }

    pub fn plastic() -> Self {
        Self {
            hardness: 0.2,
            density: 1100.0,
            roughness: 0.2,
            toughness: 0.6,
            youngs_modulus: 2.0e9,
            poisson_ratio: 0.4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partial {
    pub frequency: f32,
    pub amplitude: f32,
    pub decay_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactSound {
    pub partials: Vec<Partial>,
    pub sample_rate: f32,
    pub duration: f32,
}

impl ImpactSound {
    pub fn synthesize(material: &MaterialParams, mass: f32, velocity: f32) -> Self {
        let impact_energy = 0.5 * mass * velocity * velocity;
        let base_freq = (material.youngs_modulus / material.density).sqrt() * 0.1;
        let num_partials = 12;
        let duration = (material.hardness * 0.5 + 0.1).min(2.0);

        let mut partials = Vec::with_capacity(num_partials);
        for i in 0..num_partials {
            let harmonic = i as f32 + 1.0;
            let freq = base_freq * harmonic * (1.0 + material.poisson_ratio * (i as f32 * 0.1));
            let amp = (impact_energy / (harmonic * harmonic + 1.0)).min(1.0)
                * (1.0 - material.roughness * i as f32 * 0.05);
            let decay = (1.0 + harmonic * 2.0) / (duration * material.toughness.max(0.1));

            partials.push(Partial { frequency: freq, amplitude: amp.max(0.0), decay_rate: decay });
        }

        Self { partials, sample_rate: 44100.0, duration }
    }

    pub fn sample(&self, time: f32) -> f32 {
        if time > self.duration {
            return 0.0;
        }
        let mut value = 0.0_f32;
        for p in &self.partials {
            let envelope = (-p.decay_rate * time).exp();
            let osc = (2.0 * std::f32::consts::PI * p.frequency * time).sin();
            value += p.amplitude * envelope * osc;
        }
        value.tanh()
    }

    pub fn render(&self) -> Vec<f32> {
        let num_samples = (self.duration * self.sample_rate) as usize;
        let mut buffer = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / self.sample_rate;
            buffer.push(self.sample(t));
        }
        buffer
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrictionSound {
    pub partials: Vec<Partial>,
    pub noise_amplitude: f32,
    pub sample_rate: f32,
}

impl FrictionSound {
    pub fn synthesize(material: &MaterialParams, normal_force: f32, velocity: f32) -> Self {
        let friction_coeff = material.roughness * 0.8 + 0.1;
        let friction_energy = friction_coeff * normal_force * velocity;
        let num_partials = 16;

        let mut partials = Vec::with_capacity(num_partials);
        for i in 0..num_partials {
            let harmonic = i as f32 + 1.0;
            let base_freq = 50.0 + material.roughness * 500.0;
            let freq = base_freq * harmonic * (1.0 + (material.hardness - 0.5) * 0.3);
            let amp = (friction_energy / (harmonic * harmonic.sqrt() + 1.0)).min(0.5) * 0.3;
            let decay = (1.0 + harmonic * 0.5) / (material.toughness.max(0.1) * 0.5);

            partials.push(Partial { frequency: freq, amplitude: amp.max(0.0), decay_rate: decay });
        }

        Self {
            partials,
            noise_amplitude: friction_energy * 0.1 * material.roughness,
            sample_rate: 44100.0,
        }
    }

    pub fn sample(&self, time: f32, _seed: u32) -> f32 {
        let mut value = 0.0_f32;
        for p in &self.partials {
            let freq_mod = p.frequency * (1.0 + (time * 17.0).sin() * 0.02);
            let osc = (2.0 * std::f32::consts::PI * freq_mod * time).sin();
            value += p.amplitude * osc;
        }
        let pseudo_noise = (time * 7919.0).sin() * 0.5 + (time * 104729.0).sin() * 0.5;
        value += pseudo_noise * self.noise_amplitude * 0.1;
        value.tanh() * 0.5
    }

    pub fn render(&self, duration: f32) -> Vec<f32> {
        let num_samples = (duration * self.sample_rate) as usize;
        let mut buffer = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / self.sample_rate;
            buffer.push(self.sample(t, i as u32));
        }
        buffer
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FractureSound {
    pub crack_events: Vec<CrackEvent>,
    pub sample_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrackEvent {
    pub time: f32,
    pub amplitude: f32,
    pub frequency: f32,
    pub decay: f32,
    pub partials: Vec<Partial>,
}

impl FractureSound {
    pub fn synthesize(material: &MaterialParams, fracture_energy: f32, crack_count: usize) -> Self {
        let mut events = Vec::with_capacity(crack_count);
        let total_duration = 0.5_f32.min(fracture_energy * 0.1);
        let base_freq = (material.youngs_modulus / material.density).sqrt() * 0.05;

        for i in 0..crack_count {
            let t = if crack_count > 1 {
                (i as f32 / (crack_count - 1) as f32) * total_duration
            } else {
                0.0
            };

            let energy_per_crack = fracture_energy / crack_count as f32;
            let amp = energy_per_crack.min(1.0) * (1.0 - material.toughness * 0.5);
            let freq = base_freq * (1.0 + (i as f32 * 0.3).sin());
            let decay = 20.0 / (material.toughness.max(0.05) * 0.5);

            let mut partials = Vec::new();
            for j in 0..6 {
                let harmonic = j as f32 + 1.0;
                let pfreq = freq * harmonic * (1.0 + (j as f32 * 0.07));
                let pamp = amp / (harmonic * harmonic + 1.0)
                    * (1.0 - material.roughness * j as f32 * 0.08);
                let pdecay = decay * (1.0 + harmonic * 0.3);

                partials.push(Partial {
                    frequency: pfreq,
                    amplitude: pamp.max(0.0),
                    decay_rate: pdecay,
                });
            }

            events.push(CrackEvent { time: t, amplitude: amp, frequency: freq, decay, partials });
        }

        Self { crack_events: events, sample_rate: 44100.0 }
    }

    pub fn sample(&self, time: f32) -> f32 {
        let mut value = 0.0_f32;
        for event in &self.crack_events {
            let local_time = time - event.time;
            if !(0.0..=0.5).contains(&local_time) {
                continue;
            }
            for p in &event.partials {
                let envelope = (-p.decay_rate * local_time).exp();
                let osc = (2.0 * std::f32::consts::PI * p.frequency * local_time).sin();
                value += p.amplitude * envelope * osc;
            }
        }
        value.tanh()
    }

    pub fn render(&self, duration: f32) -> Vec<f32> {
        let num_samples = (duration * self.sample_rate) as usize;
        let mut buffer = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / self.sample_rate;
            buffer.push(self.sample(t));
        }
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impact_synthesis() {
        let mat = MaterialParams::metal();
        let sound = ImpactSound::synthesize(&mat, 1.0, 5.0);
        assert!(!sound.partials.is_empty());
        assert!(sound.duration > 0.0);
    }

    #[test]
    fn test_impact_sample() {
        let mat = MaterialParams::wood();
        let sound = ImpactSound::synthesize(&mat, 0.5, 3.0);
        let s = sound.sample(0.1);
        assert!(s.is_finite());
        let s_end = sound.sample(sound.duration + 0.1);
        assert!((s_end - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_impact_render() {
        let mat = MaterialParams::glass();
        let sound = ImpactSound::synthesize(&mat, 0.2, 10.0);
        let buffer = sound.render();
        assert!(!buffer.is_empty());
        assert!(buffer.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_friction_synthesis() {
        let mat = MaterialParams::stone();
        let sound = FrictionSound::synthesize(&mat, 10.0, 2.0);
        assert!(!sound.partials.is_empty());
        assert!(sound.noise_amplitude > 0.0);
    }

    #[test]
    fn test_friction_sample() {
        let mat = MaterialParams::plastic();
        let sound = FrictionSound::synthesize(&mat, 5.0, 1.0);
        let s = sound.sample(0.1, 0);
        assert!(s.is_finite());
    }

    #[test]
    fn test_friction_render() {
        let mat = MaterialParams::metal();
        let sound = FrictionSound::synthesize(&mat, 8.0, 3.0);
        let buffer = sound.render(0.5);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_fracture_synthesis() {
        let mat = MaterialParams::glass();
        let sound = FractureSound::synthesize(&mat, 10.0, 5);
        assert_eq!(sound.crack_events.len(), 5);
    }

    #[test]
    fn test_fracture_sample() {
        let mat = MaterialParams::stone();
        let sound = FractureSound::synthesize(&mat, 8.0, 3);
        let s = sound.sample(0.1);
        assert!(s.is_finite());
    }

    #[test]
    fn test_fracture_render() {
        let mat = MaterialParams::wood();
        let sound = FractureSound::synthesize(&mat, 5.0, 4);
        let buffer = sound.render(0.5);
        assert!(!buffer.is_empty());
        assert!(buffer.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_material_presets() {
        let metal = MaterialParams::metal();
        assert!(metal.hardness > 0.5);
        let wood = MaterialParams::wood();
        assert!(wood.hardness < 0.5);
        let glass = MaterialParams::glass();
        assert!(glass.toughness < 0.3);
    }
}
