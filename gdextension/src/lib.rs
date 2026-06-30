use godot::prelude::*;
use std::sync::Mutex;
use ae_engine::GameWorld;

mod acoustics_node;
mod ai_bridge_node;
mod ai_node;
mod ai_tools_node;
mod animation_node;
mod asset_node;
mod audio_node;
mod axiom_node;
mod biology_node;
mod character_node;
mod chemistry_node;
mod compute_node;
mod crafting_node;
mod ecology_node;
mod electro_node;
mod emergence_node;
mod eventbus_node;
mod factory_node;
mod field_node;
mod fluid_node;
mod frequency_node;
mod generalizer_node;
mod geo_node;
mod hydro_node;
mod info_node;
mod io_node;
mod materials_node;
mod memory_node;
mod meta_node;
mod modding_node;
mod network_node;
mod npc_node;
mod optics_node;
mod particle_node;
mod pathfinding_node;
mod physics_node;
mod profiler_node;
mod serialize_node;
mod simd_node;
mod storage_node;
mod terrain_node;
mod thermo_node;
mod timeslice_node;
mod weather_node;
mod weave_node;
mod xpbd_node;

mod mpss_node;

struct WastelandWorldExtension;

#[gdextension]
unsafe impl ExtensionLibrary for WastelandWorldExtension {}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub(crate) struct WastelandWorld {
    world: Mutex<Option<GameWorld>>,

    #[var]
    time_scale: f32,

    #[var]
    paused: bool,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WastelandWorld {
    fn init(base: Base<Node3D>) -> Self {
        Self { world: Mutex::new(None), time_scale: 1.0, paused: false, base }
    }

    fn process(&mut self, _delta: f64) {
        if self.paused {
            return;
        }
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                world.set_time_scale(self.time_scale);
                world.tick();
            }
        }
    }
}

#[godot_api]
impl WastelandWorld {
    #[func]
    fn init_world(&mut self, size: f32) {
        let bounds = ae_engine::WorldBounds {
            min: glam::Vec3::new(-size, -size, -size),
            max: glam::Vec3::new(size, size, size),
        };
        let world = GameWorld::new(bounds);
        if let Ok(mut guard) = self.world.lock() {
            *guard = Some(world);
        }
    }

    #[func]
    fn spawn_ecosystem(
        &mut self,
        name: GString,
        min_x: f32,
        min_y: f32,
        min_z: f32,
        max_x: f32,
        max_y: f32,
        max_z: f32,
    ) {
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                world.spawn_ecosystem(
                    name.to_string(),
                    ae_engine::Biome::Wasteland,
                    glam::Vec3::new(min_x, min_y, min_z),
                    glam::Vec3::new(max_x, max_y, max_z),
                );
            }
        }
    }

    #[func]
    fn spawn_voxel_grid(
        &mut self,
        res_x: i32,
        res_y: i32,
        res_z: i32,
        voxel_size: f32,
        ox: f32,
        oy: f32,
        oz: f32,
    ) {
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                world.spawn_voxel_grid(
                    [res_x, res_y, res_z],
                    voxel_size,
                    glam::Vec3::new(ox, oy, oz),
                    ae_engine::MaterialProperties::concrete(),
                );
            }
        }
    }

    #[func]
    fn apply_explosion(&mut self, x: f32, y: f32, z: f32, radius: f32, force: f32) {
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                world.apply_explosion(glam::Vec3::new(x, y, z), radius, force);
            }
        }
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                let s = world.stats();
                return dict! {
                    "time" => s.time,
                    "tick_count" => s.tick_count as i64,
                    "rigid_body_count" => s.rigid_body_count as i64,
                    "voxel_grid_count" => s.voxel_grid_count as i64,
                    "total_voxels" => s.total_voxels as i64,
                    "ecosystem_count" => s.ecosystem_count as i64,
                    "total_organisms" => s.total_organisms as i64,
                    "active_reactions" => s.active_reactions as i64,
                    "global_temperature" => s.global_temperature,
                    "global_radiation" => s.global_radiation,
                    "meta_entity_count" => s.meta_entity_count as i64,
                    "npc_count" => s.npc_count as i64,
                };
            }
        }
        dict! {}
    }

    #[func]
    fn voxel_grid_count(&self) -> i64 {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.voxel_grid_count() as i64;
            }
        }
        0
    }

    #[func]
    fn ecosystem_count(&self) -> i64 {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.ecosystem_count() as i64;
            }
        }
        0
    }

    #[func]
    fn get_voxel_positions(&self, grid_index: i64) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if grid_index < 0 {
            return arr;
        }
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (pos, _) in world.get_voxel_mesh_data(grid_index as usize) {
                    arr.push(Vector3::new(pos[0], pos[1], pos[2]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_voxel_colors(&self, grid_index: i64) -> PackedColorArray {
        let mut arr = PackedColorArray::new();
        if grid_index < 0 {
            return arr;
        }
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (_, color) in world.get_voxel_mesh_data(grid_index as usize) {
                    arr.push(Color::from_rgba(color[0], color[1], color[2], color[3]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_flora_positions(&self, ecosystem_index: i64) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if ecosystem_index < 0 {
            return arr;
        }
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (pos, _, _) in world.get_flora_data(ecosystem_index as usize) {
                    arr.push(Vector3::new(pos[0], pos[1], pos[2]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_organism_positions(&self, ecosystem_index: i64) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if ecosystem_index < 0 {
            return arr;
        }
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (pos, _, _) in world.get_organism_data(ecosystem_index as usize) {
                    arr.push(Vector3::new(pos[0], pos[1], pos[2]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_field_value(&self, field_name: GString, x: f32, y: f32, z: f32) -> f32 {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.get_field_value_at(&field_name.to_string(), x, y, z);
            }
        }
        0.0
    }

    #[func]
    fn get_particle_count(&self) -> i64 {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.get_particle_count() as i64;
            }
        }
        0
    }

    #[func]
    fn get_particle_positions(&self) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for pos in world.get_particle_positions() {
                    arr.push(Vector3::new(pos[0], pos[1], pos[2]));
                }
            }
        }
        arr
    }

    #[func]
    fn spawn_iron_particles(
        &mut self,
        x: f32,
        y: f32,
        z: f32,
        count: i64,
        spacing: f32,
        granular: bool,
    ) {
        if count <= 0 {
            return;
        }
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                world.spawn_iron_particles(x, y, z, count as u32, spacing, granular);
            }
        }
    }

    #[func]
    fn get_crack_positions(&self) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (pos, _, _) in world.get_crack_data() {
                    arr.push(Vector3::new(pos[0], pos[1], pos[2]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_crack_count(&self) -> i64 {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.get_crack_data().len() as i64;
            }
        }
        0
    }

    #[func]
    fn get_rust_positions(&self) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (pos, _) in world.get_rust_data() {
                    arr.push(Vector3::new(pos[0], pos[1], pos[2]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_rust_sizes(&self) -> PackedFloat32Array {
        let mut arr = PackedFloat32Array::new();
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (_, radius) in world.get_rust_data() {
                    arr.push(radius);
                }
            }
        }
        arr
    }

    #[func]
    fn get_bark_fissure_positions(&self) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (pos, _, _) in world.get_bark_fissure_data() {
                    arr.push(Vector3::new(pos[0], pos[1], pos[2]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_growth_ring_years(&self) -> PackedInt32Array {
        let mut arr = PackedInt32Array::new();
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (year, _) in world.get_growth_ring_data() {
                    arr.push(year as i32);
                }
            }
        }
        arr
    }

    #[func]
    fn get_growth_ring_thicknesses(&self) -> PackedFloat32Array {
        let mut arr = PackedFloat32Array::new();
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for (_, thickness) in world.get_growth_ring_data() {
                    arr.push(thickness);
                }
            }
        }
        arr
    }

    #[func]
    fn get_cache_stats(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                let stats = world.get_cache_stats();
                return dict! {
                    "hit_count" => stats.hit_count as i64,
                    "miss_count" => stats.miss_count as i64,
                    "hit_rate" => stats.hit_rate,
                    "cached_units" => stats.cached_units as i64,
                    "total_units" => stats.total_units as i64,
                };
            }
        }
        dict! {}
    }

    #[func]
    fn get_health_report(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                let report = world.get_health_report();
                return dict! {
                    "total_monitors" => report.total_monitors as i64,
                    "healthy" => report.healthy as i64,
                    "warning" => report.warning as i64,
                    "critical" => report.critical as i64,
                    "active_alerts" => report.active_alerts as i64,
                };
            }
        }
        dict! {}
    }

    #[func]
    fn get_time_slice_stats(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                let stats = world.data.time_slicer.system_stats();
                let system_count = stats.len() as i64;
                return dict! {
                    "system_count" => system_count,
                    "tick_count" => world.data.time_slicer.tick_count as i64,
                };
            }
        }
        dict! {}
    }

    #[func]
    fn get_event_count(&self) -> i64 {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.data.event_store.event_count() as i64;
            }
        }
        0
    }

    #[func]
    fn get_diff_graph_stats(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                let stats = world.data.diff_graph.stats();
                return dict! {
                    "total_nodes" => stats.total_nodes as i64,
                    "dirty_nodes" => stats.dirty_nodes as i64,
                    "clean_nodes" => stats.clean_nodes as i64,
                    "total_updates" => stats.total_updates as i64,
                    "update_ratio" => stats.update_ratio,
                };
            }
        }
        dict! {}
    }

    #[func]
    fn spawn_octree(&mut self, world_size: f32, max_depth: i64, ox: f32, oy: f32, oz: f32) -> i64 {
        if !(0..=255).contains(&max_depth) {
            return -1;
        }
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                let idx = world.spawn_octree(
                    world_size,
                    max_depth as u8,
                    glam::Vec3::new(ox, oy, oz),
                    ae_engine::MaterialProperties::concrete(),
                );
                return idx as i64;
            }
        }
        -1
    }

    #[func]
    fn activate_octree_voxel(&mut self, octree_idx: i64, x: f32, y: f32, z: f32) -> bool {
        if octree_idx < 0 {
            return false;
        }
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                return world.activate_octree_voxel(octree_idx as usize, glam::Vec3::new(x, y, z));
            }
        }
        false
    }

    #[func]
    fn octree_compression_ratio(&self, octree_idx: i64) -> f32 {
        if octree_idx < 0 {
            return 0.0;
        }
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.octree_compression_ratio(octree_idx as usize);
            }
        }
        1.0
    }

    #[func]
    fn spawn_dual_phase_entity(
        &mut self,
        res_x: i32,
        res_y: i32,
        res_z: i32,
        voxel_size: f32,
        ox: f32,
        oy: f32,
        oz: f32,
    ) -> GString {
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                if let Some(id) = world.spawn_dual_phase_entity(
                    ae_engine::MaterialProperties::concrete(),
                    [res_x, res_y, res_z],
                    voxel_size,
                    glam::Vec3::new(ox, oy, oz),
                ) {
                    let s: GString = GString::from(id.to_string().as_str());
                    return s;
                }
            }
        }
        GString::from("")
    }

    #[func]
    fn dual_phase_voxel_to_particles(&mut self, entity_id: GString) {
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                if let Ok(id) = uuid::Uuid::parse_str(&entity_id.to_string()) {
                    world.dual_phase_voxel_to_particles(id);
                }
            }
        }
    }

    #[func]
    fn dual_phase_particles_to_voxels(&mut self, entity_id: GString) {
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                if let Ok(id) = uuid::Uuid::parse_str(&entity_id.to_string()) {
                    world.dual_phase_particles_to_voxels(id);
                }
            }
        }
    }

    #[func]
    fn dual_phase_active_count(&self) -> i64 {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.dual_phase_active_count() as i64;
            }
        }
        0
    }

    #[func]
    fn dual_phase_particle_count(&self) -> i64 {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.dual_phase_particle_count() as i64;
            }
        }
        0
    }

    #[func]
    fn spawn_npc(
        &mut self,
        name: GString,
        px: f32,
        py: f32,
        pz: f32,
        species: GString,
        faction: GString,
    ) -> GString {
        if let Ok(mut guard) = self.world.lock() {
            if let Some(ref mut world) = *guard {
                let sp = match species.to_string().as_str() {
                    "human" => ae_engine::NpcSpecies::Human,
                    "mutant" => ae_engine::NpcSpecies::Mutant,
                    "ghoul" => ae_engine::NpcSpecies::Ghoul,
                    "robot" => ae_engine::NpcSpecies::Robot,
                    "animal" => ae_engine::NpcSpecies::Animal,
                    _ => ae_engine::NpcSpecies::Human,
                };
                let id = world.spawn_npc(
                    &name.to_string(),
                    glam::Vec3::new(px, py, pz),
                    sp,
                    &faction.to_string(),
                );
                let id_str = id.to_string();
                return GString::from(id_str.as_str());
            }
        }
        GString::new()
    }

    #[func]
    fn despawn_npc(&mut self, npc_id: GString) {
        if let Ok(parsed) = uuid::Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.world.lock() {
                if let Some(ref mut world) = *guard {
                    world.despawn_npc(parsed);
                }
            }
        }
    }

    #[func]
    fn npc_dialogue(
        &mut self,
        npc_id: GString,
        player_message: GString,
    ) -> Dictionary<Variant, Variant> {
        if let Ok(parsed) = uuid::Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.world.lock() {
                if let Some(ref mut world) = *guard {
                    if let Some(resp) = world.npc_dialogue(parsed, &player_message.to_string()) {
                        let resp_npc_id = GString::from(resp.npc_id.as_str());
                        let resp_text = GString::from(resp.text.as_str());
                        let resp_emotion = GString::from(resp.emotion.as_str());
                        let resp_action =
                            GString::from(resp.action.clone().unwrap_or_default().as_str());
                        return dict! {
                            "npc_id" => &resp_npc_id,
                            "text" => &resp_text,
                            "emotion" => &resp_emotion,
                            "action" => &resp_action,
                            "memory_updated" => resp.memory_updated,
                            "relationship_changed" => resp.relationship_changed,
                            "affinity_delta" => resp.affinity_delta,
                        };
                    }
                }
            }
        }
        dict! {}
    }

    #[func]
    fn get_npc_positions(&self) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for pos in world.get_npc_positions() {
                    arr.push(Vector3::new(pos[0], pos[1], pos[2]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_npc_colors(&self) -> PackedColorArray {
        let mut arr = PackedColorArray::new();
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                for color in world.get_npc_colors() {
                    arr.push(Color::from_rgba(color[0], color[1], color[2], color[3]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_npc_count(&self) -> i64 {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return world.get_npc_count() as i64;
            }
        }
        0
    }

    #[func]
    fn get_npc_stats(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                let stats = world.get_npc_stats();
                return dict! {
                    "total" => stats.total as i64,
                    "alive" => stats.alive as i64,
                    "dead" => stats.dead as i64,
                    "faction_count" => stats.faction_count as i64,
                    "pending_spawn" => stats.pending_spawn as i64,
                    "pending_despawn" => stats.pending_despawn as i64,
                    "interactions" => stats.interactions as i64,
                    "bridge_npcs" => stats.bridge_npcs as i64,
                };
            }
        }
        dict! {}
    }

    #[func]
    pub(crate) fn export_weather_data(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return dict! {
                    "temperature" => world.global_temperature,
                    "humidity" => world.simulation.atmosphere.humidity,
                    "pressure" => world.simulation.atmosphere.pressure,
                    "wind_x" => world.weather.wind.x,
                    "wind_y" => world.weather.wind.y,
                    "wind_z" => world.weather.wind.z,
                    "wind_speed" => world.weather.wind.length(),
                    "precipitation" => world.weather.precipitation,
                    "cloud_cover" => world.weather.cloud_cover,
                    "visibility" => world.weather.visibility,
                    "storm_intensity" => world.weather.storm_intensity,
                    "radiation_storm" => world.weather.radiation_storm,
                    "acid_rain" => world.weather.acid_rain,
                    "global_radiation" => world.global_radiation,
                };
            }
        }
        dict! {}
    }

    #[func]
    pub(crate) fn export_ecology_data(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                let mut species_arr = Array::<Variant>::new();
                for eco in &world.game_logic.ecosystems {
                    let sp_dict: Dictionary<Variant, Variant> = dict! {
                        "name" => eco.name.clone().as_str(),
                        "organism_count" => eco.organism_count() as i64,
                        "flora_count" => eco.flora.len() as i64,
                        "fauna_count" => eco.organisms.len() as i64,
                    };
                    species_arr.push(&sp_dict);
                }
                let mut pop_arr = Array::<Variant>::new();
                for pop in &world.game_logic.populations {
                    let pop_dict: Dictionary<Variant, Variant> = dict! {
                        "id" => pop.species_id.clone().as_str(),
                        "count" => pop.count as i64,
                        "carrying_capacity" => pop.carrying_capacity as i64,
                        "growth_rate" => pop.growth_rate,
                        "death_rate" => pop.death_rate,
                        "birth_rate" => pop.birth_rate,
                        "biomass" => pop.biomass,
                    };
                    pop_arr.push(&pop_dict);
                }
                return dict! {
                    "ecosystem_count" => world.game_logic.ecosystems.len() as i64,
                    "total_organisms" => world.game_logic.ecosystems.iter().map(|e| e.organism_count()).sum::<usize>() as i64,
                    "population_count" => world.game_logic.populations.len() as i64,
                    "ecosystems" => &species_arr,
                    "populations" => &pop_arr,
                };
            }
        }
        dict! {}
    }

    #[func]
    pub(crate) fn export_acoustics_data(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                let active = world.simulation.acoustic_solver.sources.iter().filter(|s| s.active).count();
                let total_energy: f32 =
                    world.simulation.acoustic_solver.pressure_field.iter().map(|p| p.abs()).sum();
                return dict! {
                    "active_sources" => active as i64,
                    "total_sources" => world.simulation.acoustic_solver.sources.len() as i64,
                    "total_energy" => total_energy,
                    "speed_of_sound" => world.simulation.acoustic_solver.speed_of_sound,
                    "grid_resolution" => world.simulation.acoustic_solver.dimensions.0 as i64,
                };
            }
        }
        dict! {}
    }

    #[func]
    pub(crate) fn export_optics_data(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                let total_luminance: f32 =
                    world.rendering.spectral_renderer.lights.iter().map(|l| l.color.length()).sum();
                return dict! {
                    "active_lights" => world.rendering.spectral_renderer.lights.len() as i64,
                    "max_bounces" => world.rendering.spectral_renderer.max_bounces as i64,
                    "samples_per_pixel" => world.rendering.spectral_renderer.samples_per_pixel as i64,
                    "total_luminance" => total_luminance,
                };
            }
        }
        dict! {}
    }

    #[func]
    pub(crate) fn export_factory_data(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.world.lock() {
            if let Some(ref world) = *guard {
                return dict! {
                    "conveyor_count" => world.game_logic.conveyor_network.conveyors.len() as i64,
                    "sensor_count" => world.game_logic.automation_controller.sensors.len() as i64,
                    "actuator_count" => world.game_logic.automation_controller.actuators.len() as i64,
                    "rule_count" => world.game_logic.automation_controller.rules.len() as i64,
                    "tick_rate" => world.game_logic.automation_controller.tick_rate,
                    "energy_generation" => world.game_logic.energy_network.total_generation,
                    "energy_consumption" => world.game_logic.energy_network.total_consumption,
                    "grid_stability" => world.game_logic.energy_network.grid_stability,
                };
            }
        }
        dict! {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_extension_library_trait() {
        // Placeholder test - actual implementation tested in integration tests
    }
}
