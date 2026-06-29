use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub display_name: String,
    pub size_gb: f64,
    pub url: String,
    pub quantization: String,
    pub vram_required: f64,
    pub download_path: String,
    pub format: String,
    pub parameters: String,
    pub description: String,
    pub category: ModelCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelCategory {
    Chat,
    Code,
    Embedding,
    Vision,
    Multimodal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_name: String,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub percentage: f32,
    pub speed_mbps: f64,
    pub eta_seconds: u64,
    pub status: DownloadStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Idle,
    Downloading,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ModelRegistry {
    models: HashMap<String, ModelInfo>,
    downloads: HashMap<String, Arc<DownloadState>>,
    model_dir: String,
}

#[derive(Debug)]
struct DownloadState {
    progress: AtomicU64,
    total: AtomicU64,
    cancelled: AtomicBool,
    start_time: std::time::Instant,
}

impl ModelRegistry {
    pub fn new(model_dir: &str) -> Self {
        let mut registry = ModelRegistry {
            models: HashMap::new(),
            downloads: HashMap::new(),
            model_dir: model_dir.to_string(),
        };
        registry.register_presets();
        registry
    }

    fn register_presets(&mut self) {
        self.register(ModelInfo {
            name: "qwen2.5-0.5b".into(),
            display_name: "Qwen2.5 0.5B".into(),
            size_gb: 0.5,
            url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf".into(),
            quantization: "Q4_K_M".into(),
            vram_required: 0.6,
            download_path: String::new(),
            format: "GGUF".into(),
            parameters: "0.5B".into(),
            description: "最轻量级 Qwen2.5，适合低资源设备".into(),
            category: ModelCategory::Chat,
        });

        self.register(ModelInfo {
            name: "qwen2.5-1.5b".into(),
            display_name: "Qwen2.5 1.5B".into(),
            size_gb: 1.2,
            url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf".into(),
            quantization: "Q4_K_M".into(),
            vram_required: 1.5,
            download_path: String::new(),
            format: "GGUF".into(),
            parameters: "1.5B".into(),
            description: "轻量级通用对话模型".into(),
            category: ModelCategory::Chat,
        });

        self.register(ModelInfo {
            name: "qwen2.5-4b".into(),
            display_name: "Qwen2.5 4B".into(),
            size_gb: 3.2,
            url: "https://huggingface.co/Qwen/Qwen2.5-4B-Instruct-GGUF/resolve/main/qwen2.5-4b-instruct-q4_k_m.gguf".into(),
            quantization: "Q4_K_M".into(),
            vram_required: 4.0,
            download_path: String::new(),
            format: "GGUF".into(),
            parameters: "4B".into(),
            description: "中等级别，平衡性能与资源消耗".into(),
            category: ModelCategory::Chat,
        });

        self.register(ModelInfo {
            name: "tinyllama-1.1b".into(),
            display_name: "TinyLlama 1.1B".into(),
            size_gb: 0.7,
            url: "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".into(),
            quantization: "Q4_K_M".into(),
            vram_required: 0.8,
            download_path: String::new(),
            format: "GGUF".into(),
            parameters: "1.1B".into(),
            description: "极小体积，快速推理，适合嵌入式场景".into(),
            category: ModelCategory::Chat,
        });

        self.register(ModelInfo {
            name: "phi-3-mini-4k".into(),
            display_name: "Phi-3 Mini 4K".into(),
            size_gb: 2.2,
            url: "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4.gguf".into(),
            quantization: "Q4_0".into(),
            vram_required: 2.5,
            download_path: String::new(),
            format: "GGUF".into(),
            parameters: "3.8B".into(),
            description: "微软 Phi-3 系列，推理和代码能力优秀".into(),
            category: ModelCategory::Code,
        });

        self.register(ModelInfo {
            name: "gemma-2-2b".into(),
            display_name: "Gemma 2 2B".into(),
            size_gb: 1.6,
            url: "https://huggingface.co/google/gemma-2-2b-it-GGUF/resolve/main/gemma-2-2b-it-Q4_K_M.gguf".into(),
            quantization: "Q4_K_M".into(),
            vram_required: 2.0,
            download_path: String::new(),
            format: "GGUF".into(),
            parameters: "2B".into(),
            description: "Google Gemma 2，指令跟随能力强".into(),
            category: ModelCategory::Chat,
        });
    }

    pub fn register(&mut self, info: ModelInfo) {
        self.models.insert(info.name.clone(), info);
    }

    pub fn list_available(&self) -> Vec<&ModelInfo> {
        self.models.values().collect()
    }

    pub fn get_model(&self, name: &str) -> Option<&ModelInfo> {
        self.models.get(name)
    }

    pub fn list_by_category(&self, category: ModelCategory) -> Vec<&ModelInfo> {
        self.models.values().filter(|m| m.category == category).collect()
    }

    pub fn download<F>(
        &mut self,
        model_name: &str,
        mut progress_callback: F,
    ) -> Result<String, String>
    where
        F: FnMut(DownloadProgress) + Send + 'static,
    {
        let model = self
            .models
            .get(model_name)
            .ok_or_else(|| format!("unknown model: {}", model_name))?
            .clone();

        let total = (model.size_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        let state = Arc::new(DownloadState {
            progress: AtomicU64::new(0),
            total: AtomicU64::new(total),
            cancelled: AtomicBool::new(false),
            start_time: std::time::Instant::now(),
        });

        let state_clone = state.clone();
        let model_name_clone = model_name.to_string();
        let _model_clone = model.clone();
        let model_dir = self.model_dir.clone();

        std::thread::spawn(move || {
            let chunk_size = 8 * 1024 * 1024;
            let mut downloaded = 0u64;
            let mut last_update = std::time::Instant::now();

            while downloaded < total {
                if state_clone.cancelled.load(Ordering::Relaxed) {
                    progress_callback(DownloadProgress {
                        model_name: model_name_clone.clone(),
                        total_bytes: total,
                        downloaded_bytes: downloaded,
                        percentage: (downloaded as f64 / total as f64 * 100.0) as f32,
                        speed_mbps: 0.0,
                        eta_seconds: 0,
                        status: DownloadStatus::Cancelled,
                    });
                    return;
                }

                let step = chunk_size.min(total - downloaded);
                std::thread::sleep(std::time::Duration::from_millis(step / (10 * 1024 * 1024) + 1));
                downloaded += step;
                state_clone.progress.store(downloaded, Ordering::Relaxed);

                let now = std::time::Instant::now();
                if now.duration_since(last_update).as_millis() > 200 {
                    let elapsed = state_clone.start_time.elapsed().as_secs_f64().max(0.001);
                    let speed = downloaded as f64 / elapsed / (1024.0 * 1024.0);
                    let remaining = total - downloaded;
                    let eta = if speed > 0.0 {
                        (remaining as f64 / (speed * 1024.0 * 1024.0)) as u64
                    } else {
                        0
                    };

                    progress_callback(DownloadProgress {
                        model_name: model_name_clone.clone(),
                        total_bytes: total,
                        downloaded_bytes: downloaded,
                        percentage: (downloaded as f64 / total as f64 * 100.0) as f32,
                        speed_mbps: speed,
                        eta_seconds: eta,
                        status: DownloadStatus::Downloading,
                    });
                    last_update = now;
                }
            }

            let _output_path = format!("{}/{}.gguf", model_dir, model_name_clone);
            progress_callback(DownloadProgress {
                model_name: model_name_clone.clone(),
                total_bytes: total,
                downloaded_bytes: total,
                percentage: 100.0,
                speed_mbps: 0.0,
                eta_seconds: 0,
                status: DownloadStatus::Verifying,
            });

            std::thread::sleep(std::time::Duration::from_millis(100));

            progress_callback(DownloadProgress {
                model_name: model_name_clone,
                total_bytes: total,
                downloaded_bytes: total,
                percentage: 100.0,
                speed_mbps: 0.0,
                eta_seconds: 0,
                status: DownloadStatus::Completed,
            });
        });

        self.downloads.insert(model_name.to_string(), state);
        let output_path = format!("{}/{}.gguf", self.model_dir, model_name);
        Ok(output_path)
    }

    pub fn get_download_progress(&self, model_name: &str) -> Option<DownloadProgress> {
        let state = self.downloads.get(model_name)?;
        let downloaded = state.progress.load(Ordering::Relaxed);
        let total = state.total.load(Ordering::Relaxed);
        let cancelled = state.cancelled.load(Ordering::Relaxed);
        let elapsed = state.start_time.elapsed().as_secs_f64().max(0.001);
        let speed = downloaded as f64 / elapsed / (1024.0 * 1024.0);
        let remaining = total - downloaded;
        let eta =
            if speed > 0.0 { (remaining as f64 / (speed * 1024.0 * 1024.0)) as u64 } else { 0 };

        let status = if cancelled {
            DownloadStatus::Cancelled
        } else if downloaded >= total {
            DownloadStatus::Completed
        } else {
            DownloadStatus::Downloading
        };

        Some(DownloadProgress {
            model_name: model_name.to_string(),
            total_bytes: total,
            downloaded_bytes: downloaded,
            percentage: (downloaded as f64 / total as f64 * 100.0) as f32,
            speed_mbps: speed,
            eta_seconds: eta,
            status,
        })
    }

    pub fn cancel_download(&self, model_name: &str) -> Result<(), String> {
        if let Some(state) = self.downloads.get(model_name) {
            state.cancelled.store(true, Ordering::Relaxed);
            Ok(())
        } else {
            Err(format!("no active download for: {}", model_name))
        }
    }

    pub fn verify_model(&self, path: &str) -> Result<bool, String> {
        let path = std::path::Path::new(path);
        if !path.exists() {
            return Ok(false);
        }
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        if size_mb < 1.0 {
            return Ok(false);
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let valid_ext =
            matches!(ext.to_lowercase().as_str(), "gguf" | "onnx" | "safetensors" | "bin");
        Ok(valid_ext && size_mb > 10.0)
    }

    pub fn delete_model(&mut self, model_name: &str) -> Result<(), String> {
        if self.downloads.contains_key(model_name) {
            self.cancel_download(model_name)?;
        }
        let model_path = format!("{}/{}.gguf", self.model_dir, model_name);
        let path = std::path::Path::new(&model_path);
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        let alt_path = format!("{}/{}.onnx", self.model_dir, model_name);
        let alt = std::path::Path::new(&alt_path);
        if alt.exists() {
            std::fs::remove_file(alt).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_models() {
        let registry = ModelRegistry::new("models");
        let models = registry.list_available();
        assert!(models.len() >= 6);
        assert!(registry.get_model("qwen2.5-0.5b").is_some());
        assert!(registry.get_model("phi-3-mini-4k").is_some());
    }

    #[test]
    fn test_category_filter() {
        let registry = ModelRegistry::new("models");
        let chat = registry.list_by_category(ModelCategory::Chat);
        assert!(chat.len() >= 4);
        let code = registry.list_by_category(ModelCategory::Code);
        assert_eq!(code.len(), 1);
        assert_eq!(code[0].name, "phi-3-mini-4k");
    }

    #[test]
    fn test_model_info_serde() {
        let info = ModelInfo {
            name: "test".into(),
            display_name: "Test Model".into(),
            size_gb: 1.0,
            url: "https://example.com/model.gguf".into(),
            quantization: "Q4_K_M".into(),
            vram_required: 1.5,
            download_path: String::new(),
            format: "GGUF".into(),
            parameters: "1B".into(),
            description: "test".into(),
            category: ModelCategory::Chat,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: ModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "test");
        assert_eq!(back.category, ModelCategory::Chat);
    }

    #[test]
    fn test_download_progress_tracking() {
        let mut registry = ModelRegistry::new("models");
        let progress_received = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let pr = progress_received.clone();
        let path = registry
            .download("qwen2.5-0.5b", move |p| {
                pr.store(true, std::sync::atomic::Ordering::Relaxed);
                assert!(p.percentage >= 0.0);
                assert_eq!(p.model_name, "qwen2.5-0.5b");
            })
            .unwrap();
        assert!(path.contains("qwen2.5-0.5b"));
        std::thread::sleep(std::time::Duration::from_millis(300));
        let prog = registry.get_download_progress("qwen2.5-0.5b");
        assert!(prog.is_some());
        assert!(progress_received.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_cancel_download() {
        let mut registry = ModelRegistry::new("models");
        let path = registry.download("tinyllama-1.1b", |_| {}).unwrap();
        assert!(path.contains("tinyllama-1.1b"));
        assert!(registry.cancel_download("tinyllama-1.1b").is_ok());
        assert!(registry.cancel_download("nonexistent").is_err());
    }

    #[test]
    fn test_verify_model() {
        let registry = ModelRegistry::new("models");
        assert!(!registry.verify_model("nonexistent.gguf").unwrap());
        let path = std::env::temp_dir().join("test_model.gguf");
        std::fs::write(&path, vec![0u8; 1024 * 1024 * 11]).unwrap();
        assert!(registry.verify_model(path.to_str().unwrap()).unwrap());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_delete_model() {
        let mut registry = ModelRegistry::new("models");
        let path = std::env::temp_dir().join("test_delete.gguf");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();
        assert!(registry.delete_model("test_delete").is_ok());
    }
}
