use godot::prelude::*;

use wasteland_terrain::erosion::{ErosionConfig, hydraulic_erosion, thermal_erosion};
use wasteland_terrain::heightmap::Heightmap;
use wasteland_terrain::marching_cubes::{VoxelGrid, generate_mesh};
use wasteland_terrain::noise::PermutationTable;

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandTerrain {
    #[var]
    seed: i64,
    #[var]
    map_width: i64,
    #[var]
    map_height: i64,
    #[var]
    noise_scale: f32,
    #[var]
    noise_octaves: i64,
    #[var]
    noise_lacunarity: f32,
    #[var]
    noise_gain: f32,
    #[var]
    iso_level: f32,

    heightmap: Option<Heightmap>,
    perm_table: Option<PermutationTable>,
    mesh_vertices: Vec<glam::Vec3>,
    mesh_triangles: Vec<wasteland_terrain::marching_cubes::Triangle>,
    generated: bool,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandTerrain {
    fn init(base: Base<Node>) -> Self {
        Self {
            seed: 42,
            map_width: 256,
            map_height: 256,
            noise_scale: 4.0,
            noise_octaves: 4,
            noise_lacunarity: 2.0,
            noise_gain: 0.5,
            iso_level: 0.0,
            heightmap: None,
            perm_table: None,
            mesh_vertices: Vec::new(),
            mesh_triangles: Vec::new(),
            generated: false,
            base,
        }
    }
}

#[godot_api]
impl WastelandTerrain {
    #[func]
    fn generate_heightmap(&mut self) {
        let table = PermutationTable::new(self.seed as u64);
        let mut hm = Heightmap::new(self.map_width as usize, self.map_height as usize);
        hm.generate_fbm(
            &table,
            self.noise_scale,
            self.noise_octaves as u32,
            self.noise_lacunarity,
            self.noise_gain,
        );
        self.perm_table = Some(table);
        self.heightmap = Some(hm);
        self.generated = true;
    }

    #[func]
    fn generate_ridged_heightmap(&mut self) {
        let table = PermutationTable::new(self.seed as u64);
        let mut hm = Heightmap::new(self.map_width as usize, self.map_height as usize);
        hm.generate_ridged(
            &table,
            self.noise_scale,
            self.noise_octaves as u32,
            self.noise_lacunarity,
            self.noise_gain,
        );
        self.perm_table = Some(table);
        self.heightmap = Some(hm);
        self.generated = true;
    }

    #[func]
    fn generate_worley_heightmap(&mut self, cell_count: i64) {
        let table = PermutationTable::new(self.seed as u64);
        let mut hm = Heightmap::new(self.map_width as usize, self.map_height as usize);
        hm.generate_fbm(&table, self.noise_scale, 1, 1.0, 1.0);
        hm.combine_worley(&table, cell_count as u32, 0.5);
        self.perm_table = Some(table);
        self.heightmap = Some(hm);
        self.generated = true;
    }

    #[func]
    fn apply_hydraulic_erosion(
        &mut self,
        iterations: i64,
        erosion_rate: f32,
        deposition_rate: f32,
    ) {
        if let Some(ref mut hm) = self.heightmap {
            let config = ErosionConfig {
                iterations: iterations as u32,
                erosion_rate,
                deposition_rate,
                ..ErosionConfig::default()
            };
            hydraulic_erosion(hm, &config, self.seed as u64);
            self.generated = true;
        }
    }

    #[func]
    fn apply_thermal_erosion(&mut self, iterations: i64, talus_angle: f32) {
        if let Some(ref mut hm) = self.heightmap {
            thermal_erosion(hm, iterations as u32, talus_angle);
            self.generated = true;
        }
    }

    #[func]
    fn get_height_at(&self, x: f32, y: f32) -> f32 {
        if let Some(ref hm) = self.heightmap {
            let nx = (x * 0.5 + 0.5).clamp(0.0, 1.0);
            let ny = (y * 0.5 + 0.5).clamp(0.0, 1.0);
            let px = ((nx * (hm.width - 1) as f32) as usize).min(hm.width - 1);
            let py = ((ny * (hm.height - 1) as f32) as usize).min(hm.height - 1);
            return hm.get(px, py);
        }
        0.0
    }

    #[func]
    fn get_heightmap_data(&self) -> PackedFloat32Array {
        let mut arr = PackedFloat32Array::new();
        if let Some(ref hm) = self.heightmap {
            for v in &hm.data {
                arr.push(*v);
            }
        }
        arr
    }

    #[func]
    fn get_heightmap_size(&self) -> Vector2 {
        if let Some(ref hm) = self.heightmap {
            return Vector2::new(hm.width as f32, hm.height as f32);
        }
        Vector2::ZERO
    }

    #[func]
    fn get_min_max_height(&self) -> Vector2 {
        if let Some(ref hm) = self.heightmap {
            return Vector2::new(hm.min_height, hm.max_height);
        }
        Vector2::ZERO
    }

    #[func]
    fn generate_mesh(&mut self, grid_size: i64) {
        if self.heightmap.is_none() {
            return;
        }
        let gs = grid_size as usize;
        let hm = self.heightmap.as_ref().unwrap();

        let mut density = vec![0.0f32; gs * gs * gs];
        let scale_x = hm.width as f32 / gs as f32;
        let scale_y = hm.height as f32 / gs as f32;

        for z in 0..gs {
            for y in 0..gs {
                for x in 0..gs {
                    let hx = (x as f32 * scale_x) as usize;
                    let hy = (y as f32 * scale_y) as usize;
                    let h = hm.get(hx.min(hm.width - 1), hy.min(hm.height - 1));
                    let vz = z as f32 / gs as f32;
                    let normalized = (h - hm.min_height) / (hm.max_height - hm.min_height + 0.001);
                    density[z * gs * gs + y * gs + x] = normalized - vz;
                }
            }
        }

        let grid = VoxelGrid::new(gs, gs, gs, 1.0, glam::Vec3::ZERO);
        let mesh = generate_mesh(&grid, &density, self.iso_level);
        self.mesh_vertices = mesh.vertices;
        self.mesh_triangles = mesh.triangles;
    }

    #[func]
    fn get_mesh_vertices(&self) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        for v in &self.mesh_vertices {
            arr.push(Vector3::new(v.x, v.y, v.z));
        }
        arr
    }

    #[func]
    fn get_mesh_triangles(&self) -> PackedInt32Array {
        let mut arr = PackedInt32Array::new();
        for t in &self.mesh_triangles {
            arr.push(t.v0.x as i32);
            arr.push(t.v0.y as i32);
            arr.push(t.v0.z as i32);
        }
        arr
    }

    #[func]
    fn get_vertex_count(&self) -> i64 {
        self.mesh_vertices.len() as i64
    }

    #[func]
    fn get_triangle_count(&self) -> i64 {
        self.mesh_triangles.len() as i64
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "seed" => self.seed,
            "map_width" => self.map_width,
            "map_height" => self.map_height,
            "noise_scale" => self.noise_scale,
            "noise_octaves" => self.noise_octaves,
            "generated" => self.generated,
            "vertex_count" => self.mesh_vertices.len() as i64,
            "triangle_count" => self.mesh_triangles.len() as i64,
            "has_heightmap" => self.heightmap.is_some(),
        }
    }
}
