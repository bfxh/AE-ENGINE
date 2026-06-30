//! 背包系统

use super::item::ItemId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InventorySlot {
    pub item_id: ItemId,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<InventorySlot>>,
    pub capacity: usize,
    pub equipped: hashbrown::HashMap<String, ItemId>,
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self { slots: vec![None; capacity], capacity, equipped: hashbrown::HashMap::new() }
    }

    pub fn add(&mut self, item_id: ItemId, count: u32, item_db: &ItemDatabase) -> u32 {
        let stack_size = item_db.get(item_id).map(|d| d.stack_size).unwrap_or(1);
        let mut remaining = count;

        for slot in &mut self.slots {
            if remaining == 0 {
                break;
            }
            if let Some(s) = slot {
                if s.item_id == item_id && s.count < stack_size {
                    let add = (stack_size - s.count).min(remaining);
                    s.count += add;
                    remaining -= add;
                }
            }
        }

        for slot in &mut self.slots {
            if remaining == 0 {
                break;
            }
            if slot.is_none() {
                let add = stack_size.min(remaining);
                *slot = Some(InventorySlot { item_id, count: add });
                remaining -= add;
            }
        }

        count - remaining
    }

    pub fn remove(&mut self, item_id: ItemId, count: u32) -> u32 {
        let mut remaining = count;
        for slot in &mut self.slots {
            if remaining == 0 {
                break;
            }
            if let Some(s) = slot {
                if s.item_id == item_id {
                    let take = s.count.min(remaining);
                    s.count -= take;
                    remaining -= take;
                    if s.count == 0 {
                        *slot = None;
                    }
                }
            }
        }
        count - remaining
    }

    pub fn count_of(&self, item_id: ItemId) -> u32 {
        self.slots
            .iter()
            .filter_map(|s| s.as_ref().copied())
            .filter(|s| s.item_id == item_id)
            .map(|s| s.count)
            .sum()
    }

    pub fn has(&self, item_id: ItemId, count: u32) -> bool {
        self.count_of(item_id) >= count
    }

    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| s.is_some())
    }

    pub fn total_weight(&self, item_db: &ItemDatabase) -> f32 {
        self.slots
            .iter()
            .filter_map(|s| s.as_ref().copied())
            .map(|s| item_db.get(s.item_id).map(|d| d.weight * s.count as f32).unwrap_or(0.0))
            .sum()
    }
}

use super::item::ItemDatabase;
#[cfg(test)]
use super::item::ItemDef;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> ItemDatabase {
        let mut db = ItemDatabase::new();
        db.register(ItemDef::builder(ItemId(1), "测试物品").stack(10).build());
        db
    }

    #[test]
    fn test_inventory_add_remove() {
        let db = test_db();
        let mut inv = Inventory::new(5);
        let added = inv.add(ItemId(1), 15, &db);
        assert_eq!(added, 15);
        assert_eq!(inv.count_of(ItemId(1)), 15);

        let removed = inv.remove(ItemId(1), 5);
        assert_eq!(removed, 5);
        assert_eq!(inv.count_of(ItemId(1)), 10);
    }

    #[test]
    fn test_inventory_full() {
        let db = test_db();
        let mut inv = Inventory::new(2);
        inv.add(ItemId(1), 10, &db);
        inv.add(ItemId(1), 10, &db);
        assert!(inv.is_full());
    }
}
