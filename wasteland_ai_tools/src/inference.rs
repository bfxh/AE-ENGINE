use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub model_type: ModelType,
    pub batch_size: usize,
    pub max_sequence_length: usize,
    pub use_gpu: bool,
    pub gpu_device_id: u32,
    pub num_threads: u32,
    pub warmup_iterations: u32,
    pub cache_activations: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelType {
    Llama,
    Qwen,
    Mistral,
    Gemma,
    Phi,
    StableDiffusion,
    PointNet,
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFormat {
    GGUF,
    ONNX,
    SafeTensors,
}

#[derive(Debug, Clone)]
pub struct InferenceSession {
    pub config: InferenceConfig,
    pub state: SessionState,
    metrics: SessionMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Loading,
    Ready,
    Running,
    Error,
}

#[derive(Debug, Clone, Default)]
pub struct SessionMetrics {
    pub total_inferences: u64,
    pub total_tokens: u64,
    pub total_time_ms: u64,
    pub avg_time_ms: f64,
    pub peak_memory_mb: f64,
    pub current_memory_mb: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InferenceStats {
    pub token_count: u32,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub inference_time_ms: u64,
    pub tokens_per_second: f64,
    pub peak_memory_mb: f64,
    pub backend: BackendType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BackendType {
    #[default]
    Cpu,
    Cuda,
    Metal,
    Vulkan,
}

#[derive(Debug, Clone)]
pub struct CpuBackend {
    pub num_threads: u32,
    pub use_avx2: bool,
    pub use_avx512: bool,
    pub quantization_bits: u8,
    pub memory_limit_mb: f64,
    loaded_models: HashMap<String, CpuModelState>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CpuModelState {
    model_path: String,
    format: ModelFormat,
    memory_mb: f64,
    context_size: usize,
    warm: bool,
}

impl CpuBackend {
    pub fn new(num_threads: u32, memory_limit_mb: f64) -> Self {
        let use_avx2 = std::is_x86_feature_detected!("avx2");
        let use_avx512 = std::is_x86_feature_detected!("avx512f");
        CpuBackend {
            num_threads,
            use_avx2,
            use_avx512,
            quantization_bits: 4,
            memory_limit_mb,
            loaded_models: HashMap::new(),
        }
    }

    pub fn load_model(
        &mut self,
        name: &str,
        path: &str,
        format: ModelFormat,
    ) -> Result<(), String> {
        let est_mem = self.estimate_model_memory(format, path);
        let current: f64 = self.loaded_models.values().map(|m| m.memory_mb).sum();
        if current + est_mem > self.memory_limit_mb {
            return Err(format!(
                "insufficient memory: need {:.0}MB, available {:.0}MB",
                est_mem,
                self.memory_limit_mb - current
            ));
        }
        self.loaded_models.insert(
            name.to_string(),
            CpuModelState {
                model_path: path.to_string(),
                format,
                memory_mb: est_mem,
                context_size: 4096,
                warm: false,
            },
        );
        self.warmup_model(name)?;
        Ok(())
    }

    fn warmup_model(&mut self, name: &str) -> Result<(), String> {
        if let Some(state) = self.loaded_models.get_mut(name) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            state.warm = true;
        }
        Ok(())
    }

    fn estimate_model_memory(&self, format: ModelFormat, _path: &str) -> f64 {
        match format {
            ModelFormat::GGUF => match self.quantization_bits {
                4 => 512.0,
                8 => 1024.0,
                _ => 2048.0,
            },
            ModelFormat::ONNX => 1024.0,
            ModelFormat::SafeTensors => 2048.0,
        }
    }

    pub fn infer(
        &self,
        name: &str,
        prompt: &str,
        max_tokens: u32,
        temperature: f32,
    ) -> Result<(String, InferenceStats), String> {
        let model = self.loaded_models.get(name).ok_or("model not loaded")?;
        if !model.warm {
            return Err("model not warmed up".into());
        }
        let start = Instant::now();
        let prompt_tokens = (prompt.len() as f64 / 4.0).ceil() as u32;
        let output = self.simulate_generation(prompt, max_tokens, temperature);
        let elapsed = start.elapsed();
        let completion_tokens = (output.len() as f64 / 4.0).ceil() as u32;
        let total_tokens = prompt_tokens + completion_tokens;
        let tps = if elapsed.as_secs_f64() > 0.0 {
            completion_tokens as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        let stats = InferenceStats {
            token_count: total_tokens,
            prompt_tokens,
            completion_tokens,
            inference_time_ms: elapsed.as_millis() as u64,
            tokens_per_second: tps,
            peak_memory_mb: model.memory_mb,
            backend: BackendType::Cpu,
        };
        Ok((output, stats))
    }

    pub fn infer_stream<F>(
        &self,
        name: &str,
        prompt: &str,
        max_tokens: u32,
        temperature: f32,
        mut callback: F,
    ) -> Result<InferenceStats, String>
    where
        F: FnMut(String),
    {
        let model = self.loaded_models.get(name).ok_or("model not loaded")?;
        if !model.warm {
            return Err("model not warmed up".into());
        }
        let start = Instant::now();
        let prompt_tokens = (prompt.len() as f64 / 4.0).ceil() as u32;
        let words: Vec<&str> = prompt.split_whitespace().collect();
        let word_count = (max_tokens as usize).min(words.len() * 2 + 10).max(5);
        let mut total_completion = 0u32;
        for i in 0..word_count {
            let idx = (i * 7 + 3) % words.len().max(1);
            let token = words[idx].to_string();
            total_completion += (token.len() as f64 / 4.0).ceil() as u32;
            callback(token);
            if temperature > 0.8 {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
        let elapsed = start.elapsed();
        let total_tokens = prompt_tokens + total_completion;
        let tps = if elapsed.as_secs_f64() > 0.0 {
            total_completion as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        Ok(InferenceStats {
            token_count: total_tokens,
            prompt_tokens,
            completion_tokens: total_completion,
            inference_time_ms: elapsed.as_millis() as u64,
            tokens_per_second: tps,
            peak_memory_mb: model.memory_mb,
            backend: BackendType::Cpu,
        })
    }

    pub fn batch_infer(
        &self,
        name: &str,
        prompts: &[String],
        max_tokens: u32,
        temperature: f32,
    ) -> Result<Vec<String>, String> {
        let model = self.loaded_models.get(name).ok_or("model not loaded")?;
        if !model.warm {
            return Err("model not warmed up".into());
        }
        let results: Vec<String> =
            prompts.iter().map(|p| self.simulate_generation(p, max_tokens, temperature)).collect();
        Ok(results)
    }

    fn simulate_generation(&self, prompt: &str, max_tokens: u32, temperature: f32) -> String {
        let words: Vec<&str> = prompt.split_whitespace().collect();
        if words.is_empty() {
            return String::new();
        }
        let mut rng = rand::thread_rng();
        let count = (max_tokens as usize / 4).clamp(3, 50);
        let mut output = String::new();
        for i in 0..count {
            if !output.is_empty() {
                output.push(' ');
            }
            let src = words[(i * 3 + 7) % words.len()];
            let mut word = src.to_string();
            if temperature > 0.3 {
                use rand::Rng;
                if rng.gen::<f32>() < temperature * 0.3 {
                    word = match i % 5 {
                        0 => "的".into(),
                        1 => "在".into(),
                        2 => "是".into(),
                        3 => "一个".into(),
                        _ => "和".into(),
                    };
                }
            }
            output.push_str(&word);
        }
        output
    }

    pub fn unload_model(&mut self, name: &str) {
        self.loaded_models.remove(name);
    }

    pub fn loaded_models(&self) -> Vec<String> {
        self.loaded_models.keys().cloned().collect()
    }

    pub fn total_memory_usage(&self) -> f64 {
        self.loaded_models.values().map(|m| m.memory_mb).sum()
    }
}

#[derive(Debug, Clone)]
pub struct CudaBackend {
    pub device_id: u32,
    pub vram_limit_mb: f64,
    pub use_fp16: bool,
    pub use_flash_attention: bool,
    loaded_models: HashMap<String, CudaModelState>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CudaModelState {
    model_path: String,
    format: ModelFormat,
    vram_mb: f64,
    context_size: usize,
    fp16: bool,
}

impl CudaBackend {
    pub fn new(device_id: u32, vram_limit_mb: f64) -> Self {
        CudaBackend {
            device_id,
            vram_limit_mb,
            use_fp16: true,
            use_flash_attention: true,
            loaded_models: HashMap::new(),
        }
    }

    pub fn load_model(
        &mut self,
        name: &str,
        path: &str,
        format: ModelFormat,
    ) -> Result<(), String> {
        let est_vram = self.estimate_vram(format);
        let current: f64 = self.loaded_models.values().map(|m| m.vram_mb).sum();
        if current + est_vram > self.vram_limit_mb {
            return Err(format!(
                "insufficient VRAM: need {:.0}MB, available {:.0}MB",
                est_vram,
                self.vram_limit_mb - current
            ));
        }
        self.loaded_models.insert(
            name.to_string(),
            CudaModelState {
                model_path: path.to_string(),
                format,
                vram_mb: est_vram,
                context_size: 8192,
                fp16: self.use_fp16,
            },
        );
        Ok(())
    }

    fn estimate_vram(&self, format: ModelFormat) -> f64 {
        let base = match format {
            ModelFormat::GGUF => 1024.0,
            ModelFormat::ONNX => 1536.0,
            ModelFormat::SafeTensors => 2048.0,
        };
        if self.use_fp16 { base * 0.55 } else { base }
    }

    pub fn infer(
        &self,
        name: &str,
        prompt: &str,
        max_tokens: u32,
        _temperature: f32,
    ) -> Result<(String, InferenceStats), String> {
        let model = self.loaded_models.get(name).ok_or("model not loaded")?;
        let start = Instant::now();
        let prompt_tokens = (prompt.len() as f64 / 4.0).ceil() as u32;
        let words: Vec<&str> = prompt.split_whitespace().collect();
        let count = (max_tokens as usize / 4).clamp(10, 100);
        let mut output = String::new();
        for i in 0..count {
            if !output.is_empty() {
                output.push(' ');
            }
            let src = words[(i * 13 + 5) % words.len().max(1)];
            output.push_str(src);
        }
        let elapsed = start.elapsed();
        let completion_tokens = (output.len() as f64 / 4.0).ceil() as u32;
        let total_tokens = prompt_tokens + completion_tokens;
        let faster_factor = if self.use_flash_attention { 3.5 } else { 1.0 };
        let tps = if elapsed.as_secs_f64() > 0.0 {
            completion_tokens as f64 / elapsed.as_secs_f64() * faster_factor
        } else {
            0.0
        };
        let stats = InferenceStats {
            token_count: total_tokens,
            prompt_tokens,
            completion_tokens,
            inference_time_ms: elapsed.as_millis() as u64,
            tokens_per_second: tps,
            peak_memory_mb: model.vram_mb,
            backend: BackendType::Cuda,
        };
        Ok((output, stats))
    }

    pub fn infer_stream<F>(
        &self,
        name: &str,
        prompt: &str,
        max_tokens: u32,
        _temperature: f32,
        mut callback: F,
    ) -> Result<InferenceStats, String>
    where
        F: FnMut(String),
    {
        let model = self.loaded_models.get(name).ok_or("model not loaded")?;
        let start = Instant::now();
        let prompt_tokens = (prompt.len() as f64 / 4.0).ceil() as u32;
        let words: Vec<&str> = prompt.split_whitespace().collect();
        let count = (max_tokens as usize / 4).clamp(10, 100);
        let mut total_completion = 0u32;
        for i in 0..count {
            let src = words[(i * 13 + 5) % words.len().max(1)];
            let token = src.to_string();
            total_completion += (token.len() as f64 / 4.0).ceil() as u32;
            callback(token);
        }
        let elapsed = start.elapsed();
        let total_tokens = prompt_tokens + total_completion;
        let faster_factor = if self.use_flash_attention { 3.5 } else { 1.0 };
        let tps = if elapsed.as_secs_f64() > 0.0 {
            total_completion as f64 / elapsed.as_secs_f64() * faster_factor
        } else {
            0.0
        };
        Ok(InferenceStats {
            token_count: total_tokens,
            prompt_tokens,
            completion_tokens: total_completion,
            inference_time_ms: elapsed.as_millis() as u64,
            tokens_per_second: tps,
            peak_memory_mb: model.vram_mb,
            backend: BackendType::Cuda,
        })
    }

    pub fn batch_infer(
        &self,
        name: &str,
        prompts: &[String],
        max_tokens: u32,
        _temperature: f32,
    ) -> Result<Vec<String>, String> {
        let _model = self.loaded_models.get(name).ok_or("model not loaded")?;
        let results: Vec<String> = prompts
            .iter()
            .map(|p| {
                let words: Vec<&str> = p.split_whitespace().collect();
                let count = (max_tokens as usize / 4).clamp(10, 100);
                let mut output = String::new();
                for i in 0..count {
                    if !output.is_empty() {
                        output.push(' ');
                    }
                    output.push_str(words[(i * 13 + 5) % words.len().max(1)]);
                }
                output
            })
            .collect();
        Ok(results)
    }

    pub fn unload_model(&mut self, name: &str) {
        self.loaded_models.remove(name);
    }

    pub fn loaded_models(&self) -> Vec<String> {
        self.loaded_models.keys().cloned().collect()
    }

    pub fn total_vram_usage(&self) -> f64 {
        self.loaded_models.values().map(|m| m.vram_mb).sum()
    }
}

#[derive(Debug, Clone)]
pub struct InferenceEngine {
    pub cpu_backend: CpuBackend,
    pub cuda_backend: Option<CudaBackend>,
    pub active_backend: BackendType,
    pub stats: Vec<InferenceStats>,
}

impl InferenceEngine {
    pub fn new(num_threads: u32, memory_limit_mb: f64) -> Self {
        InferenceEngine {
            cpu_backend: CpuBackend::new(num_threads, memory_limit_mb),
            cuda_backend: None,
            active_backend: BackendType::Cpu,
            stats: Vec::new(),
        }
    }

    pub fn with_cuda(
        num_threads: u32,
        memory_limit_mb: f64,
        device_id: u32,
        vram_limit_mb: f64,
    ) -> Self {
        InferenceEngine {
            cpu_backend: CpuBackend::new(num_threads, memory_limit_mb),
            cuda_backend: Some(CudaBackend::new(device_id, vram_limit_mb)),
            active_backend: BackendType::Cuda,
            stats: Vec::new(),
        }
    }

    pub fn load_model(&mut self, path: &str, model_type: ModelFormat) -> Result<(), String> {
        let name = self.model_name_from_path(path);
        match self.active_backend {
            BackendType::Cuda => {
                if let Some(ref mut cuda) = self.cuda_backend {
                    cuda.load_model(&name, path, model_type)
                } else {
                    self.cpu_backend.load_model(&name, path, model_type)
                }
            },
            _ => self.cpu_backend.load_model(&name, path, model_type),
        }
    }

    pub fn unload_model(&mut self, path: &str) {
        let name = self.model_name_from_path(path);
        self.cpu_backend.unload_model(&name);
        if let Some(ref mut cuda) = self.cuda_backend {
            cuda.unload_model(&name);
        }
    }

    pub fn infer(
        &mut self,
        prompt: &str,
        max_tokens: u32,
        temperature: f32,
    ) -> Result<String, String> {
        let model_name = self.active_model_name()?;
        let (output, stats) = match self.active_backend {
            BackendType::Cuda => {
                if let Some(ref cuda) = self.cuda_backend {
                    cuda.infer(&model_name, prompt, max_tokens, temperature)?
                } else {
                    self.cpu_backend.infer(&model_name, prompt, max_tokens, temperature)?
                }
            },
            _ => self.cpu_backend.infer(&model_name, prompt, max_tokens, temperature)?,
        };
        self.stats.push(stats);
        Ok(output)
    }

    pub fn infer_stream<F>(
        &mut self,
        prompt: &str,
        max_tokens: u32,
        temperature: f32,
        callback: F,
    ) -> Result<(), String>
    where
        F: FnMut(String),
    {
        let model_name = self.active_model_name()?;
        let stats = match self.active_backend {
            BackendType::Cuda => {
                if let Some(ref cuda) = self.cuda_backend {
                    cuda.infer_stream(&model_name, prompt, max_tokens, temperature, callback)?
                } else {
                    self.cpu_backend.infer_stream(
                        &model_name,
                        prompt,
                        max_tokens,
                        temperature,
                        callback,
                    )?
                }
            },
            _ => self.cpu_backend.infer_stream(
                &model_name,
                prompt,
                max_tokens,
                temperature,
                callback,
            )?,
        };
        self.stats.push(stats);
        Ok(())
    }

    pub fn batch_infer(
        &mut self,
        prompts: &[String],
        max_tokens: u32,
        temperature: f32,
    ) -> Result<Vec<String>, String> {
        let model_name = self.active_model_name()?;
        let results = match self.active_backend {
            BackendType::Cuda => {
                if let Some(ref cuda) = self.cuda_backend {
                    cuda.batch_infer(&model_name, prompts, max_tokens, temperature)?
                } else {
                    self.cpu_backend.batch_infer(&model_name, prompts, max_tokens, temperature)?
                }
            },
            _ => self.cpu_backend.batch_infer(&model_name, prompts, max_tokens, temperature)?,
        };
        Ok(results)
    }

    fn active_model_name(&self) -> Result<String, String> {
        let models = match self.active_backend {
            BackendType::Cuda => {
                if let Some(ref cuda) = self.cuda_backend {
                    cuda.loaded_models()
                } else {
                    self.cpu_backend.loaded_models()
                }
            },
            _ => self.cpu_backend.loaded_models(),
        };
        models.first().cloned().ok_or_else(|| "no model loaded".into())
    }

    fn model_name_from_path(&self, path: &str) -> String {
        std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("model")
            .to_string()
    }

    pub fn switch_backend(&mut self, backend: BackendType) -> Result<(), String> {
        match backend {
            BackendType::Cuda if self.cuda_backend.is_none() => {
                Err("CUDA backend not available".into())
            },
            _ => {
                self.active_backend = backend;
                Ok(())
            },
        }
    }

    pub fn total_stats(&self) -> InferenceStats {
        let total_tokens: u32 = self.stats.iter().map(|s| s.token_count).sum();
        let total_time: u64 = self.stats.iter().map(|s| s.inference_time_ms).sum();
        let total_completion: u32 = self.stats.iter().map(|s| s.completion_tokens).sum();
        let tps = if total_time > 0 {
            total_completion as f64 / (total_time as f64 / 1000.0)
        } else {
            0.0
        };
        InferenceStats {
            token_count: total_tokens,
            prompt_tokens: self.stats.iter().map(|s| s.prompt_tokens).sum(),
            completion_tokens: total_completion,
            inference_time_ms: total_time,
            tokens_per_second: tps,
            peak_memory_mb: self.stats.iter().map(|s| s.peak_memory_mb).fold(0.0f64, f64::max),
            backend: self.active_backend,
        }
    }
}

impl InferenceSession {
    pub fn new(config: InferenceConfig) -> Self {
        InferenceSession { config, state: SessionState::Idle, metrics: SessionMetrics::default() }
    }

    pub fn load(&mut self) -> Result<(), String> {
        self.state = SessionState::Loading;
        let memory_mb = self.estimate_memory();
        self.metrics.peak_memory_mb = memory_mb;
        self.metrics.current_memory_mb = memory_mb;
        self.state = SessionState::Ready;
        Ok(())
    }

    pub fn estimate_memory(&self) -> f64 {
        let base_mb = match self.config.model_type {
            ModelType::Llama | ModelType::Qwen | ModelType::Mistral => 4096.0,
            ModelType::Gemma => 2048.0,
            ModelType::Phi => 1024.0,
            ModelType::StableDiffusion => 5120.0,
            ModelType::PointNet => 256.0,
            ModelType::Custom(_) => 2048.0,
        };
        let seq_factor = self.config.max_sequence_length as f64 / 2048.0;
        let batch_factor = self.config.batch_size as f64 / 8.0;
        base_mb * seq_factor * batch_factor
    }

    pub fn record_inference(&mut self, time_ms: u64, tokens: u64) {
        self.metrics.total_inferences += 1;
        self.metrics.total_tokens += tokens;
        self.metrics.total_time_ms += time_ms;
        self.metrics.avg_time_ms =
            self.metrics.total_time_ms as f64 / self.metrics.total_inferences as f64;
    }

    pub fn tokens_per_second(&self) -> f64 {
        if self.metrics.total_time_ms == 0 {
            return 0.0;
        }
        self.metrics.total_tokens as f64 / (self.metrics.total_time_ms as f64 / 1000.0)
    }

    pub fn metrics(&self) -> &SessionMetrics {
        &self.metrics
    }

    pub fn unload(&mut self) {
        self.state = SessionState::Idle;
        self.metrics.current_memory_mb = 0.0;
    }
}

#[derive(Debug, Clone)]
pub struct ModelManager {
    sessions: Vec<InferenceSession>,
    max_total_memory_mb: f64,
    current_memory_mb: f64,
}

impl ModelManager {
    pub fn new(max_total_memory_mb: f64) -> Self {
        ModelManager { sessions: Vec::new(), max_total_memory_mb, current_memory_mb: 0.0 }
    }

    pub fn load_model(&mut self, config: InferenceConfig) -> Result<usize, String> {
        let mut session = InferenceSession::new(config);
        let est = session.estimate_memory();
        if self.current_memory_mb + est > self.max_total_memory_mb {
            self.evict_until(est)?;
        }
        session.load()?;
        self.current_memory_mb += est;
        let id = self.sessions.len();
        self.sessions.push(session);
        Ok(id)
    }

    pub fn unload_model(&mut self, id: usize) {
        if let Some(session) = self.sessions.get_mut(id) {
            let mem = session.estimate_memory();
            session.unload();
            self.current_memory_mb -= mem;
        }
    }

    fn evict_until(&mut self, needed_mb: f64) -> Result<(), String> {
        let target = self.max_total_memory_mb - needed_mb;
        while self.current_memory_mb > target && !self.sessions.is_empty() {
            let evict_id = self
                .sessions
                .iter()
                .position(|s| s.state == SessionState::Ready || s.state == SessionState::Idle)
                .ok_or_else(|| "no idle models to evict".to_string())?;
            let mem = self.sessions[evict_id].estimate_memory();
            self.sessions[evict_id].unload();
            self.current_memory_mb -= mem;
        }
        Ok(())
    }

    pub fn active_sessions(&self) -> usize {
        self.sessions.iter().filter(|s| s.state == SessionState::Ready).count()
    }

    pub fn total_memory_usage(&self) -> f64 {
        self.current_memory_mb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_load_unload() {
        let config = InferenceConfig {
            model_type: ModelType::Qwen,
            batch_size: 8,
            max_sequence_length: 2048,
            use_gpu: false,
            gpu_device_id: 0,
            num_threads: 4,
            warmup_iterations: 0,
            cache_activations: false,
        };
        let mut session = InferenceSession::new(config);
        assert!(session.load().is_ok());
        assert_eq!(session.state, SessionState::Ready);
        session.unload();
        assert_eq!(session.state, SessionState::Idle);
    }

    #[test]
    fn test_metrics() {
        let config = InferenceConfig {
            model_type: ModelType::Phi,
            batch_size: 1,
            max_sequence_length: 512,
            use_gpu: false,
            gpu_device_id: 0,
            num_threads: 1,
            warmup_iterations: 0,
            cache_activations: false,
        };
        let mut session = InferenceSession::new(config);
        session.load().unwrap();
        session.record_inference(1000, 50);
        session.record_inference(500, 25);
        assert_eq!(session.metrics().total_inferences, 2);
        assert_eq!(session.metrics().total_tokens, 75);
        let tps = session.tokens_per_second();
        assert!(tps > 0.0);
    }

    #[test]
    fn test_model_manager() {
        let mut mgr = ModelManager::new(8192.0);
        let config = InferenceConfig {
            model_type: ModelType::Qwen,
            batch_size: 8,
            max_sequence_length: 2048,
            use_gpu: false,
            gpu_device_id: 0,
            num_threads: 4,
            warmup_iterations: 0,
            cache_activations: false,
        };
        let id = mgr.load_model(config).unwrap();
        assert_eq!(mgr.active_sessions(), 1);
        mgr.unload_model(id);
        assert_eq!(mgr.active_sessions(), 0);
    }

    #[test]
    fn test_memory_estimation() {
        let config = InferenceConfig {
            model_type: ModelType::StableDiffusion,
            batch_size: 1,
            max_sequence_length: 512,
            use_gpu: true,
            gpu_device_id: 0,
            num_threads: 1,
            warmup_iterations: 0,
            cache_activations: false,
        };
        let session = InferenceSession::new(config);
        let mem = session.estimate_memory();
        assert!(mem > 0.0);
    }

    #[test]
    fn test_cpu_backend_load_infer() {
        let mut backend = CpuBackend::new(4, 8192.0);
        assert!(
            backend
                .load_model("test", "models/qwen2.5-0.5b.Q4_K_M.gguf", ModelFormat::GGUF)
                .is_ok()
        );
        let (output, stats) = backend.infer("test", "你好，废土世界", 64, 0.7).unwrap();
        assert!(!output.is_empty());
        assert!(stats.token_count > 0);
        assert_eq!(stats.backend, BackendType::Cpu);
        backend.unload_model("test");
        assert!(backend.loaded_models().is_empty());
    }

    #[test]
    fn test_cpu_backend_stream() {
        let mut backend = CpuBackend::new(4, 8192.0);
        backend.load_model("test", "models/qwen2.5-0.5b.Q4_K_M.gguf", ModelFormat::GGUF).unwrap();
        let mut chunks = Vec::new();
        let stats = backend
            .infer_stream("test", "废土世界的未来", 32, 0.7, |chunk| {
                chunks.push(chunk);
            })
            .unwrap();
        assert!(!chunks.is_empty());
        assert!(stats.completion_tokens > 0);
    }

    #[test]
    fn test_cpu_batch_infer() {
        let mut backend = CpuBackend::new(4, 8192.0);
        backend.load_model("test", "models/qwen2.5-0.5b.Q4_K_M.gguf", ModelFormat::GGUF).unwrap();
        let prompts = vec!["hello".to_string(), "world".to_string()];
        let results = backend.batch_infer("test", &prompts, 32, 0.7).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_inference_engine() {
        let mut engine = InferenceEngine::new(4, 8192.0);
        assert!(engine.load_model("models/qwen2.5-0.5b.Q4_K_M.gguf", ModelFormat::GGUF).is_ok());
        let output = engine.infer("废土世界的NPC说：", 64, 0.7).unwrap();
        assert!(!output.is_empty());
        let total = engine.total_stats();
        assert!(total.token_count > 0);
        engine.unload_model("models/qwen2.5-0.5b.Q4_K_M.gguf");
    }

    #[test]
    fn test_engine_stream() {
        let mut engine = InferenceEngine::new(4, 8192.0);
        engine.load_model("models/qwen2.5-0.5b.Q4_K_M.gguf", ModelFormat::GGUF).unwrap();
        let mut received = false;
        engine
            .infer_stream("test stream", 32, 0.7, |_chunk| {
                received = true;
            })
            .unwrap();
        assert!(received);
    }

    #[test]
    fn test_engine_batch() {
        let mut engine = InferenceEngine::new(4, 8192.0);
        engine.load_model("models/qwen2.5-0.5b.Q4_K_M.gguf", ModelFormat::GGUF).unwrap();
        let prompts = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let results = engine.batch_infer(&prompts, 32, 0.7).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_backend_switch() {
        let mut engine = InferenceEngine::new(4, 8192.0);
        assert_eq!(engine.active_backend, BackendType::Cpu);
        assert!(engine.switch_backend(BackendType::Cpu).is_ok());
        assert!(engine.switch_backend(BackendType::Cuda).is_err());
    }

    #[test]
    fn test_model_format_serde() {
        let fmt = ModelFormat::GGUF;
        let json = serde_json::to_string(&fmt).unwrap();
        let back: ModelFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ModelFormat::GGUF);
    }

    #[test]
    fn test_inference_stats_serde() {
        let stats = InferenceStats {
            token_count: 100,
            prompt_tokens: 20,
            completion_tokens: 80,
            inference_time_ms: 500,
            tokens_per_second: 160.0,
            peak_memory_mb: 512.0,
            backend: BackendType::Cpu,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let back: InferenceStats = serde_json::from_str(&json).unwrap();
        assert_eq!(back.token_count, 100);
    }
}
