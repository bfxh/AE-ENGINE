use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub name: String,
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub variables: Vec<String>,
    pub category: PromptCategory,
    pub max_tokens: usize,
    pub temperature: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PromptCategory {
    NpcDialogue,
    NpcStory,
    WorldDescription,
    QuestGeneration,
    ItemDescription,
    CombatNarration,
    EnvironmentalNarration,
    LoreGeneration,
    Custom(u32),
}

pub struct PromptLibrary {
    pub templates: HashMap<String, PromptTemplate>,
    pub default_category_params: HashMap<PromptCategory, (usize, f32)>,
}

impl Default for PromptLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptLibrary {
    pub fn new() -> Self {
        let mut lib =
            PromptLibrary { templates: HashMap::new(), default_category_params: HashMap::new() };
        lib.default_category_params.insert(PromptCategory::NpcDialogue, (256, 0.8));
        lib.default_category_params.insert(PromptCategory::NpcStory, (512, 0.7));
        lib.default_category_params.insert(PromptCategory::WorldDescription, (384, 0.6));
        lib.default_category_params.insert(PromptCategory::QuestGeneration, (512, 0.7));
        lib.default_category_params.insert(PromptCategory::ItemDescription, (256, 0.7));
        lib.default_category_params.insert(PromptCategory::CombatNarration, (128, 0.9));
        lib.default_category_params.insert(PromptCategory::EnvironmentalNarration, (256, 0.7));
        lib.default_category_params.insert(PromptCategory::LoreGeneration, (512, 0.6));
        lib.register_builtin_templates();
        lib
    }

    fn register_builtin_templates(&mut self) {
        self.register(PromptTemplate {
            name: "npc_dialogue_default".to_string(),
            system_prompt: "你是一个生活在废土世界的NPC。你的对话必须符合废土世界的设定：没有魔法、没有超自然力量，一切基于物理现实。你有自己的记忆、情感和人格。请用中文回复。".to_string(),
            user_prompt_template: "你是{name}，一个{role}。你的性格特点：{personality}。当前心情：{mood}。你已知的信息：{context}\n\n玩家{player_name}对你说：{player_message}\n\n请以{name}的身份回复。".to_string(),
            variables: vec!["name", "role", "personality", "mood", "context", "player_name", "player_message"]
                .into_iter().map(|s| s.to_string()).collect(),
            category: PromptCategory::NpcDialogue,
            max_tokens: 256,
            temperature: 0.8,
        });

        self.register(PromptTemplate {
            name: "npc_combat_taunt".to_string(),
            system_prompt: "你是一个正在战斗中的废土NPC。你的对话简短有力，充满战斗气息。不要使用魔法或超自然术语。用中文回复。".to_string(),
            user_prompt_template: "你是{name}，正在与{target_name}战斗。你的武器：{weapon}。当前生命值：{hp_percent}%。战斗风格：{combat_style}。\n\n请发出一句战斗台词。".to_string(),
            variables: vec!["name", "target_name", "weapon", "hp_percent", "combat_style"]
                .into_iter().map(|s| s.to_string()).collect(),
            category: PromptCategory::CombatNarration,
            max_tokens: 64,
            temperature: 0.9,
        });

        self.register(PromptTemplate {
            name: "world_environment_description".to_string(),
            system_prompt: "你是一个废土世界的环境描述生成器。描述必须基于物理现实，包含具体的光照、风化、腐蚀、植被等科学细节。用中文回复。".to_string(),
            user_prompt_template: "描述一个废土环境：\n地形：{terrain}\n气候：{climate}\n时间：{time_of_day}\n季节：{season}\n已废弃时间：{abandoned_years}年\n周围建筑：{buildings}\n特殊事件：{events}\n\n请生成一段环境描述。".to_string(),
            variables: vec!["terrain", "climate", "time_of_day", "season", "abandoned_years", "buildings", "events"]
                .into_iter().map(|s| s.to_string()).collect(),
            category: PromptCategory::EnvironmentalNarration,
            max_tokens: 256,
            temperature: 0.7,
        });

        self.register(PromptTemplate {
            name: "item_description".to_string(),
            system_prompt: "你是一个废土物品描述生成器。描述必须基于物品的物理属性：材质、重量、硬度、结构等。所有描述基于科学现实，不涉及魔法。用中文回复。".to_string(),
            user_prompt_template: "生成物品描述：\n名称：{item_name}\n材质：{material}\n重量：{weight}kg\n硬度：{hardness} HV\n来源：{origin}\n年代：{era}\n状态：{condition}\n\n请生成物品描述。".to_string(),
            variables: vec!["item_name", "material", "weight", "hardness", "origin", "era", "condition"]
                .into_iter().map(|s| s.to_string()).collect(),
            category: PromptCategory::ItemDescription,
            max_tokens: 128,
            temperature: 0.7,
        });

        self.register(PromptTemplate {
            name: "quest_generation".to_string(),
            system_prompt: "你是一个废土世界任务生成器。任务必须基于物理现实，不涉及魔法或超自然。任务目标、奖励和难度必须合理。用中文回复。".to_string(),
            user_prompt_template: "生成一个任务：\n发布者：{quest_giver}\n发布者角色：{giver_role}\n难度：{difficulty}\n类型：{quest_type}\n地理位置：{location}\n已知信息：{context}\n\n请生成任务描述、目标和奖励。".to_string(),
            variables: vec!["quest_giver", "giver_role", "difficulty", "quest_type", "location", "context"]
                .into_iter().map(|s| s.to_string()).collect(),
            category: PromptCategory::QuestGeneration,
            max_tokens: 512,
            temperature: 0.7,
        });

        self.register(PromptTemplate {
            name: "lore_generation".to_string(),
            system_prompt: "你是一个废土世界设定生成器。所有设定必须基于科学和物理现实。废土世界没有魔法，一切基于技术、生态和人类行为。用中文回复。".to_string(),
            user_prompt_template: "生成废土世界设定：\n主题：{topic}\n地区：{region}\n时间线：{timeline}\n技术水平：{tech_level}\n关键词：{keywords}\n\n请生成一段世界设定。".to_string(),
            variables: vec!["topic", "region", "timeline", "tech_level", "keywords"]
                .into_iter().map(|s| s.to_string()).collect(),
            category: PromptCategory::LoreGeneration,
            max_tokens: 512,
            temperature: 0.6,
        });
    }

    pub fn register(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    pub fn get_by_category(&self, category: PromptCategory) -> Vec<&PromptTemplate> {
        self.templates.values().filter(|t| t.category == category).collect()
    }

    pub fn render(
        &self,
        template_name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<RenderResult, String> {
        let template = self
            .templates
            .get(template_name)
            .ok_or_else(|| format!("Template not found: {}", template_name))?;

        let mut user_prompt = template.user_prompt_template.clone();
        let mut system_prompt = template.system_prompt.clone();

        for (key, value) in variables {
            let placeholder = format!("{{{}}}", key);
            user_prompt = user_prompt.replace(&placeholder, value);
            system_prompt = system_prompt.replace(&placeholder, value);
        }

        let missing: Vec<String> =
            template.variables.iter().filter(|v| !variables.contains_key(*v)).cloned().collect();

        Ok(RenderResult {
            system_prompt,
            user_prompt,
            missing_variables: missing,
            max_tokens: template.max_tokens,
            temperature: template.temperature,
        })
    }

    pub fn list_templates(&self) -> Vec<&PromptTemplate> {
        self.templates.values().collect()
    }
}

#[derive(Debug, Clone)]
pub struct RenderResult {
    pub system_prompt: String,
    pub user_prompt: String,
    pub missing_variables: Vec<String>,
    pub max_tokens: usize,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcPromptBuilder {
    pub npc_name: String,
    pub npc_role: String,
    pub personality: String,
    pub mood: String,
    pub knowledge_context: String,
    pub player_name: String,
    pub player_message: String,
}

impl NpcPromptBuilder {
    pub fn build(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), self.npc_name.clone());
        vars.insert("role".to_string(), self.npc_role.clone());
        vars.insert("personality".to_string(), self.personality.clone());
        vars.insert("mood".to_string(), self.mood.clone());
        vars.insert("context".to_string(), self.knowledge_context.clone());
        vars.insert("player_name".to_string(), self.player_name.clone());
        vars.insert("player_message".to_string(), self.player_message.clone());
        vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_templates() {
        let lib = PromptLibrary::new();
        assert!(lib.templates.len() >= 6);
        assert!(lib.get("npc_dialogue_default").is_some());
        assert!(lib.get("npc_combat_taunt").is_some());
        assert!(lib.get("world_environment_description").is_some());
    }

    #[test]
    fn test_render_template() {
        let lib = PromptLibrary::new();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "老王".to_string());
        vars.insert("role".to_string(), "铁匠".to_string());
        vars.insert("personality".to_string(), "沉默寡言".to_string());
        vars.insert("mood".to_string(), "疲惫".to_string());
        vars.insert("context".to_string(), "今天矿场出了事故".to_string());
        vars.insert("player_name".to_string(), "玩家".to_string());
        vars.insert("player_message".to_string(), "你好".to_string());

        let result = lib.render("npc_dialogue_default", &vars).unwrap();
        assert!(result.user_prompt.contains("老王"));
        assert!(result.user_prompt.contains("铁匠"));
        assert!(result.user_prompt.contains("沉默寡言"));
        assert!(result.missing_variables.is_empty());
    }

    #[test]
    fn test_missing_variables() {
        let lib = PromptLibrary::new();
        let vars = HashMap::new();
        let result = lib.render("npc_dialogue_default", &vars).unwrap();
        assert!(!result.missing_variables.is_empty());
    }

    #[test]
    fn test_category_filter() {
        let lib = PromptLibrary::new();
        let combat = lib.get_by_category(PromptCategory::CombatNarration);
        assert!(!combat.is_empty());
        let dialogue = lib.get_by_category(PromptCategory::NpcDialogue);
        assert!(!dialogue.is_empty());
    }
}
