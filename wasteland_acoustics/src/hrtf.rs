use glam::Vec3;
use serde::{Deserialize, Serialize};

const SPEED_OF_SOUND: f32 = 343.0;
const HEAD_RADIUS: f32 = 0.0875;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrirData {
    pub samples: Vec<f32>,
    pub sample_rate: f32,
    pub delay: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrtfData {
    pub azimuth: f32,
    pub elevation: f32,
    pub left_ear: HrirData,
    pub right_ear: HrirData,
    pub itd: f32,
    pub ild: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HrtfPreset {
    Front,
    Back,
    Left,
    Right,
    Up,
}

impl HrtfPreset {
    pub fn all() -> [HrtfPreset; 5] {
        [HrtfPreset::Front, HrtfPreset::Back, HrtfPreset::Left, HrtfPreset::Right, HrtfPreset::Up]
    }

    pub fn azimuth_elevation(&self) -> (f32, f32) {
        match self {
            HrtfPreset::Front => (0.0, 0.0),
            HrtfPreset::Back => (180.0_f32.to_radians(), 0.0),
            HrtfPreset::Left => (-90.0_f32.to_radians(), 0.0),
            HrtfPreset::Right => (90.0_f32.to_radians(), 0.0),
            HrtfPreset::Up => (0.0, 90.0_f32.to_radians()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrtfDatabase {
    pub entries: Vec<HrtfData>,
    pub sample_rate: f32,
    pub ir_length: usize,
}

impl HrtfDatabase {
    pub fn new() -> Self {
        Self { entries: Vec::new(), sample_rate: 44100.0, ir_length: 128 }
    }

    pub fn generate_presets() -> Self {
        let mut db = Self::new();
        for preset in HrtfPreset::all() {
            let (azimuth, elevation) = preset.azimuth_elevation();
            let entry = generate_simple_hrtf(azimuth, elevation, db.sample_rate, db.ir_length);
            db.entries.push(entry);
        }
        db
    }

    pub fn query(&self, azimuth: f32, elevation: f32) -> Option<&HrtfData> {
        self.entries.iter().min_by(|a, b| {
            let da = angle_diff(a.azimuth, azimuth) + angle_diff(a.elevation, elevation);
            let db_val = angle_diff(b.azimuth, azimuth) + angle_diff(b.elevation, elevation);
            da.partial_cmp(&db_val).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn query_interpolated(&self, azimuth: f32, elevation: f32) -> HrtfData {
        let mut nearest: Vec<&HrtfData> = self.entries.iter().collect();
        nearest.sort_by(|a, b| {
            let da = angle_diff(a.azimuth, azimuth) + angle_diff(a.elevation, elevation);
            let db_val = angle_diff(b.azimuth, azimuth) + angle_diff(b.elevation, elevation);
            da.partial_cmp(&db_val).unwrap_or(std::cmp::Ordering::Equal)
        });

        let entries: Vec<&HrtfData> = nearest.into_iter().take(4).collect();
        if entries.len() == 1 {
            return entries[0].clone();
        }

        let mut total_weight: f32 = 0.0;
        let mut weights: Vec<f32> = Vec::new();
        for e in &entries {
            let dist = angle_diff(e.azimuth, azimuth) + angle_diff(e.elevation, elevation);
            let w = if dist < 0.01 { 1.0 } else { 1.0 / dist };
            weights.push(w);
            total_weight += w;
        }
        for w in &mut weights {
            *w /= total_weight;
        }

        let mut result_left = vec![0.0_f32; entries[0].left_ear.samples.len()];
        let mut result_right = vec![0.0_f32; entries[0].right_ear.samples.len()];
        let mut result_itd = 0.0_f32;
        let mut result_ild = 0.0_f32;

        for (i, e) in entries.iter().enumerate() {
            let w = weights[i];
            for j in 0..result_left.len() {
                result_left[j] += e.left_ear.samples[j] * w;
                result_right[j] += e.right_ear.samples[j] * w;
            }
            result_itd += e.itd * w;
            result_ild += e.ild * w;
        }

        HrtfData {
            azimuth,
            elevation,
            left_ear: HrirData {
                samples: result_left,
                sample_rate: entries[0].left_ear.sample_rate,
                delay: entries[0].left_ear.delay,
            },
            right_ear: HrirData {
                samples: result_right,
                sample_rate: entries[0].right_ear.sample_rate,
                delay: entries[0].right_ear.delay,
            },
            itd: result_itd,
            ild: result_ild,
        }
    }

    pub fn compute_itd(&self, azimuth: f32) -> f32 {
        let az = azimuth.clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        let delay = HEAD_RADIUS * (az.sin() + az) / SPEED_OF_SOUND;
        delay.max(0.0)
    }

    pub fn compute_ild(&self, azimuth: f32) -> f32 {
        let az = azimuth.clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        let shadow = 1.0 + (az / 2.6).sin();
        let db = 20.0 * shadow.log10();
        db / 20.0
    }
}

impl Default for HrtfDatabase {
    fn default() -> Self {
        Self::generate_presets()
    }
}

fn generate_simple_hrtf(
    azimuth: f32,
    elevation: f32,
    sample_rate: f32,
    ir_length: usize,
) -> HrtfData {
    let itd = compute_itd_simple(azimuth);
    let ild = compute_ild_simple(azimuth);
    let delay_samples = (itd * sample_rate) as usize;

    let mut left_samples = vec![0.0_f32; ir_length];
    let mut right_samples = vec![0.0_f32; ir_length];

    for i in 0..ir_length {
        let t = i as f32 / sample_rate;
        let freq = 4000.0;
        let env = (-12.0 * t).exp();
        let sig = (2.0 * std::f32::consts::PI * freq * t).sin() * env;

        left_samples[i] = sig * (1.0 + ild) * 0.5;
        if i >= delay_samples {
            right_samples[i] = sig * (1.0 - ild) * 0.5;
        }
    }

    let elevation_factor = elevation.cos();
    for s in &mut left_samples {
        *s *= elevation_factor;
    }
    for s in &mut right_samples {
        *s *= elevation_factor;
    }

    HrtfData {
        azimuth,
        elevation,
        left_ear: HrirData { samples: left_samples, sample_rate, delay: 0.0 },
        right_ear: HrirData { samples: right_samples, sample_rate, delay: itd },
        itd,
        ild,
    }
}

fn compute_itd_simple(azimuth: f32) -> f32 {
    let az = azimuth.clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
    HEAD_RADIUS * (az.sin() + az) / SPEED_OF_SOUND
}

fn compute_ild_simple(azimuth: f32) -> f32 {
    let az = azimuth.clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
    let shadow = 1.0 + (az / 2.6).sin();
    let db = 20.0 * shadow.log10();
    db.clamp(-20.0, 20.0) / 20.0
}

fn angle_diff(a: f32, b: f32) -> f32 {
    let mut diff = (a - b).abs();
    while diff > std::f32::consts::PI {
        diff -= 2.0 * std::f32::consts::PI;
    }
    diff.abs()
}

pub fn spatialize(
    source_pos: Vec3,
    listener_pos: Vec3,
    listener_forward: Vec3,
    listener_up: Vec3,
    db: &HrtfDatabase,
) -> (f32, f32) {
    let rel = source_pos - listener_pos;
    let distance = rel.length();
    if distance < 0.001 {
        return (1.0, 1.0);
    }

    let dir = rel.normalize();
    let right = listener_forward.cross(listener_up).normalize();
    let elevation = dir.dot(listener_up).asin();
    let horizontal = dir - dir.dot(listener_up) * listener_up;
    let horizontal_len = horizontal.length();
    let azimuth = if horizontal_len < 0.001 {
        0.0
    } else {
        let h = horizontal.normalize();
        let dot = h.dot(listener_forward).clamp(-1.0, 1.0);
        let cross = h.dot(right);
        dot.acos().copysign(cross)
    };

    let attenuation = 1.0 / (distance * distance + 1.0);
    let hrtf = db.query_interpolated(azimuth, elevation);

    (attenuation * (1.0 + hrtf.ild) * 0.5, attenuation * (1.0 - hrtf.ild) * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hrtf_presets() {
        let presets = HrtfPreset::all();
        assert_eq!(presets.len(), 5);
        let (az, el) = HrtfPreset::Front.azimuth_elevation();
        assert!((az - 0.0).abs() < 0.01);
        assert!((el - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_generate_presets() {
        let db = HrtfDatabase::generate_presets();
        assert_eq!(db.entries.len(), 5);
    }

    #[test]
    fn test_query() {
        let db = HrtfDatabase::generate_presets();
        let result = db.query(0.0, 0.0);
        assert!(result.is_some());
        let hrtf = result.unwrap();
        assert!((hrtf.azimuth - 0.0).abs() < 0.01);
        assert!((hrtf.elevation - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_query_interpolated() {
        let db = HrtfDatabase::generate_presets();
        let hrtf = db.query_interpolated(0.5, 0.3);
        assert!(!hrtf.left_ear.samples.is_empty());
        assert!(!hrtf.right_ear.samples.is_empty());
    }

    #[test]
    fn test_itd_ild() {
        let db = HrtfDatabase::new();
        let itd = db.compute_itd(0.0);
        assert!(itd >= 0.0);
        let ild = db.compute_ild(0.0);
        assert!(ild.is_finite());
    }

    #[test]
    fn test_spatialize() {
        let db = HrtfDatabase::generate_presets();
        let (left, right) =
            spatialize(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, Vec3::NEG_Z, Vec3::Y, &db);
        assert!(left.is_finite());
        assert!(right.is_finite());
        assert!(left != right);
    }

    #[test]
    fn test_angle_diff() {
        assert!((angle_diff(0.0, 0.0) - 0.0).abs() < 0.01);
        assert!((angle_diff(1.0, 1.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_hrtf_preset_angles() {
        let (az, _el) = HrtfPreset::Left.azimuth_elevation();
        assert!(az < 0.0);
        let (az, _el) = HrtfPreset::Right.azimuth_elevation();
        assert!(az > 0.0);
        let (_az, el) = HrtfPreset::Up.azimuth_elevation();
        assert!(el > 0.0);
    }
}
