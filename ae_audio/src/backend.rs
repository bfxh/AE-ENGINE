use crate::decoder::AudioSample;
use crate::source::SoundSource;
use crate::spatial::SpatialAudio;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use glam::Vec3;
use parking_lot::RwLock;
use std::sync::Arc;

pub const MAX_VOICES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Stopped,
    Playing,
    Paused,
    Finished,
}

#[derive(Debug, Clone)]
pub struct Voice {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
    pub cursor: usize,
    pub volume: f32,
    pub pitch: f32,
    pub loop_mode: bool,
    pub state: PlayState,
    pub source: SoundSource,
    pub id: u32,
}

impl Voice {
    pub fn new(id: u32, sample: AudioSample) -> Self {
        let sr = sample.sample_rate;
        let ch = sample.channels;
        Voice {
            samples: sample.samples,
            channels: ch,
            sample_rate: sr,
            cursor: 0,
            volume: 1.0,
            pitch: 1.0,
            loop_mode: false,
            state: PlayState::Stopped,
            source: SoundSource::default(),
            id,
        }
    }

    pub fn with_position(mut self, pos: Vec3) -> Self {
        self.source.position = pos;
        self
    }

    pub fn with_volume(mut self, v: f32) -> Self {
        self.volume = v.clamp(0.0, 2.0);
        self
    }

    pub fn with_pitch(mut self, p: f32) -> Self {
        self.pitch = p.max(0.1);
        self
    }

    pub fn with_loop(mut self, l: bool) -> Self {
        self.loop_mode = l;
        self
    }

    pub fn play(&mut self) {
        self.state = PlayState::Playing;
    }

    pub fn pause(&mut self) {
        if self.state == PlayState::Playing {
            self.state = PlayState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.state = PlayState::Stopped;
        self.cursor = 0;
    }

    fn render(&mut self, out: &mut [f32], out_channels: usize, spatial: &SpatialAudio) -> bool {
        if self.state != PlayState::Playing {
            return false;
        }
        let ch = self.channels as usize;
        if ch == 0 || self.samples.is_empty() {
            self.state = PlayState::Finished;
            return false;
        }
        let frames_needed = out.len() / out_channels;
        let (left_gain, right_gain) =
            if out_channels >= 2 { spatial.pan_stereo(self.source.position) } else { (1.0, 1.0) };
        let att = self.source.attenuation_at(spatial.listener_pos);
        let base_vol = self.volume * att;

        for i in 0..frames_needed {
            if self.cursor >= self.samples.len() {
                if self.loop_mode {
                    self.cursor = 0;
                } else {
                    self.state = PlayState::Finished;
                    out[i * out_channels..].fill(0.0);
                    return true;
                }
            }
            let frame_start = self.cursor;
            let mono = if ch == 1 {
                self.samples[frame_start]
            } else {
                let mut s = 0.0f32;
                for c in 0..ch {
                    s += self.samples[frame_start + c];
                }
                s / ch as f32
            };
            let scaled = mono * base_vol;
            if out_channels >= 2 {
                out[i * out_channels] += scaled * left_gain;
                out[i * out_channels + 1] += scaled * right_gain;
                for c in 2..out_channels {
                    out[i * out_channels + c] += scaled;
                }
            } else {
                out[i * out_channels] += scaled;
            }
            let advance = (ch as f32 * self.pitch).round() as usize;
            let advance = advance.max(1);
            self.cursor += advance;
        }
        true
    }
}

pub struct AudioMixer {
    voices: Vec<Voice>,
    next_id: u32,
    master_volume: f32,
    spatial: SpatialAudio,
}

impl AudioMixer {
    pub fn new() -> Self {
        AudioMixer {
            voices: Vec::with_capacity(MAX_VOICES),
            next_id: 1,
            master_volume: 1.0,
            spatial: SpatialAudio::default(),
        }
    }

    pub fn play(&mut self, sample: AudioSample) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let mut voice = Voice::new(id, sample);
        voice.play();
        if self.voices.len() >= MAX_VOICES {
            self.voices.retain(|v| v.state != PlayState::Finished);
            if self.voices.len() >= MAX_VOICES {
                self.voices[0].stop();
                self.voices.remove(0);
            }
        }
        self.voices.push(voice);
        id
    }

    pub fn play_3d(&mut self, sample: AudioSample, position: Vec3) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let mut voice = Voice::new(id, sample).with_position(position);
        voice.play();
        if self.voices.len() >= MAX_VOICES {
            self.voices.retain(|v| v.state != PlayState::Finished);
        }
        self.voices.push(voice);
        id
    }

    pub fn stop(&mut self, id: u32) {
        for v in &mut self.voices {
            if v.id == id {
                v.stop();
            }
        }
    }

    pub fn pause(&mut self, id: u32) {
        for v in &mut self.voices {
            if v.id == id {
                v.pause();
            }
        }
    }

    pub fn resume(&mut self, id: u32) {
        for v in &mut self.voices {
            if v.id == id && v.state == PlayState::Paused {
                v.play();
            }
        }
    }

    pub fn set_master_volume(&mut self, v: f32) {
        self.master_volume = v.clamp(0.0, 2.0);
    }

    pub fn set_listener(&mut self, pos: Vec3, forward: Vec3, up: Vec3) {
        self.spatial.listener_pos = pos;
        self.spatial.listener_forward = forward.normalize_or_zero();
        self.spatial.listener_up = up.normalize_or_zero();
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.state == PlayState::Playing).count()
    }

    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        for s in out.iter_mut() {
            *s = 0.0;
        }
        let spatial = self.spatial.clone();
        let mut finished = false;
        for v in &mut self.voices {
            v.render(out, channels, &spatial);
            if v.state == PlayState::Finished {
                finished = true;
            }
        }
        let mv = self.master_volume;
        for s in out.iter_mut() {
            *s *= mv;
            *s = (*s).clamp(-1.0, 1.0);
        }
        if finished {
            self.voices.retain(|v| v.state != PlayState::Finished);
        }
    }
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AudioEngine {
    mixer: Arc<RwLock<AudioMixer>>,
    _stream: Option<Stream>,
    sample_rate: u32,
    channels: u16,
}

impl AudioEngine {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no output device available")?;

        let supported = device
            .supported_output_configs()
            .map_err(|e| e.to_string())?
            .next()
            .ok_or("no supported output config")?;

        let sample_format = supported.sample_format();
        let sample_rate = supported.min_sample_rate().max(cpal::SampleRate(44100));
        let channels = supported.channels();

        let config = StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let mixer = Arc::new(RwLock::new(AudioMixer::new()));
        let mixer_clone = mixer.clone();
        let ch_count = channels as usize;

        let stream = match sample_format {
            SampleFormat::F32 => {
                build_stream::<f32>(&device, &config, mixer_clone, ch_count)
            },
            SampleFormat::I16 => build_stream::<i16>(&device, &config, mixer_clone, ch_count),
            SampleFormat::U16 => build_stream::<u16>(&device, &config, mixer_clone, ch_count),
            _ => return Err(format!("unsupported sample format: {:?}", sample_format)),
        }
        .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        Ok(AudioEngine { mixer, _stream: Some(stream), sample_rate: sample_rate.0, channels })
    }

    pub fn mixer(&self) -> Arc<RwLock<AudioMixer>> {
        self.mixer.clone()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn play(&self, sample: AudioSample) -> u32 {
        self.mixer.write().play(sample)
    }

    pub fn play_3d(&self, sample: AudioSample, position: Vec3) -> u32 {
        self.mixer.write().play_3d(sample, position)
    }

    pub fn stop(&self, id: u32) {
        self.mixer.write().stop(id);
    }

    pub fn set_master_volume(&self, v: f32) {
        self.mixer.write().set_master_volume(v);
    }

    pub fn set_listener(&self, pos: Vec3, forward: Vec3, up: Vec3) {
        self.mixer.write().set_listener(pos, forward, up);
    }

    pub fn active_voices(&self) -> usize {
        self.mixer.read().active_voices()
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| AudioEngine {
            mixer: Arc::new(RwLock::new(AudioMixer::new())),
            _stream: None,
            sample_rate: 44100,
            channels: 2,
        })
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    mixer: Arc<RwLock<AudioMixer>>,
    channels: usize,
) -> Result<Stream, cpal::BuildStreamError>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let mut float_buf = Vec::<f32>::new();
    device.build_output_stream(
        config,
        move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
            let frames = out.len() / channels;
            if float_buf.len() < frames * channels {
                float_buf.resize(frames * channels, 0.0);
            }
            {
                let mut m = mixer.write();
                m.render(&mut float_buf[..frames * channels], channels);
            }
            for (dst, src) in out.iter_mut().zip(float_buf[..frames * channels].iter()) {
                *dst = T::from_sample(*src);
            }
        },
        |err| {
            log::error!("audio stream error: {}", err);
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixer_play_stop() {
        let mut mixer = AudioMixer::new();
        let sample = AudioSample { samples: vec![0.5; 44100], sample_rate: 44100, channels: 1 };
        let id = mixer.play(sample);
        assert_eq!(mixer.active_voices(), 1);
        mixer.stop(id);
        assert_eq!(mixer.active_voices(), 0);
    }

    #[test]
    fn test_mixer_render() {
        let mut mixer = AudioMixer::new();
        let sample = AudioSample { samples: vec![1.0; 100], sample_rate: 44100, channels: 1 };
        mixer.play(sample);
        let mut out = vec![0.0f32; 200];
        mixer.render(&mut out, 2);
        assert!(out[0] > 0.0);
        assert!(out[1] > 0.0);
    }

    #[test]
    fn test_voice_loop() {
        let sample = AudioSample { samples: vec![0.5; 10], sample_rate: 44100, channels: 1 };
        let mut voice = Voice::new(1, sample).with_loop(true);
        voice.play();
        let spatial = SpatialAudio::default();
        let mut out = vec![0.0f32; 100];
        voice.render(&mut out, 1, &spatial);
        assert_eq!(voice.state, PlayState::Playing);
    }

    #[test]
    fn test_voice_finish() {
        let sample = AudioSample { samples: vec![0.5; 10], sample_rate: 44100, channels: 1 };
        let mut voice = Voice::new(1, sample);
        voice.play();
        let spatial = SpatialAudio::default();
        let mut out = vec![0.0f32; 100];
        voice.render(&mut out, 1, &spatial);
        assert_eq!(voice.state, PlayState::Finished);
    }

    #[test]
    fn test_3d_attenuation() {
        let mut mixer = AudioMixer::new();
        mixer.set_listener(Vec3::new(0.0, 0.0, 0.0), Vec3::Z, Vec3::Y);
        let sample = AudioSample { samples: vec![1.0; 100], sample_rate: 44100, channels: 1 };
        mixer.play_3d(sample, Vec3::new(100.0, 0.0, 0.0));
        let mut out = vec![0.0f32; 200];
        mixer.render(&mut out, 2);
        assert!(out[0].abs() < 0.01);
    }
}
