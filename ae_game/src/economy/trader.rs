//! 交易系统

use super::item::{ItemDatabase, ItemId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeKind {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trader {
    pub id: u64,
    pub name: String,
    pub faction_rep: i32,
    pub stock: Vec<(ItemId, u32)>,
    pub buy_price_multiplier: f32,
    pub sell_price_multiplier: f32,
}

impl Trader {
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            faction_rep: 0,
            stock: Vec::new(),
            buy_price_multiplier: 1.2,
            sell_price_multiplier: 0.4,
        }
    }

    pub fn buy_price(&self, item_id: ItemId, db: &ItemDatabase) -> u32 {
        let base = db.get(item_id).map(|d| d.base_value).unwrap_or(1) as f32;
        let rep_discount = 1.0 - (self.faction_rep as f32 * 0.01).clamp(0.0, 0.3);
        (base * self.buy_price_multiplier * rep_discount).max(1.0) as u32
    }

    pub fn sell_price(&self, item_id: ItemId, db: &ItemDatabase) -> u32 {
        let base = db.get(item_id).map(|d| d.base_value).unwrap_or(1) as f32;
        let rep_bonus = 1.0 + (self.faction_rep as f32 * 0.01).clamp(0.0, 0.3);
        (base * self.sell_price_multiplier * rep_bonus).max(1.0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::super::item::ItemDef;
    use super::*;

    #[test]
    fn test_trader_pricing() {
        let mut db = ItemDatabase::new();
        db.register(ItemDef::builder(ItemId(1), "测试").value(100).build());

        let trader = Trader::new(1, "商人");
        assert!(trader.buy_price(ItemId(1), &db) >= 100);
        assert!(trader.sell_price(ItemId(1), &db) <= 100);
    }
}
