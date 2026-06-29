use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandElectro {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    electric_constant: f32,

    #[var]
    magnetic_constant: f32,

    charge_count: i64,
    e_field_magnitude: f32,
    #[allow(dead_code)]
    b_field_magnitude: f32,

    point_charges: Vec<(Vector3, f32)>,
    current_elements: Vec<(Vector3, Vector3, f32)>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandElectro {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            electric_constant: 8.854e-12,
            magnetic_constant: 1.2566e-6,
            charge_count: 0,
            e_field_magnitude: 0.0,
            b_field_magnitude: 0.0,
            point_charges: Vec::new(),
            current_elements: Vec::new(),
            base,
        }
    }

    fn ready(&mut self) {
        if let Some(parent) = self.base().get_parent() {
            if let Ok(world) = parent.try_cast::<WastelandWorld>() {
                self.world_ref = Some(world);
            }
        }
    }

    fn process(&mut self, _delta: f64) {
        self.sync_from_world();
    }
}

#[godot_api]
impl WastelandElectro {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            self.charge_count = world.bind().get_particle_count();
        }
    }

    #[func]
    fn add_point_charge(&mut self, x: f32, y: f32, z: f32, charge: f32) {
        self.point_charges.push((Vector3::new(x, y, z), charge));
        self.charge_count += 1;
        let r = (x * x + y * y + z * z).sqrt().max(0.01);
        let k = 1.0 / (4.0 * std::f32::consts::PI * self.electric_constant);
        self.e_field_magnitude = k * charge.abs() / (r * r);
    }

    #[func]
    fn add_current_element(
        &mut self,
        x: f32,
        y: f32,
        z: f32,
        current: f32,
        dx: f32,
        dy: f32,
        dz: f32,
    ) {
        self.current_elements.push((Vector3::new(x, y, z), Vector3::new(dx, dy, dz), current));
        self.charge_count += 1;
    }

    #[func]
    fn get_electric_field_at(&self, x: f32, y: f32, z: f32) -> Vector3 {
        let k = 1.0 / (4.0 * std::f32::consts::PI * self.electric_constant);
        let mut ex = 0.0f32;
        let mut ey = 0.0f32;
        let mut ez = 0.0f32;
        let target = Vector3::new(x, y, z);
        for (pos, q) in &self.point_charges {
            let diff = target - *pos;
            let r = diff.length().max(0.01);
            let factor = k * q / (r * r * r);
            ex += diff.x * factor;
            ey += diff.y * factor;
            ez += diff.z * factor;
        }
        if self.point_charges.is_empty() {
            let r = (x * x + y * y + z * z).sqrt().max(0.01);
            let mag = k * self.charge_count as f32 / (r * r);
            Vector3::new(x * mag / r, y * mag / r, z * mag / r)
        } else {
            Vector3::new(ex, ey, ez)
        }
    }

    #[func]
    fn get_magnetic_field_at(&self, x: f32, y: f32, z: f32) -> Vector3 {
        let mu0 = self.magnetic_constant;
        let mu_over_4pi = mu0 / (4.0 * std::f32::consts::PI);
        let mut bx = 0.0f32;
        let mut by = 0.0f32;
        let mut bz = 0.0f32;
        let target = Vector3::new(x, y, z);
        for (pos, dl, i) in &self.current_elements {
            let diff = target - *pos;
            let r = diff.length().max(0.01);
            let cross = dl.cross(diff);
            let factor = mu_over_4pi * i / (r * r * r);
            bx += cross.x * factor;
            by += cross.y * factor;
            bz += cross.z * factor;
        }
        if self.current_elements.is_empty() {
            let r = (x * x + y * y + z * z).sqrt().max(0.01);
            let mag = self.magnetic_constant * self.charge_count as f32 / (r * r);
            Vector3::new(-y * mag / r, x * mag / r, 0.0)
        } else {
            Vector3::new(bx, by, bz)
        }
    }

    #[func]
    fn get_charge_count(&self) -> i64 {
        self.charge_count
    }

    #[func]
    fn compute_coulomb_force(&self, q1: f32, q2: f32, distance: f32) -> f32 {
        if distance < 0.001 {
            return 0.0;
        }
        let k = 1.0 / (4.0 * std::f32::consts::PI * self.electric_constant);
        k * q1 * q2 / (distance * distance)
    }

    #[func]
    fn compute_lorentz_force(
        &self,
        charge: f32,
        vx: f32,
        vy: f32,
        vz: f32,
        bx: f32,
        by: f32,
        bz: f32,
    ) -> Vector3 {
        let v = Vector3::new(vx, vy, vz);
        let b = Vector3::new(bx, by, bz);
        let f = v.cross(b);
        Vector3::new(f.x * charge, f.y * charge, f.z * charge)
    }

    #[func]
    fn compute_biot_savart(
        &self,
        current: f32,
        dlx: f32,
        dly: f32,
        dlz: f32,
        rx: f32,
        ry: f32,
        rz: f32,
    ) -> Vector3 {
        let dl = Vector3::new(dlx, dly, dlz);
        let r = Vector3::new(rx, ry, rz);
        let r_mag = r.length().max(0.001);
        let r_hat = r / r_mag;
        let cross = dl.cross(r_hat);
        let mu0_over_4pi = 1e-7;
        let mag = mu0_over_4pi * current / (r_mag * r_mag);
        Vector3::new(cross.x * mag, cross.y * mag, cross.z * mag)
    }
}
