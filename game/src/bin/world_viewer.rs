//! Wasteland 大世界 3D Viewer
//!
//! 基于 GameWorld (2km x 600m x 2km) 的实时 3D 可视化：
//! - 建筑：6 种程序化建筑（民居/工业/军事/商业/公共/避难所）
//! - NPC：程序化人形 + GameWorld NPC 系统（位置/颜色实时同步）
//! - 异形生物：8 种母巢子实体（追猎者/碎脊者/锈骑士/蜂群/臃肿者/窃听者/编织者/践踏者）
//! - 生态：多 biome 生态系统（flora/organism 可视化）
//! - 化学/生物：GameWorld 内部系统 + stats 实时显示
//! - 物理：voxel grid + meta entity + MPSS 粒子
//!
//! 控制：
//! - 鼠标左键拖拽旋转，滚轮缩放
//! - ESC 退出，Space 暂停
//! - 1-5 切换视图模式（综合/温度/辐射/化学/生物）
//! - ↑↓ 改变全局温度 ±10K
//! - ←→ 改变仿真速度
//! - E 在原点触发爆炸
//! - R 重置

#![allow(clippy::too_many_arguments)]

use std::time::Instant;

use glam::{Mat4, Vec3};
use ae_engine::{Biome, GameWorld, MaterialProperties, NpcSpecies, WorldBounds};
use ae_engine::managers::domain_isolation::IsolationDomain;
use ae_terrain::heightmap::Heightmap;
use ae_terrain::noise::PermutationTable;
use ae_render::{
    CameraUniform, InstancedRenderer, InstanceData, LightUniform, MeshInstanceData, MeshRenderer,
    PointInstanceData, PostProcessParams, PostProcessRenderer, SkyboxRenderer, SurfaceRenderer,
    WaterRenderer,
    procedural::{
        all_templates, BuildingGenerator, BuildingParams, BuildingType, MorphGenerator,
        NpcBodyGenerator, NpcBodyParams,
    },
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

const WINDOW_WIDTH: u32 = 1600;
const WINDOW_HEIGHT: u32 = 900;
const MAX_CUBE_INSTANCES: usize = 50_000;
const MAX_POINTS: usize = 200_000;
const MAX_MESH_INSTANCES: usize = 5_000;

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Composite,
    Thermal,
    Radiation,
    Chemical,
    Biological,
    ChemicalField,
    BioField,
    Population,
}

impl ViewMode {
    fn label(self) -> &'static str {
        match self {
            ViewMode::Composite => "Composite",
            ViewMode::Thermal => "Thermal",
            ViewMode::Radiation => "Radiation",
            ViewMode::Chemical => "Chemical",
            ViewMode::Biological => "Biological",
            ViewMode::ChemicalField => "ChemField",
            ViewMode::BioField => "BioField",
            ViewMode::Population => "Population",
        }
    }
    fn next(self) -> Self {
        match self {
            ViewMode::Composite => ViewMode::Thermal,
            ViewMode::Thermal => ViewMode::Radiation,
            ViewMode::Radiation => ViewMode::Chemical,
            ViewMode::Chemical => ViewMode::Biological,
            ViewMode::Biological => ViewMode::ChemicalField,
            ViewMode::ChemicalField => ViewMode::BioField,
            ViewMode::BioField => ViewMode::Population,
            ViewMode::Population => ViewMode::Composite,
        }
    }
}

struct CameraState {
    target: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
    aspect: f32,
    fov: f32,
}

impl CameraState {
    fn position(&self) -> Vec3 {
        let cos_p = self.pitch.cos();
        self.target
            + Vec3::new(
                self.distance * cos_p * self.yaw.sin(),
                self.distance * self.pitch.sin(),
                self.distance * cos_p * self.yaw.cos(),
            )
    }
    fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }
    fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, 0.5, 5000.0)
    }
    fn view_proj(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
    fn uniform(&self) -> CameraUniform {
        let pos = self.position();
        CameraUniform {
            view_proj: self.view_proj().to_cols_array_2d(),
            view: self.view_matrix().to_cols_array_2d(),
            proj: self.projection_matrix().to_cols_array_2d(),
            position: [pos.x, pos.y, pos.z, 1.0],
        }
    }
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, 20.0, 0.0),
            distance: 180.0,
            yaw: 0.785,
            pitch: 0.45,
            aspect: WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
            fov: std::f32::consts::FRAC_PI_4,
        }
    }
}

#[derive(Default)]
struct InputState {
    mouse_dragging: bool,
    last_mouse_x: f32,
    last_mouse_y: f32,
}

fn temp_to_color(t: f32) -> [f32; 4] {
    if t < 250.0 {
        [0.0, 0.0, 0.4, 1.0]
    } else if t < 300.0 {
        let f = (t - 250.0) / 50.0;
        [0.0, f * 0.5, 0.4 + f * 0.4, 1.0]
    } else if t < 373.0 {
        let f = (t - 300.0) / 73.0;
        [0.0, 0.5 + f * 0.5, 0.8 - f * 0.2, 1.0]
    } else if t < 500.0 {
        let f = (t - 373.0) / 127.0;
        [f, 1.0, 0.6 - f * 0.6, 1.0]
    } else if t < 1000.0 {
        let f = (t - 500.0) / 500.0;
        [1.0, 1.0 - f * 0.5, 0.0, 1.0]
    } else if t < 2000.0 {
        let f = (t - 1000.0) / 1000.0;
        [1.0, 0.5 - f * 0.5, 0.0, 1.0]
    } else if t < 5000.0 {
        let f = (t - 2000.0) / 3000.0;
        [1.0, f, f * 0.5, 1.0]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    }
}

fn radiation_to_color(r: f32) -> [f32; 4] {
    if r < 0.1 {
        [0.0, 0.7, 0.0, 1.0]
    } else if r < 1.0 {
        let f = r;
        [f * 0.8, 0.7, 0.0, 1.0]
    } else if r < 5.0 {
        let f = (r - 1.0) / 4.0;
        [0.8 + f * 0.2, 0.7 - f * 0.5, 0.0, 1.0]
    } else {
        let f = ((r - 5.0) / 15.0).min(1.0);
        [1.0, 0.2 - f * 0.2, 0.0, 1.0]
    }
}

fn chemical_to_color(reactions: usize) -> [f32; 4] {
    if reactions == 0 {
        [0.4, 0.4, 0.4, 1.0]
    } else if reactions < 10 {
        let f = reactions as f32 / 10.0;
        [0.4, 0.4 - f * 0.2, 0.4 + f * 0.4, 1.0]
    } else if reactions < 50 {
        let f = (reactions - 10) as f32 / 40.0;
        [0.4 + f * 0.5, 0.2, 0.8, 1.0]
    } else {
        let f = ((reactions - 50) as f32 / 50.0).min(1.0);
        [0.9 + f * 0.1, 0.2, 0.8 + f * 0.2, 1.0]
    }
}

fn biological_to_color(organisms: usize) -> [f32; 4] {
    if organisms == 0 {
        [0.4, 0.3, 0.2, 1.0]
    } else if organisms < 50 {
        let f = organisms as f32 / 50.0;
        [0.4 - f * 0.3, 0.3 + f * 0.4, 0.2, 1.0]
    } else if organisms < 200 {
        let f = (organisms - 50) as f32 / 150.0;
        [0.1, 0.7 + f * 0.2, 0.2 + f * 0.3, 1.0]
    } else {
        let f = ((organisms - 200) as f32 / 300.0).min(1.0);
        [0.1, 0.9, 0.5 + f * 0.5, 1.0]
    }
}

/// Map chemical concentration (0..1+) to color (deep blue -> magenta -> red -> yellow)
fn chemical_field_to_color(v: f32) -> [f32; 4] {
    if v <= 0.0 {
        [0.05, 0.05, 0.15, 0.6]
    } else if v < 0.25 {
        let f = v / 0.25;
        [0.05 + f * 0.3, 0.05, 0.15 + f * 0.5, 0.6 + f * 0.2]
    } else if v < 0.5 {
        let f = (v - 0.25) / 0.25;
        [0.35 + f * 0.5, 0.05, 0.65 - f * 0.65, 0.8]
    } else if v < 0.75 {
        let f = (v - 0.5) / 0.25;
        [0.85, 0.05 + f * 0.4, 0.0, 0.85]
    } else if v < 1.0 {
        let f = (v - 0.75) / 0.25;
        [0.85 + f * 0.15, 0.45 + f * 0.5, 0.0, 0.9]
    } else {
        [1.0, 1.0, 0.4, 0.95]
    }
}

/// Map bioactivity (0..1+) to color (dark brown -> green -> cyan -> white)
fn bio_field_to_color(v: f32) -> [f32; 4] {
    if v <= 0.0 {
        [0.1, 0.05, 0.0, 0.5]
    } else if v < 0.3 {
        let f = v / 0.3;
        [0.1, 0.05 + f * 0.45, 0.0, 0.6 + f * 0.2]
    } else if v < 0.6 {
        let f = (v - 0.3) / 0.3;
        [0.1 - f * 0.1, 0.5 + f * 0.3, f * 0.4, 0.8]
    } else if v < 0.9 {
        let f = (v - 0.6) / 0.3;
        [0.0, 0.8 - f * 0.3, 0.4 + f * 0.6, 0.9]
    } else {
        let f = ((v - 0.9) / 0.3).min(1.0);
        [f, 0.5 + f * 0.5, 1.0, 0.95]
    }
}

/// Species-id based distinct color (for population visualization)
fn population_species_color(species_id: u32) -> [f32; 4] {
    // 16 distinct colors for 16 species
    const COLORS: [[f32; 4]; 16] = [
        [0.95, 0.75, 0.55, 1.0], // Human (skin)
        [0.8, 0.5, 0.4, 1.0],    // MutantHuman
        [0.6, 0.7, 0.4, 1.0],    // Ghoul
        [0.5, 0.3, 0.2, 1.0],    // SuperMutant
        [0.4, 0.2, 0.1, 1.0],    // Radroach
        [0.55, 0.4, 0.25, 1.0],  // Molerat
        [0.7, 0.15, 0.15, 1.0],  // Deathclaw (red)
        [0.75, 0.65, 0.55, 1.0], // Brahmin
        [0.9, 0.85, 0.3, 1.0],   // Bloatfly (yellow)
        [0.3, 0.4, 0.15, 1.0],   // Radscorpion
        [0.25, 0.18, 0.12, 1.0], // YaoGuai (bear)
        [0.45, 0.55, 0.3, 1.0],  // MutantHound
        [0.2, 0.3, 0.15, 1.0],   // GiantAnt
        [0.8, 0.4, 0.7, 1.0],    // Cazador (magenta)
        [0.4, 0.6, 0.3, 1.0],    // Gecko
        [0.35, 0.5, 0.2, 1.0],   // Mantis
    ];
    COLORS[(species_id as usize) % 16]
}

// test append

// ==================== õ====================

const TERRAIN_SIZE: usize = 256;
const TERRAIN_SCALE: f32 = 2000.0 / 256.0;
const TERRAIN_HEIGHT_SCALE: f32 = 50.0;
const TERRAIN_OFFSET: f32 = 1000.0;

fn make_terrain_heightmap() -> Heightmap {
    let table = PermutationTable::new(1337);
    let mut hm = Heightmap::new(TERRAIN_SIZE, TERRAIN_SIZE);
    hm.generate_fbm(&table, 4.0, 5, 2.0, 0.5);
    hm.normalize();
    hm
}

fn terrain_world_y(x: f32, z: f32, hm: &Heightmap) -> f32 {
    let u = ((x + TERRAIN_OFFSET) / 2000.0).clamp(0.0, 1.0);
    let v = ((z + TERRAIN_OFFSET) / 2000.0).clamp(0.0, 1.0);
    hm.sample(glam::Vec2::new(u, v)) * TERRAIN_HEIGHT_SCALE
}
fn generate_terrain_mesh() -> (Vec<ae_render::Vertex>, Vec<u32>) {
    let hm = make_terrain_heightmap();
    let w = hm.width;
    let h = hm.height;
    let mut vertices: Vec<ae_render::Vertex> = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let height_val = hm.get(x, y);
            let px = x as f32 * TERRAIN_SCALE - TERRAIN_OFFSET;
            let py = height_val * TERRAIN_HEIGHT_SCALE;
            let pz = y as f32 * TERRAIN_SCALE - TERRAIN_OFFSET;
            let color = if height_val < 0.25 {
                [0.6, 0.55, 0.4, 1.0]
            } else if height_val < 0.55 {
                [0.3, 0.5, 0.2, 1.0]
            } else if height_val < 0.8 {
                [0.4, 0.35, 0.3, 1.0]
            } else {
                [0.9, 0.9, 0.95, 1.0]
            };
            vertices.push(ae_render::Vertex {
                position: [px, py, pz],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [x as f32 / (w - 1) as f32, y as f32 / (h - 1) as f32],
                color,
            });
        }
    }
    for y in 0..h {
        for x in 0..w {
            let x_l = x.saturating_sub(1);
            let x_r = (x + 1).min(w - 1);
            let y_d = y.saturating_sub(1);
            let y_u = (y + 1).min(h - 1);
            let h_l = hm.get(x_l, y) * TERRAIN_HEIGHT_SCALE;
            let h_r = hm.get(x_r, y) * TERRAIN_HEIGHT_SCALE;
            let h_d = hm.get(x, y_d) * TERRAIN_HEIGHT_SCALE;
            let h_u = hm.get(x, y_u) * TERRAIN_HEIGHT_SCALE;
            let dx = (h_r - h_l) / ((x_r - x_l).max(1) as f32 * TERRAIN_SCALE);
            let dz = (h_u - h_d) / ((y_u - y_d).max(1) as f32 * TERRAIN_SCALE);
            let nx = -dx;
            let ny = 1.0;
            let nz = -dz;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            let inv = if len > f32::EPSILON { 1.0 / len } else { 1.0 };
            let idx = y * w + x;
            vertices[idx].normal = [nx * inv, ny * inv, nz * inv];
        }
    }
    let mut indices: Vec<u32> = Vec::with_capacity((w - 1) * (h - 1) * 6);
    for y in 0..(h - 1) {
        for x in 0..(w - 1) {
            let i00 = (y * w + x) as u32;
            let i10 = (y * w + (x + 1)) as u32;
            let i01 = ((y + 1) * w + x) as u32;
            let i11 = ((y + 1) * w + (x + 1)) as u32;
            indices.extend_from_slice(&[i00, i01, i10]);
            indices.extend_from_slice(&[i10, i01, i11]);
        }
    }
    (vertices, indices)
}


fn create_world() -> GameWorld {
    let bounds = WorldBounds {
        min: Vec3::new(-1000.0, -100.0, -1000.0),
        max: Vec3::new(1000.0, 500.0, 1000.0),
    };
    let mut world = GameWorld::new(bounds);

    let vg = [([20,2,20],-40.0,-40.0),([10,2,10],80.0,60.0),([15,3,15],-300.0,100.0),([12,2,12],250.0,-150.0),([20,2,20],400.0,400.0),([16,2,16],-500.0,-400.0),([10,2,10],600.0,-500.0),([14,3,14],-700.0,500.0),([18,2,18],700.0,600.0),([12,2,12],-600.0,-600.0)];
    for (res, x, z) in vg { world.spawn_voxel_grid(res, 4.0, Vec3::new(x, -2.0, z), MaterialProperties::concrete()); }

    for i in 0..20 {
        let a = (i as f32) * 0.628;
        let p = Vec3::new(a.sin() * 50.0, 5.0, a.cos() * 50.0);
        match i % 4 { 0 => world.spawn_meta_entity_iron(p), 1 => world.spawn_meta_entity_water(p), 2 => world.spawn_meta_entity_concrete(p), _ => world.spawn_meta_entity_wood(p) };
    }

    let biomes = [Biome::Wasteland, Biome::RuinedCity, Biome::ToxicForest, Biome::RadioactiveMarsh, Biome::IndustrialZone, Biome::Crater, Biome::Desert, Biome::MountainPass, Biome::CoastalWreck, Biome::Farmland, Biome::MilitaryBase, Biome::Underground, Biome::Wasteland, Biome::ToxicForest, Biome::Crater];
    let eb = [[[-150.0,0.0,-150.0],[150.0,50.0,150.0]],[[150.0,0.0,-150.0],[300.0,80.0,150.0]],[[-300.0,0.0,100.0],[-150.0,60.0,250.0]],[[100.0,0.0,200.0],[300.0,30.0,350.0]],[[-250.0,0.0,-300.0],[-50.0,100.0,-150.0]],[[50.0,-20.0,-250.0],[200.0,30.0,-100.0]],[[400.0,0.0,300.0],[800.0,50.0,700.0]],[[-800.0,100.0,-300.0],[-500.0,400.0,100.0]],[[600.0,0.0,600.0],[900.0,30.0,900.0]],[[-500.0,0.0,400.0],[-300.0,40.0,600.0]],[[300.0,0.0,-500.0],[500.0,60.0,-300.0]],[[-200.0,-80.0,-200.0],[200.0,-20.0,200.0]],[[-700.0,0.0,600.0],[-400.0,50.0,900.0]],[[500.0,0.0,-700.0],[800.0,60.0,-400.0]],[[-100.0,-30.0,300.0],[100.0,20.0,500.0]]];
    for (i, b) in biomes.iter().enumerate() {
        let e = eb[i];
        world.spawn_ecosystem(format!("Eco-{}", i), *b, Vec3::from(e[0]), Vec3::from(e[1]));
    }

    let factions = ["Survivors", "Rangers", "Ghouls", "Mutants", "Tech", "Wild", "Raiders", "Traders"];
    let species = [NpcSpecies::Human, NpcSpecies::Human, NpcSpecies::Human, NpcSpecies::Ghoul, NpcSpecies::Mutant, NpcSpecies::Robot, NpcSpecies::Animal];
    let terrain = make_terrain_heightmap();
    let mut seed = 42u32;
    for i in 0..80 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let r1 = (seed % 1000) as f32 / 1000.0;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let r2 = (seed % 1000) as f32 / 1000.0;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let r3 = (seed % 1000) as f32 / 1000.0;
        let x = (r1 - 0.5) * 1800.0;
        let z = (r2 - 0.5) * 1800.0;
        let sp = species[(r3 * 7.0) as usize % 7];
        let fac = factions[i % 8];
        let ny = terrain_world_y(x, z, &terrain);
        world.spawn_npc(&format!("NPC-{:03}", i), Vec3::new(x, ny, z), sp, fac);
    }

    // Process spawn queues so NPCs/ecosystems are immediately populated
    for _ in 0..3 {
        world.tick();
    }

    // Add point charges for electromagnetic field visualization
    // (positive charges at cardinal directions, negative at diagonals)
    let charges = [(300.0, 5.0, 300.0, 1.0), (-300.0, 5.0, -300.0, 1.0),
                   (300.0, 5.0, -300.0, -1.0), (-300.0, 5.0, 300.0, -1.0),
                   (0.0, 50.0, 0.0, 2.0)];
    for (x, y, z, c) in charges {
        world.add_point_charge(x, y, z, c);
    }

    world
}


// ==================== 程序化 mesh 生成 ====================

struct BuildingSpec {
    building_type: BuildingType,
    position: [f32; 3],
    rotation_y: f32,
    scale: f32,
    wall_color: [f32; 4],
}

fn building_specs() -> Vec<BuildingSpec> {
    let types = [BuildingType::Residential, BuildingType::Industrial, BuildingType::Military, BuildingType::Commercial, BuildingType::PublicFacility, BuildingType::Shelter];
    let colors = [[0.7,0.65,0.55,1.0],[0.45,0.42,0.38,1.0],[0.35,0.4,0.32,1.0],[0.6,0.55,0.5,1.0],[0.65,0.6,0.58,1.0],[0.4,0.4,0.42,1.0]];
    let terrain = make_terrain_heightmap();
    let mut specs = Vec::new();
    let mut seed = 12345u32;
    for _ in 0..50 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let r1 = (seed % 1000) as f32 / 1000.0;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let r2 = (seed % 1000) as f32 / 1000.0;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let r3 = (seed % 1000) as f32 / 1000.0;
        let idx = (r3 * 6.0) as usize % 6;
        let bx = (r1 - 0.5) * 1800.0;
        let bz = (r2 - 0.5) * 1800.0;
        let by = terrain_world_y(bx, bz, &terrain);
        specs.push(BuildingSpec { building_type: types[idx], position: [bx, by, bz], rotation_y: r3 * 6.28, scale: 0.8 + r1 * 0.6, wall_color: colors[idx] });
    }
    specs
}

fn generate_building_meshes() -> Vec<(Vec<ae_render::Vertex>, Vec<u32>)> {
    let specs = building_specs();
    let mut meshes = Vec::new();
    for spec in &specs {
        let params = BuildingParams {
            building_type: spec.building_type,
            width: 8.0,
            depth: 6.0,
            floor_height: 3.0,
            floors: match spec.building_type {
                BuildingType::Industrial => 1,
                BuildingType::Military => 1,
                BuildingType::Shelter => 1,
                _ => 2,
            },
            wall_color: spec.wall_color,
            ..Default::default()
        };
        let (verts, idxs) = BuildingGenerator::new().generate(&params);
        meshes.push((verts, idxs));
    }
    meshes
}

fn generate_npc_mesh() -> (Vec<ae_render::Vertex>, Vec<u32>) {
    let params = NpcBodyParams {
        height: 1.8,
        build: 0.5,
        shoulder_width: 0.42,
        hip_width: 0.36,
        head_ratio: 1.0 / 7.5,
        skin_color: [0.85, 0.7, 0.55, 1.0],
        gender: ae_render::procedural::npc::Gender::Male,
    };
    let (verts, idxs, _skel, _weights) = NpcBodyGenerator::new().generate(&params);
    (verts, idxs)
}

fn generate_morph_meshes() -> Vec<(Vec<ae_render::Vertex>, Vec<u32>)> {
    let templates = all_templates();
    let mut meshes = Vec::new();
    for (i, template) in templates.iter().enumerate() {
        let params = ae_render::procedural::npc::MorphParams {
            scale: 1.0 + (i as f32) * 0.05,
            variant_seed: i as u32 * 17,
            ..Default::default()
        };
        let (verts, idxs, _skel, _weights) = MorphGenerator::new().generate(template, &params);
        meshes.push((verts, idxs));
    }
    meshes
}


// ==================== App ====================

struct App {
    world: GameWorld,
    camera: CameraState,
    input: InputState,
    last_time: Instant,
    hud_timer: f32,
    fps_counter: u32,
    fps_timer: f32,
    fps: f32,
    paused: bool,
    view_mode: ViewMode,
    sim_speed: u32,
    window: Option<Window>,
    surface: Option<SurfaceRenderer>,
    instanced: Option<InstancedRenderer>,
    mesh_renderer: Option<MeshRenderer>,
    building_specs: Vec<BuildingSpec>,
    morph_positions: Vec<Vec<[f32; 3]>>,
    npc_count: usize,
    eco_count: usize,
}

impl App {
    fn new(world: GameWorld) -> Self {
        let building_specs = building_specs();
        let terrain = make_terrain_heightmap();
        let mut morph_positions = Vec::new();
        let mut seed = 777u32;
        for _ in 0..8 {
            let mut group = Vec::new();
            for _ in 0..5 {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let r1 = (seed % 1000) as f32 / 1000.0;
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let r2 = (seed % 1000) as f32 / 1000.0;
                let mx = (r1 - 0.5) * 1600.0;
                let mz = (r2 - 0.5) * 1600.0;
                let my = terrain_world_y(mx, mz, &terrain);
                group.push([mx, my, mz]);
            }
            morph_positions.push(group);
        }
        let npc_count = world.get_npc_count();
        let stats = world.stats();
        Self {
            world,
            camera: CameraState::default(),
            input: InputState::default(),
            last_time: Instant::now(),
            hud_timer: 0.0,
            fps_counter: 0,
            fps_timer: 0.0,
            fps: 0.0,
            paused: false,
            view_mode: ViewMode::Composite,
            sim_speed: 1,
            window: None,
            surface: None,
            instanced: None,
            mesh_renderer: None,
            building_specs,
            morph_positions,
            npc_count,
            eco_count: stats.ecosystem_count,
        }
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            let s = self.world.stats();
            let pop = self.world.get_population_count();
            let precip = self.world.get_precipitation();
            let cloud = self.world.get_cloud_cover();
            let vis = self.world.get_visibility();
            let title = format!("Wasteland | t={:.0}s T={:.0}K rad={:.1} chem={} org={} pop={} NPC={} vox={} meta={} eco={} | rain={:.2} cloud={:.2} vis={:.0}m | {} {}x {:.0}FPS",
                s.time, s.global_temperature, s.global_radiation,
                s.active_reactions, s.total_organisms, pop, s.npc_count,
                s.total_voxels, s.meta_entity_count, s.ecosystem_count,
                precip, cloud, vis,
                self.view_mode.label(), self.sim_speed, self.fps,
            );
            window.set_title(&title);
        }
    }

    fn build_mesh_instances(&self) -> Vec<Vec<MeshInstanceData>> {
        let stats = self.world.stats();
        let mut result: Vec<Vec<MeshInstanceData>> = Vec::new();

        // 0..6: buildings (6 types, 1 instance each)
        // 0: terrain (1 large mesh, identity transform)
        let terrain_tint = match self.view_mode {
            ViewMode::Composite => [1.0, 1.0, 1.0, 1.0],
            ViewMode::Thermal => temp_to_color(stats.global_temperature),
            ViewMode::Radiation => radiation_to_color(stats.global_radiation),
            ViewMode::Chemical => chemical_to_color(stats.active_reactions),
            ViewMode::Biological => biological_to_color(stats.total_organisms),
            ViewMode::ChemicalField => chemical_field_to_color(stats.active_reactions as f32 / 50.0),
            ViewMode::BioField => bio_field_to_color(stats.total_organisms as f32 / 200.0),
            ViewMode::Population => [1.0, 1.0, 1.0, 1.0],
        };
        result.push(vec![MeshInstanceData::from_position_scale([0.0, 0.0, 0.0], 1.0, terrain_tint)]);

        for spec in &self.building_specs {
            let tint = match self.view_mode {
                ViewMode::Composite => spec.wall_color,
                ViewMode::Thermal => temp_to_color(stats.global_temperature),
                ViewMode::Radiation => radiation_to_color(stats.global_radiation),
                ViewMode::Chemical => chemical_to_color(stats.active_reactions),
                ViewMode::Biological => biological_to_color(stats.total_organisms),
                ViewMode::ChemicalField => chemical_field_to_color(stats.active_reactions as f32 / 50.0),
                ViewMode::BioField => bio_field_to_color(stats.total_organisms as f32 / 200.0),
                ViewMode::Population => spec.wall_color,
            };
            let rot = glam::Quat::from_rotation_y(spec.rotation_y);
            let inst = MeshInstanceData::from_trs(spec.position, [rot.x, rot.y, rot.z, rot.w], spec.scale, tint);
            result.push(vec![inst]);
        }

        // 6: NPC mesh (1 mesh, multiple instances from GameWorld positions)
        let npc_positions = self.world.get_npc_positions();
        let npc_colors = self.world.get_npc_colors();
        let mut npc_instances: Vec<MeshInstanceData> = Vec::new();
        for (i, pos) in npc_positions.iter().enumerate() {
            let color = if i < npc_colors.len() { npc_colors[i] } else { [0.85, 0.7, 0.55, 1.0] };
            let tint = match self.view_mode {
                ViewMode::Composite => color,
                ViewMode::Thermal => temp_to_color(stats.global_temperature + 5.0),
                ViewMode::Radiation => radiation_to_color(stats.global_radiation * 1.5),
                ViewMode::Chemical => chemical_to_color(stats.active_reactions),
                ViewMode::Biological => biological_to_color(stats.total_organisms / 10),
                ViewMode::ChemicalField => chemical_field_to_color(stats.active_reactions as f32 / 50.0),
                ViewMode::BioField => bio_field_to_color(stats.total_organisms as f32 / 200.0),
                ViewMode::Population => color,
            };
            npc_instances.push(MeshInstanceData::from_position_scale(*pos, 1.0, tint));
        }
        result.push(npc_instances);

        // morph meshes (8 templates, 5 instances each = 40 total)
        for (i, group) in self.morph_positions.iter().enumerate() {
            let base_colors = [
                [0.3, 0.5, 0.3, 1.0], [0.5, 0.3, 0.2, 1.0], [0.4, 0.4, 0.3, 1.0], [0.35, 0.25, 0.15, 1.0],
                [0.6, 0.4, 0.3, 1.0], [0.4, 0.3, 0.5, 1.0], [0.3, 0.3, 0.4, 1.0], [0.5, 0.5, 0.4, 1.0],
            ];
            let mut insts = Vec::new();
            for &pos in group {
                let tint = match self.view_mode {
                    ViewMode::Composite => base_colors[i % base_colors.len()],
                    ViewMode::Thermal => temp_to_color(stats.global_temperature + 2.0),
                    ViewMode::Radiation => radiation_to_color(stats.global_radiation * 2.0),
                    ViewMode::Chemical => chemical_to_color(stats.active_reactions),
                    ViewMode::Biological => biological_to_color(stats.total_organisms),
                    ViewMode::ChemicalField => chemical_field_to_color(stats.active_reactions as f32 / 50.0),
                    ViewMode::BioField => bio_field_to_color(stats.total_organisms as f32 / 200.0),
                    ViewMode::Population => base_colors[i % base_colors.len()],
                };
                insts.push(MeshInstanceData::from_position_scale(pos, 1.0, tint));
            }
            result.push(insts);
        }

        result
    }

}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = WindowAttributes::default()
            .with_title("Wasteland World Viewer")
            .with_inner_size(winit::dpi::PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
        let window = event_loop.create_window(attrs).expect("window");
        let surface = pollster::block_on(SurfaceRenderer::new(&window));
        let instanced = InstancedRenderer::new(
            &surface.device, surface.color_format(), surface.depth_format,
            MAX_CUBE_INSTANCES, MAX_POINTS,
        );
        let mut mesh_renderer = MeshRenderer::new(
            &surface.device, surface.color_format(), surface.depth_format,
            MAX_MESH_INSTANCES,
        );

        // 注册建筑 mesh (6 个)
        let (terrain_verts, terrain_idxs) = generate_terrain_mesh();
        mesh_renderer.register_mesh(&surface.device, &terrain_verts, &terrain_idxs, Some("terrain"));

        let building_meshes = generate_building_meshes();
        for (verts, idxs) in &building_meshes {
            mesh_renderer.register_mesh(&surface.device, verts, idxs, Some("building"));
        }

        // 注册 NPC mesh (1 个)
        let (npc_verts, npc_idxs) = generate_npc_mesh();
        mesh_renderer.register_mesh(&surface.device, &npc_verts, &npc_idxs, Some("npc"));

        // 注册 morph mesh (8 个)
        let morph_meshes = generate_morph_meshes();
        for (verts, idxs) in &morph_meshes {
            mesh_renderer.register_mesh(&surface.device, verts, idxs, Some("morph"));
        }

        self.surface = Some(surface);
        self.instanced = Some(instanced);
        self.mesh_renderer = Some(mesh_renderer);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref() else { return; };
        if window_id != window.id() { return; }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed { return; }
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                    PhysicalKey::Code(KeyCode::Space) => self.paused = !self.paused,
                    PhysicalKey::Code(KeyCode::Digit1) => self.view_mode = ViewMode::Composite,
                    PhysicalKey::Code(KeyCode::Digit2) => self.view_mode = ViewMode::Thermal,
                    PhysicalKey::Code(KeyCode::Digit3) => self.view_mode = ViewMode::Radiation,
                    PhysicalKey::Code(KeyCode::Digit4) => self.view_mode = ViewMode::Chemical,
                    PhysicalKey::Code(KeyCode::Digit5) => self.view_mode = ViewMode::Biological,
                    PhysicalKey::Code(KeyCode::Digit6) => self.view_mode = ViewMode::ChemicalField,
                    PhysicalKey::Code(KeyCode::Digit7) => self.view_mode = ViewMode::BioField,
                    PhysicalKey::Code(KeyCode::Digit8) => self.view_mode = ViewMode::Population,
                    PhysicalKey::Code(KeyCode::Tab) => self.view_mode = self.view_mode.next(),
                    PhysicalKey::Code(KeyCode::ArrowUp) => {
                        self.world.global_temperature += 10.0;
                        
                    },
                    PhysicalKey::Code(KeyCode::ArrowDown) => {
                        self.world.global_temperature -= 10.0;
                        
                    },
                    PhysicalKey::Code(KeyCode::ArrowRight) => self.sim_speed = (self.sim_speed + 1).min(10),
                    PhysicalKey::Code(KeyCode::ArrowLeft) => self.sim_speed = self.sim_speed.saturating_sub(1).max(1),
                    PhysicalKey::Code(KeyCode::KeyE) => {
                        self.world.apply_explosion(Vec3::new(0.0, 5.0, 0.0), 30.0, 50.0);
                    },
                    // F = Fire: raise temperature dramatically at origin
                    PhysicalKey::Code(KeyCode::KeyF) => {
                        self.world.global_temperature += 500.0;
                        self.world.apply_explosion(Vec3::new(0.0, 5.0, 0.0), 15.0, 20.0);
                    },
                    // G = Gamma radiation spike
                    PhysicalKey::Code(KeyCode::KeyG) => {
                        self.world.global_radiation = (self.world.global_radiation + 50.0).min(1000.0);
                    },
                    // H = Heat wave (global temp up 100K)
                    PhysicalKey::Code(KeyCode::KeyH) => {
                        self.world.global_temperature += 100.0;
                    },
                    // C = Cold snap (global temp down 100K)
                    PhysicalKey::Code(KeyCode::KeyC) => {
                        self.world.global_temperature -= 100.0;
                    },
                    // V = Virus outbreak: damage all NPCs in radius
                    PhysicalKey::Code(KeyCode::KeyV) => {
                        let positions = self.world.get_npc_positions();
                        for pos in positions.iter().take(20) {
                            self.world.apply_explosion(Vec3::from(*pos), 5.0, 5.0);
                        }
                    },
                    // B = Bio-toxin release (multiple small explosions)
                    PhysicalKey::Code(KeyCode::KeyB) => {
                        for i in 0..8 {
                            let a = (i as f32) * 0.785;
                            let p = Vec3::new(a.sin() * 100.0, 5.0, a.cos() * 100.0);
                            self.world.apply_explosion(p, 10.0, 15.0);
                        }
                    },
                    _ => {},
                }
            },
            WindowEvent::MouseInput { button, state, .. } => {
                if button == MouseButton::Left {
                    self.input.mouse_dragging = state == ElementState::Pressed;
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32;
                let y = position.y as f32;
                if self.input.mouse_dragging {
                    let dx = x - self.input.last_mouse_x;
                    let dy = y - self.input.last_mouse_y;
                    self.camera.yaw -= dx * 0.008;
                    self.camera.pitch = (self.camera.pitch + dy * 0.008).clamp(-1.4, 1.4);
                }
                self.input.last_mouse_x = x;
                self.input.last_mouse_y = y;
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 8.0,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.05,
                };
                self.camera.distance = (self.camera.distance - scroll).clamp(10.0, 800.0);
            },
            WindowEvent::Resized(size) => {
                if let Some(surface) = self.surface.as_mut() {
                    if size.width > 0 && size.height > 0 {
                        surface.resize(size.width, size.height);
                        self.camera.aspect = size.width as f32 / size.height as f32;
                    }
                }
            },

            WindowEvent::RedrawRequested => {
                let (Some(surface), Some(instanced)) = (self.surface.as_ref(), self.instanced.as_ref()) else { return; };

                // 推进仿真
                if !self.paused {
                    for _ in 0..self.sim_speed {
                        self.world.tick();
                    }
                }

                // FPS 计算
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_time);
                self.last_time = now;
                let dt = elapsed.as_secs_f32();
                self.fps_counter += 1;
                self.fps_timer += dt;
                self.hud_timer += dt;
                if self.fps_timer >= 1.0 {
                    self.fps = self.fps_counter as f32 / self.fps_timer;
                    self.fps_counter = 0;
                    self.fps_timer = 0.0;
                }
                if self.hud_timer >= 0.25 {
                    self.hud_timer = 0.0;
                    self.update_title();
                }

                let stats = self.world.stats();
                let view_mode = self.view_mode;

                // 构建 cube instances (体素 + meta + flora + organism + field/population)
                let mut instances: Vec<InstanceData> = Vec::new();

                // Field heatmap / Population visualization modes
                let is_field_mode = matches!(view_mode, ViewMode::ChemicalField | ViewMode::BioField | ViewMode::Population);
                if is_field_mode {
                    match view_mode {
                        ViewMode::ChemicalField | ViewMode::BioField => {
                            let field_name = if view_mode == ViewMode::ChemicalField { "chemical" } else { "bioactivity" };
                            const GRID_N: usize = 48;
                            const RANGE: f32 = 900.0;
                            let step = (2.0 * RANGE) / (GRID_N as f32 - 1.0);
                            for ix in 0..GRID_N {
                                for iz in 0..GRID_N {
                                    let x = -RANGE + ix as f32 * step;
                                    let z = -RANGE + iz as f32 * step;
                                    let v = self.world.get_field_value_at(field_name, x, 0.0, z);
                                    if v.abs() < 0.001 { continue; }
                                    let color = if view_mode == ViewMode::ChemicalField { chemical_field_to_color(v) } else { bio_field_to_color(v) };
                                    instances.push(InstanceData::new([x, 0.5, z], color));
                                }
                            }
                            // Add chemical/bio hazard markers: rust spots + meta entities
                            if view_mode == ViewMode::ChemicalField {
                                for (pos, radius) in self.world.get_rust_data().iter().take(500) {
                                    let r = radius.max(0.5).min(5.0);
                                    instances.push(InstanceData::new(*pos, [0.7, 0.35, 0.15, 0.9]));
                                    let _ = r;
                                }
                                // EM field sampling on coarser grid (electric field magnitude)
                                const EM_GRID: usize = 24;
                                const EM_RANGE: f32 = 800.0;
                                let em_step = (2.0 * EM_RANGE) / (EM_GRID as f32 - 1.0);
                                for ix in 0..EM_GRID {
                                    for iz in 0..EM_GRID {
                                        let x = -EM_RANGE + ix as f32 * em_step;
                                        let z = -EM_RANGE + iz as f32 * em_step;
                                        let (ex, ey, ez) = self.world.get_electric_field_at(x, 5.0, z);
                                        let mag = (ex*ex + ey*ey + ez*ez).sqrt();
                                        if mag < 0.01 { continue; }
                                        let f = (mag * 0.1).min(1.0);
                                        let color = [f, 0.3, 1.0 - f, 0.8];
                                        instances.push(InstanceData::new([x, 3.0, z], color));
                                    }
                                }
                            }
                        }
                        ViewMode::Population => {
                            let pop_count = self.world.get_population_count();
                            for idx in 0..pop_count {
                                let positions = self.world.get_population_positions(idx);
                                for (px, py, pz) in positions.iter().take(300) {
                                    let color = population_species_color(idx as u32);
                                    instances.push(InstanceData::new([*px, *py + 0.5, *pz], color));
                                }
                            }
                        }
                        _ => {}
                    }

                    // Domain isolation zones visualization (visible in all field modes)
                    // Thermal=red, Chemical=magenta, Mechanical=yellow, Electromagnetic=cyan
                    let zones = &self.world.simulation.domain_isolation.zones;
                    for zone in zones.iter().take(50) {
                        let color = match zone.domain {
                            IsolationDomain::Thermal => [1.0, 0.2, 0.1, 0.6],
                            IsolationDomain::Chemical => [1.0, 0.2, 0.8, 0.6],
                            IsolationDomain::Mechanical => [1.0, 0.9, 0.2, 0.6],
                            IsolationDomain::Electromagnetic => [0.2, 0.9, 1.0, 0.6],
                        };
                        // Outer ring (8 cubes)
                        let r = zone.radius_outer.min(50.0);
                        for k in 0..8u32 {
                            let a = (k as f32) * 0.785;
                            let x = zone.center[0] + a.sin() * r;
                            let z = zone.center[2] + a.cos() * r;
                            instances.push(InstanceData::new([x, zone.center[1] + 0.5, z], color));
                        }
                        // Center marker
                        instances.push(InstanceData::new(zone.center, color));
                    }
                }

                if !is_field_mode {
                // 体素网格
                for grid_i in 0..stats.voxel_grid_count {
                    for (pos, color) in self.world.get_voxel_mesh_data(grid_i) {
                        let c = match self.view_mode {
                            ViewMode::Composite => color,
                            ViewMode::Thermal => temp_to_color(stats.global_temperature),
                            ViewMode::Radiation => radiation_to_color(stats.global_radiation),
                            ViewMode::Chemical => chemical_to_color(stats.active_reactions),
                            ViewMode::Biological => biological_to_color(stats.total_organisms),
                            _ => color,
                        };
                        instances.push(InstanceData::new(pos, c));
                    }
                }

                // 元体
                let meta_positions = self.world.get_meta_entity_positions();
                let meta_colors = self.world.get_meta_entity_colors();
                for (i, pos) in meta_positions.iter().enumerate() {
                    let color = if i < meta_colors.len() { meta_colors[i] } else { [1.0, 1.0, 1.0, 1.0] };
                    let c = match self.view_mode {
                        ViewMode::Composite => color,
                        ViewMode::Thermal => temp_to_color(stats.global_temperature + 100.0),
                        ViewMode::Radiation => radiation_to_color(stats.global_radiation),
                        ViewMode::Chemical => chemical_to_color(stats.active_reactions),
                        ViewMode::Biological => biological_to_color(stats.total_organisms),
                        _ => color,
                    };
                    instances.push(InstanceData::new(*pos, c));
                }

                // 生态系统 flora (用 cube instance 表示)
                for eco_i in 0..stats.ecosystem_count {
                    let flora = self.world.get_flora_data(eco_i);
                    let flora_color = match self.view_mode {
                        ViewMode::Composite => [0.2, 0.6, 0.2, 1.0],
                        ViewMode::Biological => biological_to_color(stats.total_organisms),
                        _ => [0.2, 0.6, 0.2, 1.0],
                    };
                    for (pos, _species, _scale) in flora.iter().take(500) {
                        instances.push(InstanceData::new(*pos, flora_color));
                    }
                    let organisms = self.world.get_organism_data(eco_i);
                    let org_color = match self.view_mode {
                        ViewMode::Composite => [0.8, 0.4, 0.2, 1.0],
                        ViewMode::Biological => biological_to_color(stats.total_organisms),
                        _ => [0.8, 0.4, 0.2, 1.0],
                    };
                    for (pos, _species, _scale) in organisms.iter().take(500) {
                        instances.push(InstanceData::new(*pos, org_color));
                    }
                }

                // MPSS 近场粒子
                let mpss_remaining = MAX_CUBE_INSTANCES.saturating_sub(instances.len());
                if mpss_remaining > 0 {
                    let particle_data = self.world.get_mpss_render_data();
                    for (pos, color) in particle_data.iter().take(mpss_remaining) {
                        instances.push(InstanceData::new(*pos, *color));
                    }
                }

                } // end if !is_field_mode

                // MPSS 中/远场粒子 (点云) - always rendered
                let mid_far = self.world.get_mpss_mid_far_render_data();
                let mut point_instances: Vec<PointInstanceData> = mid_far.iter().take(MAX_POINTS / 2).map(|(pos, color, size)| PointInstanceData::new(*pos, *size, *color)).collect();

                // 天气粒子 (雨/雪) - based on precipitation
                let precip = self.world.get_precipitation();
                if precip > 0.01 {
                    let rain_count = (precip * 5000.0) as usize;
                    let remaining = MAX_POINTS.saturating_sub(point_instances.len());
                    let count = rain_count.min(remaining);
                    let temp = self.world.get_global_temperature();
                    let is_snow = temp < 273.0;
                    let color = if is_snow { [0.95, 0.95, 1.0, 0.8] } else { [0.5, 0.65, 0.95, 0.7] };
                    let size = if is_snow { 0.08 } else { 0.04 };
                    for i in 0..count {
                        let mut seed = (i as u32).wrapping_mul(2654435761).wrapping_add(self.world.stats().tick_count as u32);
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        let x = ((seed % 2000) as f32 - 1000.0);
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        let y = ((seed % 300) as f32 + 50.0);
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        let z = ((seed % 2000) as f32 - 1000.0);
                        point_instances.push(PointInstanceData::new([x, y, z], size, color));
                    }
                }

                // 构建 mesh instances (建筑 + NPC + morph)
                let mesh_instances = self.build_mesh_instances();
                let camera_uniform = self.camera.uniform();

                let Some(mesh_renderer) = self.mesh_renderer.as_mut() else { return; };

                // 上传 + 渲染
                instanced.update_camera(&surface.queue, &camera_uniform);
                instanced.update_instances(&surface.queue, &instances);
                instanced.update_points(&surface.queue, &point_instances);
                mesh_renderer.update_camera(&surface.queue, &camera_uniform);
                mesh_renderer.update_instances(&surface.queue, &mesh_instances);

                let cube_count = instances.len() as u32;
                let point_count = point_instances.len() as u32;

                surface.render_frame(|pass| {
                    instanced.draw_cubes(pass, cube_count);
                    mesh_renderer.draw_all(pass);
                    instanced.draw_points(pass, point_count);
                });
            },
            _ => {},
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}


fn main() {
    let world = create_world();
    let stats = world.stats();
    println!("Wasteland World Viewer starting...");
    println!("  World bounds: (-1000, -100, -1000) to (1000, 500, 1000) = 2km x 600m x 2km");
    println!("  Voxel grids: {}", stats.voxel_grid_count);
    println!("  Meta entities: {}", stats.meta_entity_count);
    println!("  Ecosystems: {}", stats.ecosystem_count);
    println!("  NPCs: {}", stats.npc_count);
    println!("  Buildings: 50 (procedural, 6 types LCG-distributed)");
    println!("  Morphs: 40 (8 templates x 5 instances)");
    println!("  Biome coverage: 13/13 (full ecosystem diversity)");
    println!("  Factions: 8 | Species: 7 (full NPC diversity)");
    println!("View modes: 1=Composite 2=Thermal 3=Radiation 4=Chemical 5=Biological");
    println!("             6=ChemField 7=BioField 8=Population | Tab=cycle");
    println!("Events: E=explosion F=fire G=gamma H=heatwave C=coldsnap V=virus B=biotoxin");
    println!("Camera: Mouse drag=rotate | Wheel=zoom | Up/Down=temp | Left/Right=speed");
    println!("       Space=pause | ESC=quit");

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App::new(world);
    event_loop.run_app(&mut app).expect("event loop error");
}
