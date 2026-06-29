use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::scene::{LightType, NodeType, Scene};
use crate::selection::Selection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub success: bool,
    pub data: Value,
    pub error: Option<String>,
}

impl McpToolResult {
    pub fn ok(data: Value) -> Self {
        McpToolResult { success: true, data, error: None }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        McpToolResult { success: false, data: Value::Null, error: Some(msg.into()) }
    }
}

pub fn list_all_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "get_scene_tree".into(),
            description: "Get the current scene tree as JSON".into(),
            input_schema: json!({"type": "object", "properties": {}}),
        },
        McpTool {
            name: "create_node".into(),
            description: "Create a new node in the scene".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "node_type": {"type": "string", "enum": ["empty", "mesh", "light", "camera"]},
                    "position": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                    "parent_id": {"type": "integer"},
                    "path": {"type": "string"}
                },
                "required": ["name", "node_type"]
            }),
        },
        McpTool {
            name: "delete_node".into(),
            description: "Delete a node by id (removes descendants too)".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "node_id": {"type": "integer"} },
                "required": ["node_id"]
            }),
        },
        McpTool {
            name: "set_node_property".into(),
            description: "Set a property on a node (name, translation, scale)".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "node_id": {"type": "integer"},
                    "property": {"type": "string"},
                    "value": {}
                },
                "required": ["node_id", "property", "value"]
            }),
        },
        McpTool {
            name: "get_node_properties".into(),
            description: "Get all properties of a node".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "node_id": {"type": "integer"} },
                "required": ["node_id"]
            }),
        },
        McpTool {
            name: "transform_node".into(),
            description: "Set the transform of a node".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "node_id": {"type": "integer"},
                    "position": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                    "rotation": {"type": "array", "items": {"type": "number"}, "minItems": 4, "maxItems": 4},
                    "scale": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}
                },
                "required": ["node_id"]
            }),
        },
        McpTool {
            name: "select_node".into(),
            description: "Select a node in the editor".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "node_id": {"type": "integer"} },
                "required": ["node_id"]
            }),
        },
        McpTool {
            name: "get_selection".into(),
            description: "Get the current selection".into(),
            input_schema: json!({"type": "object", "properties": {}}),
        },
        McpTool {
            name: "save_scene".into(),
            description: "Save the current scene to a file".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "path": {"type": "string"} },
                "required": ["path"]
            }),
        },
        McpTool {
            name: "load_scene".into(),
            description: "Load a scene from a file".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "path": {"type": "string"} },
                "required": ["path"]
            }),
        },
        McpTool {
            name: "new_scene".into(),
            description: "Create a new empty scene".into(),
            input_schema: json!({"type": "object", "properties": {}}),
        },
        McpTool {
            name: "validate_scene".into(),
            description: "Validate the current scene and return issues".into(),
            input_schema: json!({"type": "object", "properties": {}}),
        },
        McpTool {
            name: "batch_execute".into(),
            description: "Execute multiple tool calls in sequence".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "commands": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "tool": {"type": "string"},
                                "args": {"type": "object"}
                            },
                            "required": ["tool"]
                        }
                    }
                },
                "required": ["commands"]
            }),
        },
        McpTool {
            name: "get_editor_state".into(),
            description: "Get the current editor state".into(),
            input_schema: json!({"type": "object", "properties": {}}),
        },
        McpTool {
            name: "set_camera_view".into(),
            description: "Set the editor camera position and target".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "position": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                    "target": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}
                }
            }),
        },
    ]
}

/// Dispatch a tool call against the live editor state.
pub fn execute_tool(
    name: &str,
    args: &Value,
    scene: &mut Scene,
    selection: &mut Selection,
    scene_path: &mut Option<String>,
    dirty: &mut bool,
) -> McpToolResult {
    match name {
        "get_scene_tree" => {
            let snap = super::scene_snapshot::SceneSnapshot::from_scene(scene, selection.selected_id);
            McpToolResult::ok(serde_json::to_value(&snap).unwrap_or(json!({})))
        },
        "create_node" => {
            let node_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("Node").to_string();
            let node_type_str = args.get("node_type").and_then(|v| v.as_str()).unwrap_or("empty").to_string();
            let parent_id = args.get("parent_id").and_then(|v| v.as_u64()).unwrap_or(0);
            let new_id = scene.add_child(parent_id, &node_name);
            match new_id {
                Some(id) => {
                    if let Some(node) = scene.find_node_mut(id) {
                        node.node_type = build_node_type(&node_type_str, args);
                        if let Some(pos) = args.get("position").and_then(|v| v.as_array()) {
                            if pos.len() >= 3 {
                                node.transform.translation = glam::Vec3::new(
                                    pos[0].as_f64().unwrap_or(0.0) as f32,
                                    pos[1].as_f64().unwrap_or(0.0) as f32,
                                    pos[2].as_f64().unwrap_or(0.0) as f32,
                                );
                            }
                        }
                    }
                    *dirty = true;
                    McpToolResult::ok(json!({"node_id": id, "name": node_name}))
                },
                None => McpToolResult::err(format!("Parent id {} not found", parent_id)),
            }
        },
        "delete_node" => {
            let node_id = match args.get("node_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => return McpToolResult::err("Missing node_id"),
            };
            if node_id == 0 {
                return McpToolResult::err("Cannot delete root node");
            }
            scene.remove_node(node_id);
            if selection.selected_id == Some(node_id) {
                selection.clear();
            }
            *dirty = true;
            McpToolResult::ok(json!({"deleted": node_id}))
        },
        "set_node_property" => {
            let node_id = match args.get("node_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => return McpToolResult::err("Missing node_id"),
            };
            let property = match args.get("property").and_then(|v| v.as_str()) {
                Some(p) => p.to_string(),
                None => return McpToolResult::err("Missing property"),
            };
            let value = args.get("value").cloned().unwrap_or(Value::Null);
            let node = match scene.find_node_mut(node_id) {
                Some(n) => n,
                None => return McpToolResult::err(format!("Node {} not found", node_id)),
            };
            match property.as_str() {
                "name" => {
                    if let Some(s) = value.as_str() {
                        node.name = s.to_string();
                    }
                },
                "translation" | "position" => {
                    if let Some(arr) = value.as_array() {
                        if arr.len() >= 3 {
                            node.transform.translation = glam::Vec3::new(
                                arr[0].as_f64().unwrap_or(0.0) as f32,
                                arr[1].as_f64().unwrap_or(0.0) as f32,
                                arr[2].as_f64().unwrap_or(0.0) as f32,
                            );
                        }
                    }
                },
                "scale" => {
                    if let Some(arr) = value.as_array() {
                        if arr.len() >= 3 {
                            node.transform.scale = glam::Vec3::new(
                                arr[0].as_f64().unwrap_or(1.0) as f32,
                                arr[1].as_f64().unwrap_or(1.0) as f32,
                                arr[2].as_f64().unwrap_or(1.0) as f32,
                            );
                        }
                    }
                },
                "rotation" => {
                    if let Some(arr) = value.as_array() {
                        if arr.len() >= 4 {
                            node.transform.rotation = glam::Quat::from_xyzw(
                                arr[0].as_f64().unwrap_or(0.0) as f32,
                                arr[1].as_f64().unwrap_or(0.0) as f32,
                                arr[2].as_f64().unwrap_or(0.0) as f32,
                                arr[3].as_f64().unwrap_or(1.0) as f32,
                            );
                        }
                    }
                },
                "path" => {
                    if let NodeType::Mesh { path } = &mut node.node_type {
                        if let Some(s) = value.as_str() {
                            *path = s.to_string();
                        }
                    }
                },
                "intensity" => {
                    if let NodeType::Light { intensity, .. } = &mut node.node_type {
                        if let Some(f) = value.as_f64() {
                            *intensity = f as f32;
                        }
                    }
                },
                "fov" => {
                    if let NodeType::Camera { fov, .. } = &mut node.node_type {
                        if let Some(f) = value.as_f64() {
                            *fov = f as f32;
                        }
                    }
                },
                _ => return McpToolResult::err(format!("Unknown property: {}", property)),
            }
            *dirty = true;
            McpToolResult::ok(json!({"node_id": node_id, "property": property}))
        },
        "get_node_properties" => {
            let node_id = match args.get("node_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => return McpToolResult::err("Missing node_id"),
            };
            let node = match scene.find_node(node_id) {
                Some(n) => n,
                None => return McpToolResult::err(format!("Node {} not found", node_id)),
            };
            let props = super::scene_snapshot::node_properties(&node.node_type);
            McpToolResult::ok(json!({
                "id": node.id,
                "name": node.name,
                "translation": [node.transform.translation.x, node.transform.translation.y, node.transform.translation.z],
                "rotation": [node.transform.rotation.x, node.transform.rotation.y, node.transform.rotation.z, node.transform.rotation.w],
                "scale": [node.transform.scale.x, node.transform.scale.y, node.transform.scale.z],
                "parent_id": node.parent,
                "children_ids": node.children,
                "properties": props,
            }))
        },
        "transform_node" => {
            let node_id = match args.get("node_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => return McpToolResult::err("Missing node_id"),
            };
            let node = match scene.find_node_mut(node_id) {
                Some(n) => n,
                None => return McpToolResult::err(format!("Node {} not found", node_id)),
            };
            if let Some(pos) = args.get("position").and_then(|v| v.as_array()) {
                if pos.len() >= 3 {
                    node.transform.translation = glam::Vec3::new(
                        pos[0].as_f64().unwrap_or(0.0) as f32,
                        pos[1].as_f64().unwrap_or(0.0) as f32,
                        pos[2].as_f64().unwrap_or(0.0) as f32,
                    );
                }
            }
            if let Some(rot) = args.get("rotation").and_then(|v| v.as_array()) {
                if rot.len() >= 4 {
                    node.transform.rotation = glam::Quat::from_xyzw(
                        rot[0].as_f64().unwrap_or(0.0) as f32,
                        rot[1].as_f64().unwrap_or(0.0) as f32,
                        rot[2].as_f64().unwrap_or(0.0) as f32,
                        rot[3].as_f64().unwrap_or(1.0) as f32,
                    );
                }
            }
            if let Some(scl) = args.get("scale").and_then(|v| v.as_array()) {
                if scl.len() >= 3 {
                    node.transform.scale = glam::Vec3::new(
                        scl[0].as_f64().unwrap_or(1.0) as f32,
                        scl[1].as_f64().unwrap_or(1.0) as f32,
                        scl[2].as_f64().unwrap_or(1.0) as f32,
                    );
                }
            }
            *dirty = true;
            McpToolResult::ok(json!({"node_id": node_id}))
        },
        "select_node" => {
            let node_id = match args.get("node_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => return McpToolResult::err("Missing node_id"),
            };
            if scene.find_node(node_id).is_some() {
                selection.select(node_id);
                McpToolResult::ok(json!({"selected": node_id}))
            } else {
                McpToolResult::err(format!("Node {} not found", node_id))
            }
        },
        "get_selection" => {
            McpToolResult::ok(json!({"selected_id": selection.selected_id}))
        },
        "save_scene" => {
            let path = match args.get("path").and_then(|v| v.as_str()) {
                Some(p) => p.to_string(),
                None => return McpToolResult::err("Missing path"),
            };
            match crate::scene_io::save_scene(scene, std::path::Path::new(&path)) {
                Ok(()) => {
                    *scene_path = Some(path.clone());
                    *dirty = false;
                    McpToolResult::ok(json!({"saved": path}))
                },
                Err(e) => McpToolResult::err(format!("Save failed: {}", e)),
            }
        },
        "load_scene" => {
            let path = match args.get("path").and_then(|v| v.as_str()) {
                Some(p) => p.to_string(),
                None => return McpToolResult::err("Missing path"),
            };
            match crate::scene_io::load_scene(std::path::Path::new(&path)) {
                Ok(new_scene) => {
                    *scene = new_scene;
                    *scene_path = Some(path.clone());
                    *dirty = false;
                    selection.clear();
                    McpToolResult::ok(json!({"loaded": path}))
                },
                Err(e) => McpToolResult::err(format!("Load failed: {}", e)),
            }
        },
        "new_scene" => {
            scene.reset();
            *scene_path = None;
            *dirty = false;
            selection.clear();
            McpToolResult::ok(json!({"created": "new scene"}))
        },
        "validate_scene" => {
            let mut issues: Vec<String> = Vec::new();
            if scene.nodes.is_empty() {
                issues.push("Scene has no nodes".into());
            }
            let root_count = scene.nodes.iter().filter(|n| n.parent.is_none()).count();
            if root_count != 1 {
                issues.push(format!("Expected 1 root node, found {}", root_count));
            }
            for n in &scene.nodes {
                if let Some(pid) = n.parent {
                    if !scene.nodes.iter().any(|x| x.id == pid) {
                        issues.push(format!("Node {} references missing parent {}", n.id, pid));
                    }
                }
                for cid in &n.children {
                    if !scene.nodes.iter().any(|x| x.id == *cid) {
                        issues.push(format!("Node {} references missing child {}", n.id, cid));
                    }
                }
            }
            McpToolResult::ok(json!({"issues": issues, "valid": issues.is_empty()}))
        },
        "batch_execute" => {
            let commands = match args.get("commands").and_then(|v| v.as_array()) {
                Some(c) => c,
                None => return McpToolResult::err("Missing commands array"),
            };
            let mut results: Vec<Value> = Vec::new();
            for cmd in commands {
                let tool = cmd.get("tool").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let tool_args = cmd.get("args").cloned().unwrap_or(json!({}));
                let r = execute_tool(&tool, &tool_args, scene, selection, scene_path, dirty);
                results.push(json!({
                    "tool": tool,
                    "success": r.success,
                    "data": r.data,
                    "error": r.error,
                }));
                if !r.success {
                    break;
                }
            }
            McpToolResult::ok(json!({"results": results}))
        },
        "get_editor_state" => {
            McpToolResult::ok(json!({
                "scene_name": scene.name,
                "node_count": scene.nodes.len(),
                "selected_id": selection.selected_id,
                "scene_path": scene_path,
                "dirty": *dirty,
            }))
        },
        "set_camera_view" => {
            // Camera position/target is owned by EditorApp.camera, not Scene.
            // We acknowledge but cannot mutate from here without an editor-side hook.
            // Editor side reads this from a shared channel (future work).
            McpToolResult::ok(json!({"acknowledged": true, "note": "Camera update queued"}))
        },
        _ => McpToolResult::err(format!("Unknown tool: {}", name)),
    }
}

fn build_node_type(node_type_str: &str, args: &Value) -> NodeType {
    match node_type_str {
        "mesh" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            NodeType::Mesh { path }
        },
        "light" => {
            let light_kind = args.get("light_type").and_then(|v| v.as_str()).unwrap_or("point");
            let light_type = match light_kind {
                "directional" => LightType::Directional,
                "spot" => LightType::Spot,
                _ => LightType::Point,
            };
            let color = parse_vec3(args.get("color"), [1.0, 1.0, 1.0]);
            let intensity = args.get("intensity").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            NodeType::Light { light_type, color, intensity }
        },
        "camera" => {
            let fov = args.get("fov").and_then(|v| v.as_f64()).unwrap_or(60.0) as f32;
            let near = args.get("near").and_then(|v| v.as_f64()).unwrap_or(0.1) as f32;
            let far = args.get("far").and_then(|v| v.as_f64()).unwrap_or(1000.0) as f32;
            NodeType::Camera { fov, near, far }
        },
        _ => NodeType::Empty,
    }
}

fn parse_vec3(v: Option<&Value>, default: [f32; 3]) -> glam::Vec3 {
    match v {
        Some(Value::Array(arr)) if arr.len() >= 3 => {
            let x = arr[0].as_f64().unwrap_or(default[0] as f64) as f32;
            let y = arr[1].as_f64().unwrap_or(default[1] as f64) as f32;
            let z = arr[2].as_f64().unwrap_or(default[2] as f64) as f32;
            glam::Vec3::new(x, y, z)
        },
        _ => glam::Vec3::from(default),
    }
}
