use godot::prelude::*;

use ae_ai_tools::model_optimizer::ModelOptimizer;
use ae_ai_tools::npc_knowledge::{
    KnowledgeInjectConfig, MoralCompass, NpcKnowledgeInjector, PersonalityProfile,
};
use ae_ai_tools::pipeline::{AiPipeline, PipelineConfig, TaskPriority, TaskType};
use ae_ai_tools::prompt_templates::{PromptCategory, PromptLibrary};
use ae_ai_tools::rag::{KnowledgeSource, RagConfig, RagEngine};
use ae_ai_tools::world_gen::{
    PopulationConfig, TerrainConfig, WorldGenConfig, WorldGenRequest, WorldGenerator,
};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandAITools {
    #[var]
    model_name: GString,
    #[var]
    temperature: f32,
    #[var]
    max_tokens: i64,
    #[var]
    use_quantization: bool,

    pipeline: AiPipeline,
    rag: RagEngine,
    prompts: PromptLibrary,
    optimizer: ModelOptimizer,
    world_gen: WorldGenerator,
    injector: NpcKnowledgeInjector,
    task_count: i64,
    completed_count: i64,
    failed_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandAITools {
    fn init(base: Base<Node>) -> Self {
        let config = PipelineConfig {
            model_name: "qwen2.5-1.5b".into(),
            temperature: 0.7,
            max_tokens: 512,
            use_quantization: true,
            cache_enabled: true,
            cache_ttl_seconds: 300,
            max_context_length: 4096,
            default_system_prompt: "你是一个废土世界的AI助手。".into(),
        };
        Self {
            model_name: GString::from("qwen2.5-1.5b"),
            temperature: 0.7,
            max_tokens: 512,
            use_quantization: true,
            pipeline: AiPipeline::with_config(4096.0, config),
            rag: RagEngine::new(RagConfig::default()),
            prompts: PromptLibrary::new(),
            optimizer: ModelOptimizer::new(),
            world_gen: WorldGenerator::new(WorldGenConfig::default(), 42),
            injector: NpcKnowledgeInjector::new(KnowledgeInjectConfig::default()),
            task_count: 0,
            completed_count: 0,
            failed_count: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandAITools {
    #[func]
    fn submit_task(&mut self, task_type: GString) -> Dictionary<Variant, Variant> {
        self.task_count += 1;
        let tt = match task_type.to_string().as_str() {
            "text_generation" => TaskType::TextGeneration,
            "model_generation" => TaskType::ModelGeneration,
            "npc_dialogue" => TaskType::NpcDialogue,
            "world_simulation" => TaskType::WorldSimulation,
            "quest_generation" => TaskType::QuestGeneration,
            "item_description" => TaskType::ItemDescription,
            "world_lore" => TaskType::WorldLore,
            "environment_description" => TaskType::EnvironmentDescription,
            _ => TaskType::TextGeneration,
        };
        let id = self.pipeline.submit_task(tt, TaskPriority::Normal);
        let results = self.pipeline.process_queue();
        let success = !results.is_empty();
        if success {
            self.completed_count += 1;
        } else {
            self.failed_count += 1;
        }
        dict! {
            "task_id" => id as i64,
            "success" => success,
            "results_count" => results.len() as i64,
        }
    }

    #[func]
    fn generate_npc_dialogue(
        &mut self,
        context: GString,
        npc_personality: GString,
        player_message: GString,
    ) -> Dictionary<Variant, Variant> {
        self.task_count += 1;
        let result = self.pipeline.generate_npc_dialogue(
            &context.to_string(),
            &npc_personality.to_string(),
            &player_message.to_string(),
        );
        self.completed_count += 1;
        dict! {
            "success" => true,
            "response" => &GString::from(result.npc_response.as_str()),
            "emotion" => &GString::from(result.emotion.as_str()),
            "tokens_used" => result.tokens_used as i64,
        }
    }

    #[func]
    fn generate_quest(
        &mut self,
        context: GString,
        difficulty: GString,
        location: GString,
    ) -> Dictionary<Variant, Variant> {
        self.task_count += 1;
        let quest = self.pipeline.generate_quest(
            &context.to_string(),
            &difficulty.to_string(),
            &location.to_string(),
        );
        self.completed_count += 1;
        dict! {
            "success" => true,
            "title" => &GString::from(quest.title.as_str()),
            "description" => &GString::from(quest.description.as_str()),
            "difficulty" => &GString::from(quest.difficulty.as_str()),
            "objective_count" => quest.objectives.len() as i64,
        }
    }

    #[func]
    fn generate_item_description(
        &mut self,
        material: GString,
        method: GString,
    ) -> Dictionary<Variant, Variant> {
        self.task_count += 1;
        let desc =
            self.pipeline.generate_item_description(&material.to_string(), &method.to_string());
        self.completed_count += 1;
        dict! {
            "success" => true,
            "description" => &GString::from(desc.as_str()),
        }
    }

    #[func]
    fn generate_world_lore(
        &mut self,
        seed: i64,
        world_age: i64,
        events_json: GString,
    ) -> Dictionary<Variant, Variant> {
        self.task_count += 1;
        let events: Vec<String> =
            events_json.to_string().split(',').map(|s| s.trim().to_string()).collect();
        let lore = self.pipeline.generate_world_lore(seed as u64, world_age as u32, &events);
        self.completed_count += 1;
        dict! {
            "success" => true,
            "lore" => &GString::from(lore.as_str()),
        }
    }

    #[func]
    fn generate_environment(
        &mut self,
        weather: GString,
        time_of_day: GString,
        biome: GString,
    ) -> Dictionary<Variant, Variant> {
        self.task_count += 1;
        let desc = self.pipeline.generate_environment_description(
            &weather.to_string(),
            &time_of_day.to_string(),
            &biome.to_string(),
        );
        self.completed_count += 1;
        dict! {
            "success" => true,
            "description" => &GString::from(desc.as_str()),
        }
    }

    #[func]
    fn add_knowledge(&mut self, content: GString, source: GString, importance: f32) {
        let src = match source.to_string().as_str() {
            "world_data" => KnowledgeSource::WorldData,
            "npc_memory" => KnowledgeSource::NpcMemory,
            "player_interaction" => KnowledgeSource::PlayerInteraction,
            "book" => KnowledgeSource::Book,
            "environmental" => KnowledgeSource::Environmental,
            _ => KnowledgeSource::Custom(0),
        };
        self.rag.ingest(content.to_string(), src, importance, vec![]);
    }

    #[func]
    fn search_knowledge(&self, query_embedding: PackedFloat32Array) -> GString {
        let embedding: Vec<f32> = query_embedding.as_slice().to_vec();
        let results = self.rag.query(&embedding);
        let texts: Vec<String> = results.iter().map(|r| r.content.clone()).collect();
        GString::from(texts.join("\n---\n").as_str())
    }

    #[func]
    fn get_prompt_template(&self, name: GString) -> GString {
        if let Some(template) = self.prompts.get(&name.to_string()) {
            GString::from(template.system_prompt.as_str())
        } else {
            GString::from("")
        }
    }

    #[func]
    fn get_prompts_by_category(&self, category: GString) -> PackedStringArray {
        let cat = match category.to_string().as_str() {
            "npc_dialogue" => PromptCategory::NpcDialogue,
            "npc_story" => PromptCategory::NpcStory,
            "world_description" => PromptCategory::WorldDescription,
            "quest_generation" => PromptCategory::QuestGeneration,
            "item_description" => PromptCategory::ItemDescription,
            "combat_narration" => PromptCategory::CombatNarration,
            "environmental_narration" => PromptCategory::EnvironmentalNarration,
            "lore_generation" => PromptCategory::LoreGeneration,
            _ => PromptCategory::NpcDialogue,
        };
        let templates = self.prompts.get_by_category(cat);
        let mut arr = PackedStringArray::new();
        for t in templates.iter() {
            arr.push(&GString::from(t.name.as_str()));
        }
        arr
    }

    #[func]
    fn estimate_optimize(&mut self, vertices: i64, triangles: i64) -> Dictionary<Variant, Variant> {
        let result = self.optimizer.estimate_simplify_output(vertices as u32, triangles as u32);
        dict! {
            "original_vertices" => result.original_vertices as i64,
            "optimized_vertices" => result.optimized_vertices as i64,
            "original_triangles" => result.original_triangles as i64,
            "optimized_triangles" => result.optimized_triangles as i64,
            "reduction_ratio" => result.reduction_ratio,
        }
    }

    #[func]
    fn generate_world_region(
        &mut self,
        seed: i64,
        size_km: f32,
        _biome: GString,
    ) -> Dictionary<Variant, Variant> {
        self.task_count += 1;
        let request = WorldGenRequest {
            world_size: [size_km * 1000.0, size_km * 1000.0],
            seed: seed as u64,
            biomes: vec![],
            terrain: TerrainConfig {
                height_scale: 1.0,
                noise_octaves: 4,
                noise_persistence: 0.5,
                noise_lacunarity: 2.0,
                erosion_iterations: 5,
                river_count: 3,
                lake_threshold: 0.3,
                cliff_threshold: 0.7,
            },
            structures: vec![],
            population: PopulationConfig {
                npc_density: 0.1,
                creature_density: 0.3,
                faction_count: 3,
                resource_scatter: true,
            },
        };
        let result = self.world_gen.generate(&request);
        self.completed_count += 1;
        dict! {
            "success" => true,
            "seed" => result.seed as i64,
            "biome_count" => result.metadata.biome_count as i64,
            "structure_count" => result.metadata.structure_count as i64,
            "total_cells" => result.metadata.total_cells as i64,
        }
    }

    #[func]
    fn create_npc(
        &mut self,
        npc_id: GString,
        name: GString,
        occupation: GString,
    ) -> Dictionary<Variant, Variant> {
        let personality = PersonalityProfile {
            openness: 0.5,
            conscientiousness: 0.5,
            extraversion: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
            traits: vec![],
            quirks: vec![],
            moral_compass: MoralCompass {
                honesty: 0.5,
                compassion: 0.5,
                loyalty: 0.5,
                courage: 0.5,
                selfishness: 0.5,
                cruelty: 0.5,
            },
        };
        let npc = self.injector.create_npc(
            npc_id.to_string(),
            name.to_string(),
            personality,
            occupation.to_string(),
            None,
        );
        dict! {
            "success" => true,
            "npc_id" => &GString::from(npc.npc_id.as_str()),
            "name" => &GString::from(npc.name.as_str()),
        }
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        let stats = self.pipeline.stats();
        dict! {
            "task_count" => self.task_count,
            "completed_count" => self.completed_count,
            "failed_count" => self.failed_count,
            "active_tasks" => stats.active_tasks as i64,
            "completed_tasks" => stats.completed_tasks as i64,
            "loaded_models" => stats.loaded_models as i64,
            "memory_usage_mb" => stats.memory_usage_mb,
            "model_name" => &self.model_name,
            "temperature" => self.temperature,
            "use_quantization" => self.use_quantization,
        }
    }
}
