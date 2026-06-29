//! 制作配方

use super::item::ItemId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CraftStation {
    Hand,
    Workbench,
    Forge,
    Chemistry,
    Factory,
    Campfire,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub item_id: ItemId,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: String,
    pub station: CraftStation,
    pub ingredients: Vec<RecipeIngredient>,
    pub output: ItemId,
    pub output_count: u32,
    pub craft_time: f32,
}

#[derive(Debug, Default)]
pub struct CraftingSystem {
    pub recipes: hashbrown::HashMap<RecipeId, Recipe>,
}

impl CraftingSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, recipe: Recipe) {
        self.recipes.insert(recipe.id, recipe);
    }

    pub fn can_craft(&self, id: RecipeId, inventory: &super::inventory::Inventory) -> bool {
        self.recipes.get(&id).is_some_and(|r| {
            r.ingredients.iter().all(|ing| inventory.has(ing.item_id, ing.count))
        })
    }

    pub fn craft(
        &self,
        id: RecipeId,
        inventory: &mut super::inventory::Inventory,
        item_db: &super::item::ItemDatabase,
    ) -> bool {
        let recipe = match self.recipes.get(&id) {
            Some(r) => r.clone(),
            None => return false,
        };
        if !recipe.ingredients.iter().all(|ing| inventory.has(ing.item_id, ing.count)) {
            return false;
        }
        for ing in &recipe.ingredients {
            inventory.remove(ing.item_id, ing.count);
        }
        inventory.add(recipe.output, recipe.output_count, item_db);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::item::{ItemDatabase, ItemDef};
    use super::*;

    #[test]
    fn test_crafting() {
        let mut db = ItemDatabase::new();
        db.register(ItemDef::builder(ItemId(1), "材料").stack(100).build());
        db.register(ItemDef::builder(ItemId(2), "产物").stack(10).build());

        let mut crafting = CraftingSystem::new();
        crafting.register(Recipe {
            id: RecipeId(1),
            name: "测试配方".to_string(),
            station: CraftStation::Hand,
            ingredients: vec![RecipeIngredient { item_id: ItemId(1), count: 3 }],
            output: ItemId(2),
            output_count: 1,
            craft_time: 1.0,
        });

        let mut inv = super::super::inventory::Inventory::new(10);
        inv.add(ItemId(1), 5, &db);

        assert!(crafting.can_craft(RecipeId(1), &inv));
        assert!(crafting.craft(RecipeId(1), &mut inv, &db));
        assert_eq!(inv.count_of(ItemId(1)), 2);
        assert_eq!(inv.count_of(ItemId(2)), 1);
    }
}
