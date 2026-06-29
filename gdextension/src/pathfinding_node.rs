use godot::prelude::*;

use slotmap::Key;
use wasteland_pathfinding::astar::AStarPathfinder;
use wasteland_pathfinding::flowfield::FlowField;
use wasteland_pathfinding::navmesh::{NavMesh, NavNode, NavPoly};
use wasteland_pathfinding::smoothing::{SmoothConfig, smooth_path};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandPathfinding {
    #[var]
    cell_size: f32,
    #[var]
    cell_height: f32,
    #[var]
    max_slope_deg: f32,
    #[var]
    max_climb: f32,

    navmesh: NavMesh,
    astar: AStarPathfinder,
    flow_field: Option<FlowField>,
    smooth_config: SmoothConfig,
    last_path: Vec<[f32; 3]>,
    path_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandPathfinding {
    fn init(base: Base<Node>) -> Self {
        Self {
            cell_size: 0.5,
            cell_height: 0.2,
            max_slope_deg: 45.0,
            max_climb: 0.5,
            navmesh: NavMesh::default(),
            astar: AStarPathfinder::default(),
            flow_field: None,
            smooth_config: SmoothConfig::default(),
            last_path: Vec::new(),
            path_count: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandPathfinding {
    #[func]
    fn build_navmesh(
        &mut self,
        min_x: f32,
        min_y: f32,
        min_z: f32,
        max_x: f32,
        max_y: f32,
        max_z: f32,
    ) {
        self.navmesh.bounds_min = [min_x, min_y, min_z];
        self.navmesh.bounds_max = [max_x, max_y, max_z];
        self.navmesh.cell_size = self.cell_size;
        self.navmesh.cell_height = self.cell_height;
        self.navmesh.max_slope = self.max_slope_deg.to_radians();
        self.navmesh.max_climb = self.max_climb;
    }

    #[func]
    fn add_nav_node(&mut self, x: f32, y: f32, z: f32, radius: f32, flags: i64) -> i64 {
        let node = NavNode { position: [x, y, z], radius, flags: flags as u32 };
        let key = self.navmesh.nodes.insert(node);
        key.data().as_ffi() as i64
    }

    #[func]
    fn add_nav_poly(&mut self, v0: i64, v1: i64, v2: i64, area_cost: f32, flags: i64) -> i64 {
        if v0 < 0 || v1 < 0 || v2 < 0 {
            return -1;
        }
        let poly = NavPoly {
            vertices: [v0 as usize, v1 as usize, v2 as usize],
            center: [0.0; 3],
            neighbors: Vec::new(),
            area_cost,
            flags: flags as u32,
        };
        let key = self.navmesh.polys.insert(poly);
        key.data().as_ffi() as i64
    }

    #[func]
    fn find_path(
        &mut self,
        sx: f32,
        sy: f32,
        sz: f32,
        ex: f32,
        ey: f32,
        ez: f32,
    ) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        let start = [sx, sy, sz];
        let end = [ex, ey, ez];

        let result = self.astar.find_path(&self.navmesh, &start, &end);
        if result.success {
            self.last_path = result.path.clone();
            self.path_count += 1;
            for p in &result.path {
                arr.push(Vector3::new(p[0], p[1], p[2]));
            }
        }
        arr
    }

    #[func]
    fn find_path_smoothed(
        &mut self,
        sx: f32,
        sy: f32,
        sz: f32,
        ex: f32,
        ey: f32,
        ez: f32,
    ) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        let start = [sx, sy, sz];
        let end = [ex, ey, ez];

        let result = self.astar.find_path(&self.navmesh, &start, &end);
        if result.success {
            let smoothed = smooth_path(&result.path, &self.smooth_config);
            self.last_path = smoothed.clone();
            self.path_count += 1;
            for p in &smoothed {
                arr.push(Vector3::new(p[0], p[1], p[2]));
            }
        }
        arr
    }

    #[func]
    fn build_flow_field(&mut self, width: i64, depth: i64, origin_x: f32, origin_y: f32) {
        let ff =
            FlowField::new(width as usize, depth as usize, self.cell_size, [origin_x, origin_y]);
        self.flow_field = Some(ff);
    }

    #[func]
    fn query_flow_field(&self, x: f32, _y: f32, z: f32) -> Vector3 {
        if let Some(ref ff) = self.flow_field {
            let (cx, cz) = ff.world_to_cell(&[x, 0.0, z]);
            if cx < ff.width && cz < ff.depth {
                let idx = cz * ff.width + cx;
                let dir = ff.cells[idx].direction;
                return Vector3::new(dir[0], 0.0, dir[1]);
            }
        }
        Vector3::ZERO
    }

    #[func]
    fn get_path_length(&self) -> f32 {
        if self.last_path.len() < 2 {
            return 0.0;
        }
        let mut len = 0.0f32;
        for i in 1..self.last_path.len() {
            let a = self.last_path[i - 1];
            let b = self.last_path[i];
            len += ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        }
        len
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "node_count" => self.navmesh.nodes.len() as i64,
            "poly_count" => self.navmesh.polys.len() as i64,
            "path_count" => self.path_count,
            "cell_size" => self.cell_size,
            "has_flow_field" => self.flow_field.is_some(),
        }
    }
}
