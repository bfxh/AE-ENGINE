use crate::WastelandWorld;
use godot::prelude::*;

struct Obstacle {
    position: Vector3,
    radius: f32,
}

struct FluidParticle {
    position: Vector3,
    velocity: Vector3,
    mass: f32,
}

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandFluid {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    resolution_x: i64,

    #[var]
    resolution_y: i64,

    #[var]
    resolution_z: i64,

    #[var]
    time_scale: f32,

    #[var]
    paused: bool,

    max_velocity: f32,
    avg_pressure: f32,
    reynolds_number: f32,

    obstacles: Vec<Obstacle>,
    particles: Vec<FluidParticle>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandFluid {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            resolution_x: 32,
            resolution_y: 32,
            resolution_z: 32,
            time_scale: 1.0,
            paused: false,
            max_velocity: 0.0,
            avg_pressure: 0.0,
            reynolds_number: 0.0,
            obstacles: Vec::new(),
            particles: Vec::new(),
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
impl WastelandFluid {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().get_stats();
            if let Some(v) = data.get("active_reactions") {
                self.max_velocity = v.to::<f32>() * 0.1;
                self.avg_pressure = v.to::<f32>() * 0.05;
                self.reynolds_number = v.to::<f32>() * 0.01;
            }
        }
    }

    #[func]
    fn get_velocity_at(&self, x: f32, y: f32, z: f32) -> Vector3 {
        let dx = (x * 0.1).sin() * self.max_velocity;
        let dy = (y * 0.1).cos() * self.max_velocity * 0.5;
        let dz = (z * 0.1).cos() * self.max_velocity * 0.3;
        Vector3::new(dx, dy, dz)
    }

    #[func]
    fn get_pressure_at(&self, x: f32, y: f32, z: f32) -> f32 {
        let base = self.avg_pressure;
        let variation = (x * 0.05 + y * 0.03 + z * 0.04).sin() * base * 0.2;
        base + variation
    }

    #[func]
    fn get_grid_size(&self) -> Vector3i {
        Vector3i::new(self.resolution_x as i32, self.resolution_y as i32, self.resolution_z as i32)
    }

    #[func]
    fn step_simulation(&mut self, dt: f32) {
        if !self.paused {
            self.max_velocity = (self.max_velocity + dt * self.time_scale * 0.5).min(100.0);
            self.avg_pressure = (self.avg_pressure + dt * self.time_scale * 0.1).min(1000.0);
            self.reynolds_number =
                (self.reynolds_number + dt * self.time_scale * 0.01).min(10000.0);
        }
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "resolution" => self.get_grid_size(),
            "max_velocity" => self.max_velocity,
            "avg_pressure" => self.avg_pressure,
            "reynolds_number" => self.reynolds_number,
        }
    }

    #[func]
    fn add_obstacle(&mut self, px: f32, py: f32, pz: f32, radius: f32) -> i64 {
        let obs = Obstacle { position: Vector3::new(px, py, pz), radius };
        self.obstacles.push(obs);
        (self.obstacles.len() - 1) as i64
    }

    #[func]
    fn remove_obstacle(&mut self, index: i64) -> bool {
        let idx = index as usize;
        if idx < self.obstacles.len() {
            self.obstacles.remove(idx);
            true
        } else {
            false
        }
    }

    #[func]
    fn get_obstacles(&self) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        for o in &self.obstacles {
            let d: Dictionary<Variant, Variant> = dict! {
                "position" => o.position,
                "radius" => o.radius,
            };
            arr.push(&d);
        }
        arr
    }

    #[func]
    fn add_particle(
        &mut self,
        px: f32,
        py: f32,
        pz: f32,
        vx: f32,
        vy: f32,
        vz: f32,
        mass: f32,
    ) -> i64 {
        let p = FluidParticle {
            position: Vector3::new(px, py, pz),
            velocity: Vector3::new(vx, vy, vz),
            mass,
        };
        self.particles.push(p);
        (self.particles.len() - 1) as i64
    }

    #[func]
    fn get_particles(&self) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        for p in &self.particles {
            let d: Dictionary<Variant, Variant> = dict! {
                "position" => p.position,
                "velocity" => p.velocity,
                "mass" => p.mass,
            };
            arr.push(&d);
        }
        arr
    }

    #[func]
    fn get_velocity_field_at(&self, x: f32, y: f32, z: f32) -> Vector3 {
        let mut vx = 0.0f32;
        let mut vy = 0.0f32;
        let mut vz = 0.0f32;
        for o in &self.obstacles {
            let dx = x - o.position.x;
            let dy = y - o.position.y;
            let dz = z - o.position.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
            if dist < o.radius * 2.0 {
                let influence = (1.0 - dist / (o.radius * 2.0)).max(0.0);
                let tangent = Vector3::new(-dz, 0.0, dx).normalized();
                vx += tangent.x * influence * self.max_velocity * 0.5;
                vy += tangent.y * influence * self.max_velocity * 0.5;
                vz += tangent.z * influence * self.max_velocity * 0.5;
            }
        }
        vx += (x * 0.1 + y * 0.05).sin() * self.max_velocity * 0.3;
        vy += (y * 0.1 + z * 0.05).cos() * self.max_velocity * 0.15;
        vz += (z * 0.1 + x * 0.05).sin() * self.max_velocity * 0.2;
        Vector3::new(vx, vy, vz)
    }

    #[func]
    fn get_pressure_field_at(&self, x: f32, y: f32, z: f32) -> f32 {
        let mut pressure = self.avg_pressure;
        for o in &self.obstacles {
            let dx = x - o.position.x;
            let dy = y - o.position.y;
            let dz = z - o.position.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
            if dist < o.radius {
                let influence = (1.0 - dist / o.radius) * self.max_velocity * 10.0;
                pressure += influence;
            }
        }
        let variation = (x * 0.03 + y * 0.02 + z * 0.04).sin() * pressure * 0.1;
        (pressure + variation).max(0.0)
    }

    #[func]
    fn get_vorticity_at(&self, x: f32, y: f32, z: f32) -> f32 {
        let eps = 0.1;
        let v_xp = self.get_velocity_field_at(x + eps, y, z);
        let v_xn = self.get_velocity_field_at(x - eps, y, z);
        let v_yp = self.get_velocity_field_at(x, y + eps, z);
        let v_yn = self.get_velocity_field_at(x, y - eps, z);
        let v_zp = self.get_velocity_field_at(x, y, z + eps);
        let v_zn = self.get_velocity_field_at(x, y, z - eps);
        let dvz_dy = (v_zp.y - v_zn.y) / (2.0 * eps);
        let dvy_dz = (v_yp.z - v_yn.z) / (2.0 * eps);
        let dvx_dz = (v_xp.z - v_xn.z) / (2.0 * eps);
        let dvz_dx = (v_zp.x - v_zn.x) / (2.0 * eps);
        let dvy_dx = (v_yp.x - v_yn.x) / (2.0 * eps);
        let dvx_dy = (v_xp.y - v_xn.y) / (2.0 * eps);
        let wx = dvz_dy - dvy_dz;
        let wy = dvx_dz - dvz_dx;
        let wz = dvy_dx - dvx_dy;
        (wx * wx + wy * wy + wz * wz).sqrt()
    }
}
