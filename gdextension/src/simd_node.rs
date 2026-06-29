use godot::prelude::*;

use wasteland_simd::batch;
use wasteland_simd::simd_vec::has_avx2;
use wasteland_simd::soa::SoaVec3;

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandSIMD {
    #[var]
    use_avx2: bool,
    #[var]
    batch_size: i64,
    #[var]
    auto_detect: bool,

    soa_positions: SoaVec3,
    soa_velocities: SoaVec3,
    element_count: i64,
    operation_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandSIMD {
    fn init(base: Base<Node>) -> Self {
        let detected = has_avx2();
        Self {
            use_avx2: detected,
            batch_size: 256,
            auto_detect: true,
            soa_positions: SoaVec3::with_capacity(1024),
            soa_velocities: SoaVec3::with_capacity(1024),
            element_count: 0,
            operation_count: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandSIMD {
    #[func]
    fn push_element(&mut self, x: f32, y: f32, z: f32, vx: f32, vy: f32, vz: f32) {
        self.soa_positions.push(x, y, z);
        self.soa_velocities.push(vx, vy, vz);
        self.element_count += 1;
    }

    #[func]
    fn get_element(&self, index: i64) -> Dictionary<Variant, Variant> {
        if index < 0 || index as usize >= self.soa_positions.len() {
            return dict! {};
        }
        let (px, py, pz) = self.soa_positions.get(index as usize);
        let (vx, vy, vz) = self.soa_velocities.get(index as usize);
        dict! {
            "px" => px, "py" => py, "pz" => pz,
            "vx" => vx, "vy" => vy, "vz" => vz,
        }
    }

    #[func]
    fn set_element(&mut self, index: i64, x: f32, y: f32, z: f32) {
        if index < 0 || index as usize >= self.soa_positions.len() {
            return;
        }
        self.soa_positions.set(index as usize, x, y, z);
    }

    #[func]
    fn batch_add_velocity(&mut self, dx: f32, dy: f32, dz: f32) -> i64 {
        let count = self.soa_velocities.len();
        if count == 0 {
            return 0;
        }
        for i in 0..count {
            let (vx, vy, vz) = self.soa_velocities.get(i);
            self.soa_velocities.set(i, vx + dx, vy + dy, vz + dz);
        }
        self.operation_count += 1;
        count as i64
    }

    #[func]
    fn batch_integrate(&mut self, dt: f32) -> i64 {
        let count = self.soa_positions.len();
        if count == 0 {
            return 0;
        }
        for i in 0..count {
            let (px, py, pz) = self.soa_positions.get(i);
            let (vx, vy, vz) = self.soa_velocities.get(i);
            self.soa_positions.set(i, px + vx * dt, py + vy * dt, pz + vz * dt);
        }
        self.operation_count += 1;
        count as i64
    }

    #[func]
    fn batch_dot(&self) -> PackedFloat32Array {
        let count = self.soa_positions.len();
        let mut results = vec![0.0f32; count];
        if count == 0 {
            return PackedFloat32Array::new();
        }
        batch::batch_dot3(
            &self.soa_positions.x,
            &self.soa_positions.y,
            &self.soa_positions.z,
            &self.soa_velocities.x,
            &self.soa_velocities.y,
            &self.soa_velocities.z,
            &mut results,
            count,
        );
        let mut arr = PackedFloat32Array::new();
        for &v in results.iter() {
            arr.push(v);
        }
        arr
    }

    #[func]
    fn batch_normalize_velocities(&mut self) -> i64 {
        let count = self.soa_velocities.len();
        if count == 0 {
            return 0;
        }
        batch::batch_normalize3(
            &mut self.soa_velocities.x,
            &mut self.soa_velocities.y,
            &mut self.soa_velocities.z,
            count,
        );
        self.operation_count += 1;
        count as i64
    }

    #[func]
    fn get_all_positions(&self) -> PackedFloat32Array {
        let mut arr = PackedFloat32Array::new();
        let count = self.soa_positions.len();
        for i in 0..count {
            let (x, y, z) = self.soa_positions.get(i);
            arr.push(x);
            arr.push(y);
            arr.push(z);
        }
        arr
    }

    #[func]
    fn get_all_velocities(&self) -> PackedFloat32Array {
        let mut arr = PackedFloat32Array::new();
        let count = self.soa_velocities.len();
        for i in 0..count {
            let (x, y, z) = self.soa_velocities.get(i);
            arr.push(x);
            arr.push(y);
            arr.push(z);
        }
        arr
    }

    #[func]
    fn clear(&mut self) {
        self.soa_positions.clear();
        self.soa_velocities.clear();
        self.element_count = 0;
    }

    #[func]
    fn detect_simd_support(&self) -> GString {
        if has_avx2() { GString::from("avx2") } else { GString::from("scalar") }
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "element_count" => self.element_count,
            "operation_count" => self.operation_count,
            "batch_size" => self.batch_size,
            "use_avx2" => self.use_avx2,
            "soa_capacity" => self.soa_positions.len() as i64,
        }
    }
}
