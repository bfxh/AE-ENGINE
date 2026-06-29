//! 模拟层管理器
//!
//! 管理物理、化学、场、粒子、流体、声学、地质等模拟子系统。

use glam::Vec3;
use rayon::prelude::*;

use wasteland_acoustics::prelude::*;
use wasteland_character::*;
use wasteland_chemistry::prelude::*;
use wasteland_electro::prelude::*;
use wasteland_emergence::prelude::*;
use wasteland_field::prelude::*;
use wasteland_fluid::prelude::*;
use wasteland_geo::prelude::*;
use wasteland_hydro::prelude::*;
use wasteland_particle::prelude::*;
use wasteland_physics::fixed_point::{FixedPoint, FixedVec3};
use wasteland_physics::prelude::*;
use wasteland_thermo::prelude::*;
use wasteland_weather::prelude::*;

use crate::architecture::WorldContext;
use crate::managers::domain_isolation::DomainIsolationManager;

/// 物理LOD（Level of Detail）配置 — 支持8GB VRAM下的300万粒子
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicsLod {
    pub near_field_distance: f32,       // 近场距离
    pub mid_field_distance: f32,        // 中场距离
    pub far_field_update_interval: u32, // 远场更新间隔(帧)
    pub max_near_particles: usize,      // 近场最大粒子数
    pub max_total_particles: usize,     // 总最大粒子数
}

impl Default for PhysicsLod {
    fn default() -> Self {
        PhysicsLod {
            near_field_distance: 50.0,
            // §3.2: mid-field grid covers ±mid_field_distance.
            // 200m with 32³ grid → dx=12.5m. CFL = max_v * mid_dt / dx = 50 * 0.2 / 12.5 = 0.8 < 1 (stable).
            // Reducing to 150m would give dx=9.375m → CFL=1.07 > 1 (unstable), so keep 200m.
            mid_field_distance: 200.0,
            // §3.3: far particles update at 1Hz (every 60 frames @ 60fps)
            far_field_update_interval: 60,
            max_near_particles: 10_000,
            max_total_particles: 3_000_000,
        }
    }
}

/// LOD 统计信息 — 记录各层级粒子数
#[derive(Debug, Clone, Copy, Default)]
pub struct LodStats {
    pub near: usize,
    pub mid: usize,
    pub far: usize,
}

/// 模拟层管理器，负责所有物理和自然模拟子系统
pub struct SimulationManager {
    // 物理系统
    pub physics: PhysicsWorld,
    pub surface_contact_detector: SurfaceContactDetector,
    pub sparse_octrees: Vec<SparseOctree>,
    pub dual_phase_manager: DualPhaseManager,

    // 化学系统
    pub chemistry: ReactionSystem,

    // 场系统
    pub coupled_field_solver: CoupledFieldSystem,

    // 粒子系统
    pub particle_system: ParticleSystem,
    pub mpss: MpssBuffer,
    pub mpm_config: wasteland_particle::mpm_solver::MpmConfig,
    pub mpm_grid: wasteland_particle::mpm_solver::Grid,
    /// §3.2 3-layer grid: mid-field grid for simplified MPM (velocity-only, no deformation).
    /// Covers mid_field_distance (200m) with coarser dx than near-field grid.
    /// mid particles (50-200m) get pressure coupling via this grid without stress computation.
    pub mpm_grid_mid: wasteland_particle::mpm_solver::Grid,
    /// §3.2 3-layer grid: coarse scalar field for far-field temperature diffusion.
    /// 16³ grid covering 400m (dx=25m). Far particles (>200m) sample temperature from this
    /// field instead of storing independent temperature. Enables large-scale heat conduction
    /// (fire/blast thermal effects propagate to far distance).
    pub coarse_temperature: Vec<f32>,
    /// §3.2 Coarse field grid dimensions (nx=ny=nz for cubic field).
    pub coarse_field_n: usize,
    /// §3.2 Coarse field grid spacing (meters per cell).
    pub coarse_field_dx: f32,
    /// §3.2 Coarse field world-space origin (corner of cell (0,0,0)).
    /// Updated per-frame to follow player so the field always centers on the player.
    pub coarse_field_origin: [f32; 3],
    pub physics_lod: PhysicsLod,
    pub player_position: glam::Vec3,
    pub lod_stats: LodStats,
    pub field_step_counter: u32,
    pub lod_reclassify_counter: u32,
    pub cached_near_indices: Vec<(usize, f32)>,
    pub cached_mid_indices: Vec<usize>,
    pub cached_far_indices: Vec<usize>,
    pub cached_player_pos: glam::Vec3,
    pub material_table: Vec<wasteland_particle::constitutive::MaterialParams>,
    pub model_table: Vec<wasteland_particle::constitutive::ConstitutiveModel>,
    pub emergence: EmergentDetailSystem,

    // 热力学系统
    pub conduction_solver: ConductionSolver,
    pub convection_solver: ConvectionSolver,
    pub radiation_solver: RadiationSolver,

    // 流体和气象
    pub atmosphere: Atmosphere,
    pub wind_field: WindField,
    pub fluid_solver: NavierStokesSolver,
    pub acoustic_solver: AcousticSolver,
    pub next_acoustic_source_id: u64,

    // 地质系统
    pub erosion_solver: ErosionSolver,
    pub tectonic_solver: TectonicSolver,
    pub surface_runoff: SurfaceRunoff,

    // 电学系统
    pub electrostatic_solver: ElectrostaticSolver,
    pub magnetostatic_solver: MagnetostaticSolver,
    pub point_charges: Vec<PointCharge>,
    pub current_elements: Vec<CurrentElement>,

    // 工厂数据
    pub furnace_positions: Vec<(glam::Vec3, f32)>,
    pub assembler_positions: Vec<glam::Vec3>,
    pub conveyor_segment_data: Vec<(glam::Vec3, glam::Vec3, f32)>,

    // 8GB架构新系统
    pub npc_manager: wasteland_game::npc::manager::NpcManager,
    pub combat_system: wasteland_game::combat::CombatSystem,
    pub navigation_manager: crate::navigation_manager::NavigationManager,
    pub animation_manager: crate::animation_manager::AnimationManager,

    // 优化系统
    pub interaction_system: crate::interaction_system::InteractionSystem,
    pub reinforcement_system: wasteland_game::combat::optimizer::ReinforcementSystem,

    pub domain_isolation: DomainIsolationManager,

    /// NPC 生成联动：递增的实体 ID（用于 AnimationManager 的 u64 id）
    pub next_entity_id: u64,
    /// NpcId -> animation_id 映射，用于同步 NPC 动画状态
    pub npc_anim_ids: std::collections::HashMap<wasteland_game::npc::NpcId, u64>,

    /// 情绪引擎（全局单实例，主循环每帧 update；战斗事件通过 trigger_event 注入）
    pub emotion_engine: wasteland_ai::EmotionEngine,

    /// 统一 ECS 世界（查询索引层）。
    /// 主存储仍为各管理器的 slotmap/Vec；EcsWorld 同步关键字段（Position/Velocity/Faction/Health/EntityKind），
    /// 提供跨系统统一查询入口（query_by_faction / query_in_range / query_faction_in_range）。
    pub ecs_world: crate::ecs::EcsWorld,
    /// NpcId -> Uuid 映射（NPC 在 EcsWorld 中的实体 ID）
    pub npc_ecs_ids: std::collections::HashMap<wasteland_game::npc::NpcId, uuid::Uuid>,
    /// NpcId -> CombatEntityId 映射（用于双向同步：NPC.position→CombatEntity, CombatEntity.health→NPC）
    pub npc_combat_ids: std::collections::HashMap<
        wasteland_game::npc::NpcId,
        wasteland_game::combat::CombatEntityId,
    >,
}

impl std::fmt::Debug for SimulationManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimulationManager")
            .field("physics", &self.physics)
            .field("chemistry", &self.chemistry)
            .field("coupled_field_solver", &self.coupled_field_solver)
            .field("particle_system", &self.particle_system)
            .finish()
    }
}

impl SimulationManager {
    /// 创建新的模拟层管理器
    pub fn new(bounds: crate::WorldBounds) -> Self {
        let particle_bounds = ParticleBounds {
            min: bounds.min,
            max: bounds.max,
            boundary_type: ParticleBoundaryType::Reflecting,
        };

        let mut coupled_field_solver = CoupledFieldSystem::new();
        let field_config = FieldConfiguration {
            name: "temperature".into(),
            resolution: [16, 16, 16],
            origin: bounds.min,
            cell_size: (bounds.max.x - bounds.min.x) / 16.0,
        };
        coupled_field_solver
            .solver
            .create_scalar_field(field_config.clone(), FieldType::Temperature);
        coupled_field_solver.solver.create_scalar_field(
            FieldConfiguration { name: "density".into(), ..field_config.clone() },
            FieldType::Density,
        );
        coupled_field_solver.solver.create_scalar_field(
            FieldConfiguration { name: "chemical".into(), ..field_config.clone() },
            FieldType::ChemicalConcentration { compound_id: 0 },
        );
        coupled_field_solver.solver.create_scalar_field(
            FieldConfiguration { name: "bioactivity".into(), ..field_config },
            FieldType::BiologicalActivity,
        );
        coupled_field_solver.set_default_thermodynamic_couplings();

        let physics_lod = PhysicsLod::default();
        // §3.1 Moving Window MPM: grid must cover ±near_field_distance to avoid
        // P2G stencil truncation. Previous 16³/dx=1.6 grid only covered 25.6m,
        // but near_field_distance=50m → particles at 25-50m had P2G contributions
        // silently discarded by boundary checks (mpm_solver.rs:242-244, 309-311).
        // grid_size = next_power_of_two(ceil(2*near_dist / dx)) for alignment.
        let grid_dx = 1.6;
        let grid_n =
            ((2.0 * physics_lod.near_field_distance / grid_dx).ceil() as usize).next_power_of_two();
        let mpm_config = wasteland_particle::mpm_solver::MpmConfig {
            grid_dx,
            grid_size: [grid_n; 3],
            ..Default::default()
        };
        let mpm_grid = wasteland_particle::mpm_solver::Grid::new(grid_n, grid_n, grid_n, grid_dx);

        // §3.2 3-layer grid: mid-field grid covers ±mid_field_distance (400m span).
        // 16³ grid (4k nodes × 28B = 112KB) with dx=25m → covers 400m centered on player.
        // Optimized from 32³→16³ (8x fewer nodes, 8x faster reset, better cache locality).
        // CFL = max_v * mid_dt / dx = 50 * 0.2 / 25 = 0.4 < 1 ✓ (stable).
        // Mid particles (50-200m) use velocity-only MPM, 25m resolution is sufficient
        // for pressure coupling (fluid flow, smoke drift) without fine stress/strain.
        let mid_grid_n = 16;
        let mid_grid_dx = (2.0 * physics_lod.mid_field_distance) / mid_grid_n as f32; // 25m
        let mpm_grid_mid = wasteland_particle::mpm_solver::Grid::new(
            mid_grid_n,
            mid_grid_n,
            mid_grid_n,
            mid_grid_dx,
        );

        // §3.2 3-layer grid: coarse temperature field for far particles.
        // 16³ grid covering 800m (dx=50m). Far particles (>200m) sample temperature here.
        // Memory: 16³ × 4B = 16KB (negligible). Diffusion is O(n) = 4k ops, very fast.
        // Field is centered on player: origin = player - 400m, covers [player-400, player+400].
        let coarse_field_n = 16usize;
        let coarse_field_dx = 50.0f32; // 16 * 50 = 800m span
        let coarse_temperature = vec![288.15f32; coarse_field_n * coarse_field_n * coarse_field_n];
        let material_table: Vec<wasteland_particle::constitutive::MaterialParams> = vec![
            wasteland_particle::constitutive::MaterialParams {
                young_modulus: 1.0e6,
                poisson_ratio: 0.3,
                yield_stress: 1.0e4,
                hardening: 0.01,
                density: 1000.0,
                friction_angle: 35.0,
                cohesion: 0.0,
            },
            wasteland_particle::constitutive::MaterialParams {
                young_modulus: 2.0e6,
                poisson_ratio: 0.45,
                yield_stress: 0.0,
                hardening: 0.0,
                density: 1000.0,
                friction_angle: 0.0,
                cohesion: 0.0,
            },
            wasteland_particle::constitutive::MaterialParams {
                young_modulus: 5.0e7,
                poisson_ratio: 0.3,
                yield_stress: 0.0,
                hardening: 0.0,
                density: 1600.0,
                friction_angle: 35.0,
                cohesion: 0.0,
            },
            wasteland_particle::constitutive::MaterialParams {
                young_modulus: 2.0e11,
                poisson_ratio: 0.3,
                yield_stress: 2.5e8,
                hardening: 0.01,
                density: 7850.0,
                friction_angle: 0.0,
                cohesion: 0.0,
            },
        ];
        let model_table = vec![
            wasteland_particle::constitutive::ConstitutiveModel::NeoHookean,
            wasteland_particle::constitutive::ConstitutiveModel::NewtonianFluid,
            wasteland_particle::constitutive::ConstitutiveModel::DruckerPrager,
            wasteland_particle::constitutive::ConstitutiveModel::VonMises,
        ];

        Self {
            physics: PhysicsWorld::default(),
            chemistry: ReactionSystem::new(),
            coupled_field_solver,
            particle_system: ParticleSystem::new(particle_bounds),
            mpss: MpssBuffer::new(3000000),
            mpm_config,
            mpm_grid,
            mpm_grid_mid,
            coarse_temperature,
            coarse_field_n,
            coarse_field_dx,
            coarse_field_origin: [0.0; 3],
            physics_lod,
            player_position: glam::Vec3::ZERO,
            lod_stats: LodStats::default(),
            field_step_counter: 0,
            lod_reclassify_counter: 0,
            cached_near_indices: Vec::new(),
            cached_mid_indices: Vec::new(),
            cached_far_indices: Vec::new(),
            cached_player_pos: glam::Vec3::ZERO,
            material_table,
            model_table,
            emergence: EmergentDetailSystem::new(),
            sparse_octrees: Vec::new(),
            dual_phase_manager: DualPhaseManager::new(1000),
            conduction_solver: ConductionSolver::new(),
            convection_solver: ConvectionSolver::new(),
            radiation_solver: RadiationSolver::new(),
            atmosphere: Atmosphere::default(),
            wind_field: WindField::default(),
            fluid_solver: NavierStokesSolver::new((16, 16, 16), 1.0, FluidProperties::default()),
            acoustic_solver: AcousticSolver::new((16, 16, 16), 1.0),
            next_acoustic_source_id: 0,
            erosion_solver: ErosionSolver::new((32, 32), 1.0),
            tectonic_solver: TectonicSolver::new(),
            surface_runoff: SurfaceRunoff::default(),
            electrostatic_solver: ElectrostaticSolver::new(),
            magnetostatic_solver: MagnetostaticSolver::new(),
            point_charges: Vec::new(),
            current_elements: Vec::new(),
            surface_contact_detector: SurfaceContactDetector::new(),
            furnace_positions: Vec::new(),
            assembler_positions: Vec::new(),
            conveyor_segment_data: Vec::new(),
            npc_manager: wasteland_game::npc::manager::NpcManager::new(300),
            combat_system: wasteland_game::combat::CombatSystem::new(),
            navigation_manager: crate::navigation_manager::NavigationManager::new(
                crate::navigation_manager::NavigationConfig::default(),
            ),
            animation_manager: crate::animation_manager::AnimationManager::new(300, 5000),
            interaction_system: crate::interaction_system::InteractionSystem::new(500),
            reinforcement_system: wasteland_game::combat::optimizer::ReinforcementSystem::new(20),
            domain_isolation: DomainIsolationManager::new(),
            next_entity_id: 0,
            npc_anim_ids: std::collections::HashMap::new(),
            emotion_engine: wasteland_ai::EmotionEngine::new(
                wasteland_ai::PersonalityTraits::default(),
            ),
            ecs_world: crate::ecs::EcsWorld::new(),
            npc_ecs_ids: std::collections::HashMap::new(),
            npc_combat_ids: std::collections::HashMap::new(),
        }
    }

    /// 生成战斗 NPC 并联动注册到 CombatSystem + AnimationManager + EcsWorld。
    ///
    /// 接入点：NPC 不再是孤儿——spawn 时同时创建战斗实体和动画角色，
    /// 主循环 step() 会同步动画状态。EcsWorld 作为统一查询索引层。
    pub fn spawn_combatant_npc(&mut self, pos: Vec3, faction: u8) -> wasteland_game::npc::NpcId {
        let npc_id = self.npc_manager.spawn_combatant(pos, faction);
        let anim_id = self.next_entity_id;
        self.next_entity_id += 1;
        self.npc_anim_ids.insert(npc_id, anim_id);
        // 联动 CombatSystem：创建对应战斗实体
        let combat_entity = wasteland_game::combat::CombatEntity::new(pos, faction, 100.0);
        let combat_id = self.combat_system.spawn_entity(combat_entity);
        self.npc_combat_ids.insert(npc_id, combat_id);
        // 联动 AnimationManager：注册人形角色（12 骨骼）
        self.animation_manager.register_character(anim_id, 12);
        // 联动 EcsWorld：注册实体并初始化关键字段（Position/Faction/Health/EntityKind）
        let ecs_id = self.ecs_world.spawn();
        self.npc_ecs_ids.insert(npc_id, ecs_id);
        self.ecs_world.set_component(
            ecs_id,
            crate::ecs::ComponentType::Position,
            crate::ecs::ComponentValue::Vec3(pos),
            crate::ecs::SystemId::External(0),
        );
        self.ecs_world.set_component(
            ecs_id,
            crate::ecs::ComponentType::Faction,
            crate::ecs::ComponentValue::Uint8(faction),
            crate::ecs::SystemId::External(0),
        );
        self.ecs_world.set_component(
            ecs_id,
            crate::ecs::ComponentType::Health,
            crate::ecs::ComponentValue::Float32(100.0),
            crate::ecs::SystemId::External(0),
        );
        self.ecs_world.set_component(
            ecs_id,
            crate::ecs::ComponentType::MaxHealth,
            crate::ecs::ComponentValue::Float32(100.0),
            crate::ecs::SystemId::External(0),
        );
        self.ecs_world.set_component(
            ecs_id,
            crate::ecs::ComponentType::EntityKind,
            crate::ecs::ComponentValue::Uint8(0), // 0 = NPC
            crate::ecs::SystemId::External(0),
        );
        npc_id
    }

    /// 移除 NPC 并联动清理 CombatSystem + AnimationManager + EcsWorld。
    pub fn despawn_npc(&mut self, npc_id: wasteland_game::npc::NpcId) {
        if let Some(anim_id) = self.npc_anim_ids.remove(&npc_id) {
            self.animation_manager.unregister_character(anim_id);
        }
        if let Some(ecs_id) = self.npc_ecs_ids.remove(&npc_id) {
            self.ecs_world.despawn(ecs_id);
        }
        if let Some(combat_id) = self.npc_combat_ids.remove(&npc_id) {
            self.combat_system.entities.remove(combat_id);
        }
        self.npc_manager.remove_npc(npc_id);
    }

    /// 更新气象系统
    pub fn update_weather(&mut self, dt: f32, _ctx: &WorldContext) {
        // Phase 6 fix S2: atmosphere.temperature follows global_temperature (init sync in GameWorld::new)
        // atmosphere.update() still evolves temp toward SEA_LEVEL_TEMP, providing convergence.
        // The reverse sync (global_temperature = atmosphere.temperature) in tick() lets global
        // benefit from atmosphere's convergence so a 500K initial temp settles to ~296K.
        let target_temp = _ctx.global_temperature;
        let convergence_rate = 0.01;
        self.atmosphere.temperature +=
            (target_temp - self.atmosphere.temperature) * convergence_rate * dt;
        self.atmosphere.update(20.0, 0.3, dt);
        self.wind_field.apply_coriolis(45.0, dt);
    }

    /// 更新物理系统
    pub fn update_physics(&mut self, ctx: &WorldContext) {
        self.physics.temperature = FixedPoint::from_f32(ctx.global_temperature);
        self.physics.ambient_radiation = FixedPoint::from_f32(ctx.global_radiation);
        self.physics.wind = FixedVec3::from_glam(ctx.wind);
        self.physics.step();
    }

    /// 更新化学系统
    pub fn update_chemistry(&mut self, dt: f32, time: f64, ctx: &WorldContext) {
        self.chemistry.temperature = ctx.global_temperature;
        self.chemistry.radiation_level = ctx.global_radiation;
        self.chemistry.update(dt, time);
    }

    /// 更新场和粒子系统
    pub fn update_fields_and_particles(&mut self, dt: f32, precipitation: f32) {
        let __prof = false;
        let mut __t0 = std::time::Instant::now();
        macro_rules! Tm {
            ($label:expr) => {
                if __prof {
                    let now = std::time::Instant::now();
                    eprintln!(
                        "[SIM PROFILE] {}: {:.2}ms",
                        $label,
                        now.duration_since(__t0).as_secs_f64() * 1000.0
                    );
                    __t0 = now;
                }
            };
        }
        // Field solver runs at 1/4 frequency (15Hz) with 4x dt to maintain integration accuracy
        self.field_step_counter = self.field_step_counter.wrapping_add(1);
        if self.field_step_counter % 4 == 0 {
            self.coupled_field_solver.step((dt * 4.0) as f64);
        }
        Tm!("coupled_field_solver");
        self.particle_system.step(dt);
        Tm!("particle_system");
        if !self.mpss.is_empty() {
            let near_dist = self.physics_lod.near_field_distance;
            let mid_dist = self.physics_lod.mid_field_distance;
            let player_pos = self.player_position;
            let near_dist_sq = near_dist * near_dist;
            let mid_dist_sq = mid_dist * mid_dist;

            // LOD reclassification: full scan every 8 frames OR when player moved >5m
            // (caches near/mid/far index lists between scans to amortize the 1M-particle cost)
            let player_moved = (player_pos - self.cached_player_pos).length_squared() > 25.0;
            self.lod_reclassify_counter = self.lod_reclassify_counter.wrapping_add(1);
            let need_reclassify = player_moved
                || self.cached_near_indices.is_empty()
                || self.lod_reclassify_counter % 8 == 0;

            if need_reclassify {
                self.cached_player_pos = player_pos;
                let mut near_indices: Vec<(usize, f32)> = Vec::new();
                let mut far_indices: Vec<usize> = Vec::new();
                let mut mid_indices: Vec<usize> = Vec::new();

                for i in 0..self.mpss.count {
                    if !self.mpss.active[i] {
                        continue;
                    }
                    let dx = self.mpss.pos[i][0] - player_pos.x;
                    let dy = self.mpss.pos[i][1] - player_pos.y;
                    let dz = self.mpss.pos[i][2] - player_pos.z;
                    let dist_sq = dx * dx + dy * dy + dz * dz;
                    if dist_sq < near_dist_sq {
                        near_indices.push((i, dist_sq));
                    } else if dist_sq < mid_dist_sq {
                        mid_indices.push(i);
                    } else {
                        far_indices.push(i);
                    }
                }

                // Cull near particles to max_near_particles (keep closest)
                let max_near = self.physics_lod.max_near_particles;
                if near_indices.len() > max_near {
                    near_indices.select_nth_unstable_by(max_near, |a, b| {
                        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let culled_to_mid: Vec<usize> =
                        near_indices[max_near..].iter().map(|(i, _)| *i).collect();
                    mid_indices.extend(culled_to_mid);
                    near_indices.truncate(max_near);
                }
                self.cached_near_indices = near_indices;
                self.cached_mid_indices = mid_indices;
                self.cached_far_indices = far_indices;
            }
            Tm!("lod_classify");

            let near_count = self.cached_near_indices.len();
            let mid_count = self.cached_mid_indices.len();
            let far_count = self.cached_far_indices.len();
            self.lod_stats = LodStats { near: near_count, mid: mid_count, far: far_count };

            // §3.1 Moving Window MPM: center grid on player so near-field particles
            // (within near_field_distance) always fall inside the grid volume.
            // Grid covers [origin, origin + nx*dx]; origin = player - half_extent.
            let half_extent = self.mpm_grid.nx as f32 * self.mpm_grid.dx * 0.5;
            self.mpm_grid.origin = [
                player_pos.x - half_extent,
                player_pos.y - half_extent,
                player_pos.z - half_extent,
            ];

            // §3.2 3-layer grid: center mid-field grid on player (same pattern as near grid).
            // mid grid covers ±mid_field_distance (400m span with 16³/dx=25m).
            let mid_half_extent = self.mpm_grid_mid.nx as f32 * self.mpm_grid_mid.dx * 0.5;
            self.mpm_grid_mid.origin = [
                player_pos.x - mid_half_extent,
                player_pos.y - mid_half_extent,
                player_pos.z - mid_half_extent,
            ];

            // §3.2 3-layer grid: center coarse temperature field on player.
            // Field covers ±400m (800m span with 16³/dx=50m).
            let coarse_half_extent = self.coarse_field_n as f32 * self.coarse_field_dx * 0.5;
            self.coarse_field_origin = [
                player_pos.x - coarse_half_extent,
                player_pos.y - coarse_half_extent,
                player_pos.z - coarse_half_extent,
            ];

            // LOD 差异化更新：近场完整MPM，中/远场简化更新
            if near_count > 0 {
                let near_idx: Vec<usize> =
                    self.cached_near_indices.iter().map(|(i, _)| *i).collect();
                wasteland_particle::mpm_solver::mpm_step_parallel_with_grid(
                    &mut self.mpss,
                    &mut self.mpm_grid,
                    &self.mpm_config,
                    &self.material_table,
                    &self.model_table,
                    dt,
                    &near_idx,
                );
                Tm!("mpm_step_parallel");
            }

            // §3.3 FrequencyScheduler extension: mid/far particles run at reduced rates
            // to match their visual acuity requirements (ARCHITECTURE_V7.md §3.3).
            //   mid (50-200m): 10Hz (every 6 frames), dt scaled by 6 to preserve integration
            //   far (>200m):   1Hz  (every far_field_update_interval frames, dt scaled to match)
            // Previously both ran at 60Hz (mid with normal dt, far with dt*4 = effective 15Hz
            // accuracy but 60Hz call cost). This saves ~6x mid CPU and ~60x far CPU.
            const MID_INTERVAL: u32 = 12;
            let far_interval = self.physics_lod.far_field_update_interval.max(1);

            // §3.2 3-layer grid: mid particles use simplified MPM (velocity-only) on
            // mpm_grid_mid. This gives pressure coupling (fluid flow, smoke drift) without
            // the cost of full MPM stress/strain computation. ~3-5x faster than full MPM.
            // Runs at 10Hz with dt scaled by MID_INTERVAL to preserve integration accuracy.
            if self.field_step_counter % MID_INTERVAL == 0 && mid_count > 0 {
                let mid_dt = dt * MID_INTERVAL as f32;
                wasteland_particle::mpm_solver::mpm_step_velocity_only_parallel(
                    &mut self.mpss,
                    &mut self.mpm_grid_mid,
                    &self.mpm_config,
                    mid_dt,
                    &self.cached_mid_indices,
                );
            }
            Tm!("mid_field_update");

            // §3.2 3-layer grid: far particles (>200m) sample temperature from coarse_field.
            // Gravity + position integration (y-axis only, far particles are 2D-ish from player view).
            // Temperature is sampled from coarse_temperature field (16³/dx=50m covering 800m)
            // so large-scale heat conduction (fire/blast thermal effects) reaches far particles.
            if self.field_step_counter % far_interval == 0 {
                let far_dt = dt * far_interval as f32;
                let coarse_n = self.coarse_field_n;
                let coarse_dx = self.coarse_field_dx;
                let coarse_origin = self.coarse_field_origin;
                let coarse_temp = &self.coarse_temperature[..];
                if far_count > 10_000 {
                    let far_idx = &self.cached_far_indices[..];
                    // SAFETY: each i is unique (cached_far_indices contains distinct particle indices).
                    let vel_addr = self.mpss.vel.as_mut_ptr() as usize;
                    let pos_addr = self.mpss.pos.as_mut_ptr() as usize;
                    let temp_addr = self.mpss.temperature.as_mut_ptr() as usize;
                    far_idx.par_iter().for_each(move |&i| unsafe {
                        let v = &mut *(vel_addr as *mut [f32; 3]).add(i);
                        let p = &mut *(pos_addr as *mut [f32; 3]).add(i);
                        let t = &mut *(temp_addr as *mut f32).add(i);
                        v[1] -= 9.81 * far_dt;
                        p[1] += v[1] * far_dt;
                        // Sample coarse temperature field (trilinear-ish via nearest cell)
                        let gx = ((p[0] - coarse_origin[0]) / coarse_dx) as isize;
                        let gy = ((p[1] - coarse_origin[1]) / coarse_dx) as isize;
                        let gz = ((p[2] - coarse_origin[2]) / coarse_dx) as isize;
                        if gx >= 0
                            && gy >= 0
                            && gz >= 0
                            && (gx as usize) < coarse_n
                            && (gy as usize) < coarse_n
                            && (gz as usize) < coarse_n
                        {
                            let gid = gx as usize
                                + gy as usize * coarse_n
                                + gz as usize * coarse_n * coarse_n;
                            if gid < coarse_temp.len() {
                                // Blend toward field temperature (slow convergence, far particles)
                                let field_t = coarse_temp[gid];
                                *t += (field_t - *t) * 0.02 * far_dt;
                            }
                        }
                    });
                } else {
                    for &i in &self.cached_far_indices {
                        self.mpss.vel[i][1] -= 9.81 * far_dt;
                        self.mpss.pos[i][1] += self.mpss.vel[i][1] * far_dt;
                        // Sample coarse temperature field
                        let p = self.mpss.pos[i];
                        let gx = ((p[0] - coarse_origin[0]) / coarse_dx) as isize;
                        let gy = ((p[1] - coarse_origin[1]) / coarse_dx) as isize;
                        let gz = ((p[2] - coarse_origin[2]) / coarse_dx) as isize;
                        if gx >= 0
                            && gy >= 0
                            && gz >= 0
                            && (gx as usize) < coarse_n
                            && (gy as usize) < coarse_n
                            && (gz as usize) < coarse_n
                        {
                            let gid = gx as usize
                                + gy as usize * coarse_n
                                + gz as usize * coarse_n * coarse_n;
                            if gid < self.coarse_temperature.len() {
                                let field_t = self.coarse_temperature[gid];
                                let t = &mut self.mpss.temperature[i];
                                *t += (field_t - *t) * 0.02 * far_dt;
                            }
                        }
                    }
                }
            }
            Tm!("far_field_update");

            // §3.2 3-layer grid: update coarse temperature field at 1Hz (same cadence as far).
            // P2G inject from near+mid particles, then FTCS diffuse across 16³ grid.
            if self.field_step_counter % far_interval == 0 {
                let far_dt = dt * far_interval as f32;
                self.update_coarse_field(far_dt);
            }
            Tm!("coarse_field_update");

            // 更新生命周期（所有粒子）
            self.mpss.update_lifetimes(dt);
            Tm!("update_lifetimes");
        } else {
            self.mpss.apply_gravity(9.81, dt);
            self.mpss.integrate_positions(dt);
            self.mpss.update_lifetimes(dt);
        }
        self.emergence.step(dt, precipitation);
        self.npc_manager.step(dt, self.player_position, &[]);
        // NPC.position → CombatEntity 同步（让 combat 用最新位置判伤害范围）
        let mut npc_pos_sync: Vec<(wasteland_game::combat::CombatEntityId, Vec3)> = Vec::new();
        for (npc_id, npc) in &self.npc_manager.npcs {
            if let Some(combat_id) = self.npc_combat_ids.get(&npc_id).copied() {
                npc_pos_sync.push((combat_id, npc.position));
            }
        }
        for (combat_id, pos) in npc_pos_sync {
            if let Some(entity) = self.combat_system.entities.get_mut(combat_id) {
                entity.position = pos;
            }
        }
        self.combat_system.update(dt);
        // CombatEntity.health → NPC 同步（让 NPC 行为读最新血量：逃跑/死亡判定）
        let mut health_sync: Vec<(wasteland_game::npc::NpcId, f32)> = Vec::new();
        for (npc_id, combat_id) in &self.npc_combat_ids {
            if let Some(entity) = self.combat_system.entities.get(*combat_id) {
                health_sync.push((*npc_id, entity.health));
            }
        }
        for (npc_id, health) in health_sync {
            if let Some(npc) = self.npc_manager.npcs.get_mut(npc_id) {
                npc.health = health;
            }
        }
        // 同步 NPC 状态到 AnimationManager + EcsWorld（health 现在是最新值）
        // slotmap 迭代器 yield owned copy 的 NpcId，HashMap::get 需要传引用
        let mut npc_sync: Vec<(u64, uuid::Uuid, Vec3, f32, u8)> = Vec::new();
        for (npc_id, npc) in &self.npc_manager.npcs {
            let anim_id = self.npc_anim_ids.get(&npc_id).copied();
            let ecs_id = self.npc_ecs_ids.get(&npc_id).copied();
            if let (Some(anim_id), Some(ecs_id)) = (anim_id, ecs_id) {
                npc_sync.push((anim_id, ecs_id, npc.position, npc.health, npc.animation_state));
            }
        }
        for (anim_id, ecs_id, pos, health, anim_state) in npc_sync {
            self.animation_manager.set_character_state_u8(anim_id, anim_state);
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Position,
                crate::ecs::ComponentValue::Vec3(pos),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Health,
                crate::ecs::ComponentValue::Float32(health),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::AnimationState,
                crate::ecs::ComponentValue::Uint8(anim_state),
                crate::ecs::SystemId::External(0),
            );
        }
        // 同步 Projectile/DamageZone 到 EcsWorld（短生命周期实体每帧重建，避免维护 ID 映射）
        self.ecs_world.despawn_by_kind(2); // Projectile
        self.ecs_world.despawn_by_kind(3); // DamageZone
        for proj in &self.combat_system.projectiles {
            let ecs_id = self.ecs_world.spawn();
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Position,
                crate::ecs::ComponentValue::Vec3(proj.position),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Velocity,
                crate::ecs::ComponentValue::Vec3(proj.velocity),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Faction,
                crate::ecs::ComponentValue::Uint8(proj.faction),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::EntityKind,
                crate::ecs::ComponentValue::Uint8(2),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Lifetime,
                crate::ecs::ComponentValue::Float32(proj.max_lifetime),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Age,
                crate::ecs::ComponentValue::Float32(proj.lifetime),
                crate::ecs::SystemId::External(0),
            );
        }
        for zone in &self.combat_system.damage_zones {
            let ecs_id = self.ecs_world.spawn();
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Position,
                crate::ecs::ComponentValue::Vec3(zone.center),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Faction,
                crate::ecs::ComponentValue::Uint8(zone.faction),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::Radius,
                crate::ecs::ComponentValue::Float32(zone.radius),
                crate::ecs::SystemId::External(0),
            );
            self.ecs_world.set_component(
                ecs_id,
                crate::ecs::ComponentType::EntityKind,
                crate::ecs::ComponentValue::Uint8(3),
                crate::ecs::SystemId::External(0),
            );
        }
        // 战斗事件桥接到情绪引擎（全局情绪，不区分 NPC；读取 events 不消费）
        let current_time = self.field_step_counter as f32 * dt;
        for event in &self.combat_system.events {
            match event {
                wasteland_game::combat::CombatEvent::EntityDamaged { damage, .. } => {
                    // 伤害值归一化到 [0, 1] 作为情绪强度
                    let intensity = (*damage / 100.0).clamp(0.0, 1.0);
                    self.emotion_engine.trigger_event("damage_taken", intensity, current_time);
                },
                wasteland_game::combat::CombatEvent::EntityKilled { .. } => {
                    self.emotion_engine.trigger_event("enemy_died", 0.5, current_time);
                },
                _ => {},
            }
        }
        self.emotion_engine.update(dt);
        self.navigation_manager.step_orca(dt);
        self.animation_manager.step(dt);
        self.interaction_system.step(dt, self.player_position);
        let _reinforcements = self.reinforcement_system.step(dt);
        Tm!("post_step");
    }

    /// 更新流体、声学系统（6Hz 频率）
    ///
    /// 地质系统（erosion/tectonic/surface_runoff）已分离到 `update_geology`，以 0.1Hz 跑。
    /// `precipitation` 参数保留用于未来流体耦合（当前 fluid_solver.step() 不消费它）。
    pub fn update_fluid_acoustic_geo(&mut self, dt: f32, _precipitation: f32) {
        self.fluid_solver.step();
        self.acoustic_solver.step(dt);
    }

    /// 更新地质系统（0.1Hz 频率）
    ///
    /// 侵蚀、构造、地表径流本质是慢过程，6Hz 跑是浪费 CPU。每 600 帧（10s）跑一次，
    /// dt 放大 100 倍（dt_0.1hz = dt * 100.0）以保持积分稳定性。
    pub fn update_geology(&mut self, dt: f32, precipitation: f32) {
        self.erosion_solver.step(&ErosionParams::default(), RockType::Granite, dt);
        self.tectonic_solver.step(dt);
        self.surface_runoff.update(precipitation, 0.3, -Vec3::Y, dt);
    }

    /// §3.2 3-layer grid: update coarse temperature field for far particles.
    ///
    /// Two phases:
    ///   1. P2G: inject temperature from near+mid particles into coarse field cells
    ///      (weighted average — hot particles raise cell temp, cold particles lower it)
    ///   2. FTCS diffusion: 6-neighbor Laplacian diffusion with small diffusion coefficient
    ///
    /// Runs at 1Hz (same as far_field_update_interval) — coarse field is large-scale,
    /// sub-second updates are imperceptible. dt scaled to match 1Hz cadence.
    pub fn update_coarse_field(&mut self, dt: f32) {
        let n = self.coarse_field_n;
        let dx = self.coarse_field_dx;
        let origin = self.coarse_field_origin;
        let inv_dx = 1.0 / dx;

        // Phase 1: P2G temperature injection from NEAR particles only (10k vs 500k).
        // Mid particles (50-200m) have their temperature evolved locally via mid MPM
        // grid; their contribution to far-field temperature is negligible because:
        //   1. Far field covers 800m span — mid particles at 50-200m are <25% of span
        //   2. Blend rate is 0.1/s — only 10% per second convergence anyway
        //   3. Sampling 10k near particles vs 500k near+mid saves ~9ms per call
        // Hot near particles (fire, explosion) still propagate to far field via FTCS
        // diffusion (Phase 2), which spreads heat across the 16³ grid each tick.
        let mut temp_accum = vec![0.0f32; n * n * n];
        let mut mass_accum = vec![0.0f32; n * n * n];

        for &(i, _) in &self.cached_near_indices {
            if !self.mpss.active[i] {
                continue;
            }
            let p = self.mpss.pos[i];
            let gx = ((p[0] - origin[0]) * inv_dx) as isize;
            let gy = ((p[1] - origin[1]) * inv_dx) as isize;
            let gz = ((p[2] - origin[2]) * inv_dx) as isize;
            if gx < 0 || gy < 0 || gz < 0 {
                continue;
            }
            let (gx, gy, gz) = (gx as usize, gy as usize, gz as usize);
            if gx >= n || gy >= n || gz >= n {
                continue;
            }
            let gid = gx + gy * n + gz * n * n;
            let m = self.mpss.mass[i].max(1e-6);
            temp_accum[gid] += self.mpss.temperature[i] * m;
            mass_accum[gid] += m;
        }

        // Blend injected temperature with existing field (relaxation toward particle temp)
        let inject_rate = 0.1; // 10% per second convergence
        let blend = (inject_rate * dt).min(1.0);
        for i in 0..n * n * n {
            if mass_accum[i] > 1e-6 {
                let particle_temp = temp_accum[i] / mass_accum[i];
                self.coarse_temperature[i] += (particle_temp - self.coarse_temperature[i]) * blend;
            }
        }

        // Phase 2: FTCS diffusion (6-neighbor Laplacian)
        // Stability: alpha * dt / dx² <= 1/6 for 3D FTCS. With alpha=0.1, dt=1, dx=50:
        //   0.1 * 1 / 2500 = 4e-5 << 1/6, stable. Use alpha=10 for visible diffusion.
        let alpha = 10.0f32;
        let coef = (alpha * dt / (dx * dx)).min(0.16); // stability cap
        let mut new_temp = self.coarse_temperature.clone();
        for k in 0..n {
            for j in 0..n {
                for i in 0..n {
                    let gid = i + j * n + k * n * n;
                    let mut lap = 0.0f32;
                    let mut count = 0;
                    if i > 0 {
                        lap += self.coarse_temperature[gid - 1];
                        count += 1;
                    }
                    if i + 1 < n {
                        lap += self.coarse_temperature[gid + 1];
                        count += 1;
                    }
                    if j > 0 {
                        lap += self.coarse_temperature[gid - n];
                        count += 1;
                    }
                    if j + 1 < n {
                        lap += self.coarse_temperature[gid + n];
                        count += 1;
                    }
                    if k > 0 {
                        lap += self.coarse_temperature[gid - n * n];
                        count += 1;
                    }
                    if k + 1 < n {
                        lap += self.coarse_temperature[gid + n * n];
                        count += 1;
                    }
                    if count > 0 {
                        let avg = lap / count as f32;
                        new_temp[gid] += coef * (avg - self.coarse_temperature[gid]) * count as f32;
                    }
                }
            }
        }
        self.coarse_temperature = new_temp;
    }

    /// 更新热力学系统
    pub fn update_thermal(&mut self, dt: f32, global_temperature: f32) {
        self.conduction_solver.ambient_temperature = global_temperature;
        self.convection_solver.ambient_temperature = global_temperature;
        self.radiation_solver.ambient_temperature = global_temperature;
        self.conduction_solver.time_step = dt;
        self.convection_solver.time_step = dt;
        self.radiation_solver.time_step = dt;
    }
}
