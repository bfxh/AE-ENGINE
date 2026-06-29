use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone)]
pub struct AudioSample {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioSample {
    pub fn duration_secs(&self) -> f32 {
        if self.channels == 0 || self.sample_rate == 0 {
            return 0.0;
        }
        (self.samples.len() as f32) / (self.channels as f32 * self.sample_rate as f32)
    }

    pub fn frames(&self) -> usize {
        if self.channels == 0 { 0 } else { self.samples.len() / self.channels as usize }
    }

    pub fn mono(&self) -> Vec<f32> {
        if self.channels == 1 {
            return self.samples.clone();
        }
        let ch = self.channels as usize;
        let n = self.frames();
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let mut sum = 0.0f32;
            for c in 0..ch {
                sum += self.samples[i * ch + c];
            }
            out.push(sum / ch as f32);
        }
        out
    }
}

#[derive(Debug)]
pub enum DecodeError {
    InvalidHeader,
    UnsupportedFormat(u16),
    UnexpectedEof,
    Io(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::InvalidHeader => write!(f, "invalid WAV header"),
            DecodeError::UnsupportedFormat(c) => write!(f, "unsupported format code: {}", c),
            DecodeError::UnexpectedEof => write!(f, "unexpected end of data"),
            DecodeError::Io(e) => write!(f, "io error: {}", e),
        }
    }
}

impl std::error::Error for DecodeError {}

fn read_u32_le<R: Read>(r: &mut R) -> Result<u32, DecodeError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|e| DecodeError::Io(e.to_string()))?;
    Ok(u32::from_le_bytes(b))
}

fn read_u16_le<R: Read>(r: &mut R) -> Result<u16, DecodeError> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b).map_err(|e| DecodeError::Io(e.to_string()))?;
    Ok(u16::from_le_bytes(b))
}

fn read_fourcc<R: Read>(r: &mut R) -> Result<[u8; 4], DecodeError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|e| DecodeError::Io(e.to_string()))?;
    Ok(b)
}

pub fn decode_wav<R: Read + Seek>(r: &mut R) -> Result<AudioSample, DecodeError> {
    let riff = read_fourcc(r)?;
    if &riff != b"RIFF" {
        return Err(DecodeError::InvalidHeader);
    }
    let _file_size = read_u32_le(r)?;
    let wave = read_fourcc(r)?;
    if &wave != b"WAVE" {
        return Err(DecodeError::InvalidHeader);
    }

    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut bits_per_sample = 0u16;
    let mut format_code = 1u16;
    let mut data_bytes: Vec<u8> = Vec::new();

    while let Ok(chunk_id) = read_fourcc(r) {
        let chunk_size = read_u32_le(r)?;

        match &chunk_id {
            b"fmt " => {
                format_code = read_u16_le(r)?;
                channels = read_u16_le(r)?;
                sample_rate = read_u32_le(r)?;
                let _byte_rate = read_u32_le(r)?;
                let _block_align = read_u16_le(r)?;
                bits_per_sample = read_u16_le(r)?;
                if chunk_size > 16 {
                    r.seek(SeekFrom::Current((chunk_size - 16) as i64))
                        .map_err(|e| DecodeError::Io(e.to_string()))?;
                }
            },
            b"data" => {
                data_bytes.resize(chunk_size as usize, 0u8);
                r.read_exact(&mut data_bytes).map_err(|e| DecodeError::Io(e.to_string()))?;
            },
            _ => {
                r.seek(SeekFrom::Current(chunk_size as i64))
                    .map_err(|e| DecodeError::Io(e.to_string()))?;
            },
        }
    }

    if channels == 0 || sample_rate == 0 {
        return Err(DecodeError::InvalidHeader);
    }

    let samples = match (format_code, bits_per_sample) {
        (1, 8) => decode_pcm8(&data_bytes),
        (1, 16) => decode_pcm16(&data_bytes),
        (1, 24) => decode_pcm24(&data_bytes),
        (1, 32) => decode_pcm32(&data_bytes),
        (3, 32) => decode_f32(&data_bytes),
        (3, 64) => decode_f64(&data_bytes),
        _ => return Err(DecodeError::UnsupportedFormat(format_code)),
    };

    Ok(AudioSample { samples, sample_rate, channels })
}

fn decode_pcm8(b: &[u8]) -> Vec<f32> {
    b.iter().map(|&v| (v as f32 - 128.0) / 128.0).collect()
}

fn decode_pcm16(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect()
}

fn decode_pcm24(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(3)
        .map(|c| {
            let v = (c[0] as i32) | ((c[1] as i32) << 8) | ((c[2] as i32) << 16);
            let sign_extended = if v & 0x800000 != 0 { v | !0xFFFFFF } else { v };
            sign_extended as f32 / 8388608.0
        })
        .collect()
}

fn decode_pcm32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2147483648.0)
        .collect()
}

fn decode_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn decode_f64(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(8)
        .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32)
        .collect()
}

pub fn encode_wav_mono(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_size = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_size);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_size as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_size as u32).to_le_bytes());
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_encode_decode_roundtrip() {
        let original: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.001).sin() * 0.5).collect();
        let wav_bytes = encode_wav_mono(&original, 44100);
        let mut cursor = Cursor::new(wav_bytes);
        let decoded = decode_wav(&mut cursor).unwrap();
        assert_eq!(decoded.sample_rate, 44100);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.samples.len(), original.len());
        for (a, b) in original.iter().zip(decoded.samples.iter()) {
            assert!((a - b).abs() < 0.001, "mismatch: {} vs {}", a, b);
        }
    }

    #[test]
    fn test_duration_calc() {
        let s = AudioSample { samples: vec![0.0; 44100 * 2], sample_rate: 44100, channels: 2 };
        assert!((s.duration_secs() - 1.0).abs() < 0.0001);
        assert_eq!(s.frames(), 44100);
    }

    #[test]
    fn test_mono_downmix() {
        let s = AudioSample { samples: vec![0.5, 0.3, 0.5, 0.3], sample_rate: 44100, channels: 2 };
        let m = s.mono();
        assert_eq!(m.len(), 2);
        assert!((m[0] - 0.4).abs() < 0.0001);
    }

    #[test]
    fn test_invalid_header() {
        let data = b"NOTRIFFxxxxWAVE";
        let mut cursor = Cursor::new(data.to_vec());
        let result = decode_wav(&mut cursor);
        assert!(matches!(result, Err(DecodeError::InvalidHeader)));
    }
}
