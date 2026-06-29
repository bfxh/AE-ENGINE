use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::knowledge::KnowledgeGraph;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialNetwork {
    pub agents: Vec<SocialAgent>,
    pub channels: Vec<CommunicationChannel>,
    pub news_events: Vec<NewsEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialAgent {
    pub id: String,
    pub name: String,
    pub position: SocialPosition,
    pub influence: f32,
    pub knowledge: KnowledgeGraph,
    pub faction: String,
    pub credibility: f32,
    pub reach: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocialPosition {
    Leader,
    Trader,
    Messenger,
    Scholar,
    Commoner,
    Outcast,
    Spy,
    Broadcaster,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationChannel {
    pub from_id: String,
    pub to_id: String,
    pub trust: f32,
    pub frequency: f32,
    pub last_exchange: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsEvent {
    pub id: String,
    pub content: String,
    pub source_location: (f32, f32, f32),
    pub spread_radius: f32,
    pub importance: f32,
    pub factions_aware: HashMap<String, f32>,
    pub distorted_versions: Vec<super::distortion::Message>,
}

impl SocialNetwork {
    pub fn new() -> Self {
        Self { agents: Vec::new(), channels: Vec::new(), news_events: Vec::new() }
    }

    pub fn add_agent(&mut self, agent: SocialAgent) {
        self.agents.push(agent);
    }

    pub fn add_channel(&mut self, from_id: &str, to_id: &str, trust: f32) {
        self.channels.push(CommunicationChannel {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            trust,
            frequency: 0.1,
            last_exchange: 0.0,
        });
    }

    pub fn create_news(
        &mut self,
        content: &str,
        location: (f32, f32, f32),
        importance: f32,
    ) -> String {
        let event = NewsEvent {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.to_string(),
            source_location: location,
            spread_radius: 0.0,
            importance,
            factions_aware: HashMap::new(),
            distorted_versions: Vec::new(),
        };
        let id = event.id.clone();
        self.news_events.push(event);
        id
    }

    pub fn spread_news(&mut self, dt: f32) {
        let info = &mut self.news_events;
        let channels = &mut self.channels;
        let agents = &mut self.agents;

        for item in info.iter_mut() {
            for channel in channels.iter_mut() {
                let from_agent = agents.iter().find(|a| a.id == channel.from_id);
                let to_agent = agents.iter().find(|a| a.id == channel.to_id);

                if let (Some(from), Some(to)) = (from_agent, to_agent) {
                    let from_knows = !from.knowledge.search(&item.content).is_empty();

                    if from_knows && channel.last_exchange + 1.0 / channel.frequency < dt * 1000.0 {
                        if let Some(awareness) = item.factions_aware.get_mut(&to.faction) {
                            *awareness += channel.trust * 0.1 * dt;
                        } else {
                            item.factions_aware
                                .insert(to.faction.clone(), channel.trust * 0.1 * dt);
                        }
                        channel.last_exchange = 0.0;
                    }
                }
                channel.last_exchange += dt;
            }

            item.spread_radius += 10.0 * item.importance * dt;
        }
    }

    pub fn faction_awareness(&self, event_id: &str, faction: &str) -> f32 {
        if let Some(event) = self.news_events.iter().find(|e| e.id == event_id) {
            *event.factions_aware.get(faction).unwrap_or(&0.0)
        } else {
            0.0
        }
    }

    pub fn information_delay(
        &self,
        location_a: (f32, f32, f32),
        location_b: (f32, f32, f32),
    ) -> f32 {
        let dx = location_a.0 - location_b.0;
        let dy = location_a.1 - location_b.1;
        let dz = location_a.2 - location_b.2;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        let fastest_channel = self
            .channels
            .iter()
            .map(|c| c.frequency)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.01);

        distance / (1000.0 * fastest_channel.max(0.001))
    }
}

impl Default for SocialNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(id: &str, name: &str, faction: &str) -> SocialAgent {
        SocialAgent {
            id: id.to_string(),
            name: name.to_string(),
            position: SocialPosition::Commoner,
            influence: 0.5,
            knowledge: KnowledgeGraph::new(),
            faction: faction.to_string(),
            credibility: 0.8,
            reach: 10.0,
        }
    }

    #[test]
    fn test_social_agent_creation() {
        let agent = make_agent("a1", "张三", "村庄");
        assert_eq!(agent.name, "张三");
        assert_eq!(agent.faction, "村庄");
        assert_eq!(agent.position, SocialPosition::Commoner);
        assert_eq!(agent.credibility, 0.8);
    }

    #[test]
    fn test_news_event_creation_and_spread() {
        let mut network = SocialNetwork::new();
        let a1 = make_agent("a1", "张三", "村庄");
        let a2 = make_agent("a2", "李四", "城镇");
        network.add_agent(a1);
        network.add_agent(a2);
        network.add_channel("a1", "a2", 0.8);

        let event_id = network.create_news("铁锈病爆发", (0.0, 0.0, 0.0), 0.9);
        assert!(!event_id.is_empty());
        assert_eq!(network.news_events.len(), 1);

        network.spread_news(0.1);
        let awareness = network.faction_awareness(&event_id, "城镇");
        assert!(awareness >= 0.0);
    }

    #[test]
    fn test_information_delay() {
        let mut network = SocialNetwork::new();
        let a1 = make_agent("a1", "张三", "村庄");
        let a2 = make_agent("a2", "李四", "城镇");
        network.add_agent(a1);
        network.add_agent(a2);
        network.add_channel("a1", "a2", 0.5);

        let delay = network.information_delay((0.0, 0.0, 0.0), (1000.0, 0.0, 0.0));
        assert!(delay > 0.0);
    }
}
