use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub inputs: Vec<(String, u32)>,
    pub tools: Vec<String>,
    pub skills: Vec<(String, u8)>,
    pub workstation: Option<String>,
    pub time: f32,
    pub output: CraftOutput,
    pub category: CraftCategory,
    pub discovered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftOutput {
    pub item_type: String,
    pub quantity: u32,
    pub quality_range: (f32, f32),
    pub byproducts: Vec<(String, f32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CraftCategory {
    Weapon,
    Armor,
    Tool,
    Consumable,
    Building,
    Vehicle,
    Electronics,
    Chemical,
    Biological,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftResult {
    pub recipe_id: Uuid,
    pub success: bool,
    pub quality: f32,
    pub output: Option<CraftOutput>,
    pub byproducts: Vec<(String, f32)>,
    pub experience_gained: Vec<(String, f32)>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feasibility {
    pub possible: bool,
    pub missing_items: Vec<(String, u32)>,
    pub missing_tools: Vec<String>,
    pub missing_skills: Vec<(String, u8, u8)>,
    pub missing_workstation: Option<String>,
    pub estimated_time: f32,
    pub estimated_quality: (f32, f32),
}

impl Recipe {
    pub fn new(name: &str, category: CraftCategory) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: String::new(),
            inputs: Vec::new(),
            tools: Vec::new(),
            skills: Vec::new(),
            workstation: None,
            time: 1.0,
            output: CraftOutput {
                item_type: name.to_string(),
                quantity: 1,
                quality_range: (0.5, 1.0),
                byproducts: Vec::new(),
            },
            category,
            discovered: false,
        }
    }

    pub fn check_feasibility(
        &self,
        inventory: &[(String, u32)],
        available_tools: &[String],
        skill_levels: &[(String, u8)],
        has_workstation: Option<&str>,
    ) -> Feasibility {
        let mut missing_items = Vec::new();
        let mut missing_tools = Vec::new();
        let mut missing_skills = Vec::new();
        let mut missing_workstation = None;

        for (item, required) in &self.inputs {
            let available =
                inventory.iter().find(|(i, _)| i == item).map(|(_, qty)| *qty).unwrap_or(0);
            if available < *required {
                missing_items.push((item.clone(), *required - available));
            }
        }

        for tool in &self.tools {
            if !available_tools.contains(tool) {
                missing_tools.push(tool.clone());
            }
        }

        for (skill, required_level) in &self.skills {
            let current =
                skill_levels.iter().find(|(s, _)| s == skill).map(|(_, l)| *l).unwrap_or(0);
            if current < *required_level {
                missing_skills.push((skill.clone(), *required_level, current));
            }
        }

        if let Some(ref ws) = self.workstation {
            if has_workstation != Some(ws.as_str()) {
                missing_workstation = Some(ws.clone());
            }
        }

        let possible = missing_items.is_empty()
            && missing_tools.is_empty()
            && missing_skills.is_empty()
            && missing_workstation.is_none();

        Feasibility {
            possible,
            missing_items,
            missing_tools,
            missing_skills,
            missing_workstation,
            estimated_time: self.time,
            estimated_quality: self.output.quality_range,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDatabase {
    pub recipes: Vec<Recipe>,
    pub categories: Vec<CraftCategory>,
}

impl RecipeDatabase {
    pub fn new() -> Self {
        Self {
            recipes: Vec::new(),
            categories: vec![
                CraftCategory::Weapon,
                CraftCategory::Armor,
                CraftCategory::Tool,
                CraftCategory::Consumable,
                CraftCategory::Building,
            ],
        }
    }

    pub fn add_recipe(&mut self, recipe: Recipe) {
        self.recipes.push(recipe);
    }

    pub fn find_by_category(&self, category: CraftCategory) -> Vec<&Recipe> {
        self.recipes.iter().filter(|r| r.category == category).collect()
    }

    pub fn find_feasible(
        &self,
        inventory: &[(String, u32)],
        tools: &[String],
        skills: &[(String, u8)],
        workstation: Option<&str>,
    ) -> Vec<(&Recipe, Feasibility)> {
        self.recipes
            .iter()
            .map(|r| (r, r.check_feasibility(inventory, tools, skills, workstation)))
            .filter(|(_, f)| f.possible)
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&Recipe> {
        let query = query.to_lowercase();
        self.recipes
            .iter()
            .filter(|r| {
                r.name.to_lowercase().contains(&query)
                    || r.description.to_lowercase().contains(&query)
            })
            .collect()
    }
}

impl Default for RecipeDatabase {
    fn default() -> Self {
        Self::new()
    }
}
