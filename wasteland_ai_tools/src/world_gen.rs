use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenRequest {
    pub world_size: [f32; 2],
    pub seed: u64,
    pub biomes: Vec<BiomeConfig>,
    pub terrain: TerrainConfig,
    pub structures: Vec<StructureConfig>,
    pub population: PopulationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeConfig {
    pub biome_type: BiomeType,
    pub coverage: f32,
    pub temperature_range: [f32; 2],
    pub humidity_range: [f32; 2],
    pub elevation_range: [f32; 2],
    pub flora_density: f32,
    pub fauna_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiomeType {
    Desert,
    Tundra,
    Grassland,
    Forest,
    Rainforest,
    Swamp,
    Mountain,
    Coastal,
    UrbanRuins,
    Wasteland,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainConfig {
    pub height_scale: f32,
    pub noise_octaves: u32,
    pub noise_persistence: f32,
    pub noise_lacunarity: f32,
    pub erosion_iterations: u32,
    pub river_count: u32,
    pub lake_threshold: f32,
    pub cliff_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureConfig {
    pub structure_type: StructureType,
    pub count: u32,
    pub min_distance: f32,
    pub biome_constraints: Vec<BiomeType>,
    pub size_range: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureType {
    Settlement,
    Outpost,
    Ruins,
    Bunker,
    Cave,
    Mine,
    Tower,
    Bridge,
    Road,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationConfig {
    pub npc_density: f32,
    pub creature_density: f32,
    pub faction_count: u32,
    pub resource_scatter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenResult {
    pub seed: u64,
    pub world_size: [f32; 2],
    pub heightmap: HeightmapData,
    pub biome_map: Vec<BiomeCell>,
    pub water_bodies: Vec<WaterBody>,
    pub structures: Vec<PlacedStructure>,
    pub resources: Vec<ResourceNode>,
    pub spawn_points: Vec<SpawnPoint>,
    pub metadata: WorldGenMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightmapData {
    pub resolution: [u32; 2],
    pub heights: Vec<f32>,
    pub min_height: f32,
    pub max_height: f32,
    pub normals: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeCell {
    pub position: [f32; 2],
    pub biome: BiomeType,
    pub temperature: f32,
    pub humidity: f32,
    pub elevation: f32,
    pub fertility: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterBody {
    pub water_type: WaterType,
    pub boundary: Vec<[f32; 2]>,
    pub depth: f32,
    pub flow_direction: [f32; 2],
    pub flow_rate: f32,
    pub pollution: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterType {
    Ocean,
    Lake,
    River,
    Swamp,
    Underground,
    Polluted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedStructure {
    pub structure_type: StructureType,
    pub position: [f32; 3],
    pub rotation: f32,
    pub scale: [f32; 3],
    pub biome: BiomeType,
    pub integrity: f32,
    pub loot_tier: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub resource_type: ResourceType,
    pub position: [f32; 3],
    pub quantity: f32,
    pub quality: f32,
    pub accessibility: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Water,
    Iron,
    Copper,
    Coal,
    Oil,
    Uranium,
    Wood,
    Stone,
    Food,
    Medicine,
    Scrap,
    Crystal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPoint {
    pub position: [f32; 3],
    pub spawn_type: SpawnType,
    pub faction: Option<String>,
    pub danger_level: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnType {
    PlayerStart,
    NpcSpawn,
    CreatureSpawn,
    EventTrigger,
    ResourceSpawn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenMetadata {
    pub total_cells: u32,
    pub biome_count: u32,
    pub structure_count: u32,
    pub water_body_count: u32,
    pub resource_count: u32,
    pub generation_time_ms: u64,
    pub memory_bytes: u64,
}

pub struct WorldGenerator {
    pub config: WorldGenConfig,
    rng_state: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenConfig {
    pub chunk_size: u32,
    pub max_height: f32,
    pub sea_level: f32,
    pub temperature_latitude_falloff: f32,
    pub river_min_length: f32,
    pub structure_min_spacing: f32,
    pub resource_cluster_size: u32,
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        WorldGenConfig {
            chunk_size: 64,
            max_height: 512.0,
            sea_level: 0.3,
            temperature_latitude_falloff: 0.6,
            river_min_length: 50.0,
            structure_min_spacing: 100.0,
            resource_cluster_size: 5,
        }
    }
}

impl WorldGenerator {
    pub fn new(config: WorldGenConfig, seed: u64) -> Self {
        WorldGenerator { config, rng_state: seed }
    }

    pub fn generate(&mut self, request: &WorldGenRequest) -> WorldGenResult {
        let resolution = [
            (request.world_size[0] / self.config.chunk_size as f32) as u32,
            (request.world_size[1] / self.config.chunk_size as f32) as u32,
        ];
        let cell_count = (resolution[0] * resolution[1]) as usize;

        let heightmap = self.generate_heightmap(resolution, &request.terrain);
        let biome_map = self.generate_biomes(&heightmap, request);
        let water_bodies = self.generate_water_bodies(&heightmap, &biome_map, request);
        let structures = self.generate_structures(&heightmap, &biome_map, request);
        let resources = self.generate_resources(&heightmap, &biome_map, &water_bodies, request);
        let spawn_points = self.generate_spawn_points(&heightmap, &biome_map, &structures);

        WorldGenResult {
            seed: request.seed,
            world_size: request.world_size,
            heightmap,
            biome_map,
            water_bodies,
            structures,
            resources,
            spawn_points,
            metadata: WorldGenMetadata {
                total_cells: cell_count as u32,
                biome_count: 0,
                structure_count: 0,
                water_body_count: 0,
                resource_count: 0,
                generation_time_ms: 0,
                memory_bytes: (cell_count * 64) as u64,
            },
        }
    }

    fn generate_heightmap(
        &mut self,
        resolution: [u32; 2],
        terrain: &TerrainConfig,
    ) -> HeightmapData {
        let count = (resolution[0] * resolution[1]) as usize;
        let mut heights = vec![0.0f32; count];
        let mut normals = vec![[0.0, 1.0, 0.0]; count];

        for y in 0..resolution[1] {
            for x in 0..resolution[0] {
                let idx = (y * resolution[0] + x) as usize;
                let fx = x as f32 / resolution[0] as f32;
                let fy = y as f32 / resolution[1] as f32;
                let mut height = 0.0;
                let mut amplitude = 1.0;
                let mut frequency = 1.0;
                let mut max_amp = 0.0;

                for _ in 0..terrain.noise_octaves {
                    let nx = fx * frequency;
                    let ny = fy * frequency;
                    height += Self::hash_noise(nx, ny, self.rng_state) * amplitude;
                    max_amp += amplitude;
                    amplitude *= terrain.noise_persistence;
                    frequency *= terrain.noise_lacunarity;
                }
                height /= max_amp;
                height = (height + 1.0) * 0.5;
                height *= terrain.height_scale * self.config.max_height;
                heights[idx] = height;
            }
        }

        for y in 1..resolution[1] - 1 {
            for x in 1..resolution[0] - 1 {
                let idx = (y * resolution[0] + x) as usize;
                let dx = heights[(y * resolution[0] + x + 1) as usize]
                    - heights[(y * resolution[0] + x - 1) as usize];
                let dz = heights[((y + 1) * resolution[0] + x) as usize]
                    - heights[((y - 1) * resolution[0] + x) as usize];
                let len = (dx * dx + dz * dz + 4.0).sqrt();
                normals[idx] = [-dx / len, 2.0 / len, -dz / len];
            }
        }

        let min_height = heights.iter().copied().fold(f32::MAX, f32::min);
        let max_height = heights.iter().copied().fold(f32::MIN, f32::max);

        HeightmapData { resolution, heights, min_height, max_height, normals }
    }

    fn hash_noise(x: f32, y: f32, seed: u64) -> f32 {
        let ix = x as u64;
        let iy = y as u64;
        let h =
            ix.wrapping_mul(374761393).wrapping_add(iy.wrapping_mul(668265263)).wrapping_add(seed);
        let h =
            h.wrapping_mul(h.wrapping_mul(1274126177).wrapping_add(82378291)).wrapping_add(488423);
        let h = (h ^ (h >> 13)) as u32;
        (h as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn generate_biomes(
        &mut self,
        heightmap: &HeightmapData,
        request: &WorldGenRequest,
    ) -> Vec<BiomeCell> {
        let count = (heightmap.resolution[0] * heightmap.resolution[1]) as usize;
        let mut cells = Vec::with_capacity(count);

        for y in 0..heightmap.resolution[1] {
            for x in 0..heightmap.resolution[0] {
                let idx = (y * heightmap.resolution[0] + x) as usize;
                let height = heightmap.heights[idx];
                let normalized_h = (height - heightmap.min_height)
                    / (heightmap.max_height - heightmap.min_height + 0.001);
                let fy = y as f32 / heightmap.resolution[1] as f32;
                let temperature = 1.0
                    - (fy - 0.5).abs() * 2.0 * self.config.temperature_latitude_falloff
                    - normalized_h * 0.5;
                let humidity =
                    Self::hash_noise(x as f32 * 0.01, y as f32 * 0.01, self.rng_state + 1) * 0.5
                        + 0.5;

                let biome =
                    self.classify_biome(normalized_h, temperature, humidity, &request.biomes);

                cells.push(BiomeCell {
                    position: [x as f32, y as f32],
                    biome,
                    temperature,
                    humidity,
                    elevation: normalized_h,
                    fertility: humidity * (1.0 - normalized_h) * 0.8 + 0.2,
                });
            }
        }
        cells
    }

    fn classify_biome(
        &self,
        elevation: f32,
        temperature: f32,
        humidity: f32,
        biomes: &[BiomeConfig],
    ) -> BiomeType {
        if elevation > 0.8 {
            return BiomeType::Mountain;
        }
        if elevation < self.config.sea_level {
            return BiomeType::Coastal;
        }
        let mut best_score = f32::MIN;
        let mut best_biome = BiomeType::Wasteland;
        for biome in biomes {
            if elevation < biome.elevation_range[0] || elevation > biome.elevation_range[1] {
                continue;
            }
            if temperature < biome.temperature_range[0] || temperature > biome.temperature_range[1]
            {
                continue;
            }
            if humidity < biome.humidity_range[0] || humidity > biome.humidity_range[1] {
                continue;
            }
            let t_center = (biome.temperature_range[0] + biome.temperature_range[1]) * 0.5;
            let h_center = (biome.humidity_range[0] + biome.humidity_range[1]) * 0.5;
            let t_dist = (temperature - t_center).abs()
                / (biome.temperature_range[1] - biome.temperature_range[0] + 0.001);
            let h_dist = (humidity - h_center).abs()
                / (biome.humidity_range[1] - biome.humidity_range[0] + 0.001);
            let score = biome.coverage - (t_dist + h_dist) * 0.5;
            if score > best_score {
                best_score = score;
                best_biome = biome.biome_type.clone();
            }
        }
        best_biome
    }

    fn generate_water_bodies(
        &mut self,
        heightmap: &HeightmapData,
        biome_map: &[BiomeCell],
        request: &WorldGenRequest,
    ) -> Vec<WaterBody> {
        let mut water_bodies = Vec::new();
        for y in 0..heightmap.resolution[1] {
            for x in 0..heightmap.resolution[0] {
                let idx = (y * heightmap.resolution[0] + x) as usize;
                let normalized_h = (heightmap.heights[idx] - heightmap.min_height)
                    / (heightmap.max_height - heightmap.min_height + 0.001);
                if normalized_h < self.config.sea_level {
                    water_bodies.push(WaterBody {
                        water_type: WaterType::Ocean,
                        boundary: vec![[x as f32, y as f32]],
                        depth: self.config.sea_level - normalized_h,
                        flow_direction: [0.0, -1.0],
                        flow_rate: 0.0,
                        pollution: 0.0,
                    });
                }
            }
        }
        for _ in 0..request.terrain.river_count {
            let rx = (Self::hash_noise(0.0, 0.0, self.rng_state + 2) * 0.5 + 0.5)
                * heightmap.resolution[0] as f32;
            let ry = (Self::hash_noise(1.0, 0.0, self.rng_state + 2) * 0.5 + 0.5)
                * heightmap.resolution[1] as f32;
            let mut river_points = Vec::new();
            let mut cx = rx;
            let mut cy = ry;
            let length = request.terrain.river_count as f32 * 10.0 + self.config.river_min_length;
            for _ in 0..(length as u32) {
                river_points.push([cx, cy]);
                cx += Self::hash_noise(cy, cx, self.rng_state + 3) * 2.0;
                cy += 1.0;
                if cx < 0.0
                    || cx >= heightmap.resolution[0] as f32
                    || cy >= heightmap.resolution[1] as f32
                {
                    break;
                }
            }
            if river_points.len() > 2 {
                let biome = biome_map
                    .get(
                        (river_points[0][1] as u32 * heightmap.resolution[0]
                            + river_points[0][0] as u32) as usize,
                    )
                    .map(|b| b.biome.clone())
                    .unwrap_or(BiomeType::Wasteland);
                let pollution = if biome == BiomeType::UrbanRuins || biome == BiomeType::Wasteland {
                    0.7
                } else {
                    0.0
                };
                water_bodies.push(WaterBody {
                    water_type: WaterType::River,
                    boundary: river_points,
                    depth: 2.0,
                    flow_direction: [0.0, 1.0],
                    flow_rate: 1.5,
                    pollution,
                });
            }
        }
        water_bodies
    }

    fn generate_structures(
        &mut self,
        heightmap: &HeightmapData,
        biome_map: &[BiomeCell],
        request: &WorldGenRequest,
    ) -> Vec<PlacedStructure> {
        let mut structures = Vec::new();
        for config in &request.structures {
            let mut placed = 0u32;
            let mut attempts = 0u32;
            while placed < config.count && attempts < config.count * 10 {
                attempts += 1;
                let x = (Self::hash_noise(placed as f32, attempts as f32, self.rng_state + 4)
                    * 0.5
                    + 0.5)
                    * heightmap.resolution[0] as f32;
                let y = (Self::hash_noise(attempts as f32, placed as f32, self.rng_state + 5)
                    * 0.5
                    + 0.5)
                    * heightmap.resolution[1] as f32;
                let idx = (y as u32 * heightmap.resolution[0] + x as u32) as usize;
                if idx >= biome_map.len() {
                    continue;
                }
                let cell = &biome_map[idx];
                if !config.biome_constraints.is_empty()
                    && !config.biome_constraints.contains(&cell.biome)
                {
                    continue;
                }
                let too_close = structures.iter().any(|s: &PlacedStructure| {
                    let dx = s.position[0] - x;
                    let dy = s.position[1] - y;
                    (dx * dx + dy * dy).sqrt() < config.min_distance
                });
                if too_close {
                    continue;
                }
                let height = heightmap.heights[idx];
                let scale_factor = Self::hash_noise(x, y, self.rng_state + 6) * 0.5 + 0.5;
                let scale = config.size_range[0]
                    + (config.size_range[1] - config.size_range[0]) * scale_factor;
                structures.push(PlacedStructure {
                    structure_type: config.structure_type.clone(),
                    position: [x, height, y],
                    rotation: Self::hash_noise(y, x, self.rng_state + 7) * std::f32::consts::PI,
                    scale: [scale; 3],
                    biome: cell.biome.clone(),
                    integrity: 0.5 + Self::hash_noise(x + y, x - y, self.rng_state + 8) * 0.5,
                    loot_tier: (cell.elevation * 3.0 + cell.fertility * 2.0) as u32,
                });
                placed += 1;
            }
        }
        structures
    }

    fn generate_resources(
        &mut self,
        heightmap: &HeightmapData,
        biome_map: &[BiomeCell],
        _water_bodies: &[WaterBody],
        request: &WorldGenRequest,
    ) -> Vec<ResourceNode> {
        let mut resources = Vec::new();
        let resource_types = [
            ResourceType::Iron,
            ResourceType::Copper,
            ResourceType::Coal,
            ResourceType::Oil,
            ResourceType::Uranium,
            ResourceType::Scrap,
            ResourceType::Crystal,
            ResourceType::Wood,
            ResourceType::Stone,
            ResourceType::Water,
            ResourceType::Food,
            ResourceType::Medicine,
        ];
        let total_resources = (request.population.resource_scatter as usize * 100) + 50;
        for _ in 0..total_resources {
            let x = (Self::hash_noise(resources.len() as f32, 0.0, self.rng_state + 9) * 0.5 + 0.5)
                * heightmap.resolution[0] as f32;
            let y = (Self::hash_noise(0.0, resources.len() as f32, self.rng_state + 10) * 0.5
                + 0.5)
                * heightmap.resolution[1] as f32;
            let idx = (y as u32 * heightmap.resolution[0] + x as u32) as usize;
            if idx >= biome_map.len() {
                continue;
            }
            let cell = &biome_map[idx];
            let rt_idx = (Self::hash_noise(x, y, self.rng_state + 11).abs()
                * resource_types.len() as f32) as usize
                % resource_types.len();
            let resource_type = resource_types[rt_idx];
            let height = heightmap.heights[idx];
            resources.push(ResourceNode {
                resource_type,
                position: [x, height, y],
                quantity: 50.0 + Self::hash_noise(y, x, self.rng_state + 12).abs() * 200.0,
                quality: 0.3 + Self::hash_noise(x + 1.0, y + 1.0, self.rng_state + 13).abs() * 0.7,
                accessibility: match cell.biome {
                    BiomeType::Mountain => 0.3,
                    BiomeType::Swamp => 0.4,
                    BiomeType::Grassland => 0.9,
                    _ => 0.6,
                },
            });
        }
        resources
    }

    fn generate_spawn_points(
        &mut self,
        heightmap: &HeightmapData,
        biome_map: &[BiomeCell],
        structures: &[PlacedStructure],
    ) -> Vec<SpawnPoint> {
        let mut spawns = Vec::new();
        for y in 0..heightmap.resolution[1] {
            for x in 0..heightmap.resolution[0] {
                let idx = (y * heightmap.resolution[0] + x) as usize;
                if idx >= biome_map.len() {
                    continue;
                }
                let cell = &biome_map[idx];
                let height = heightmap.heights[idx];
                let normalized_h = (height - heightmap.min_height)
                    / (heightmap.max_height - heightmap.min_height + 0.001);
                let danger = match cell.biome {
                    BiomeType::Wasteland | BiomeType::UrbanRuins => 0.8,
                    BiomeType::Mountain => 0.6,
                    BiomeType::Swamp => 0.7,
                    BiomeType::Grassland => 0.2,
                    _ => 0.4,
                };
                if normalized_h > self.config.sea_level && cell.fertility > 0.3 {
                    let near_structure = structures.iter().any(|s| {
                        let dx = s.position[0] - x as f32;
                        let dy = s.position[2] - y as f32;
                        (dx * dx + dy * dy).sqrt() < 50.0
                    });
                    let spawn_type = if x == 0 && y == 0 {
                        SpawnType::PlayerStart
                    } else if near_structure {
                        SpawnType::NpcSpawn
                    } else {
                        SpawnType::CreatureSpawn
                    };
                    spawns.push(SpawnPoint {
                        position: [x as f32, height, y as f32],
                        spawn_type,
                        faction: None,
                        danger_level: danger,
                    });
                }
            }
        }
        spawns
    }

    pub fn generate_dungeon(
        &self,
        width: u32,
        height: u32,
        room_count: u32,
        seed: u64,
    ) -> DungeonResult {
        let mut grid = vec![DungeonTile::Wall; (width * height) as usize];
        let mut rooms = Vec::new();
        let mut rng = seed;
        for _ in 0..room_count {
            let rw = 4 + (Self::hash_noise(rng as f32, 0.0, rng) * 0.5 + 0.5) as u32 * 6;
            let rh = 4 + (Self::hash_noise(0.0, rng as f32, rng + 1) * 0.5 + 0.5) as u32 * 6;
            let rx = (Self::hash_noise(1.0, rng as f32, rng + 2) * 0.5 + 0.5) as u32
                * (width - rw - 2)
                + 1;
            let ry = (Self::hash_noise(rng as f32, 1.0, rng + 3) * 0.5 + 0.5) as u32
                * (height - rh - 2)
                + 1;
            let overlaps = rooms.iter().any(|(ox, oy, ow, oh): &(u32, u32, u32, u32)| {
                rx < ox + ow + 1 && rx + rw + 1 > *ox && ry < oy + oh + 1 && ry + rh + 1 > *oy
            });
            if overlaps {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                continue;
            }
            for dy in 0..rh {
                for dx in 0..rw {
                    let idx = ((ry + dy) * width + rx + dx) as usize;
                    grid[idx] = DungeonTile::Floor;
                }
            }
            if !rooms.is_empty() {
                let (prev_x, prev_y, prev_w, prev_h) = rooms[rooms.len() - 1];
                let prev_cx = prev_x + prev_w / 2;
                let prev_cy = prev_y + prev_h / 2;
                let curr_cx = rx + rw / 2;
                let curr_cy = ry + rh / 2;
                if rng.is_multiple_of(2) {
                    for x in prev_cx.min(curr_cx)..=prev_cx.max(curr_cx) {
                        let idx = (prev_cy * width + x) as usize;
                        grid[idx] = DungeonTile::Corridor;
                    }
                    for y in prev_cy.min(curr_cy)..=prev_cy.max(curr_cy) {
                        let idx = (y * width + curr_cx) as usize;
                        grid[idx] = DungeonTile::Corridor;
                    }
                } else {
                    for y in prev_cy.min(curr_cy)..=prev_cy.max(curr_cy) {
                        let idx = (y * width + prev_cx) as usize;
                        grid[idx] = DungeonTile::Corridor;
                    }
                    for x in prev_cx.min(curr_cx)..=prev_cx.max(curr_cx) {
                        let idx = (curr_cy * width + x) as usize;
                        grid[idx] = DungeonTile::Corridor;
                    }
                }
            }
            rooms.push((rx, ry, rw, rh));
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        }
        DungeonResult {
            width,
            height,
            tiles: grid,
            rooms: rooms
                .iter()
                .map(|(x, y, w, h)| DungeonRoom { x: *x, y: *y, width: *w, height: *h })
                .collect(),
            total_rooms: rooms.len() as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DungeonTile {
    Wall,
    Floor,
    Corridor,
    Door,
    Stairs,
    Trap,
    Treasure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonResult {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<DungeonTile>,
    pub rooms: Vec<DungeonRoom>,
    pub total_rooms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonRoom {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenStats {
    pub total_attempts: u64,
    pub successful_generations: u64,
    pub average_time_ms: f64,
    pub peak_memory_mb: f64,
    pub biome_distribution: HashMap<String, u32>,
}

impl Default for WorldGenStats {
    fn default() -> Self {
        WorldGenStats {
            total_attempts: 0,
            successful_generations: 0,
            average_time_ms: 0.0,
            peak_memory_mb: 0.0,
            biome_distribution: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_biomes() -> Vec<BiomeConfig> {
        vec![
            BiomeConfig {
                biome_type: BiomeType::Grassland,
                coverage: 0.3,
                temperature_range: [0.2, 0.8],
                humidity_range: [0.2, 0.7],
                elevation_range: [0.3, 0.7],
                flora_density: 0.6,
                fauna_types: vec!["deer".into(), "rabbit".into()],
            },
            BiomeConfig {
                biome_type: BiomeType::Desert,
                coverage: 0.15,
                temperature_range: [0.6, 1.0],
                humidity_range: [0.0, 0.3],
                elevation_range: [0.2, 0.6],
                flora_density: 0.1,
                fauna_types: vec!["scorpion".into(), "lizard".into()],
            },
            BiomeConfig {
                biome_type: BiomeType::Forest,
                coverage: 0.2,
                temperature_range: [0.3, 0.7],
                humidity_range: [0.5, 0.9],
                elevation_range: [0.3, 0.8],
                flora_density: 0.8,
                fauna_types: vec!["wolf".into(), "bear".into()],
            },
            BiomeConfig {
                biome_type: BiomeType::Wasteland,
                coverage: 0.25,
                temperature_range: [0.0, 1.0],
                humidity_range: [0.0, 0.5],
                elevation_range: [0.1, 0.9],
                flora_density: 0.05,
                fauna_types: vec!["mutant".into(), "rat".into()],
            },
        ]
    }

    fn default_request() -> WorldGenRequest {
        WorldGenRequest {
            world_size: [512.0, 512.0],
            seed: 42,
            biomes: default_biomes(),
            terrain: TerrainConfig {
                height_scale: 1.0,
                noise_octaves: 6,
                noise_persistence: 0.5,
                noise_lacunarity: 2.0,
                erosion_iterations: 0,
                river_count: 5,
                lake_threshold: 0.05,
                cliff_threshold: 0.7,
            },
            structures: vec![
                StructureConfig {
                    structure_type: StructureType::Ruins,
                    count: 10,
                    min_distance: 30.0,
                    biome_constraints: vec![BiomeType::Wasteland, BiomeType::UrbanRuins],
                    size_range: [5.0, 20.0],
                },
                StructureConfig {
                    structure_type: StructureType::Settlement,
                    count: 5,
                    min_distance: 50.0,
                    biome_constraints: vec![BiomeType::Grassland, BiomeType::Forest],
                    size_range: [10.0, 30.0],
                },
            ],
            population: PopulationConfig {
                npc_density: 0.1,
                creature_density: 0.3,
                faction_count: 3,
                resource_scatter: true,
            },
        }
    }

    #[test]
    fn test_generate_world() {
        let mut gen = WorldGenerator::new(WorldGenConfig::default(), 42);
        let request = default_request();
        let result = gen.generate(&request);
        assert_eq!(result.seed, 42);
        assert!(!result.heightmap.heights.is_empty());
        assert!(!result.biome_map.is_empty());
        assert!(result.heightmap.min_height <= result.heightmap.max_height);
    }

    #[test]
    fn test_heightmap_bounds() {
        let mut gen = WorldGenerator::new(WorldGenConfig::default(), 123);
        let request = WorldGenRequest {
            world_size: [256.0, 256.0],
            seed: 123,
            biomes: default_biomes(),
            terrain: TerrainConfig {
                height_scale: 1.0,
                noise_octaves: 4,
                noise_persistence: 0.5,
                noise_lacunarity: 2.0,
                erosion_iterations: 0,
                river_count: 3,
                lake_threshold: 0.05,
                cliff_threshold: 0.7,
            },
            structures: vec![],
            population: PopulationConfig {
                npc_density: 0.0,
                creature_density: 0.0,
                faction_count: 0,
                resource_scatter: false,
            },
        };
        let result = gen.generate(&request);
        for h in &result.heightmap.heights {
            assert!(*h >= 0.0);
            assert!(*h <= gen.config.max_height);
        }
    }

    #[test]
    fn test_biome_classification() {
        let mut gen = WorldGenerator::new(WorldGenConfig::default(), 42);
        let request = default_request();
        let result = gen.generate(&request);
        let biomes: std::collections::HashSet<BiomeType> =
            result.biome_map.iter().map(|c| c.biome.clone()).collect();
        assert!(biomes.len() >= 2);
        let wasteland_count =
            result.biome_map.iter().filter(|c| c.biome == BiomeType::Wasteland).count();
        assert!(wasteland_count > 0);
    }

    #[test]
    fn test_water_bodies() {
        let mut gen = WorldGenerator::new(WorldGenConfig::default(), 42);
        let request = default_request();
        let result = gen.generate(&request);
        assert!(!result.water_bodies.is_empty());
        let has_ocean = result.water_bodies.iter().any(|w| w.water_type == WaterType::Ocean);
        assert!(has_ocean);
    }

    #[test]
    fn test_structures_generation() {
        let mut gen = WorldGenerator::new(WorldGenConfig::default(), 42);
        let request = default_request();
        let result = gen.generate(&request);
        assert!(!result.structures.is_empty());
        for s in &result.structures {
            assert!(s.integrity >= 0.0 && s.integrity <= 1.0);
        }
    }

    #[test]
    fn test_resources() {
        let mut gen = WorldGenerator::new(WorldGenConfig::default(), 42);
        let mut request = default_request();
        request.population.resource_scatter = true;
        let result = gen.generate(&request);
        assert!(!result.resources.is_empty());
        let resource_types: std::collections::HashSet<ResourceType> =
            result.resources.iter().map(|r| r.resource_type).collect();
        assert!(resource_types.len() >= 3);
    }

    #[test]
    fn test_spawn_points() {
        let mut gen = WorldGenerator::new(WorldGenConfig::default(), 42);
        let request = default_request();
        let result = gen.generate(&request);
        assert!(!result.spawn_points.is_empty());
        let has_player_start =
            result.spawn_points.iter().any(|s| s.spawn_type == SpawnType::PlayerStart);
        let has_any_spawn = result
            .spawn_points
            .iter()
            .any(|s| s.spawn_type == SpawnType::PlayerStart || s.spawn_type == SpawnType::NpcSpawn);
        assert!(has_player_start || has_any_spawn);
    }

    #[test]
    fn test_dungeon_generation() {
        let gen = WorldGenerator::new(WorldGenConfig::default(), 42);
        let dungeon = gen.generate_dungeon(50, 50, 8, 12345);
        assert_eq!(dungeon.width, 50);
        assert_eq!(dungeon.height, 50);
        assert!(dungeon.total_rooms >= 1);
        assert!(dungeon.total_rooms <= 8);
        let floor_count = dungeon.tiles.iter().filter(|t| **t == DungeonTile::Floor).count();
        let corridor_count = dungeon.tiles.iter().filter(|t| **t == DungeonTile::Corridor).count();
        assert!(floor_count > 0);
        assert!(corridor_count > 0 || dungeon.total_rooms == 1);
    }

    #[test]
    fn test_deterministic_seed() {
        let mut gen1 = WorldGenerator::new(WorldGenConfig::default(), 42);
        let mut gen2 = WorldGenerator::new(WorldGenConfig::default(), 42);
        let request = default_request();
        let r1 = gen1.generate(&request);
        let r2 = gen2.generate(&request);
        assert_eq!(r1.heightmap.heights, r2.heightmap.heights);
    }
}
