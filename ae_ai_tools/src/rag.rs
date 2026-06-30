use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeChunk {
    pub id: String,
    pub content: String,
    pub embeddings: Vec<f32>,
    pub source: KnowledgeSource,
    pub importance: f32,
    pub timestamp: u64,
    pub tags: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeSource {
    WorldData,
    NpcMemory,
    PlayerInteraction,
    Book,
    Sign,
    Environmental,
    Inferred,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub chunks: Vec<KnowledgeChunk>,
    pub embeddings_dim: usize,
    pub max_chunks: usize,
    pub total_tokens: usize,
    pub max_tokens: usize,
}

impl KnowledgeBase {
    pub fn new(embeddings_dim: usize, max_chunks: usize, max_tokens: usize) -> Self {
        KnowledgeBase {
            chunks: Vec::with_capacity(max_chunks),
            embeddings_dim,
            max_chunks,
            total_tokens: 0,
            max_tokens,
        }
    }

    pub fn add_chunk(&mut self, chunk: KnowledgeChunk) -> bool {
        if self.chunks.len() >= self.max_chunks {
            self.evict_lowest_importance();
        }
        let token_estimate = chunk.content.len() / 4;
        if self.total_tokens + token_estimate > self.max_tokens {
            self.evict_lowest_importance();
        }
        self.total_tokens += token_estimate;
        self.chunks.push(chunk);
        true
    }

    fn evict_lowest_importance(&mut self) {
        if let Some(idx) = self
            .chunks
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.importance.partial_cmp(&b.importance).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
        {
            let removed = self.chunks.remove(idx);
            self.total_tokens = self.total_tokens.saturating_sub(removed.content.len() / 4);
        }
    }

    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na < 1e-8 || nb < 1e-8 {
            return 0.0;
        }
        dot / (na * nb)
    }

    pub fn retrieve(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        min_similarity: f32,
    ) -> Vec<&KnowledgeChunk> {
        let mut scored: Vec<(f32, &KnowledgeChunk)> = self
            .chunks
            .iter()
            .map(|c| (Self::cosine_similarity(query_embedding, &c.embeddings), c))
            .filter(|(s, _)| *s >= min_similarity)
            .collect();
        scored.sort_by(|(a, _), (b, _)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored.into_iter().map(|(_, c)| c).collect()
    }

    pub fn retrieve_by_tags(&self, tags: &[String], top_k: usize) -> Vec<&KnowledgeChunk> {
        let mut scored: Vec<(f32, &KnowledgeChunk)> = self
            .chunks
            .iter()
            .map(|c| {
                let overlap = c.tags.iter().filter(|t| tags.contains(t)).count() as f32;
                (overlap * c.importance, c)
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();
        scored.sort_by(|(a, _), (b, _)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored.into_iter().map(|(_, c)| c).collect()
    }

    pub fn retrieve_by_source(
        &self,
        source: KnowledgeSource,
        top_k: usize,
    ) -> Vec<&KnowledgeChunk> {
        let mut results: Vec<&KnowledgeChunk> =
            self.chunks.iter().filter(|c| c.source == source).collect();
        results.sort_by(|a, b| {
            b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);
        results
    }

    pub fn build_context_string(&self, chunks: &[&KnowledgeChunk]) -> String {
        chunks
            .iter()
            .map(|c| {
                format!("[来源: {:?}, 重要性: {:.1}]\n{}\n", c.source, c.importance, c.content)
            })
            .collect::<Vec<_>>()
            .join("\n---\n")
    }

    pub fn stats(&self) -> KnowledgeBaseStats {
        KnowledgeBaseStats {
            total_chunks: self.chunks.len(),
            total_tokens: self.total_tokens,
            avg_importance: if self.chunks.is_empty() {
                0.0
            } else {
                self.chunks.iter().map(|c| c.importance).sum::<f32>() / self.chunks.len() as f32
            },
            source_distribution: {
                let mut dist = HashMap::new();
                for c in &self.chunks {
                    *dist.entry(format!("{:?}", c.source)).or_insert(0) += 1;
                }
                dist
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseStats {
    pub total_chunks: usize,
    pub total_tokens: usize,
    pub avg_importance: f32,
    pub source_distribution: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub embeddings_dim: usize,
    pub max_chunks: usize,
    pub max_tokens: usize,
    pub top_k: usize,
    pub min_similarity: f32,
    pub context_max_tokens: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        RagConfig {
            embeddings_dim: 384,
            max_chunks: 10000,
            max_tokens: 1_000_000,
            top_k: 5,
            min_similarity: 0.6,
            context_max_tokens: 1024,
        }
    }
}

pub struct RagEngine {
    pub knowledge_base: KnowledgeBase,
    pub config: RagConfig,
}

impl RagEngine {
    pub fn new(config: RagConfig) -> Self {
        RagEngine {
            knowledge_base: KnowledgeBase::new(
                config.embeddings_dim,
                config.max_chunks,
                config.max_tokens,
            ),
            config,
        }
    }

    pub fn ingest(
        &mut self,
        content: String,
        source: KnowledgeSource,
        importance: f32,
        tags: Vec<String>,
    ) {
        let chunk = KnowledgeChunk {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            embeddings: vec![0.0; self.config.embeddings_dim],
            source,
            importance,
            timestamp: 0,
            tags,
            references: Vec::new(),
        };
        self.knowledge_base.add_chunk(chunk);
    }

    pub fn query(&self, query_embedding: &[f32]) -> Vec<&KnowledgeChunk> {
        self.knowledge_base.retrieve(query_embedding, self.config.top_k, self.config.min_similarity)
    }

    pub fn query_context_string(&self, query_embedding: &[f32]) -> String {
        let chunks = self.query(query_embedding);
        self.knowledge_base.build_context_string(&chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(
        id: &str,
        content: &str,
        embeddings: Vec<f32>,
        importance: f32,
    ) -> KnowledgeChunk {
        KnowledgeChunk {
            id: id.to_string(),
            content: content.to_string(),
            embeddings,
            source: KnowledgeSource::WorldData,
            importance,
            timestamp: 0,
            tags: vec!["test".to_string()],
            references: Vec::new(),
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((KnowledgeBase::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((KnowledgeBase::cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_kb_add_and_retrieve() {
        let mut kb = KnowledgeBase::new(3, 100, 10000);
        kb.add_chunk(make_chunk("1", "iron sword recipe", vec![1.0, 0.0, 0.0], 0.8));
        kb.add_chunk(make_chunk("2", "water source location", vec![0.0, 1.0, 0.0], 0.5));
        kb.add_chunk(make_chunk("3", "steel forging guide", vec![0.9, 0.1, 0.0], 0.9));

        let results = kb.retrieve(&[1.0, 0.0, 0.0], 2, 0.5);
        assert!(!results.is_empty());
        assert!(results[0].content.contains("iron"));
    }

    #[test]
    fn test_kb_eviction() {
        let mut kb = KnowledgeBase::new(3, 3, 10000);
        kb.add_chunk(make_chunk("1", "a", vec![0.0; 3], 0.1));
        kb.add_chunk(make_chunk("2", "b", vec![0.0; 3], 0.5));
        kb.add_chunk(make_chunk("3", "c", vec![0.0; 3], 0.9));
        kb.add_chunk(make_chunk("4", "d", vec![0.0; 3], 0.8));

        assert_eq!(kb.chunks.len(), 3);
        assert!(!kb.chunks.iter().any(|c| c.importance < 0.2));
    }

    #[test]
    fn test_rag_engine() {
        let config = RagConfig {
            embeddings_dim: 3,
            max_chunks: 100,
            max_tokens: 10000,
            top_k: 3,
            min_similarity: 0.0,
            context_max_tokens: 512,
        };
        let mut engine = RagEngine::new(config);
        engine.ingest(
            "iron ore found near mountain".to_string(),
            KnowledgeSource::WorldData,
            0.7,
            vec!["resource".to_string()],
        );
        engine.ingest(
            "water source at cave entrance".to_string(),
            KnowledgeSource::WorldData,
            0.8,
            vec!["resource".to_string()],
        );

        engine.knowledge_base.chunks[0].embeddings = vec![1.0, 0.5, 0.2];
        engine.knowledge_base.chunks[1].embeddings = vec![0.2, 0.9, 0.1];

        let results = engine.query(&[1.0, 0.5, 0.2]);
        assert!(!results.is_empty());
        assert!(results[0].content.contains("iron"));
    }
}
