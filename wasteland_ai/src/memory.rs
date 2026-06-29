use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NpcMemoryType {
    Episodic,
    Semantic,
    Spatial,
    Social,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMemory {
    pub id: Uuid,
    pub memory_type: NpcMemoryType,
    pub content: String,
    pub location: Option<Vec3>,
    pub timestamp: f32,
    pub importance: f32,
    pub decay_rate: f32,
    pub emotional_tag: String,
    pub associated_npc_ids: Vec<Uuid>,
    pub recall_count: u32,
    pub last_recalled: f32,
}

impl NpcMemory {
    pub fn new(memory_type: NpcMemoryType, content: &str, importance: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            memory_type,
            content: content.to_string(),
            location: None,
            timestamp: 0.0,
            importance: importance.clamp(0.0, 1.0),
            decay_rate: 0.001 * (1.0 - importance),
            emotional_tag: String::new(),
            associated_npc_ids: Vec::new(),
            recall_count: 0,
            last_recalled: 0.0,
        }
    }

    pub fn with_location(mut self, location: Vec3) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_emotion(mut self, tag: &str) -> Self {
        self.emotional_tag = tag.to_string();
        self
    }

    pub fn with_associated(mut self, npc_ids: Vec<Uuid>) -> Self {
        self.associated_npc_ids = npc_ids;
        self
    }

    pub fn strength(&self) -> f32 {
        let decay = (-self.decay_rate * self.timestamp).exp();
        let recall_bonus = 1.0 + (self.recall_count as f32) * 0.1;
        self.importance * decay * recall_bonus
    }

    pub fn recall(&mut self, current_time: f32) {
        self.recall_count += 1;
        self.last_recalled = current_time;
        self.decay_rate *= 0.95;
    }

    pub fn is_forgotten(&self) -> bool {
        self.strength() < 0.01
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMemorySystem {
    pub memories: Vec<NpcMemory>,
    pub max_memories: usize,
    pub current_time: f32,
}

impl NpcMemorySystem {
    pub fn new(max_memories: usize) -> Self {
        Self { memories: Vec::new(), max_memories, current_time: 0.0 }
    }

    pub fn add_memory(&mut self, memory: NpcMemory) {
        if self.memories.len() >= self.max_memories {
            self.memories.sort_by(|a, b| a.strength().partial_cmp(&b.strength()).unwrap());
            self.memories.remove(0);
        }
        self.memories.push(memory);
    }

    pub fn update(&mut self, dt: f32) {
        self.current_time += dt;
        for memory in &mut self.memories {
            memory.timestamp += dt;
        }
        self.memories.retain(|m| !m.is_forgotten());
    }

    pub fn recall_recent(&self, count: usize) -> Vec<&NpcMemory> {
        let mut recent: Vec<&NpcMemory> = self.memories.iter().collect();
        recent.sort_by(|a, b| b.last_recalled.partial_cmp(&a.last_recalled).unwrap());
        recent.truncate(count);
        recent
    }

    pub fn recall_by_type(&self, memory_type: NpcMemoryType) -> Vec<&NpcMemory> {
        self.memories.iter().filter(|m| m.memory_type == memory_type).collect()
    }

    pub fn recall_by_emotion(&self, tag: &str) -> Vec<&NpcMemory> {
        self.memories.iter().filter(|m| m.emotional_tag == tag).collect()
    }

    pub fn recall_by_importance(&self, min_importance: f32) -> Vec<&NpcMemory> {
        self.memories.iter().filter(|m| m.importance >= min_importance).collect()
    }

    pub fn recall_about_npc(&self, npc_id: Uuid) -> Vec<&NpcMemory> {
        self.memories.iter().filter(|m| m.associated_npc_ids.contains(&npc_id)).collect()
    }

    pub fn recall_near_location(&self, location: Vec3, radius: f32) -> Vec<&NpcMemory> {
        self.memories
            .iter()
            .filter(|m| {
                if let Some(loc) = m.location { loc.distance(location) <= radius } else { false }
            })
            .collect()
    }

    pub fn forget_about_npc(&mut self, npc_id: Uuid) {
        for memory in &mut self.memories {
            memory.associated_npc_ids.retain(|id| *id != npc_id);
        }
    }

    pub fn memory_count(&self) -> usize {
        self.memories.len()
    }
}

impl Default for NpcMemorySystem {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let mem = NpcMemory::new(NpcMemoryType::Episodic, "met_player", 0.8);
        assert!(mem.importance > 0.5);
        assert_eq!(mem.recall_count, 0);
    }

    #[test]
    fn test_memory_importance_clamped() {
        let mem = NpcMemory::new(NpcMemoryType::Semantic, "test", 1.5);
        assert!((mem.importance - 1.0).abs() < 0.01);
        let mem2 = NpcMemory::new(NpcMemoryType::Semantic, "test", -0.5);
        assert!((mem2.importance - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_memory_with_location() {
        let loc = Vec3::new(1.0, 2.0, 3.0);
        let mem = NpcMemory::new(NpcMemoryType::Spatial, "place", 0.5).with_location(loc);
        assert!(mem.location.is_some());
        assert!((mem.location.unwrap().x - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_memory_with_emotion() {
        let mem = NpcMemory::new(NpcMemoryType::Episodic, "event", 0.7).with_emotion("fear");
        assert_eq!(mem.emotional_tag, "fear");
    }

    #[test]
    fn test_memory_with_associated() {
        let npc_id = Uuid::new_v4();
        let mem =
            NpcMemory::new(NpcMemoryType::Social, "friend", 0.6).with_associated(vec![npc_id]);
        assert!(mem.associated_npc_ids.contains(&npc_id));
    }

    #[test]
    fn test_memory_strength() {
        let mem = NpcMemory::new(NpcMemoryType::Semantic, "fact", 0.8);
        assert!(mem.strength() > 0.7);
    }

    #[test]
    fn test_memory_recall_bonus() {
        let mut mem = NpcMemory::new(NpcMemoryType::Episodic, "key_event", 0.5);
        let initial = mem.strength();
        mem.recall(1.0);
        assert_eq!(mem.recall_count, 1);
        assert!((mem.last_recalled - 1.0).abs() < 0.01);
        assert!(mem.strength() > initial);
    }

    #[test]
    fn test_memory_is_forgotten() {
        let mut mem = NpcMemory::new(NpcMemoryType::Semantic, "trivial", 0.01);
        mem.timestamp = 10000.0;
        assert!(mem.is_forgotten());
    }

    #[test]
    fn test_memory_not_forgotten() {
        let mem = NpcMemory::new(NpcMemoryType::Episodic, "important", 0.9);
        assert!(!mem.is_forgotten());
    }

    #[test]
    fn test_memory_decay() {
        let mut system = NpcMemorySystem::new(100);
        system.add_memory(NpcMemory::new(NpcMemoryType::Semantic, "test_fact", 0.1));
        system.update(5000.0);
        assert_eq!(system.memory_count(), 0);
    }

    #[test]
    fn test_memory_recall_by_type() {
        let mut system = NpcMemorySystem::new(100);
        system.add_memory(NpcMemory::new(NpcMemoryType::Spatial, "location_a", 0.9));
        system.add_memory(NpcMemory::new(NpcMemoryType::Social, "npc_b_friendly", 0.7));
        let spatial = system.recall_by_type(NpcMemoryType::Spatial);
        assert!(!spatial.is_empty());
    }

    #[test]
    fn test_memory_recall_by_emotion() {
        let mut system = NpcMemorySystem::new(100);
        system.add_memory(
            NpcMemory::new(NpcMemoryType::Episodic, "scary_event", 0.8).with_emotion("fear"),
        );
        system.add_memory(
            NpcMemory::new(NpcMemoryType::Episodic, "happy_event", 0.6).with_emotion("joy"),
        );
        let fearful = system.recall_by_emotion("fear");
        assert_eq!(fearful.len(), 1);
        assert_eq!(fearful[0].content, "scary_event");
    }

    #[test]
    fn test_memory_recall_by_importance() {
        let mut system = NpcMemorySystem::new(100);
        system.add_memory(NpcMemory::new(NpcMemoryType::Semantic, "important", 0.9));
        system.add_memory(NpcMemory::new(NpcMemoryType::Semantic, "trivial", 0.2));
        let important = system.recall_by_importance(0.7);
        assert_eq!(important.len(), 1);
        assert_eq!(important[0].content, "important");
    }

    #[test]
    fn test_memory_recall_about_npc() {
        let mut system = NpcMemorySystem::new(100);
        let npc_id = Uuid::new_v4();
        system.add_memory(
            NpcMemory::new(NpcMemoryType::Social, "met_bob", 0.7).with_associated(vec![npc_id]),
        );
        system.add_memory(NpcMemory::new(NpcMemoryType::Social, "met_alice", 0.6));
        let about_bob = system.recall_about_npc(npc_id);
        assert_eq!(about_bob.len(), 1);
        assert_eq!(about_bob[0].content, "met_bob");
    }

    #[test]
    fn test_memory_recall_near_location() {
        let mut system = NpcMemorySystem::new(100);
        system.add_memory(
            NpcMemory::new(NpcMemoryType::Spatial, "tavern", 0.5)
                .with_location(Vec3::new(10.0, 0.0, 10.0)),
        );
        system.add_memory(
            NpcMemory::new(NpcMemoryType::Spatial, "castle", 0.5)
                .with_location(Vec3::new(100.0, 0.0, 100.0)),
        );
        let nearby = system.recall_near_location(Vec3::new(10.0, 0.0, 10.0), 5.0);
        assert_eq!(nearby.len(), 1);
        assert_eq!(nearby[0].content, "tavern");
    }

    #[test]
    fn test_memory_forget_about_npc() {
        let mut system = NpcMemorySystem::new(100);
        let npc_id = Uuid::new_v4();
        system.add_memory(
            NpcMemory::new(NpcMemoryType::Social, "met_bob", 0.7).with_associated(vec![npc_id]),
        );
        system.forget_about_npc(npc_id);
        let about_bob = system.recall_about_npc(npc_id);
        assert!(about_bob.is_empty());
    }

    #[test]
    fn test_memory_max_capacity() {
        let mut system = NpcMemorySystem::new(3);
        system.add_memory(NpcMemory::new(NpcMemoryType::Semantic, "a", 0.5));
        system.add_memory(NpcMemory::new(NpcMemoryType::Semantic, "b", 0.5));
        system.add_memory(NpcMemory::new(NpcMemoryType::Semantic, "c", 0.5));
        system.add_memory(NpcMemory::new(NpcMemoryType::Semantic, "d", 0.9));
        assert_eq!(system.memory_count(), 3);
    }

    #[test]
    fn test_memory_recall_recent() {
        let mut system = NpcMemorySystem::new(100);
        let mut m1 = NpcMemory::new(NpcMemoryType::Episodic, "old", 0.5);
        m1.last_recalled = 0.0;
        let mut m2 = NpcMemory::new(NpcMemoryType::Episodic, "recent", 0.5);
        m2.last_recalled = 100.0;
        system.add_memory(m1);
        system.add_memory(m2);
        let recent = system.recall_recent(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content, "recent");
    }
}
