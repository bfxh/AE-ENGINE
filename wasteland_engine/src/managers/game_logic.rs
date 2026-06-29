//! 游戏逻辑层管理器
//!
//! 管理 NPC、生态系统、元实体、工厂自动化等游戏逻辑子系统。

use glam::Vec3;
use uuid::Uuid;

use wasteland_axiom::prelude::*;
use wasteland_biology::prelude::*;
use wasteland_eco::prelude::*;
use wasteland_factory::prelude::*;
use wasteland_frequency::prelude::*;
use wasteland_info::prelude::*;
use wasteland_metaentity::prelude::*;

use wasteland_game::npc::{create_default_npc_definition, NpcSpecies, NpcSystem};

/// 游戏逻辑层管理器
pub struct GameLogicManager {
    // 生态系统
    pub ecosystems: Vec<Ecosystem>,
    pub populations: Vec<Population>,

    // 元实体系统
    pub meta_entities: Vec<MetaEntity>,
    pub interaction_cache: InteractionCache,
    pub structural_field: StructuralField,
    pub functional_derivation: FunctionalDerivationEngine,
    pub pending_interactions: Vec<(usize, usize, InteractionResult)>,
    pub frequency_scheduler: FrequencyScheduler,

    // NPC 系统
    pub npc_system: NpcSystem,

    // 工厂自动化
    pub conveyor_network: ConveyorNetwork,
    pub automation_controller: AutomationController,
    pub energy_network: EnergyNetwork,

    // 知识系统
    pub fork_manager: ForkManager,
    pub knowledge_graph: KnowledgeGraph,
}

impl std::fmt::Debug for GameLogicManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameLogicManager")
            .field("ecosystems", &self.ecosystems.len())
            .field("meta_entities", &self.meta_entities.len())
            .field("npc_system", &self.npc_system)
            .field("populations", &self.populations.len())
            .finish()
    }
}

impl GameLogicManager {
    /// 创建新的游戏逻辑层管理器
    pub fn new() -> Self {
        Self {
            ecosystems: Vec::new(),
            populations: Vec::new(),
            meta_entities: Vec::new(),
            interaction_cache: InteractionCache::new(1000),
            structural_field: StructuralField::new(0),
            functional_derivation: FunctionalDerivationEngine::new(),
            pending_interactions: Vec::new(),
            frequency_scheduler: FrequencyScheduler::new(SchedulerConfig::default()),
            npc_system: NpcSystem::new(500),
            conveyor_network: ConveyorNetwork::new(),
            automation_controller: AutomationController::new(),
            energy_network: EnergyNetwork::new(),
            fork_manager: ForkManager::new(),
            knowledge_graph: KnowledgeGraph::new(),
        }
    }

    /// 更新生态系统
    pub fn update_ecosystems(&mut self, dt: f32, global_radiation: f32, global_temperature: f32) {
        for ecosystem in &mut self.ecosystems {
            ecosystem.radiation_level = global_radiation;
            ecosystem.temperature = global_temperature;
            ecosystem.update(dt);
        }
    }

    /// 更新 NPC 系统
    pub fn update_npcs(&mut self, dt: f32, time: f64) {
        self.npc_system.update(dt, time);
    }

    /// 更新种群
    pub fn update_populations(&mut self, dt: f32) {
        for pop in &mut self.populations {
            pop.logistic_growth(dt);
        }
    }

    /// 更新工厂自动化系统
    pub fn update_factory(&mut self, dt: f32) {
        self.conveyor_network.update_all(dt);
        self.automation_controller.tick(dt);
        self.energy_network.update(dt);
    }

    /// 更新知识系统
    pub fn update_knowledge(&mut self) {
        self.fork_manager.update_dominance();
    }

    /// 同步频率调度器
    pub fn sync_frequency_scheduler(&mut self) {
        for entity in &self.meta_entities {
            let id = entity.id;
            if self.frequency_scheduler.get_tier(&id).is_none() {
                self.frequency_scheduler.register(id, entity.position, entity.velocity, false);
            }
            self.frequency_scheduler.update_entity_state(
                &id,
                entity.position,
                entity.velocity,
                false,
            );
        }
    }

    /// 频率调度器 tick
    pub fn tick_frequency_scheduler(&mut self) -> Vec<Uuid> {
        self.frequency_scheduler.tick()
    }

    /// 功能推导 tick
    pub fn tick_functional_derivation(&mut self) {
        self.functional_derivation.tick();
    }

    /// 生成元实体
    pub fn spawn_meta_entity(&mut self, entity: MetaEntity) -> Uuid {
        let id = entity.id;
        self.meta_entities.push(entity);
        id
    }

    /// 生成生态系统
    pub fn spawn_ecosystem(&mut self, ecosystem: Ecosystem) -> usize {
        let idx = self.ecosystems.len();
        self.ecosystems.push(ecosystem);
        idx
    }

    /// 生成 NPC
    pub fn spawn_npc(
        &mut self,
        name: &str,
        position: Vec3,
        species: NpcSpecies,
        faction: &str,
    ) -> Uuid {
        let def = create_default_npc_definition(name, position, species, faction);
        let id = def.id;
        self.npc_system.queue_spawn(def);
        id
    }
}

impl Default for GameLogicManager {
    fn default() -> Self {
        Self::new()
    }
}
