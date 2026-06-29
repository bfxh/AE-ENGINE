use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

slotmap::new_key_type! { pub struct BuildObjectId; }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BuildTool {
    Place,
    Remove,
    Paint,
    Measure,
    Terrain,
    Vegetation,
    Water,
    Road,
    Wall,
    Structure,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BuildCategory {
    Terrain,
    Natural,
    Structure,
    Decoration,
    Utility,
    Combat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildItem {
    pub id: u32,
    pub name: String,
    pub category: BuildCategory,
    pub mesh_id: u32,
    pub material_id: u32,
    pub size: Vec3,
    pub snap: bool,
    pub snap_size: f32,
    pub cost: u32,
    pub requires_foundation: bool,
    pub max_per_area: u32,
    pub placement_rules: PlacementRules,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlacementRules {
    pub on_terrain: bool,
    pub on_water: bool,
    pub on_structure: bool,
    pub min_distance_to_enemy: f32,
    pub requires_line_of_sight: bool,
    pub max_slope_angle: f32,
    pub min_height: f32,
    pub max_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildObject {
    pub id: BuildObjectId,
    pub item_id: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: f32,
    pub health: f32,
    pub max_health: f32,
    pub owner_faction: u8,
    pub is_preview: bool,
    pub placement_valid: bool,
    pub built_timestamp: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainModification {
    pub position: Vec3,
    pub radius: f32,
    pub modification_type: TerrainModType,
    pub amount: f32,
    pub applied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TerrainModType {
    Raise,
    Lower,
    Smooth,
    Flatten,
    Paint { layer: u8 },
}

// NOTE: BuildingEditor (原 92-419 行) 已于 2026-06-27 删除。
// 原因：孤儿模块，全仓零实例化零调用（仅定义/导出链内命中）。
// 备份：D:\AI\storage\CC\2_Old\building_mod_rs_20260627_editor.rs
// 保留：BuildItem / BuildObject / BuildTool / BuildCategory / PlacementRules /
//       TerrainModification / BuildObjectId 等数据结构（类型定义，非孤儿）。
// 未来建筑系统接入主循环时，可在 game_logic.rs 中实现新的 BuildingManager。
