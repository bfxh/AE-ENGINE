use hashbrown::HashMap;
use ae_metaentity::meta_entity::{EntityChanges, MetaEntity};
use ae_unified_interface::UnifiedWorld;

pub type ResponseFunction = fn(&MetaEntity, &MetaEntity, &dyn UnifiedWorld) -> Vec<EntityChanges>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteractionType {
    Collision,
    ChemicalReaction,
    Biological,
    ThermalExchange,
    Electrical,
    Magnetic,
    PhaseTransition,
    Custom(u64),
}

pub struct ResponseRegistry {
    rules: HashMap<u64, ResponseFunction>,
    default_rules: HashMap<InteractionType, ResponseFunction>,
    custom_rules: HashMap<String, ResponseFunction>,
}

impl ResponseRegistry {
    pub fn new() -> Self {
        Self { rules: HashMap::new(), default_rules: HashMap::new(), custom_rules: HashMap::new() }
    }

    pub fn register_rule(&mut self, hash: u64, func: ResponseFunction) {
        self.rules.insert(hash, func);
    }

    pub fn register_default(&mut self, interaction_type: InteractionType, func: ResponseFunction) {
        self.default_rules.insert(interaction_type, func);
    }

    pub fn register_custom(&mut self, name: &str, func: ResponseFunction) {
        self.custom_rules.insert(name.to_string(), func);
    }

    pub fn find_response(&self, a: &MetaEntity, b: &MetaEntity) -> Option<ResponseFunction> {
        let combined_hash = Self::combine_hash(a.attribute_hash(), b.attribute_hash());

        if let Some(&func) = self.rules.get(&combined_hash) {
            return Some(func);
        }

        None
    }

    pub fn find_default(&self, interaction_type: InteractionType) -> Option<ResponseFunction> {
        self.default_rules.get(&interaction_type).copied()
    }

    pub fn find_custom(&self, name: &str) -> Option<ResponseFunction> {
        self.custom_rules.get(name).copied()
    }

    fn combine_hash(a: u64, b: u64) -> u64 {
        let mut sorted = [a, b];
        sorted.sort();
        sorted[0].wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(sorted[1])
    }

    pub fn unregister_rule(&mut self, hash: u64) -> bool {
        self.rules.remove(&hash).is_some()
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }
}

impl Default for ResponseRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn noop_response(
        _a: &MetaEntity,
        _b: &MetaEntity,
        _world: &dyn UnifiedWorld,
    ) -> Vec<EntityChanges> {
        Vec::new()
    }

    #[test]
    fn test_register_and_find() {
        let mut registry = ResponseRegistry::new();
        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let water = MetaEntity::water(Vec3::new(1.0, 0.0, 0.0), 0);
        let hash = ResponseRegistry::combine_hash(iron.attribute_hash(), water.attribute_hash());

        registry.register_rule(hash, noop_response);
        assert!(registry.find_response(&iron, &water).is_some());
    }

    #[test]
    fn test_default_rule() {
        let mut registry = ResponseRegistry::new();
        registry.register_default(InteractionType::Collision, noop_response);
        assert!(registry.find_default(InteractionType::Collision).is_some());
        assert!(registry.find_default(InteractionType::ChemicalReaction).is_none());
    }

    #[test]
    fn test_custom_rule() {
        let mut registry = ResponseRegistry::new();
        registry.register_custom("iron_water", noop_response);
        assert!(registry.find_custom("iron_water").is_some());
        assert!(registry.find_custom("nonexistent").is_none());
    }

    #[test]
    fn test_unregister() {
        let mut registry = ResponseRegistry::new();
        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let hash = iron.attribute_hash();

        registry.register_rule(hash, noop_response);
        assert_eq!(registry.rule_count(), 1);
        assert!(registry.unregister_rule(hash));
        assert_eq!(registry.rule_count(), 0);
    }

    #[test]
    fn test_combine_hash_commutative() {
        let a = 0x1234567890ABCDEF;
        let b = 0xFEDCBA0987654321;
        assert_eq!(ResponseRegistry::combine_hash(a, b), ResponseRegistry::combine_hash(b, a));
    }
}
