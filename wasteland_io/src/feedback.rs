#[derive(Debug, Clone)]
pub struct ForceFeedbackEffect {
    pub waveform: Waveform,
    pub amplitude: f32,
    pub frequency: f32,
    pub duration_ms: u32,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    pub envelope: Envelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Constant,
    Sine,
    Square,
    Triangle,
    Sawtooth,
    Impact,
    Rumble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Envelope {
    None,
    FadeIn,
    FadeOut,
    FadeInOut,
    AttackDecay { attack_ms: u32, decay_ms: u32 },
}

pub struct ForceFeedbackDevice {
    effects: Vec<ForceFeedbackEffect>,
    active_effects: Vec<ActiveEffect>,
    master_gain: f32,
    left_motor: f32,
    right_motor: f32,
}

#[derive(Debug, Clone)]
struct ActiveEffect {
    effect: ForceFeedbackEffect,
    elapsed_ms: u32,
    completed: bool,
}

impl ForceFeedbackDevice {
    pub fn new() -> Self {
        ForceFeedbackDevice {
            effects: Vec::new(),
            active_effects: Vec::new(),
            master_gain: 1.0,
            left_motor: 0.0,
            right_motor: 0.0,
        }
    }

    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain.clamp(0.0, 1.0);
    }

    pub fn upload_effect(&mut self, effect: ForceFeedbackEffect) -> usize {
        let id = self.effects.len();
        self.effects.push(effect);
        id
    }

    pub fn play_effect(&mut self, effect_id: usize) {
        if let Some(effect) = self.effects.get(effect_id) {
            self.active_effects.push(ActiveEffect {
                effect: effect.clone(),
                elapsed_ms: 0,
                completed: false,
            });
        }
    }

    pub fn stop_all(&mut self) {
        self.active_effects.clear();
        self.left_motor = 0.0;
        self.right_motor = 0.0;
    }

    pub fn update(&mut self, delta_ms: u32) {
        let mut left = 0.0f32;
        let mut right = 0.0f32;

        self.active_effects.retain_mut(|active| {
            active.elapsed_ms += delta_ms;
            if active.elapsed_ms > active.effect.duration_ms {
                active.completed = true;
                return false;
            }
            let t = active.elapsed_ms as f32;
            let dur = active.effect.duration_ms as f32;
            let fi = active.effect.fade_in_ms as f32;
            let fo = active.effect.fade_out_ms as f32;
            let phase = (t / 1000.0 * active.effect.frequency * std::f32::consts::TAU)
                % std::f32::consts::TAU;

            let base = match active.effect.waveform {
                Waveform::Constant => active.effect.amplitude,
                Waveform::Sine => active.effect.amplitude * phase.sin(),
                Waveform::Square => {
                    active.effect.amplitude * if phase.sin() >= 0.0 { 1.0 } else { -1.0 }
                },
                Waveform::Triangle => {
                    active.effect.amplitude
                        * (2.0 * (phase / std::f32::consts::TAU % 1.0 - 0.5).abs() * 2.0 - 1.0)
                },
                Waveform::Sawtooth => {
                    active.effect.amplitude * (2.0 * (phase / std::f32::consts::TAU % 1.0) - 1.0)
                },
                Waveform::Impact => {
                    let decay = (-t / (dur * 0.1)).exp();
                    active.effect.amplitude * decay * phase.sin()
                },
                Waveform::Rumble => active.effect.amplitude * (t * 73.0).sin() * (t * 97.0).cos(),
            };

            let envelope = match active.effect.envelope {
                Envelope::None => 1.0,
                Envelope::FadeIn => (t / fi).min(1.0),
                Envelope::FadeOut => ((dur - t) / fo).clamp(0.0, 1.0),
                Envelope::FadeInOut => {
                    let fi = (t / fi).min(1.0);
                    let fo = ((dur - t) / fo).clamp(0.0, 1.0);
                    fi * fo
                },
                Envelope::AttackDecay { attack_ms, decay_ms } => {
                    let a = (t / attack_ms as f32).min(1.0);
                    let d = 1.0 - ((t - attack_ms as f32) / decay_ms as f32).clamp(0.0, 1.0);
                    a * d
                },
            };

            let value = base * envelope;
            left += value;
            right += value;
            true
        });

        self.left_motor = (left * self.master_gain).clamp(-1.0, 1.0);
        self.right_motor = (right * self.master_gain).clamp(-1.0, 1.0);
    }

    pub fn motor_state(&self) -> (f32, f32) {
        (self.left_motor, self.right_motor)
    }

    pub fn active_count(&self) -> usize {
        self.active_effects.len()
    }
}

impl Default for ForceFeedbackDevice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_and_play() {
        let mut ff = ForceFeedbackDevice::new();
        let id = ff.upload_effect(ForceFeedbackEffect {
            waveform: Waveform::Constant,
            amplitude: 0.5,
            frequency: 0.0,
            duration_ms: 100,
            fade_in_ms: 0,
            fade_out_ms: 0,
            envelope: Envelope::None,
        });
        ff.play_effect(id);
        ff.update(50);
        let (l, r) = ff.motor_state();
        assert!(l > 0.0);
        assert!(r > 0.0);
    }

    #[test]
    fn test_effect_expires() {
        let mut ff = ForceFeedbackDevice::new();
        let id = ff.upload_effect(ForceFeedbackEffect {
            waveform: Waveform::Constant,
            amplitude: 1.0,
            frequency: 0.0,
            duration_ms: 50,
            fade_in_ms: 0,
            fade_out_ms: 0,
            envelope: Envelope::None,
        });
        ff.play_effect(id);
        ff.update(50);
        assert_eq!(ff.active_count(), 1);
        ff.update(1);
        assert_eq!(ff.active_count(), 0);
    }

    #[test]
    fn test_stop_all() {
        let mut ff = ForceFeedbackDevice::new();
        let id = ff.upload_effect(ForceFeedbackEffect {
            waveform: Waveform::Sine,
            amplitude: 0.5,
            frequency: 10.0,
            duration_ms: 1000,
            fade_in_ms: 0,
            fade_out_ms: 0,
            envelope: Envelope::None,
        });
        ff.play_effect(id);
        ff.update(50);
        ff.stop_all();
        assert_eq!(ff.active_count(), 0);
        assert_eq!(ff.motor_state(), (0.0, 0.0));
    }

    #[test]
    fn test_master_gain() {
        let mut ff = ForceFeedbackDevice::new();
        ff.set_master_gain(0.0);
        let id = ff.upload_effect(ForceFeedbackEffect {
            waveform: Waveform::Constant,
            amplitude: 0.5,
            frequency: 0.0,
            duration_ms: 100,
            fade_in_ms: 0,
            fade_out_ms: 0,
            envelope: Envelope::None,
        });
        ff.play_effect(id);
        ff.update(50);
        let (l, r) = ff.motor_state();
        assert!((l - 0.0).abs() < 0.001);
        assert!((r - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_waveform_types() {
        let mut ff = ForceFeedbackDevice::new();
        let waveforms = [
            Waveform::Sine,
            Waveform::Square,
            Waveform::Triangle,
            Waveform::Sawtooth,
            Waveform::Impact,
            Waveform::Rumble,
        ];
        for wf in &waveforms {
            let id = ff.upload_effect(ForceFeedbackEffect {
                waveform: *wf,
                amplitude: 0.5,
                frequency: 10.0,
                duration_ms: 100,
                fade_in_ms: 10,
                fade_out_ms: 10,
                envelope: Envelope::FadeInOut,
            });
            ff.play_effect(id);
        }
        ff.update(50);
        assert_eq!(ff.active_count(), 6);
    }
}
