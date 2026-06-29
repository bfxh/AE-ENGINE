//! 战利品系统

use super::item::ItemId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LootEntry {
    pub item_id: ItemId,
    pub weight: f32,
    pub min_count: u32,
    pub max_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootPoint {
    pub id: u64,
    pub loot_table: Vec<LootEntry>,
    pub respawn_time: f32,
    pub cooldown_timer: f32,
    pub seed: u64,
}

impl LootPoint {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            loot_table: Vec::new(),
            respawn_time: 300.0,
            cooldown_timer: 0.0,
            seed: id.wrapping_mul(0x5851F42D4C957F2D),
        }
    }

    pub fn roll_drops(&mut self) -> Vec<(ItemId, u32)> {
        let total_weight: f32 = self.loot_table.iter().map(|e| e.weight).sum();
        if total_weight <= 0.0 {
            return Vec::new();
        }

        let loot_table: Vec<LootEntry> = self.loot_table.to_vec();
        let mut rng = self.seed;
        let mut drops = Vec::new();

        for entry in &loot_table {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let r = (rng >> 33) as f32 / (1u64 << 31) as f32;
            if r < entry.weight / total_weight {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let count_r = (rng >> 33) as f32 / (1u64 << 31) as f32;
                let count = entry.min_count
                    + (count_r * (entry.max_count - entry.min_count + 1) as f32) as u32;
                drops.push((entry.item_id, count.min(entry.max_count)));
            }
        }
        self.seed = rng;
        drops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loot_point() {
        let mut lp = LootPoint::new(1);
        lp.loot_table = vec![
            LootEntry { item_id: ItemId(1), weight: 1.0, min_count: 1, max_count: 3 },
            LootEntry { item_id: ItemId(2), weight: 0.5, min_count: 1, max_count: 1 },
        ];
        let drops = lp.roll_drops();
        assert!(drops.iter().all(|(_, c)| *c >= 1));
    }
}
