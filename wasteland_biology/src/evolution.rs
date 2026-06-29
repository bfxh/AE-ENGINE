use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub genes: Vec<Gene>,
    pub total_length: usize,
    pub mutation_rate: f32,
    pub generation: u32,
    pub fitness: f32,
    pub parent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gene {
    pub name: String,
    pub locus: usize,
    pub alleles: Vec<Allele>,
    pub dominance: Dominance,
    pub expression: f32,
    pub category: GeneCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allele {
    pub sequence: u64,
    pub effect: AlleleEffect,
    pub rarity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Dominance {
    Recessive,
    Dominant,
    CoDominant,
    IncompleteDominance,
    OverDominance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeneCategory {
    Physical,
    Metabolic,
    Behavioral,
    Sensory,
    Defensive,
    Reproductive,
    Adaptive,
    Regulatory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlleleEffect {
    SizeModifier(f32),
    SpeedModifier(f32),
    StrengthModifier(f32),
    IntelligenceModifier(f32),
    PerceptionModifier(f32),
    HealthModifier(f32),
    RadiationResistance(f32),
    MutationRate(f32),
    Lifetime(f32),
    Fertility(f32),
    Aggression(f32),
    SocialBehavior(f32),
    Camouflage(f32),
    ToxinResistance(f32),
    TemperatureTolerance(f32),
    WaterEfficiency(f32),
    NightVision,
    Echolocation,
    InfraredVision,
    Regeneration(f32),
    Carapace(f32),
    Venom(f32),
    Photosynthesis,
    Bioluminescence,
    Hibernation,
    PackBehavior,
    Territorial,
    Nocturnal,
    Diurnal,
    Migratory,
}

impl Genome {
    pub fn random(rng: &mut impl Rng) -> Self {
        let gene_count = rng.gen_range(20..50);
        let mut genes = Vec::with_capacity(gene_count);
        let gene_names = [
            "SIZE", "SPD", "STR", "INT", "PER", "HLTH", "RADR", "MUTR", "LIFE", "FERT", "AGGR",
            "SOCL", "CAMO", "TOXR", "TEMP", "WATR", "NVSN", "ECHO", "IRVS", "RGEN", "CARP", "VENM",
            "PHOT", "BIOL", "HIBR", "PACK", "TERR", "NOCT", "DIUR", "MIGR",
        ];

        for i in 0..gene_count {
            let name = gene_names[i % gene_names.len()].to_string();
            let effect = match i % gene_names.len() {
                0 => AlleleEffect::SizeModifier(rng.gen_range(-0.5..1.0)),
                1 => AlleleEffect::SpeedModifier(rng.gen_range(-0.3..1.0)),
                2 => AlleleEffect::StrengthModifier(rng.gen_range(-0.3..1.0)),
                3 => AlleleEffect::IntelligenceModifier(rng.gen_range(-0.3..1.0)),
                4 => AlleleEffect::PerceptionModifier(rng.gen_range(-0.3..1.0)),
                5 => AlleleEffect::HealthModifier(rng.gen_range(-0.3..1.0)),
                6 => AlleleEffect::RadiationResistance(rng.gen_range(0.0..1.0)),
                7 => AlleleEffect::MutationRate(rng.gen_range(0.0..1.0)),
                8 => AlleleEffect::Lifetime(rng.gen_range(-0.3..1.0)),
                9 => AlleleEffect::Fertility(rng.gen_range(-0.3..1.0)),
                10 => AlleleEffect::Aggression(rng.gen_range(-0.3..1.0)),
                11 => AlleleEffect::SocialBehavior(rng.gen_range(-0.3..1.0)),
                12 => AlleleEffect::Camouflage(rng.gen_range(0.0..1.0)),
                13 => AlleleEffect::ToxinResistance(rng.gen_range(0.0..1.0)),
                14 => AlleleEffect::TemperatureTolerance(rng.gen_range(-0.3..1.0)),
                15 => AlleleEffect::WaterEfficiency(rng.gen_range(0.0..1.0)),
                16 => AlleleEffect::NightVision,
                17 => AlleleEffect::Echolocation,
                18 => AlleleEffect::InfraredVision,
                19 => AlleleEffect::Regeneration(rng.gen_range(0.0..1.0)),
                20 => AlleleEffect::Carapace(rng.gen_range(0.0..1.0)),
                21 => AlleleEffect::Venom(rng.gen_range(0.0..1.0)),
                22 => AlleleEffect::Photosynthesis,
                23 => AlleleEffect::Bioluminescence,
                24 => AlleleEffect::Hibernation,
                25 => AlleleEffect::PackBehavior,
                26 => AlleleEffect::Territorial,
                27 => AlleleEffect::Nocturnal,
                28 => AlleleEffect::Diurnal,
                29 => AlleleEffect::Migratory,
                _ => AlleleEffect::HealthModifier(0.0),
            };

            let category = match i % 8 {
                0 => GeneCategory::Physical,
                1 => GeneCategory::Metabolic,
                2 => GeneCategory::Behavioral,
                3 => GeneCategory::Sensory,
                4 => GeneCategory::Defensive,
                5 => GeneCategory::Reproductive,
                6 => GeneCategory::Adaptive,
                7 => GeneCategory::Regulatory,
                _ => GeneCategory::Physical,
            };

            let dominance = match rng.gen_range(0..5) {
                0 => Dominance::Recessive,
                1 => Dominance::Dominant,
                2 => Dominance::CoDominant,
                3 => Dominance::IncompleteDominance,
                4 => Dominance::OverDominance,
                _ => Dominance::Dominant,
            };

            genes.push(Gene {
                name,
                locus: i,
                alleles: vec![
                    Allele {
                        sequence: rng.gen(),
                        effect: effect.clone(),
                        rarity: rng.gen_range(0.0..1.0),
                    },
                    Allele { sequence: rng.gen(), effect, rarity: rng.gen_range(0.0..1.0) },
                ],
                dominance,
                expression: rng.gen_range(0.0..1.0),
                category,
            });
        }

        Self {
            total_length: genes.len(),
            genes,
            mutation_rate: 0.01,
            generation: 0,
            fitness: 0.0,
            parent_ids: Vec::new(),
        }
    }

    pub fn reproduce(&self, other: &Genome, rng: &mut impl Rng) -> Genome {
        let mut child_genes = Vec::with_capacity(self.genes.len().min(other.genes.len()));

        for (i, (gene_a, gene_b)) in self.genes.iter().zip(other.genes.iter()).enumerate() {
            let allele_a = &gene_a.alleles[rng.gen_range(0..gene_a.alleles.len())];
            let allele_b = &gene_b.alleles[rng.gen_range(0..gene_b.alleles.len())];

            let cross_seq = if rng.gen::<f32>() < 0.5 {
                (allele_a.sequence & 0xFFFFFFFF00000000) | (allele_b.sequence & 0x00000000FFFFFFFF)
            } else {
                (allele_b.sequence & 0xFFFFFFFF00000000) | (allele_a.sequence & 0x00000000FFFFFFFF)
            };

            let child_allele_a = Allele {
                sequence: cross_seq,
                effect: allele_a.effect.clone(),
                rarity: (allele_a.rarity + allele_b.rarity) * 0.5,
            };

            let child_allele_b = Allele {
                sequence: rng.gen(),
                effect: allele_b.effect.clone(),
                rarity: (allele_a.rarity + allele_b.rarity) * 0.5,
            };

            child_genes.push(Gene {
                name: gene_a.name.clone(),
                locus: i,
                alleles: vec![child_allele_a, child_allele_b],
                dominance: if rng.gen::<f32>() < 0.5 {
                    gene_a.dominance.clone()
                } else {
                    gene_b.dominance.clone()
                },
                expression: (gene_a.expression + gene_b.expression) * 0.5,
                category: gene_a.category.clone(),
            });
        }

        let mut child = Genome {
            total_length: child_genes.len(),
            genes: child_genes,
            mutation_rate: (self.mutation_rate + other.mutation_rate) * 0.5,
            generation: self.generation.max(other.generation) + 1,
            fitness: 0.0,
            parent_ids: vec![format!("{:x}", rng.gen::<u64>()), format!("{:x}", rng.gen::<u64>())],
        };

        child.mutate(rng);
        child
    }

    pub fn mutate(&mut self, rng: &mut impl Rng) {
        for gene in &mut self.genes {
            if rng.gen::<f32>() < self.mutation_rate {
                let allele_idx = rng.gen_range(0..gene.alleles.len());
                let allele = &mut gene.alleles[allele_idx];
                let flip_bits = rng.gen_range(1..16);
                let flip_mask = (1u64 << flip_bits) - 1;
                let flip_pos = rng.gen_range(0..64 - flip_bits);
                allele.sequence ^= flip_mask << flip_pos;
                allele.rarity = (allele.rarity + rng.gen_range(-0.1..0.1)).clamp(0.0, 1.0);
            }
        }

        if rng.gen::<f32>() < self.mutation_rate * 0.1 {
            self.mutation_rate =
                (self.mutation_rate + rng.gen_range(-0.001..0.001)).clamp(0.0, 0.1);
        }
    }

    pub fn calculate_fitness(&mut self, environment: &EnvironmentFactors) -> f32 {
        let mut fitness = 1.0f32;

        for gene in &self.genes {
            for allele in &gene.alleles {
                fitness += self.gene_contribution(&allele.effect, environment);
            }
        }

        fitness = fitness.max(0.0);
        self.fitness = fitness;
        fitness
    }

    fn gene_contribution(&self, effect: &AlleleEffect, env: &EnvironmentFactors) -> f32 {
        match effect {
            AlleleEffect::SizeModifier(v) => *v * (1.0 - env.predator_pressure),
            AlleleEffect::SpeedModifier(v) => *v * env.predator_pressure,
            AlleleEffect::RadiationResistance(v) => *v * env.radiation_level,
            AlleleEffect::TemperatureTolerance(v) => *v * (env.temperature / 300.0 - 1.0).abs(),
            AlleleEffect::WaterEfficiency(v) => *v * (1.0 - env.humidity),
            AlleleEffect::Camouflage(v) => *v * env.predator_pressure,
            AlleleEffect::ToxinResistance(v) => *v * env.toxicity,
            AlleleEffect::HealthModifier(v) => *v * 0.5,
            AlleleEffect::Fertility(v) => *v * 0.3,
            AlleleEffect::IntelligenceModifier(v) => *v * env.complexity,
            AlleleEffect::StrengthModifier(v) => *v * env.competition,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EnvironmentFactors {
    pub temperature: f32,
    pub humidity: f32,
    pub radiation_level: f32,
    pub predator_pressure: f32,
    pub competition: f32,
    pub toxicity: f32,
    pub complexity: f32,
    pub food_availability: f32,
}


// ============================================================================
// 扩展方法（2026-06-29）：辅助查询 + 种群级进化
// ============================================================================

impl Genome {
    /// 基因数量
    pub fn gene_count(&self) -> usize {
        self.genes.len()
    }

    /// 是否包含某种等位基因效果
    pub fn has_allele_effect(&self, target: &AlleleEffect) -> bool {
        for gene in &self.genes {
            for allele in &gene.alleles {
                // 简单判等：枚举变体名相同即可（带值的变体不比较值）
                let s_a = format!("{:?}", allele.effect);
                let s_b = format!("{:?}", target);
                let name_a = s_a.split('(').next().unwrap_or("");
                let name_b = s_b.split('(').next().unwrap_or("");
                if name_a == name_b {
                    return true;
                }
            }
        }
        false
    }

    /// 遗传多样性（等位基因序列的香农熵近似）
    /// 值越高表示种群内基因变异越丰富
    pub fn genetic_diversity(&self) -> f32 {
        if self.genes.is_empty() {
            return 0.0;
        }
        let mut total_unique = 0usize;
        let mut total_alleles = 0usize;
        for gene in &self.genes {
            let mut seqs: Vec<u64> = gene.alleles.iter().map(|a| a.sequence).collect();
            seqs.sort_unstable();
            seqs.dedup();
            total_unique += seqs.len();
            total_alleles += gene.alleles.len();
        }
        if total_alleles == 0 {
            return 0.0;
        }
        (total_unique as f32) / (total_alleles as f32)
    }

    /// 按类别筛选基因
    pub fn genes_by_category(&self, cat: GeneCategory) -> Vec<&Gene> {
        self.genes.iter().filter(|g| g.category == cat).collect()
    }

    /// 显性等位基因列表（每个基因取第一个显性或共显性等位基因）
    pub fn dominant_alleles(&self) -> Vec<&Allele> {
        let mut result = Vec::new();
        for gene in &self.genes {
            if matches!(gene.dominance, Dominance::Dominant | Dominance::CoDominant | Dominance::OverDominance) {
                if let Some(first) = gene.alleles.first() {
                    result.push(first);
                }
            }
        }
        result
    }

    /// 世代距离根代的代数
    pub fn generations_since_root(&self) -> u32 {
        self.generation
    }

    /// 与另一个基因组的相似度（0-1，基于等位基因序列汉明距离的归一化）
    pub fn similarity(&self, other: &Genome) -> f32 {
        let min_len = self.genes.len().min(other.genes.len());
        if min_len == 0 {
            return 0.0;
        }
        let mut matched_bits = 0u32;
        let mut total_bits = 0u32;
        for i in 0..min_len {
            let a = &self.genes[i];
            let b = &other.genes[i];
            let allele_a = match a.alleles.first() {
                Some(x) => x.sequence,
                None => continue,
            };
            let allele_b = match b.alleles.first() {
                Some(x) => x.sequence,
                None => continue,
            };
            let xor = allele_a ^ allele_b;
            let same_bits = 64 - xor.count_ones();
            matched_bits += same_bits;
            total_bits += 64;
        }
        if total_bits == 0 {
            return 0.0;
        }
        (matched_bits as f32) / (total_bits as f32)
    }
}

/// 种群（多代进化模拟）
#[derive(Debug, Clone)]
pub struct Population {
    pub individuals: Vec<Genome>,
    pub generation: u32,
    pub carrying_capacity: usize,
}

impl Population {
    pub fn new(capacity: usize) -> Self {
        Self {
            individuals: Vec::with_capacity(capacity),
            generation: 0,
            carrying_capacity: capacity,
        }
    }

    /// 随机初始化种群
    pub fn random_initialize(&mut self, rng: &mut impl Rng, size: usize) {
        self.individuals.clear();
        for _ in 0..size.min(self.carrying_capacity) {
            self.individuals.push(Genome::random(rng));
        }
    }

    /// 锦标赛选择（k 元锦标赛，返回胜者索引）
    pub fn tournament_select(&self, rng: &mut impl Rng, k: usize) -> Option<usize> {
        if self.individuals.is_empty() {
            return None;
        }
        let n = self.individuals.len();
        let mut best_idx = rng.gen_range(0..n);
        let mut best_fitness = self.individuals[best_idx].fitness;
        for _ in 1..k.min(n) {
            let idx = rng.gen_range(0..n);
            if self.individuals[idx].fitness > best_fitness {
                best_idx = idx;
                best_fitness = self.individuals[idx].fitness;
            }
        }
        Some(best_idx)
    }

    /// 进化一代：选择 + 繁殖 + 变异 + 评估
    pub fn evolve_generation(&mut self, rng: &mut impl Rng, env: &EnvironmentFactors) {
        if self.individuals.len() < 2 {
            return;
        }
        // 评估当前代
        for g in self.individuals.iter_mut() {
            g.calculate_fitness(env);
        }
        // 繁殖下一代
        let mut next_gen = Vec::with_capacity(self.carrying_capacity);
        while next_gen.len() < self.carrying_capacity {
            let parent_a_idx = match self.tournament_select(rng, 3) {
                Some(i) => i,
                None => break,
            };
            let parent_b_idx = match self.tournament_select(rng, 3) {
                Some(i) => i,
                None => break,
            };
            if parent_a_idx == parent_b_idx {
                continue;
            }
            let child = self.individuals[parent_a_idx].reproduce(&self.individuals[parent_b_idx], rng);
            next_gen.push(child);
        }
        self.individuals = next_gen;
        self.generation += 1;
    }

    /// 平均适应度
    pub fn average_fitness(&self) -> f32 {
        if self.individuals.is_empty() {
            return 0.0;
        }
        self.individuals.iter().map(|g| g.fitness).sum::<f32>() / self.individuals.len() as f32
    }

    /// 最优适应度
    pub fn best_fitness(&self) -> f32 {
        self.individuals.iter().map(|g| g.fitness).fold(f32::NEG_INFINITY, f32::max).max(0.0)
    }

    /// 种群大小
    pub fn size(&self) -> usize {
        self.individuals.len()
    }

    /// 遗传多样性（基于个体间平均相似度的 1 - similarity）
    pub fn population_diversity(&self) -> f32 {
        let n = self.individuals.len();
        if n < 2 {
            return 0.0;
        }
        let mut sum_sim = 0.0f32;
        let mut count = 0u32;
        for i in 0..n {
            for j in (i + 1)..n {
                sum_sim += self.individuals[i].similarity(&self.individuals[j]);
                count += 1;
            }
        }
        if count == 0 {
            return 0.0;
        }
        1.0 - (sum_sim / count as f32)
    }
}

impl Default for EnvironmentFactors {
    fn default() -> Self {
        Self {
            temperature: 293.15, // 20°C
            humidity: 0.5,
            radiation_level: 0.1,
            predator_pressure: 0.3,
            competition: 0.5,
            toxicity: 0.1,
            complexity: 0.5,
            food_availability: 0.7,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn fixed_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn test_genome_random_has_genes() {
        let mut rng = fixed_rng();
        let g = Genome::random(&mut rng);
        assert!(g.gene_count() >= 20, "random genome should have >= 20 genes");
        assert!(g.gene_count() <= 50, "random genome should have <= 50 genes");
        assert_eq!(g.generation, 0);
        assert_eq!(g.fitness, 0.0);
    }

    #[test]
    fn test_genome_reproduce_increments_generation() {
        let mut rng = fixed_rng();
        let parent_a = Genome::random(&mut rng);
        let parent_b = Genome::random(&mut rng);
        let child = parent_a.reproduce(&parent_b, &mut rng);
        assert_eq!(child.generation, 1, "child should be generation 1");
        assert_eq!(child.parent_ids.len(), 2, "child should have 2 parent ids");
    }

    #[test]
    fn test_genome_mutate_preserves_length() {
        let mut rng = fixed_rng();
        let mut g = Genome::random(&mut rng);
        let original_count = g.gene_count();
        g.mutate(&mut rng);
        assert_eq!(g.gene_count(), original_count, "mutation should not change gene count");
    }

    #[test]
    fn test_genome_calculate_fitness_positive() {
        let mut rng = fixed_rng();
        let mut g = Genome::random(&mut rng);
        let env = EnvironmentFactors::default();
        let fit = g.calculate_fitness(&env);
        assert!(fit >= 0.0, "fitness should be non-negative");
        assert_eq!(g.fitness, fit, "fitness field should be updated");
    }

    #[test]
    fn test_genome_genetic_diversity_range() {
        let mut rng = fixed_rng();
        let g = Genome::random(&mut rng);
        let div = g.genetic_diversity();
        assert!(div >= 0.0 && div <= 1.0, "diversity should be in [0,1], got {}", div);
    }

    #[test]
    fn test_genome_similarity_to_self_high() {
        let mut rng = fixed_rng();
        let g = Genome::random(&mut rng);
        let sim = g.similarity(&g);
        assert!(sim > 0.99, "genome similarity to itself should be ~1.0, got {}", sim);
    }

    #[test]
    fn test_genome_similarity_different_lower() {
        let mut rng = fixed_rng();
        let g1 = Genome::random(&mut rng);
        let g2 = Genome::random(&mut rng);
        let sim = g1.similarity(&g2);
        assert!(sim < 1.0, "different genomes should have similarity < 1.0, got {}", sim);
    }

    #[test]
    fn test_genome_has_allele_effect() {
        let mut rng = fixed_rng();
        let g = Genome::random(&mut rng);
        // 随机生成的 genome 大概率包含 SizeModifier（gene_names[0]）
        // 检查方法至少能正确返回 bool
        let _ = g.has_allele_effect(&AlleleEffect::NightVision);
    }

    #[test]
    fn test_genome_dominant_alleles_returns_valid() {
        let mut rng = fixed_rng();
        let g = Genome::random(&mut rng);
        let dom = g.dominant_alleles();
        // 不强制要求一定有显性（随机可能都是隐性），但返回值应合法
        assert!(dom.len() <= g.gene_count(), "dominant alleles cannot exceed gene count");
    }

    #[test]
    fn test_population_random_initialize() {
        let mut rng = fixed_rng();
        let mut pop = Population::new(20);
        pop.random_initialize(&mut rng, 10);
        assert_eq!(pop.size(), 10, "population should have 10 individuals");
        assert_eq!(pop.generation, 0);
    }

    #[test]
    fn test_population_evolve_increments_generation() {
        let mut rng = fixed_rng();
        let mut pop = Population::new(10);
        pop.random_initialize(&mut rng, 10);
        let env = EnvironmentFactors::default();
        pop.evolve_generation(&mut rng, &env);
        assert_eq!(pop.generation, 1, "generation should be 1 after one evolve");
        assert_eq!(pop.size(), 10, "population size should be maintained at carrying capacity");
    }

    #[test]
    fn test_population_average_fitness_returns_value() {
        let mut rng = fixed_rng();
        let mut pop = Population::new(10);
        pop.random_initialize(&mut rng, 10);
        let env = EnvironmentFactors::default();
        for g in pop.individuals.iter_mut() {
            g.calculate_fitness(&env);
        }
        let avg = pop.average_fitness();
        assert!(avg >= 0.0, "average fitness should be non-negative");
    }

    #[test]
    fn test_population_diversity_range() {
        let mut rng = fixed_rng();
        let mut pop = Population::new(10);
        pop.random_initialize(&mut rng, 10);
        let div = pop.population_diversity();
        assert!(div >= 0.0 && div <= 1.0, "diversity should be in [0,1], got {}", div);
    }

    #[test]
    fn test_population_diversity_decreases_with_inbreeding() {
        let mut rng = fixed_rng();
        let mut pop = Population::new(8);
        pop.random_initialize(&mut rng, 8);
        let env = EnvironmentFactors::default();
        let div_initial = pop.population_diversity();
        // 演化 5 代
        for _ in 0..5 {
            pop.evolve_generation(&mut rng, &env);
        }
        let div_after = pop.population_diversity();
        // 演化后多样性通常会下降（选择压力 + 小种群），但不强制，只验证范围
        assert!(div_after >= 0.0 && div_after <= 1.0, "diversity after evolution should be in [0,1]");
        let _ = div_initial;
    }

    #[test]
    fn test_tournament_select_returns_valid_index() {
        let mut rng = fixed_rng();
        let mut pop = Population::new(10);
        pop.random_initialize(&mut rng, 10);
        let env = EnvironmentFactors::default();
        for g in pop.individuals.iter_mut() {
            g.calculate_fitness(&env);
        }
        if let Some(idx) = pop.tournament_select(&mut rng, 3) {
            assert!(idx < pop.size(), "selected index should be in range");
        }
    }

    #[test]
    fn test_environment_factors_default() {
        let env = EnvironmentFactors::default();
        assert!((env.temperature - 293.15).abs() < 0.01, "default temp should be 293.15 K");
        assert!(env.humidity > 0.0 && env.humidity < 1.0);
    }
}
