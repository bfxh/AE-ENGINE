use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub category: ToolCategory,
    pub requires_model: bool,
    pub model_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: ParamType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
    Enum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    ThreeD,
    Texture,
    World,
    Npc,
    Physics,
    Audio,
    Utility,
}

#[derive(Debug, Clone)]
pub struct ToolAdapter {
    tools: HashMap<String, ToolDefinition>,
    enabled: HashMap<String, bool>,
}

impl ToolAdapter {
    pub fn new() -> Self {
        let mut adapter = ToolAdapter { tools: HashMap::new(), enabled: HashMap::new() };
        adapter.register_default_tools();
        adapter
    }

    fn register_default_tools(&mut self) {
        self.register(ToolDefinition {
            name: "generate_3d_model".into(),
            description: "Generate a 3D model from text description".into(),
            parameters: vec![
                ToolParameter {
                    name: "prompt".into(),
                    param_type: ParamType::String,
                    required: true,
                    default: None,
                    description: "Text description of the model".into(),
                },
                ToolParameter {
                    name: "style".into(),
                    param_type: ParamType::Enum,
                    required: false,
                    default: Some(serde_json::json!("realistic")),
                    description: "Generation style: realistic, stylized, low_poly, voxel, cad"
                        .into(),
                },
            ],
            category: ToolCategory::ThreeD,
            requires_model: true,
            model_type: Some("stable_diffusion_3d".into()),
        });
        self.register(ToolDefinition {
            name: "validate_mesh".into(),
            description: "Validate a 3D mesh for issues".into(),
            parameters: vec![ToolParameter {
                name: "mesh_data".into(),
                param_type: ParamType::Object,
                required: true,
                default: None,
                description: "Mesh binary data".into(),
            }],
            category: ToolCategory::ThreeD,
            requires_model: false,
            model_type: None,
        });
        self.register(ToolDefinition {
            name: "generate_lod".into(),
            description: "Generate LOD levels for a mesh".into(),
            parameters: vec![ToolParameter {
                name: "levels".into(),
                param_type: ParamType::Integer,
                required: true,
                default: Some(serde_json::json!(3)),
                description: "Number of LOD levels".into(),
            }],
            category: ToolCategory::ThreeD,
            requires_model: false,
            model_type: None,
        });
        self.register(ToolDefinition {
            name: "npc_dialogue".into(),
            description: "Generate NPC dialogue response".into(),
            parameters: vec![
                ToolParameter {
                    name: "npc_id".into(),
                    param_type: ParamType::Integer,
                    required: true,
                    default: None,
                    description: "NPC unique ID".into(),
                },
                ToolParameter {
                    name: "player_message".into(),
                    param_type: ParamType::String,
                    required: true,
                    default: None,
                    description: "Player's message to the NPC".into(),
                },
                ToolParameter {
                    name: "context".into(),
                    param_type: ParamType::Object,
                    required: false,
                    default: None,
                    description: "World context for the NPC".into(),
                },
            ],
            category: ToolCategory::Npc,
            requires_model: true,
            model_type: Some("qwen".into()),
        });
        self.register(ToolDefinition {
            name: "world_query".into(),
            description: "Query world simulation state".into(),
            parameters: vec![
                ToolParameter {
                    name: "query_type".into(),
                    param_type: ParamType::Enum,
                    required: true,
                    default: None,
                    description: "weather, ecology, geology, population".into(),
                },
                ToolParameter {
                    name: "location".into(),
                    param_type: ParamType::Array,
                    required: false,
                    default: None,
                    description: "[x, y, z] world coordinates".into(),
                },
            ],
            category: ToolCategory::World,
            requires_model: false,
            model_type: None,
        });
        self.register(ToolDefinition {
            name: "physics_simulate".into(),
            description: "Run a physics simulation step".into(),
            parameters: vec![ToolParameter {
                name: "entity_ids".into(),
                param_type: ParamType::Array,
                required: true,
                default: None,
                description: "List of entity IDs to simulate".into(),
            }],
            category: ToolCategory::Physics,
            requires_model: false,
            model_type: None,
        });
    }

    pub fn register(&mut self, tool: ToolDefinition) {
        self.enabled.insert(tool.name.clone(), true);
        self.tools.insert(tool.name.clone(), tool);
    }

    pub fn enable(&mut self, name: &str) {
        self.enabled.insert(name.to_string(), true);
    }

    pub fn disable(&mut self, name: &str) {
        self.enabled.insert(name.to_string(), false);
    }

    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        if self.enabled.get(name).copied().unwrap_or(false) { self.tools.get(name) } else { None }
    }

    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools
            .values()
            .filter(|t| self.enabled.get(&t.name).copied().unwrap_or(false))
            .collect()
    }

    pub fn list_by_category(&self, category: ToolCategory) -> Vec<&ToolDefinition> {
        self.list_tools().into_iter().filter(|t| t.category == category).collect()
    }

    pub fn tool_count(&self) -> usize {
        self.enabled.values().filter(|&&e| e).count()
    }

    pub fn generate_tool_schema_json(&self) -> String {
        let tools: Vec<serde_json::Value> = self
            .list_tools()
            .iter()
            .map(|t| {
                let properties: serde_json::Map<String, serde_json::Value> = t
                    .parameters
                    .iter()
                    .map(|p| {
                        let mut prop = serde_json::Map::new();
                        prop.insert(
                            "type".into(),
                            serde_json::json!(format!("{:?}", p.param_type).to_lowercase()),
                        );
                        prop.insert("description".into(), serde_json::json!(p.description));
                        (p.name.clone(), serde_json::Value::Object(prop))
                    })
                    .collect();
                let required: Vec<String> =
                    t.parameters.iter().filter(|p| p.required).map(|p| p.name.clone()).collect();
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": {
                            "type": "object",
                            "properties": properties,
                            "required": required,
                        }
                    }
                })
            })
            .collect();
        serde_json::to_string_pretty(&tools).unwrap_or_default()
    }
}

impl Default for ToolAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let adapter = ToolAdapter::new();
        let tool = adapter.get_tool("generate_3d_model");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().category, ToolCategory::ThreeD);
    }

    #[test]
    fn test_enable_disable() {
        let mut adapter = ToolAdapter::new();
        assert!(adapter.get_tool("npc_dialogue").is_some());
        adapter.disable("npc_dialogue");
        assert!(adapter.get_tool("npc_dialogue").is_none());
        adapter.enable("npc_dialogue");
        assert!(adapter.get_tool("npc_dialogue").is_some());
    }

    #[test]
    fn test_list_by_category() {
        let adapter = ToolAdapter::new();
        let three_d = adapter.list_by_category(ToolCategory::ThreeD);
        assert!(three_d.len() >= 2);
        let npc = adapter.list_by_category(ToolCategory::Npc);
        assert_eq!(npc.len(), 1);
    }

    #[test]
    fn test_tool_count() {
        let adapter = ToolAdapter::new();
        assert!(adapter.tool_count() >= 6);
    }

    #[test]
    fn test_generate_schema_json() {
        let adapter = ToolAdapter::new();
        let json = adapter.generate_tool_schema_json();
        assert!(json.contains("generate_3d_model"));
        assert!(json.contains("npc_dialogue"));
        assert!(json.contains("function"));
    }
}
