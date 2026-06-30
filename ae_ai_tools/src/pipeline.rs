use crate::adapter::ToolAdapter;
use crate::inference::ModelManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub model_name: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub use_quantization: bool,
    pub cache_enabled: bool,
    pub cache_ttl_seconds: u64,
    pub max_context_length: usize,
    pub default_system_prompt: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        PipelineConfig {
            model_name: "qwen2.5-1.5b".into(),
            temperature: 0.7,
            max_tokens: 512,
            use_quantization: true,
            cache_enabled: true,
            cache_ttl_seconds: 300,
            max_context_length: 4096,
            default_system_prompt: "你是一个废土世界的AI助手。所有回答基于物理现实，不涉及魔法或超自然力量。请用中文回复。".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AiPipeline {
    pub model_manager: ModelManager,
    pub tool_adapter: ToolAdapter,
    pub config: PipelineConfig,
    active_tasks: Vec<PipelineTask>,
    completed_tasks: Vec<PipelineResult>,
    cache: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: String,
    created_at: std::time::Instant,
    ttl_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct PipelineTask {
    pub id: u64,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub created_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    TextGeneration,
    ModelGeneration,
    MeshValidation,
    LodGeneration,
    TextureGeneration,
    NpcDialogue,
    WorldSimulation,
    QuestGeneration,
    ItemDescription,
    WorldLore,
    EnvironmentDescription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub task_id: u64,
    pub success: bool,
    pub output: Option<Vec<u8>>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueResult {
    pub npc_response: String,
    pub emotion: String,
    pub action: Option<String>,
    pub knowledge_updates: Vec<String>,
    pub relationship_change: f32,
    pub tokens_used: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub title: String,
    pub description: String,
    pub objectives: Vec<QuestObjective>,
    pub rewards: Vec<QuestReward>,
    pub difficulty: String,
    pub location: String,
    pub giver: String,
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    pub description: String,
    pub target_count: u32,
    pub objective_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestReward {
    pub reward_type: String,
    pub amount: u32,
    pub item_name: Option<String>,
}

impl AiPipeline {
    pub fn new(max_memory_mb: f64) -> Self {
        AiPipeline {
            model_manager: ModelManager::new(max_memory_mb),
            tool_adapter: ToolAdapter::new(),
            config: PipelineConfig::default(),
            active_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            cache: HashMap::new(),
        }
    }

    pub fn with_config(max_memory_mb: f64, config: PipelineConfig) -> Self {
        AiPipeline {
            model_manager: ModelManager::new(max_memory_mb),
            tool_adapter: ToolAdapter::new(),
            config,
            active_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            cache: HashMap::new(),
        }
    }

    pub fn submit_task(&mut self, task_type: TaskType, priority: TaskPriority) -> u64 {
        let id = self.active_tasks.len() as u64 + 1;
        self.active_tasks.push(PipelineTask {
            id,
            task_type,
            status: TaskStatus::Queued,
            priority,
            created_at: 0,
        });
        self.active_tasks.sort_by_key(|t| t.priority);
        id
    }

    pub fn process_queue(&mut self) -> Vec<PipelineResult> {
        let mut results = Vec::new();
        for task in &mut self.active_tasks {
            if task.status == TaskStatus::Queued {
                task.status = TaskStatus::Running;
            }
        }
        for task in &mut self.active_tasks {
            if task.status == TaskStatus::Running {
                let result = PipelineResult {
                    task_id: task.id,
                    success: true,
                    output: None,
                    error: None,
                    duration_ms: 0,
                };
                task.status = TaskStatus::Completed;
                results.push(result);
            }
        }
        self.completed_tasks.extend(results.iter().cloned());
        self.active_tasks.retain(|t| t.status != TaskStatus::Completed);
        results
    }

    fn cache_get(&self, key: &str) -> Option<String> {
        if !self.config.cache_enabled {
            return None;
        }
        self.cache.get(key).and_then(|entry| {
            if entry.created_at.elapsed().as_secs() < entry.ttl_seconds {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    fn cache_set(&mut self, key: String, value: String) {
        if !self.config.cache_enabled {
            return;
        }
        self.cache.insert(
            key,
            CacheEntry {
                value,
                created_at: std::time::Instant::now(),
                ttl_seconds: self.config.cache_ttl_seconds,
            },
        );
    }

    pub fn generate_npc_dialogue(
        &mut self,
        context: &str,
        npc_personality: &str,
        player_message: &str,
    ) -> DialogueResult {
        let cache_key = format!("npc_dialogue:{}:{}:{}", context, npc_personality, player_message);
        if let Some(cached) = self.cache_get(&cache_key) {
            return serde_json::from_str(&cached).unwrap_or_else(|_| DialogueResult {
                npc_response: cached,
                emotion: "neutral".into(),
                action: None,
                knowledge_updates: vec![],
                relationship_change: 0.0,
                tokens_used: 0,
            });
        }

        let emotions = ["平静", "警惕", "好奇", "疲惫", "友善", "愤怒", "悲伤", "兴奋"];
        let emotion_idx = (context.len() + player_message.len()) % emotions.len();

        let response = format!(
            "{}（{}地看了你一眼）「{}？在这个废土上，{}可不是随便说说的事。」",
            npc_personality, emotions[emotion_idx], player_message, context,
        );

        let result = DialogueResult {
            npc_response: response.clone(),
            emotion: emotions[emotion_idx].to_string(),
            action: Some("glances at the player".into()),
            knowledge_updates: vec![format!("玩家询问了关于{}的事", context)],
            relationship_change: 0.05,
            tokens_used: (response.len() as u32 / 4).max(10),
        };

        if let Ok(json) = serde_json::to_string(&result) {
            self.cache_set(cache_key, json);
        }

        result
    }

    pub fn generate_quest(&mut self, context: &str, difficulty: &str, location: &str) -> Quest {
        let cache_key = format!("quest:{}:{}:{}", context, difficulty, location);
        if let Some(cached) = self.cache_get(&cache_key) {
            return serde_json::from_str(&cached)
                .unwrap_or_else(|_| self.fallback_quest(difficulty, location));
        }

        let quest = match difficulty {
            "easy" | "简单" => Quest {
                title: format!("废土拾荒 - {}", location),
                description: format!("在{}附近的废墟中寻找有用的物资。{}", location, context),
                objectives: vec![
                    QuestObjective {
                        description: "搜索废墟".into(),
                        target_count: 3,
                        objective_type: "explore".into(),
                    },
                    QuestObjective {
                        description: "收集废金属".into(),
                        target_count: 5,
                        objective_type: "collect".into(),
                    },
                ],
                rewards: vec![
                    QuestReward { reward_type: "exp".into(), amount: 100, item_name: None },
                    QuestReward {
                        reward_type: "item".into(),
                        amount: 1,
                        item_name: Some("净化水".into()),
                    },
                ],
                difficulty: difficulty.to_string(),
                location: location.to_string(),
                giver: "废土居民".into(),
                prerequisites: vec![],
            },
            "hard" | "困难" => Quest {
                title: format!("危机四伏 - {}", location),
                description: format!("{}的{}，需要清除威胁并收集关键资源。", location, context),
                objectives: vec![
                    QuestObjective {
                        description: "清除变异生物".into(),
                        target_count: 8,
                        objective_type: "kill".into(),
                    },
                    QuestObjective {
                        description: "回收科技零件".into(),
                        target_count: 3,
                        objective_type: "collect".into(),
                    },
                    QuestObjective {
                        description: "护送幸存者".into(),
                        target_count: 1,
                        objective_type: "escort".into(),
                    },
                ],
                rewards: vec![
                    QuestReward { reward_type: "exp".into(), amount: 500, item_name: None },
                    QuestReward {
                        reward_type: "item".into(),
                        amount: 1,
                        item_name: Some("军用弹药箱".into()),
                    },
                    QuestReward { reward_type: "currency".into(), amount: 200, item_name: None },
                ],
                difficulty: difficulty.to_string(),
                location: location.to_string(),
                giver: "聚居地长老".into(),
                prerequisites: vec!["完成基础生存训练".into()],
            },
            _ => Quest {
                title: format!("废土日常 - {}", location),
                description: format!("{}的{}需要你的帮助。{}", location, context, context),
                objectives: vec![
                    QuestObjective {
                        description: "调查异常信号".into(),
                        target_count: 1,
                        objective_type: "investigate".into(),
                    },
                    QuestObjective {
                        description: "收集样本".into(),
                        target_count: 3,
                        objective_type: "collect".into(),
                    },
                ],
                rewards: vec![
                    QuestReward { reward_type: "exp".into(), amount: 250, item_name: None },
                    QuestReward {
                        reward_type: "item".into(),
                        amount: 2,
                        item_name: Some("医疗包".into()),
                    },
                ],
                difficulty: difficulty.to_string(),
                location: location.to_string(),
                giver: "流浪商人".into(),
                prerequisites: vec![],
            },
        };

        if let Ok(json) = serde_json::to_string(&quest) {
            self.cache_set(cache_key, json);
        }

        quest
    }

    fn fallback_quest(&self, difficulty: &str, location: &str) -> Quest {
        Quest {
            title: format!("废土探索 - {}", location),
            description: format!("在{}探索未知区域", location),
            objectives: vec![QuestObjective {
                description: "探索区域".into(),
                target_count: 1,
                objective_type: "explore".into(),
            }],
            rewards: vec![QuestReward { reward_type: "exp".into(), amount: 100, item_name: None }],
            difficulty: difficulty.to_string(),
            location: location.to_string(),
            giver: "未知".into(),
            prerequisites: vec![],
        }
    }

    pub fn generate_item_description(
        &mut self,
        material_properties: &str,
        crafting_method: &str,
    ) -> String {
        let cache_key = format!("item_desc:{}:{}", material_properties, crafting_method);
        if let Some(cached) = self.cache_get(&cache_key) {
            return cached;
        }

        let materials: Vec<&str> = material_properties.split(',').map(|s| s.trim()).collect();
        let primary = materials.first().unwrap_or(&"未知材质");

        let description = format!(
            "这件物品由{}制成，采用{}工艺精心打造。表面呈现出废土特有的风化痕迹，{}的质地坚硬而耐用。在如今这个资源匮乏的世界里，这样的物品弥足珍贵。",
            primary, crafting_method, primary,
        );

        self.cache_set(cache_key, description.clone());
        description
    }

    pub fn generate_world_lore(
        &mut self,
        seed: u64,
        world_age: u32,
        major_events: &[String],
    ) -> String {
        let events_key = major_events.join("|");
        let cache_key = format!("lore:{}:{}:{}", seed, world_age, events_key);
        if let Some(cached) = self.cache_get(&cache_key) {
            return cached;
        }

        let era_names = ["大崩塌时代", "黑暗纪元", "曙光重建期", "部落纷争期", "技术复兴期"];
        let era_idx = (seed as usize) % era_names.len();

        let mut lore = String::new();
        lore.push_str("=== 废土世界编年史 ===\n");
        lore.push_str(&format!("世界年龄：{}年\n", world_age));
        lore.push_str(&format!("当前纪元：{}\n\n", era_names[era_idx]));

        lore.push_str("重大历史事件：\n");
        for (i, event) in major_events.iter().enumerate() {
            let year = world_age.saturating_sub((major_events.len() - i) as u32 * 50);
            lore.push_str(&format!("  第{}年：{}\n", year, event));
        }

        lore.push_str(&format!(
            "\n在{}年的废土历史中，人类文明经历了从巅峰到毁灭再到重建的循环。",
            world_age
        ));
        lore.push_str("没有魔法，没有超自然力量，只有科技、生存意志和人性的光辉与黑暗交织。");

        self.cache_set(cache_key, lore.clone());
        lore
    }

    pub fn generate_environment_description(
        &mut self,
        weather: &str,
        time_of_day: &str,
        biome: &str,
    ) -> String {
        let cache_key = format!("env:{}:{}:{}", weather, time_of_day, biome);
        if let Some(cached) = self.cache_get(&cache_key) {
            return cached;
        }

        let time_desc = match time_of_day {
            "dawn" | "黎明" => "黎明的微光穿透厚重的云层，在地面上投下长长的影子",
            "noon" | "正午" => "正午的烈日无情地炙烤着龟裂的大地",
            "dusk" | "黄昏" => "黄昏的余晖将天空染成病态的橘红色",
            "night" | "夜晚" => "夜晚的黑暗笼罩着废墟，只有远处偶尔闪烁的磷光",
            _ => "天空呈现出废土特有的灰黄色调",
        };

        let weather_desc = match weather {
            "rain" | "雨" => "酸雨淅淅沥沥地落下，在金属残骸上激起腐蚀的泡沫",
            "storm" | "风暴" => "辐射风暴席卷而来，空气中弥漫着电离的刺鼻气味",
            "fog" | "雾" => "浓重的雾气弥漫在废墟之间，能见度不足十米",
            "clear" | "晴" => "难得一见的晴天使整个废土暴露在强烈的紫外线下",
            _ => "天气变幻莫测，一如废土本身",
        };

        let biome_desc = match biome {
            "desert" | "沙漠" => "无尽的沙海中埋藏着旧世界的城市遗迹",
            "ruins" | "废墟" => "倒塌的摩天大楼和锈蚀的车辆构成了这片钢铁丛林",
            "forest" | "森林" => "变异植物疯长，将曾经的居民区变成了绿色地狱",
            "swamp" | "沼泽" => "化学废料形成的沼泽散发着有毒气体",
            "mountain" | "山区" => "风化严重的山脊上，裸露的矿脉在阳光下闪烁",
            _ => "这片废土展现出独特的后末日景观",
        };

        let description = format!(
            "{}。{}。{}。空气中混合着铁锈、臭氧和腐殖质的气味，远处的金属结构在风中发出低沉的呻吟。",
            time_desc, weather_desc, biome_desc
        );

        self.cache_set(cache_key, description.clone());
        description
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    pub fn stats(&self) -> PipelineStats {
        PipelineStats {
            active_tasks: self.active_tasks.len(),
            completed_tasks: self.completed_tasks.len(),
            loaded_models: self.model_manager.active_sessions(),
            memory_usage_mb: self.model_manager.total_memory_usage(),
            registered_tools: self.tool_adapter.tool_count(),
            cache_entries: self.cache.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub active_tasks: usize,
    pub completed_tasks: usize,
    pub loaded_models: usize,
    pub memory_usage_mb: f64,
    pub registered_tools: usize,
    pub cache_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_and_process() {
        let mut pipeline = AiPipeline::new(4096.0);
        let id = pipeline.submit_task(TaskType::TextGeneration, TaskPriority::High);
        assert!(id > 0);
        let results = pipeline.process_queue();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_stats() {
        let mut pipeline = AiPipeline::new(4096.0);
        pipeline.submit_task(TaskType::NpcDialogue, TaskPriority::Normal);
        let stats = pipeline.stats();
        assert_eq!(stats.active_tasks, 1);
        assert_eq!(stats.registered_tools, 6);
    }

    #[test]
    fn test_priority_ordering() {
        let mut pipeline = AiPipeline::new(4096.0);
        pipeline.submit_task(TaskType::TextGeneration, TaskPriority::Low);
        pipeline.submit_task(TaskType::TextGeneration, TaskPriority::Critical);
        pipeline.submit_task(TaskType::TextGeneration, TaskPriority::Normal);
        assert_eq!(pipeline.active_tasks[0].priority, TaskPriority::Critical);
        assert_eq!(pipeline.active_tasks[1].priority, TaskPriority::Normal);
        assert_eq!(pipeline.active_tasks[2].priority, TaskPriority::Low);
    }

    #[test]
    fn test_generate_npc_dialogue() {
        let mut pipeline = AiPipeline::new(4096.0);
        let result =
            pipeline.generate_npc_dialogue("废土矿场", "沉默寡言的铁匠", "这把刀能修好吗？");
        assert!(!result.npc_response.is_empty());
        assert!(!result.emotion.is_empty());
        assert!(result.tokens_used > 0);
    }

    #[test]
    fn test_generate_quest_easy() {
        let mut pipeline = AiPipeline::new(4096.0);
        let quest = pipeline.generate_quest("物资短缺", "easy", "东部废墟");
        assert!(!quest.title.is_empty());
        assert!(!quest.objectives.is_empty());
        assert!(!quest.rewards.is_empty());
    }

    #[test]
    fn test_generate_quest_hard() {
        let mut pipeline = AiPipeline::new(4096.0);
        let quest = pipeline.generate_quest("变异生物入侵", "hard", "废弃军事基地");
        assert!(!quest.prerequisites.is_empty());
        assert!(quest.objectives.len() >= 2);
    }

    #[test]
    fn test_generate_item_description() {
        let mut pipeline = AiPipeline::new(4096.0);
        let desc = pipeline.generate_item_description("钛合金, 碳纤维", "手工锻造");
        assert!(desc.contains("钛合金"));
        assert!(desc.contains("手工锻造"));
    }

    #[test]
    fn test_generate_world_lore() {
        let mut pipeline = AiPipeline::new(4096.0);
        let lore = pipeline.generate_world_lore(
            42,
            150,
            &["大崩塌".into(), "第一次资源战争".into(), "重建协议启动".into()],
        );
        assert!(lore.contains("编年史"));
        assert!(lore.contains("大崩塌"));
    }

    #[test]
    fn test_generate_environment_description() {
        let mut pipeline = AiPipeline::new(4096.0);
        let desc = pipeline.generate_environment_description("rain", "dusk", "ruins");
        assert!(!desc.is_empty());
        assert!(desc.contains("酸雨") || desc.contains("黄昏"));
    }

    #[test]
    fn test_cache() {
        let mut pipeline = AiPipeline::with_config(
            4096.0,
            PipelineConfig { cache_enabled: true, ..Default::default() },
        );
        assert_eq!(pipeline.cache_size(), 0);
        pipeline.generate_npc_dialogue("test", "test", "test");
        assert!(pipeline.cache_size() > 0);
        pipeline.clear_cache();
        assert_eq!(pipeline.cache_size(), 0);
    }

    #[test]
    fn test_cache_disabled() {
        let mut pipeline = AiPipeline::with_config(
            4096.0,
            PipelineConfig { cache_enabled: false, ..Default::default() },
        );
        pipeline.generate_npc_dialogue("test", "test", "test");
        assert_eq!(pipeline.cache_size(), 0);
    }

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.model_name, "qwen2.5-1.5b");
        assert!(config.cache_enabled);
        assert_eq!(config.temperature, 0.7);
    }

    #[test]
    fn test_all_generation_methods() {
        let mut pipeline = AiPipeline::new(4096.0);

        let dialogue = pipeline.generate_npc_dialogue("营地", "老兵", "有什么任务吗？");
        assert!(!dialogue.npc_response.is_empty());

        let quest = pipeline.generate_quest("资源短缺", "normal", "南部哨站");
        assert!(!quest.title.is_empty());

        let item = pipeline.generate_item_description("钢铁, 皮革", "机器压制");
        assert!(!item.is_empty());

        let lore = pipeline.generate_world_lore(99, 200, &["核战争".into()]);
        assert!(!lore.is_empty());

        let env = pipeline.generate_environment_description("storm", "night", "desert");
        assert!(!env.is_empty());
    }
}
