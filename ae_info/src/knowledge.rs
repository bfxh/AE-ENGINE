use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub fact: String,
    pub confidence: f32,
    pub source: String,
    pub timestamp: f32,
    pub decay_rate: f32,
    pub tags: Vec<String>,
}

impl KnowledgeNode {
    pub fn new(fact: &str, source: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            fact: fact.to_string(),
            confidence: 1.0,
            source: source.to_string(),
            timestamp: 0.0,
            decay_rate: 0.001,
            tags: Vec::new(),
        }
    }

    pub fn decay(&mut self, dt: f32) {
        self.confidence -= self.decay_rate * dt;
        self.confidence = self.confidence.max(0.0);
    }

    pub fn reinforce(&mut self, boost: f32) {
        self.confidence = (self.confidence + boost).min(1.0);
    }

    pub fn is_reliable(&self) -> bool {
        self.confidence > 0.5
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<KnowledgeNode>,
    pub edges: Vec<(usize, usize, f32)>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), edges: Vec::new() }
    }

    pub fn add_fact(&mut self, node: KnowledgeNode) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }

    pub fn link(&mut self, from: usize, to: usize, weight: f32) {
        self.edges.push((from, to, weight));
    }

    pub fn related_facts(&self, node_idx: usize) -> Vec<&KnowledgeNode> {
        self.edges
            .iter()
            .filter(|(from, to, _)| *from == node_idx || *to == node_idx)
            .map(
                |(from, to, _)| {
                    if *from == node_idx { &self.nodes[*to] } else { &self.nodes[*from] }
                },
            )
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&KnowledgeNode> {
        let query_lower = query.to_lowercase();
        self.nodes.iter().filter(|n| n.fact.to_lowercase().contains(&query_lower)).collect()
    }

    pub fn decay_all(&mut self, dt: f32) {
        for node in &mut self.nodes {
            node.decay(dt);
        }
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_node_creation() {
        let node = KnowledgeNode::new("天空是蓝色的", "观察者");
        assert_eq!(node.fact, "天空是蓝色的");
        assert_eq!(node.source, "观察者");
        assert_eq!(node.confidence, 1.0);
        assert!(node.is_reliable());
    }

    #[test]
    fn test_knowledge_node_decay_and_reinforce() {
        let mut node = KnowledgeNode::new("测试", "测试源");
        node.decay(100.0);
        assert_eq!(node.confidence, 0.9);
        node.reinforce(0.5);
        assert_eq!(node.confidence, 1.0);
        node.decay(2000.0);
        assert!(!node.is_reliable());
    }

    #[test]
    fn test_knowledge_graph_search() {
        let mut graph = KnowledgeGraph::new();
        let a = graph.add_fact(KnowledgeNode::new("铁会生锈", "实验"));
        let b = graph.add_fact(KnowledgeNode::new("铜会生锈", "观察"));
        graph.add_fact(KnowledgeNode::new("金不会生锈", "实验"));
        graph.link(a, b, 0.5);
        let results = graph.search("生锈");
        assert_eq!(results.len(), 3);
        let related = graph.related_facts(a);
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].fact, "铜会生锈");
    }
}
