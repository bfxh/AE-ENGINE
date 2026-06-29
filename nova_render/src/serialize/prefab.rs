//! Prefab（借鉴 Fyrox）
//!
//! 设计：
//! - 属性继承：子 Prefab 继承父 Prefab 的属性
//! - 层级嵌套：Prefab 可嵌套
//! - UUID 系统：每个 Prefab 有唯一 UUID

use uuid::Uuid;
use hashbrown::HashMap;

/// Prefab Node
#[derive(Debug, Clone)]
pub struct PrefabNode {
    pub uuid: Uuid,
    pub name: String,
    pub parent: Option<Uuid>,
    pub children: Vec<Uuid>,
    pub properties: HashMap<String, String>,
}

/// Prefab
#[derive(Debug, Clone)]
pub struct Prefab {
    pub uuid: Uuid,
    pub name: String,
    pub root: Uuid,
    pub nodes: HashMap<Uuid, PrefabNode>,
}

/// Prefab Registry
pub struct PrefabRegistry {
    pub prefabs: HashMap<Uuid, Prefab>,
}

impl PrefabRegistry {
    pub fn new() -> Self {
        Self { prefabs: HashMap::new() }
    }
    pub fn register(&mut self, prefab: Prefab) {
        self.prefabs.insert(prefab.uuid, prefab);
    }
    pub fn get(&self, uuid: &Uuid) -> Option<&Prefab> {
        self.prefabs.get(uuid)
    }
}

impl Default for PrefabRegistry {
    fn default() -> Self { Self::new() }
}