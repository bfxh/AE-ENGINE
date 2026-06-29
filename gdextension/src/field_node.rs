use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandField {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    field_resolution: i64,

    #[var]
    field_type: GString,

    #[allow(dead_code)]
    active_fields: i64,
    field_energy: f32,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandField {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            field_resolution: 64,
            field_type: GString::from("scalar"),
            active_fields: 0,
            field_energy: 0.0,
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
impl WastelandField {
    fn sync_from_world(&mut self) {
        if let Some(ref _world) = self.world_ref {
            self.field_energy =
                _world.bind().get_field_value(GString::from("energy"), 0.0, 0.0, 0.0);
        }
    }

    #[func]
    fn get_field_value(&self, field_name: GString, x: f32, y: f32, z: f32) -> f32 {
        if let Some(ref _world) = self.world_ref {
            return _world.bind().get_field_value(field_name.clone(), x, y, z);
        }
        0.0
    }

    #[func]
    fn sample_field_line(
        &self,
        field_name: GString,
        x: f32,
        y: f32,
        z: f32,
        steps: i64,
    ) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        let mut cx = x;
        let mut cy = y;
        let mut cz = z;
        let step_size = 0.5;
        for _ in 0..steps {
            arr.push(Vector3::new(cx, cy, cz));
            if let Some(ref _world) = self.world_ref {
                let g = self.compute_gradient(field_name.clone(), cx, cy, cz);
                cx += g.x * step_size;
                cy += g.y * step_size;
                cz += g.z * step_size;
                if cx.abs() > 1000.0 || cy.abs() > 1000.0 || cz.abs() > 1000.0 {
                    break;
                }
            } else {
                break;
            }
        }
        arr
    }

    #[func]
    fn compute_gradient(&self, field_name: GString, x: f32, y: f32, z: f32) -> Vector3 {
        let eps = 0.01;
        let v0 = self.get_field_value(field_name.clone(), x, y, z);
        let vx = self.get_field_value(field_name.clone(), x + eps, y, z);
        let vy = self.get_field_value(field_name.clone(), x, y + eps, z);
        let vz = self.get_field_value(field_name, x, y, z + eps);
        Vector3::new((vx - v0) / eps, (vy - v0) / eps, (vz - v0) / eps)
    }

    #[func]
    fn compute_laplacian(&self, field_name: GString, x: f32, y: f32, z: f32) -> f32 {
        let eps = 0.01;
        let v0 = self.get_field_value(field_name.clone(), x, y, z);
        let vx1 = self.get_field_value(field_name.clone(), x + eps, y, z);
        let vx2 = self.get_field_value(field_name.clone(), x - eps, y, z);
        let vy1 = self.get_field_value(field_name.clone(), x, y + eps, z);
        let vy2 = self.get_field_value(field_name.clone(), x, y - eps, z);
        let vz1 = self.get_field_value(field_name.clone(), x, y, z + eps);
        let vz2 = self.get_field_value(field_name, x, y, z - eps);
        (vx1 + vx2 + vy1 + vy2 + vz1 + vz2 - 6.0 * v0) / (eps * eps)
    }

    #[func]
    fn compute_divergence(&self, field_name: GString, x: f32, y: f32, z: f32) -> f32 {
        let g = self.compute_gradient(field_name, x, y, z);
        g.x + g.y + g.z
    }

    #[func]
    fn get_reaction_diffusion_state(&self, x: f32, y: f32, z: f32) -> Dictionary<Variant, Variant> {
        let u = self.get_field_value(GString::from("activator"), x, y, z);
        let v = self.get_field_value(GString::from("inhibitor"), x, y, z);
        dict! {
            "activator" => u,
            "inhibitor" => v,
            "pattern" => &GString::from(if u > v { "spots" } else { "stripes" }),
            "ratio" => if v > 0.0 { u / v } else { 0.0f32 },
        }
    }

    #[func]
    fn get_field_energy(&self) -> f32 {
        self.field_energy
    }

    #[func]
    fn list_field_names(&self) -> Array<Variant> {
        let mut arr = Array::new();
        for name in &[
            "temperature",
            "pressure",
            "density",
            "humidity",
            "energy",
            "potential",
            "activator",
            "inhibitor",
        ] {
            arr.push(&GString::from(*name));
        }
        arr
    }
}
