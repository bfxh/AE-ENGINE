use serde::{Deserialize, Serialize};

use crate::scene::{NodeType, Scene};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSnapshot {
    pub scene_name: String,
    pub node_count: usize,
    pub nodes: Vec<NodeSnapshot>,
    pub selected_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSnapshot {
    pub id: u64,
    pub name: String,
    pub node_type: String,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub parent_id: Option<u64>,
    pub children_ids: Vec<u64>,
    pub properties: serde_json::Value,
}

impl SceneSnapshot {
    pub fn from_scene(scene: &Scene, selected_id: Option<u64>) -> Self {
        let nodes: Vec<NodeSnapshot> = scene
            .nodes
            .iter()
            .map(|n| NodeSnapshot {
                id: n.id,
                name: n.name.clone(),
                node_type: node_type_string(&n.node_type),
                translation: [
                    n.transform.translation.x,
                    n.transform.translation.y,
                    n.transform.translation.z,
                ],
                rotation: [
                    n.transform.rotation.x,
                    n.transform.rotation.y,
                    n.transform.rotation.z,
                    n.transform.rotation.w,
                ],
                scale: [n.transform.scale.x, n.transform.scale.y, n.transform.scale.z],
                parent_id: n.parent,
                children_ids: n.children.clone(),
                properties: node_properties(&n.node_type),
            })
            .collect();

        SceneSnapshot {
            scene_name: scene.name.clone(),
            node_count: scene.nodes.len(),
            nodes,
            selected_id,
        }
    }
}

pub fn node_type_string(nt: &NodeType) -> String {
    match nt {
        NodeType::Empty => "empty".into(),
        NodeType::Mesh { path } => format!("mesh:{}", path),
        NodeType::Light { light_type, .. } => format!("light:{:?}", light_type),
        NodeType::Camera { .. } => "camera".into(),
    }
}

pub fn node_properties(nt: &NodeType) -> serde_json::Value {
    match nt {
        NodeType::Empty => serde_json::json!({}),
        NodeType::Mesh { path } => serde_json::json!({"path": path}),
        NodeType::Light { light_type, color, intensity } => serde_json::json!({
            "light_type": format!("{:?}", light_type),
            "color": [color.x, color.y, color.z],
            "intensity": intensity
        }),
        NodeType::Camera { fov, near, far } => serde_json::json!({
            "fov": fov,
            "near": near,
            "far": far
        }),
    }
}
