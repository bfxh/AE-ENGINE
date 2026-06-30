//! GameWorld ↔ nova_render 桥接（trait 解耦版）
//!
//! 从 GameWorld 提取渲染所需数据：
//! - meta entity 位置/颜色（铁球/水球/混凝土球等）
//! - MPSS 粒子（近/中/远场）
//! - voxel grid 数据
//! - NPC 渲染数据
//! - 物化生场数据（环境/场/生态/涌现/光学/域隔离）
//!
//! 设计：
//! - 通过 `GameWorldSource` trait 解耦，nova_render 不直接依赖 ae_engine
//! - game crate 负责为 ae_engine::GameWorld impl GameWorldSource
//! - 由 nova_render 的 Extract 阶段调用

// ============================================================================
// 数据结构（GPU 无关，纯数据）
// ============================================================================

/// 提取的场景数据
pub struct ExtractedSceneData {
    pub meta_entities: Vec<MetaEntityRender>,
    pub near_particles: Vec<ParticleRender>,
    pub mid_far_particles: Vec<PointRender>,
    pub voxel_grids: Vec<VoxelGridRender>,
    pub npcs: Vec<NpcRender>,

    pub weather: WeatherData,
    pub wind: [f32; 3],
    pub global_temperature: f32,
    pub global_radiation: f32,
    pub active_reactions: usize,
    pub time_of_day: f32,

    pub chemical_field: FieldGrid,
    pub bioactivity_field: FieldGrid,
    pub electric_field: VecFieldGrid,

    pub flora: Vec<([f32; 3], u32, f32)>,
    pub organisms: Vec<([f32; 3], u32, f32)>,
    pub populations: Vec<PopulationRender>,

    pub cracks: Vec<([f32; 3], [f32; 3], f32)>,
    pub rust_spots: Vec<([f32; 3], f32)>,

    pub blackbody_rgb: [f32; 3],
    pub dynamic_lights: Vec<DynamicLightRender>,

    pub domain_zones: Vec<DomainZoneRender>,
}

#[derive(Debug, Clone)]
pub struct MetaEntityRender {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub radius: f32,
}

#[derive(Debug, Clone)]
pub struct ParticleRender {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct PointRender {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub size: f32,
}

#[derive(Debug, Clone)]
pub struct VoxelGridRender {
    pub center: [f32; 3],
    pub half_extent: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct NpcRender {
    pub position: [f32; 3],
    pub rotation_y: f32,
    pub color: [f32; 4],
    pub species_id: u32,
}

#[derive(Debug, Clone, Default)]
pub struct WeatherData {
    pub precipitation: f32,
    pub cloud_cover: f32,
    pub visibility: f32,
}

#[derive(Debug, Clone, Default)]
pub struct FieldGrid {
    pub origin: [f32; 3],
    pub step: f32,
    pub count: [usize; 2],
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct VecFieldGrid {
    pub origin: [f32; 3],
    pub step: f32,
    pub count: [usize; 2],
    pub values: Vec<[f32; 3]>,
}

#[derive(Debug, Clone)]
pub struct PopulationRender {
    pub species_id: String,
    pub count: usize,
    pub positions: Vec<[f32; 3]>,
}

#[derive(Debug, Clone)]
pub struct DynamicLightRender {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub power: f32,
}

#[derive(Debug, Clone)]
pub struct DomainZoneRender {
    pub center: [f32; 3],
    pub radius_outer: f32,
    pub domain: u8,
}

impl Default for ExtractedSceneData {
    fn default() -> Self {
        Self {
            meta_entities: Vec::new(),
            near_particles: Vec::new(),
            mid_far_particles: Vec::new(),
            voxel_grids: Vec::new(),
            npcs: Vec::new(),
            weather: WeatherData::default(),
            wind: [0.0; 3],
            global_temperature: 0.0,
            global_radiation: 0.0,
            active_reactions: 0,
            time_of_day: 0.0,
            chemical_field: FieldGrid::default(),
            bioactivity_field: FieldGrid::default(),
            electric_field: VecFieldGrid::default(),
            flora: Vec::new(),
            organisms: Vec::new(),
            populations: Vec::new(),
            cracks: Vec::new(),
            rust_spots: Vec::new(),
            blackbody_rgb: [0.0; 3],
            dynamic_lights: Vec::new(),
            domain_zones: Vec::new(),
        }
    }
}

// ============================================================================
// GameWorldSource trait（解耦 ae_engine）
// ============================================================================
//
// game crate 负责 impl GameWorldSource for ae_engine::GameWorld。
// nova_render 只依赖此 trait，不依赖 ae_engine。

/// 世界统计信息
#[derive(Debug, Clone, Default)]
pub struct WorldStats {
    pub global_temperature: f32,
    pub global_radiation: f32,
    pub active_reactions: usize,
    pub time: f32,
}

/// 世界边界
#[derive(Debug, Clone, Default)]
pub struct WorldBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// 域隔离信息
#[derive(Debug, Clone)]
pub struct DomainZoneInfo {
    pub center: [f32; 3],
    pub radius_outer: f32,
    pub domain: u8,
}

/// GameWorld 数据源 trait
///
/// 由上层（game crate）为具体引擎类型实现。
pub trait GameWorldSource {
    fn meta_entity_positions(&self) -> Vec<[f32; 3]>;
    fn meta_entity_colors(&self) -> Vec<[f32; 4]>;
    fn mpss_render_data(&self) -> Vec<([f32; 3], [f32; 4])>;
    fn mpss_mid_far_render_data(&self) -> Vec<([f32; 3], [f32; 4], f32)>;
    fn voxel_grid_count(&self) -> usize;
    fn voxel_mesh_data(&self, grid_i: usize) -> Vec<([f32; 3], [f32; 4])>;
    fn npc_positions(&self) -> Vec<[f32; 3]>;
    fn npc_colors(&self) -> Vec<[f32; 4]>;
    fn precipitation(&self) -> f32;
    fn cloud_cover(&self) -> f32;
    fn visibility(&self) -> f32;
    fn wind_vector(&self) -> (f32, f32, f32);
    fn stats(&self) -> WorldStats;
    fn world_bounds(&self) -> WorldBounds;
    fn field_value_at(&self, name: &str, x: f32, y: f32, z: f32) -> f32;
    fn electric_field_at(&self, x: f32, y: f32, z: f32) -> (f32, f32, f32);
    fn ecosystem_count(&self) -> usize;
    fn flora_data(&self, i: usize) -> Vec<([f32; 3], u32, f32)>;
    fn organism_data(&self, i: usize) -> Vec<([f32; 3], u32, f32)>;
    fn population_count(&self) -> usize;
    fn population_species(&self, i: usize) -> String;
    fn population_size(&self, i: usize) -> usize;
    fn population_positions(&self, i: usize) -> Vec<(f32, f32, f32)>;
    fn crack_data(&self) -> Vec<([f32; 3], [f32; 3], f32)>;
    fn rust_data(&self) -> Vec<([f32; 3], f32)>;
    fn compute_blackbody_rgb(&self, temperature: f32) -> (f32, f32, f32);
    fn domain_zones(&self) -> Vec<DomainZoneInfo>;
}

// ============================================================================
// GameWorldExtractor
// ============================================================================

pub struct GameWorldExtractor;

impl GameWorldExtractor {
    /// 从 GameWorldSource 提取一帧的渲染数据
    pub fn extract(world: &dyn GameWorldSource) -> ExtractedSceneData {
        let mut data = ExtractedSceneData::default();

        // 1. Meta entities
        let positions = world.meta_entity_positions();
        let colors = world.meta_entity_colors();
        for (pos, col) in positions.iter().zip(colors.iter()) {
            data.meta_entities.push(MetaEntityRender {
                position: *pos,
                color: *col,
                radius: 2.0,
            });
        }

        // 2. Near MPSS particles
        for (pos, col) in world.mpss_render_data() {
            data.near_particles.push(ParticleRender { position: pos, color: col });
        }

        // 3. Mid/Far MPSS particles
        for (pos, col, size) in world.mpss_mid_far_render_data() {
            data.mid_far_particles.push(PointRender { position: pos, color: col, size });
        }

        // 4. Voxel grids
        let grid_count = world.voxel_grid_count();
        for grid_i in 0..grid_count {
            for (pos, col) in world.voxel_mesh_data(grid_i) {
                data.voxel_grids.push(VoxelGridRender {
                    center: pos,
                    half_extent: [0.5, 0.5, 0.5],
                    color: col,
                });
            }
        }

        // 5. NPCs
        let npc_positions = world.npc_positions();
        let npc_colors = world.npc_colors();
        for (pos, col) in npc_positions.iter().zip(npc_colors.iter()) {
            data.npcs.push(NpcRender {
                position: *pos,
                rotation_y: 0.0,
                color: *col,
                species_id: 0,
            });
        }

        // 6. 环境
        data.weather = WeatherData {
            precipitation: world.precipitation(),
            cloud_cover: world.cloud_cover(),
            visibility: world.visibility(),
        };
        let (wx, wy, wz) = world.wind_vector();
        data.wind = [wx, wy, wz];

        let stats = world.stats();
        data.global_temperature = stats.global_temperature;
        data.global_radiation = stats.global_radiation;
        data.active_reactions = stats.active_reactions;
        data.time_of_day = (stats.time % 86400.0) / 3600.0;

        // 7. 场数据（2D 网格采样）
        let bounds = world.world_bounds();
        let min_x = bounds.min[0];
        let max_x = bounds.max[0];
        let min_z = bounds.min[2];

        const FIELD_N: usize = 48;
        let field_span = (max_x - min_x).max(1.0);
        let field_step = field_span / FIELD_N as f32;
        let field_origin = [min_x, 0.0, min_z];
        let mut chem_values = Vec::with_capacity(FIELD_N * FIELD_N);
        let mut bio_values = Vec::with_capacity(FIELD_N * FIELD_N);
        for iz in 0..FIELD_N {
            let z = min_z + (iz as f32 + 0.5) * field_step;
            for ix in 0..FIELD_N {
                let x = min_x + (ix as f32 + 0.5) * field_step;
                chem_values.push(world.field_value_at("chemical", x, 0.0, z));
                bio_values.push(world.field_value_at("bioactivity", x, 0.0, z));
            }
        }
        data.chemical_field = FieldGrid {
            origin: field_origin,
            step: field_step,
            count: [FIELD_N, FIELD_N],
            values: chem_values,
        };
        data.bioactivity_field = FieldGrid {
            origin: field_origin,
            step: field_step,
            count: [FIELD_N, FIELD_N],
            values: bio_values,
        };

        const EFIELD_N: usize = 24;
        let espan = (max_x - min_x).max(1.0);
        let estep = espan / EFIELD_N as f32;
        let eorigin = [min_x, 5.0, min_z];
        let mut evals = Vec::with_capacity(EFIELD_N * EFIELD_N);
        for iz in 0..EFIELD_N {
            let z = min_z + (iz as f32 + 0.5) * estep;
            for ix in 0..EFIELD_N {
                let x = min_x + (ix as f32 + 0.5) * estep;
                let (ex, ey, ez) = world.electric_field_at(x, 5.0, z);
                evals.push([ex, ey, ez]);
            }
        }
        data.electric_field = VecFieldGrid {
            origin: eorigin,
            step: estep,
            count: [EFIELD_N, EFIELD_N],
            values: evals,
        };

        // 8. 生态
        let eco_count = world.ecosystem_count();
        for i in 0..eco_count {
            for f in world.flora_data(i) {
                data.flora.push(f);
            }
            for o in world.organism_data(i) {
                data.organisms.push(o);
            }
        }
        let pop_count = world.population_count();
        for i in 0..pop_count {
            let species = world.population_species(i);
            let size = world.population_size(i);
            let positions = world
                .population_positions(i)
                .into_iter()
                .map(|(x, y, z)| [x, y, z])
                .collect();
            data.populations.push(PopulationRender {
                species_id: species,
                count: size,
                positions,
            });
        }

        // 9. 涌现几何
        data.cracks = world.crack_data();
        data.rust_spots = world.rust_data();

        // 10. 光学
        let (r, g, b) = world.compute_blackbody_rgb(stats.global_temperature);
        data.blackbody_rgb = [r, g, b];
        data.dynamic_lights = Vec::new();

        // 11. 域隔离
        for z in world.domain_zones() {
            data.domain_zones.push(DomainZoneRender {
                center: z.center,
                radius_outer: z.radius_outer,
                domain: z.domain,
            });
        }

        data
    }
}

// ============================================================================
// EngineBridge
// ============================================================================

pub struct EngineBridge {
    last_extracted: ExtractedSceneData,
}

impl EngineBridge {
    pub fn new() -> Self {
        Self { last_extracted: ExtractedSceneData::default() }
    }

    pub fn extract(&mut self, world: &dyn GameWorldSource) {
        self.last_extracted = GameWorldExtractor::extract(world);
    }

    pub fn data(&self) -> &ExtractedSceneData {
        &self.last_extracted
    }

    pub fn meta_entity_count(&self) -> usize {
        self.last_extracted.meta_entities.len()
    }

    pub fn particle_count(&self) -> usize {
        self.last_extracted.near_particles.len() + self.last_extracted.mid_far_particles.len()
    }
}

impl Default for EngineBridge {
    fn default() -> Self { Self::new() }
}
