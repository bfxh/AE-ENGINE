pub mod adapter;
pub mod animation_gen;
pub mod finetune;
pub mod inference;
pub mod llm;
pub mod memory_pool;
pub mod model_downloader;
pub mod model_gen;
pub mod model_optimizer;
pub mod npc_knowledge;
pub mod pipeline;
pub mod prompt_templates;
pub mod quantization;
pub mod rag;
pub mod simd_inference;
pub mod world_gen;

pub mod prelude {
    pub use crate::adapter::{ParamType, ToolAdapter, ToolCategory, ToolDefinition, ToolParameter};
    pub use crate::animation_gen::{
        AnimationGenConfig, AnimationGenerator, AnimationRequest, AnimationResult,
    };
    pub use crate::finetune::{
        DataSample, Dataset, FinetuneConfig, LoRALayer, LoRAWeights, Metrics, TrainingProgress,
        evaluate, evaluate_lora, merge_lora, prepare_dataset, train_lora,
    };
    pub use crate::inference::{
        BackendType, CpuBackend, CudaBackend, InferenceConfig, InferenceEngine, InferenceSession,
        InferenceStats, ModelFormat, ModelManager, ModelType, SessionState,
    };
    pub use crate::llm::{
        ChatMessage, ContextMemory, FinishReason, LlmConfig, LlmContext, LlmResponse, MessageRole,
        ToolCall,
    };
    pub use crate::memory_pool::{AiMemoryPool, PoolStats, ScratchBuffer};
    pub use crate::model_downloader::{
        DownloadProgress, DownloadStatus, ModelCategory, ModelInfo, ModelRegistry,
    };
    pub use crate::model_gen::{
        GenerationStyle, LodGenerator, MaterialSlot, MeshValidator, ModelFormat as GenModelFormat,
        ModelGenRequest, ModelGenResult, ModelMetadata, ValidationReport,
    };
    pub use crate::model_optimizer::{
        ModelConvFormat, ModelFormatConverter, ModelOptimizeResult, ModelOptimizer, SimplifyConfig,
    };
    pub use crate::npc_knowledge::{
        DialogueContext, KnowledgeGraph, KnowledgeInjectConfig, Memory, NpcKnowledgeBase,
        NpcKnowledgeInjector, PersonalityProfile,
    };
    pub use crate::pipeline::{
        AiPipeline, DialogueResult, PipelineConfig, PipelineResult, PipelineStats, PipelineTask,
        Quest, QuestObjective, QuestReward, TaskPriority, TaskStatus, TaskType,
    };
    pub use crate::prompt_templates::{
        NpcPromptBuilder, PromptCategory, PromptLibrary, PromptTemplate, RenderResult,
    };
    pub use crate::quantization::{
        QuantizationConfig, QuantizationMode, QuantizedTensor, Quantizer,
    };
    pub use crate::rag::{KnowledgeBase, KnowledgeChunk, KnowledgeSource, RagConfig, RagEngine};
    pub use crate::simd_inference::{SimdArch, SimdDetector};
    pub use crate::world_gen::{WorldGenConfig, WorldGenRequest, WorldGenResult, WorldGenerator};
}
