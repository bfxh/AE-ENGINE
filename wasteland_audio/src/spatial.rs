use glam::Vec3;

#[derive(Debug, Clone)]
pub struct SpatialAudio {
    pub listener_pos: Vec3,
    pub listener_forward: Vec3,
    pub listener_up: Vec3,
    pub speed_of_sound: f32,
    pub hrtf_enabled: bool,
}

impl Default for SpatialAudio {
    fn default() -> Self {
        SpatialAudio {
            listener_pos: Vec3::ZERO,
            listener_forward: Vec3::Z,
            listener_up: Vec3::Y,
            speed_of_sound: 343.0,
            hrtf_enabled: true,
        }
    }
}

impl SpatialAudio {
    pub fn new(listener_pos: Vec3, forward: Vec3, up: Vec3) -> Self {
        SpatialAudio {
            listener_pos,
            listener_forward: forward.normalize(),
            listener_up: up.normalize(),
            speed_of_sound: 343.0,
            hrtf_enabled: true,
        }
    }

    pub fn pan_stereo(&self, source_pos: Vec3) -> (f32, f32) {
        let to_source = (source_pos - self.listener_pos).normalize_or_zero();
        let right = self.listener_forward.cross(self.listener_up).normalize_or_zero();
        let lateral = to_source.dot(right);

        let angle = lateral.clamp(-1.0, 1.0);
        let pan = (angle + 1.0) / 2.0;

        let left = (1.0 - pan).sqrt();
        let right_pan = pan.sqrt();
        (left, right_pan)
    }

    pub fn hrtf_pan(&self, source_pos: Vec3, _frequency: f32) -> (f32, f32, f32) {
        let to_source = (source_pos - self.listener_pos).normalize_or_zero();
        if to_source == Vec3::ZERO {
            return (1.0, 1.0, 0.0);
        }

        let right = self.listener_forward.cross(self.listener_up).normalize_or_zero();
        let azimuth = to_source.dot(right).atan2(to_source.dot(self.listener_forward));
        let elevation = to_source.dot(self.listener_up).asin();

        let itd = self.interaural_time_difference(azimuth);
        let ild = self.interaural_level_difference(azimuth, elevation);

        let left = 10.0f32.powf(-ild / 20.0).min(1.0);
        let right = 1.0;
        let delay = itd;

        (left, right, delay)
    }

    fn interaural_time_difference(&self, azimuth: f32) -> f32 {
        let head_radius = 0.0875;
        let angle = azimuth.abs().min(std::f32::consts::FRAC_PI_2);
        (head_radius / self.speed_of_sound) * (angle + angle.sin())
    }

    fn interaural_level_difference(&self, azimuth: f32, elevation: f32) -> f32 {
        let az = azimuth.abs();
        let el = elevation.abs();
        let base = 20.0 * az.sin();
        let elevation_factor = 1.0 - el * 0.5;
        base * elevation_factor
    }

    pub fn distance_attenuation(&self, source_pos: Vec3, max_distance: f32) -> f32 {
        let distance = self.listener_pos.distance(source_pos);
        if distance < 0.1 {
            return 1.0;
        }
        if distance > max_distance {
            return 0.0;
        }
        let ref_distance = 1.0;
        let rolloff = (ref_distance / distance).min(1.0);
        let clamped = (distance / max_distance).clamp(0.0, 1.0);
        rolloff * (1.0 - clamped * clamped)
    }

    pub fn reverb_early_reflections(
        &self,
        source_pos: Vec3,
        room_size: Vec3,
        wall_absorption: f32,
    ) -> Vec<Reflection> {
        let mut reflections = Vec::new();
        let to_source = source_pos - self.listener_pos;

        let surfaces = [
            (Vec3::X, room_size.x * 0.5),
            (Vec3::NEG_X, room_size.x * 0.5),
            (Vec3::Y, room_size.y * 0.5),
            (Vec3::NEG_Y, room_size.y * 0.5),
            (Vec3::Z, room_size.z * 0.5),
            (Vec3::NEG_Z, room_size.z * 0.5),
        ];

        for (normal, _extent) in &surfaces {
            let mirrored = to_source - 2.0 * to_source.dot(*normal) * *normal;
            let distance = mirrored.length();
            let delay = distance / self.speed_of_sound;
            let attenuation = (1.0 - wall_absorption) / distance.max(0.1);

            reflections.push(Reflection {
                delay,
                attenuation: attenuation.min(1.0),
                direction: mirrored.normalize_or_zero(),
            });
        }

        reflections
            .sort_by(|a, b| a.delay.partial_cmp(&b.delay).unwrap_or(std::cmp::Ordering::Equal));
        reflections
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Reflection {
    pub delay: f32,
    pub attenuation: f32,
    pub direction: Vec3,
}

pub struct BinauralMixer {
    pub left_buffer: Vec<f32>,
    pub right_buffer: Vec<f32>,
    pub sample_rate: u32,
}

impl BinauralMixer {
    pub fn new(sample_rate: u32, buffer_size: usize) -> Self {
        BinauralMixer {
            left_buffer: vec![0.0; buffer_size],
            right_buffer: vec![0.0; buffer_size],
            sample_rate,
        }
    }

    pub fn mix_source(
        &mut self,
        spatial: &SpatialAudio,
        source_pos: Vec3,
        samples: &[f32],
        volume: f32,
    ) {
        let (left_gain, right_gain, delay) = spatial.hrtf_pan(source_pos, 1000.0);
        let attenuation = spatial.distance_attenuation(source_pos, 100.0);
        let gain = volume * attenuation;

        let delay_samples = (delay * self.sample_rate as f32) as usize;

        for (i, sample) in samples.iter().enumerate() {
            let val = sample * gain;
            let idx = i + delay_samples;
            if idx < self.left_buffer.len() {
                self.left_buffer[idx] += val * left_gain;
                self.right_buffer[idx] += val * right_gain;
            }
        }
    }

    pub fn clear(&mut self) {
        self.left_buffer.fill(0.0);
        self.right_buffer.fill(0.0);
    }

    pub fn interleaved_stereo(&self) -> Vec<f32> {
        let mut output = Vec::with_capacity(self.left_buffer.len() * 2);
        for i in 0..self.left_buffer.len() {
            output.push(self.left_buffer[i]);
            output.push(self.right_buffer[i]);
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stereo_pan_center() {
        let spatial = SpatialAudio::default();
        let (l, r) = spatial.pan_stereo(Vec3::new(0.0, 0.0, 1.0));
        assert!((l - r).abs() < 0.1);
    }

    #[test]
    fn test_stereo_pan_left() {
        let spatial = SpatialAudio::default();
        let (l, r) = spatial.pan_stereo(Vec3::new(1.0, 0.0, 1.0));
        assert!(l > r);
    }

    #[test]
    fn test_stereo_pan_right() {
        let spatial = SpatialAudio::default();
        let (l, r) = spatial.pan_stereo(Vec3::new(-1.0, 0.0, 1.0));
        assert!(r > l);
    }

    #[test]
    fn test_distance_attenuation() {
        let spatial = SpatialAudio::default();
        let near = spatial.distance_attenuation(Vec3::new(0.0, 0.0, 1.0), 100.0);
        let far = spatial.distance_attenuation(Vec3::new(0.0, 0.0, 50.0), 100.0);
        assert!(near > far);
    }

    #[test]
    fn test_reverb_reflections() {
        let spatial = SpatialAudio::default();
        let reflections = spatial.reverb_early_reflections(
            Vec3::new(1.0, 0.5, 2.0),
            Vec3::new(5.0, 3.0, 4.0),
            0.3,
        );
        assert_eq!(reflections.len(), 6);
    }

    #[test]
    fn test_binaural_mixer() {
        let mut mixer = BinauralMixer::new(44100, 1024);
        let spatial = SpatialAudio::default();
        let samples = vec![0.5; 100];
        mixer.mix_source(&spatial, Vec3::new(0.0, 0.0, 1.0), &samples, 1.0);
        let stereo = mixer.interleaved_stereo();
        assert_eq!(stereo.len(), 2048);
    }
}
