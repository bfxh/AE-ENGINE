//! 物品数据库

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipSlot {
    None,
    Head,
    Chest,
    Legs,
    Hands,
    Feet,
    Weapon,
    Accessory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConsumeEffect {
    pub heal: f32,
    pub radiation_reduce: f32,
    pub hunger_restore: f32,
    pub thirst_restore: f32,
}

impl Default for ConsumeEffect {
    fn default() -> Self {
        Self { heal: 0.0, radiation_reduce: 0.0, hunger_restore: 0.0, thirst_restore: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct EquipStats {
    pub armor: f32,
    pub damage_bonus: f32,
    pub speed_bonus: f32,
    pub radiation_resist: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: String,
    pub stack_size: u32,
    pub weight: f32,
    pub rarity: ItemRarity,
    pub equip_slot: EquipSlot,
    pub consume_effect: Option<ConsumeEffect>,
    pub equip_stats: EquipStats,
    pub base_value: u32,
}

impl ItemDef {
    pub fn builder(id: ItemId, name: &str) -> ItemDefBuilder {
        ItemDefBuilder {
            id,
            name: name.to_string(),
            stack_size: 1,
            weight: 0.1,
            rarity: ItemRarity::Common,
            equip_slot: EquipSlot::None,
            consume_effect: None,
            equip_stats: EquipStats::default(),
            base_value: 1,
        }
    }
}

pub struct ItemDefBuilder {
    id: ItemId,
    name: String,
    stack_size: u32,
    weight: f32,
    rarity: ItemRarity,
    equip_slot: EquipSlot,
    consume_effect: Option<ConsumeEffect>,
    equip_stats: EquipStats,
    base_value: u32,
}

impl ItemDefBuilder {
    pub fn stack(mut self, n: u32) -> Self { self.stack_size = n; self }
    pub fn weight(mut self, w: f32) -> Self { self.weight = w; self }
    pub fn rarity(mut self, r: ItemRarity) -> Self { self.rarity = r; self }
    pub fn equip(mut self, slot: EquipSlot, stats: EquipStats) -> Self {
        self.equip_slot = slot;
        self.equip_stats = stats;
        self
    }
    pub fn consume(mut self, effect: ConsumeEffect) -> Self {
        self.consume_effect = Some(effect);
        self
    }
    pub fn value(mut self, v: u32) -> Self { self.base_value = v; self }
    pub fn build(self) -> ItemDef {
        ItemDef {
            id: self.id,
            name: self.name,
            stack_size: self.stack_size,
            weight: self.weight,
            rarity: self.rarity,
            equip_slot: self.equip_slot,
            consume_effect: self.consume_effect,
            equip_stats: self.equip_stats,
            base_value: self.base_value,
        }
    }
}

#[derive(Debug, Default)]
pub struct ItemDatabase {
    pub items: hashbrown::HashMap<ItemId, ItemDef>,
}

impl ItemDatabase {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, def: ItemDef) {
        self.items.insert(def.id, def);
    }

    pub fn get(&self, id: ItemId) -> Option<&ItemDef> {
        self.items.get(&id)
    }

    pub fn default_items() -> Self {
        let mut db = Self::new();
        let items = vec![
            ItemDef::builder(ItemId(1), "纯净水").stack(99).consume(ConsumeEffect { thirst_restore: 30.0, ..Default::default() }).value(5).build(),
            ItemDef::builder(ItemId(2), "压缩饼干").stack(50).consume(ConsumeEffect { hunger_restore: 40.0, ..Default::default() }).value(8).build(),
            ItemDef::builder(ItemId(3), "急救包").stack(10).consume(ConsumeEffect { heal: 50.0, ..Default::default() }).value(25).build(),
            ItemDef::builder(ItemId(4), "辐射清药剂").stack(10).consume(ConsumeEffect { radiation_reduce: 30.0, ..Default::default() }).value(40).build(),
            ItemDef::builder(ItemId(16), "铁管刀").equip(EquipSlot::Weapon, EquipStats { damage_bonus: 15.0, ..Default::default() }).value(30).build(),
            ItemDef::builder(ItemId(17), "生锈手枪").equip(EquipSlot::Weapon, EquipStats { damage_bonus: 25.0, ..Default::default() }).value(80).build(),
            ItemDef::builder(ItemId(23), "皮甲").equip(EquipSlot::Chest, EquipStats { armor: 0.2, ..Default::default() }).value(40).build(),
            ItemDef::builder(ItemId(31), "废铁").stack(100).value(2).build(),
            ItemDef::builder(ItemId(32), "木材").stack(100).value(1).build(),
            ItemDef::builder(ItemId(46), "9mm子弹").stack(200).value(3).build(),
        ];
        for item in items {
            db.register(item);
        }
        db
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_database() {
        let db = ItemDatabase::default_items();
        assert!(db.get(ItemId(1)).is_some());
        assert_eq!(db.get(ItemId(1)).unwrap().name, "纯净水");
        assert!(db.get(ItemId(999)).is_none());
    }

    #[test]
    fn test_item_builder() {
        let item = ItemDef::builder(ItemId(100), "测试物品")
            .stack(50)
            .weight(2.0)
            .rarity(ItemRarity::Rare)
            .value(100)
            .build();
        assert_eq!(item.stack_size, 50);
        assert_eq!(item.rarity, ItemRarity::Rare);
    }
}
