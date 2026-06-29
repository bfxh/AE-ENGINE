#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationMode {
    Fp32,
    Fp16,
    Int8,
    Int4,
    Nf4,
    Int4Group128,
    Int8Group128,
}

#[derive(Debug, Clone)]
pub struct QuantizationConfig {
    pub mode: QuantizationMode,
    pub group_size: usize,
    pub symmetric: bool,
    pub per_channel: bool,
    pub zero_point: bool,
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        QuantizationConfig {
            mode: QuantizationMode::Int4,
            group_size: 128,
            symmetric: true,
            per_channel: false,
            zero_point: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QuantizedTensor {
    pub data: Vec<u8>,
    pub scales: Vec<f32>,
    pub zero_points: Option<Vec<i32>>,
    pub shape: Vec<usize>,
    pub original_dtype: String,
    pub bits_per_element: u8,
    pub group_size: usize,
}

impl QuantizedTensor {
    pub fn memory_bytes(&self) -> usize {
        self.data.len()
            + self.scales.len() * 4
            + self.zero_points.as_ref().map_or(0, |z| z.len() * 4)
    }

    pub fn compression_ratio(&self) -> f32 {
        let original_bytes = self.shape.iter().product::<usize>() * 4;
        original_bytes as f32 / self.memory_bytes() as f32
    }

    pub fn is_valid(&self) -> bool {
        if self.data.is_empty() || self.scales.is_empty() {
            return false;
        }
        let num_elements: usize = self.shape.iter().product();
        let elements_per_byte = 8 / self.bits_per_element as usize;
        let expected_bytes = num_elements.div_ceil(elements_per_byte);
        let num_groups = num_elements.div_ceil(self.group_size);
        self.data.len() >= expected_bytes && self.scales.len() == num_groups
    }
}

pub struct Quantizer;

impl Quantizer {
    pub fn quantize_fp32_to_int8(data: &[f32], config: &QuantizationConfig) -> QuantizedTensor {
        let group_size = config.group_size;
        let num_groups = data.len().div_ceil(group_size);
        let mut quantized = Vec::with_capacity(data.len());
        let mut scales = Vec::with_capacity(num_groups);

        for group_idx in 0..num_groups {
            let start = group_idx * group_size;
            let end = (start + group_size).min(data.len());
            let group = &data[start..end];

            let max_val = group.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
            let scale = if max_val == 0.0 { 1.0 } else { max_val / 127.0 };
            scales.push(scale);

            for &val in group {
                let q = (val / scale).round().clamp(-128.0, 127.0) as i8;
                quantized.push(q as u8);
            }
        }

        QuantizedTensor {
            data: quantized,
            scales,
            zero_points: None,
            shape: vec![data.len()],
            original_dtype: "fp32".to_string(),
            bits_per_element: 8,
            group_size,
        }
    }

    pub fn quantize_fp32_to_int4(data: &[f32], config: &QuantizationConfig) -> QuantizedTensor {
        let group_size = config.group_size;
        let num_groups = data.len().div_ceil(group_size);
        let num_elements = data.len();
        let packed_bytes = num_elements.div_ceil(2);
        let mut quantized = vec![0u8; packed_bytes];
        let mut scales = Vec::with_capacity(num_groups);

        for group_idx in 0..num_groups {
            let start = group_idx * group_size;
            let end = (start + group_size).min(data.len());
            let group = &data[start..end];

            let max_val = group.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
            let scale = if max_val == 0.0 { 1.0 } else { max_val / 7.0 };
            scales.push(scale);

            for (i, &val) in group.iter().enumerate() {
                let q = (val / scale).round().clamp(-8.0, 7.0) as i8;
                let q_u4 = (q & 0x0F) as u8;
                let global_idx = start + i;
                if global_idx.is_multiple_of(2) {
                    quantized[global_idx / 2] = q_u4 & 0x0F;
                } else {
                    quantized[global_idx / 2] |= (q_u4 & 0x0F) << 4;
                }
            }
        }

        QuantizedTensor {
            data: quantized,
            scales,
            zero_points: None,
            shape: vec![data.len()],
            original_dtype: "fp32".to_string(),
            bits_per_element: 4,
            group_size,
        }
    }

    pub fn dequantize_int8(tensor: &QuantizedTensor) -> Vec<f32> {
        let num_elements = tensor.shape.iter().product();
        let mut result = vec![0.0f32; num_elements];

        for (i, &byte) in tensor.data.iter().enumerate().take(num_elements) {
            let group_idx = i / tensor.group_size;
            let scale = tensor.scales[group_idx];
            result[i] = (byte as i8) as f32 * scale;
        }
        result
    }

    pub fn dequantize_int4(tensor: &QuantizedTensor) -> Vec<f32> {
        let num_elements = tensor.shape.iter().product();
        let mut result = vec![0.0f32; num_elements];

        for (i, byte_idx) in (0..num_elements).enumerate() {
            let byte = tensor.data[byte_idx / 2];
            let val = if byte_idx.is_multiple_of(2) { (byte & 0x0F) as i8 } else { ((byte >> 4) & 0x0F) as i8 };
            let signed_val = if val > 7 { val - 16 } else { val };
            let group_idx = byte_idx / tensor.group_size;
            let scale = tensor.scales[group_idx.min(tensor.scales.len() - 1)];
            result[i] = signed_val as f32 * scale;
        }
        result
    }

    pub fn estimate_vram(config: &QuantizationConfig, param_count_billions: f32) -> f32 {
        let bits = match config.mode {
            QuantizationMode::Fp32 => 32,
            QuantizationMode::Fp16 => 16,
            QuantizationMode::Int8 => 8,
            QuantizationMode::Int4 | QuantizationMode::Nf4 => 4,
            QuantizationMode::Int4Group128 => 4,
            QuantizationMode::Int8Group128 => 8,
        };
        let base_mb = param_count_billions * 1024.0 * bits as f32 / 8.0;
        let scale_overhead = if matches!(
            config.mode,
            QuantizationMode::Int4Group128 | QuantizationMode::Int8Group128
        ) {
            param_count_billions * 1024.0 * 32.0 / 8.0 / config.group_size as f32
        } else {
            0.0
        };
        base_mb + scale_overhead
    }

    pub fn vram_constrained_config(
        available_vram_mb: f32,
        param_count_billions: f32,
    ) -> QuantizationConfig {
        let fp32_mb = Self::estimate_vram(
            &QuantizationConfig { mode: QuantizationMode::Fp32, ..Default::default() },
            param_count_billions,
        );
        let fp16_mb = Self::estimate_vram(
            &QuantizationConfig { mode: QuantizationMode::Fp16, ..Default::default() },
            param_count_billions,
        );
        let int8_mb = Self::estimate_vram(
            &QuantizationConfig { mode: QuantizationMode::Int8, ..Default::default() },
            param_count_billions,
        );
        let int4_mb = Self::estimate_vram(
            &QuantizationConfig { mode: QuantizationMode::Int4, ..Default::default() },
            param_count_billions,
        );

        if fp32_mb <= available_vram_mb {
            QuantizationConfig { mode: QuantizationMode::Fp32, ..Default::default() }
        } else if fp16_mb <= available_vram_mb {
            QuantizationConfig { mode: QuantizationMode::Fp16, ..Default::default() }
        } else if int8_mb <= available_vram_mb {
            QuantizationConfig { mode: QuantizationMode::Int8, ..Default::default() }
        } else if int4_mb <= available_vram_mb {
            QuantizationConfig {
                mode: QuantizationMode::Int4,
                group_size: 128,
                symmetric: true,
                per_channel: false,
                zero_point: false,
            }
        } else {
            QuantizationConfig {
                mode: QuantizationMode::Int4,
                group_size: 256,
                symmetric: true,
                per_channel: false,
                zero_point: false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int8_quantize_dequantize() {
        let data: Vec<f32> = (0..256).map(|i| (i as f32 - 128.0) * 0.1).collect();
        let config = QuantizationConfig {
            mode: QuantizationMode::Int8,
            group_size: 64,
            ..Default::default()
        };
        let tensor = Quantizer::quantize_fp32_to_int8(&data, &config);
        assert!(tensor.is_valid());
        let recovered = Quantizer::dequantize_int8(&tensor);
        assert_eq!(recovered.len(), data.len());
        for (orig, rec) in data.iter().zip(recovered.iter()) {
            let error = (orig - rec).abs();
            assert!(error < 0.2, "error {} too large for value {}", error, orig);
        }
    }

    #[test]
    fn test_int4_quantize_dequantize() {
        let data: Vec<f32> = (0..128).map(|i| (i as f32 - 64.0) * 0.1).collect();
        let config = QuantizationConfig {
            mode: QuantizationMode::Int4,
            group_size: 32,
            ..Default::default()
        };
        let tensor = Quantizer::quantize_fp32_to_int4(&data, &config);
        assert!(tensor.is_valid());
        let recovered = Quantizer::dequantize_int4(&tensor);
        assert_eq!(recovered.len(), data.len());
    }

    #[test]
    fn test_compression_ratio() {
        let data = vec![1.0f32; 1024];
        let config = QuantizationConfig {
            mode: QuantizationMode::Int4,
            group_size: 128,
            ..Default::default()
        };
        let tensor = Quantizer::quantize_fp32_to_int4(&data, &config);
        let ratio = tensor.compression_ratio();
        assert!(ratio > 4.0, "compression ratio {} too low", ratio);
    }

    #[test]
    fn test_vram_estimation() {
        let int4_mb = Quantizer::estimate_vram(
            &QuantizationConfig {
                mode: QuantizationMode::Int4,
                group_size: 128,
                symmetric: true,
                per_channel: false,
                zero_point: false,
            },
            4.0,
        );
        assert!(int4_mb < 3000.0);
        let fp32_mb = Quantizer::estimate_vram(
            &QuantizationConfig { mode: QuantizationMode::Fp32, ..Default::default() },
            4.0,
        );
        assert!(fp32_mb > int4_mb * 2.0);
    }

    #[test]
    fn test_vram_constrained_8gb() {
        let config = Quantizer::vram_constrained_config(3000.0, 4.0);
        assert_eq!(config.mode, QuantizationMode::Int4);
    }
}
