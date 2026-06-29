use glam::Vec3;
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

slotmap::new_key_type! { pub struct InteractionId; }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InteractionCategory {
    PlayerAction,
    NpcAction,
    SystemEvent,
    Environmental,
    Combat,
    Crafting,
    Dialogue,
    Building,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InteractionPriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub id: InteractionId,
    pub category: InteractionCategory,
    pub priority: InteractionPriority,
    pub position: Vec3,
    pub radius: f32,
    pub source_entity: u64,
    pub target_entity: Option<u64>,
    pub interaction_type: String,
    pub payload: InteractionPayload,
    pub timestamp: f32,
    pub processed: bool,
    pub lod_level: u8,
}

/// 通用交互载荷
///
/// 引擎层只提供通用的载荷容器，具体游戏行为（伤害/治疗/拾取等）
/// 由游戏层通过 `Custom` 变体传递，避免引擎层耦合游戏逻辑。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionPayload {
    /// 通用移动指令
    Move { target: Vec3 },
    /// 通用自定义载荷（游戏层负责解析）
    Custom { kind: String, data: Vec<u8> },
}

impl InteractionPayload {
    /// 创建自定义载荷的便捷方法
    pub fn custom(kind: &str, data: Vec<u8>) -> Self {
        Self::Custom { kind: kind.to_string(), data }
    }
}

pub struct InteractionSystem {
    pub interactions: SlotMap<InteractionId, Interaction>,
    pub max_per_frame: usize,
    pub player_position: Vec3,
    pub lod_distances: [f32; 4],
    pub category_filters: Vec<InteractionCategory>,
    pub stats: InteractionStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InteractionStats {
    pub total_this_frame: usize,
    pub processed_this_frame: usize,
    pub filtered_out: usize,
    pub by_category: [usize; 8],
}

impl InteractionSystem {
    pub fn new(max_per_frame: usize) -> Self {
        InteractionSystem {
            interactions: SlotMap::with_key(),
            max_per_frame,
            player_position: Vec3::ZERO,
            lod_distances: [20.0, 80.0, 200.0, 500.0],
            category_filters: Vec::new(),
            stats: InteractionStats::default(),
        }
    }

    pub fn submit(&mut self, mut interaction: Interaction) -> InteractionId {
        let dist = (interaction.position - self.player_position).length();
        interaction.lod_level = self.calculate_lod(dist);
        self.interactions.insert(interaction)
    }

    fn calculate_lod(&self, dist: f32) -> u8 {
        for (i, &d) in self.lod_distances.iter().enumerate() {
            if dist < d {
                return i as u8;
            }
        }
        3
    }

    pub fn step(&mut self, _dt: f32, player_pos: Vec3) {
        self.player_position = player_pos;
        self.stats = InteractionStats::default();

        let mut to_process: Vec<InteractionId> = Vec::new();
        let mut to_remove: Vec<InteractionId> = Vec::new();

        for (id, interaction) in &self.interactions {
            if interaction.processed {
                to_remove.push(id);
                continue;
            }

            let cat_idx = match interaction.category {
                InteractionCategory::PlayerAction => 0,
                InteractionCategory::NpcAction => 1,
                InteractionCategory::SystemEvent => 2,
                InteractionCategory::Environmental => 3,
                InteractionCategory::Combat => 4,
                InteractionCategory::Crafting => 5,
                InteractionCategory::Dialogue => 6,
                InteractionCategory::Building => 7,
            };
            self.stats.by_category[cat_idx] += 1;
            self.stats.total_this_frame += 1;

            if self.category_filters.contains(&interaction.category) {
                self.stats.filtered_out += 1;
                to_remove.push(id);
                continue;
            }

            let dist = (interaction.position - player_pos).length();
            let lod = self.calculate_lod(dist);

            if interaction.priority == InteractionPriority::Background && lod >= 2 {
                self.stats.filtered_out += 1;
                continue;
            }

            if interaction.priority == InteractionPriority::Low && lod >= 3 {
                self.stats.filtered_out += 1;
                continue;
            }

            if interaction.category == InteractionCategory::Environmental && lod >= 2 {
                self.stats.filtered_out += 1;
                continue;
            }

            to_process.push(id);

            if to_process.len() >= self.max_per_frame {
                break;
            }
        }

        for id in &to_process {
            if let Some(interaction) = self.interactions.get_mut(*id) {
                interaction.processed = true;
                self.stats.processed_this_frame += 1;
            }
        }

        for id in to_remove {
            self.interactions.remove(id);
        }
    }

    pub fn filter_category(&mut self, category: InteractionCategory) {
        if !self.category_filters.contains(&category) {
            self.category_filters.push(category);
        }
    }

    pub fn unfilter_category(&mut self, category: InteractionCategory) {
        self.category_filters.retain(|&c| c != category);
    }

    pub fn pending_count(&self) -> usize {
        self.interactions.values().filter(|i| !i.processed).count()
    }

    pub fn get_processed(&self) -> Vec<&Interaction> {
        self.interactions.values().filter(|i| i.processed).collect()
    }
}
