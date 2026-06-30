use godot::prelude::*;
use std::sync::Mutex;
use ae_particle::mpss::{MpssBuffer, ParticleKind};

#[derive(GodotClass)]
#[class(base=Node3D)]
pub(crate) struct WastelandMpss {
    buffer: Mutex<Option<MpssBuffer>>,

    #[var]
    particle_count: i64,

    #[var]
    gravity: f64,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WastelandMpss {
    fn init(base: Base<Node3D>) -> Self {
        Self { buffer: Mutex::new(None), particle_count: 0, gravity: 9.81, base }
    }

    fn ready(&mut self) {
        godot_print!("[MPSS] Node ready");
    }

    fn process(&mut self, delta: f64) {
        let dt = delta as f32;
        if let Ok(mut guard) = self.buffer.lock() {
            if let Some(ref mut buf) = *guard {
                buf.apply_gravity(self.gravity as f32, dt);
                buf.integrate_positions(dt);
                buf.update_lifetimes(dt);
                self.particle_count = buf.len() as i64;
            }
        }
    }
}

#[godot_api]
impl WastelandMpss {
    #[func]
    fn spawn_grid(&mut self, size: i64, spacing: f64) {
        let n = size as usize;
        if n == 0 {
            return;
        }
        let total = n * n * n;
        if let Ok(mut guard) = self.buffer.lock() {
            // Always recreate buffer so repeated calls produce a fresh grid
            *guard = Some(MpssBuffer::new(total));
            let buf = guard.as_mut().unwrap();
            let half = (n as f32 * spacing as f32) / 2.0;
            for x in 0..n {
                for y in 0..n {
                    for z in 0..n {
                        if let Some(idx) = buf.spawn() {
                            let pos = [
                                x as f32 * spacing as f32 - half,
                                y as f32 * spacing as f32,
                                z as f32 * spacing as f32 - half,
                            ];
                            buf.pos[idx] = pos;
                            buf.kind[idx] = ParticleKind::Inert;
                            buf.mass[idx] = 1.0;
                        }
                    }
                }
            }
            self.particle_count = buf.len() as i64;
            godot_print!("[MPSS] Spawned grid: {n}x{n}x{n} = {} particles", buf.len());
        }
    }

    #[func]
    fn spawn_fluid_column(&mut self, count: i64, height: f64, radius: f64) {
        let n = count as usize;
        if n == 0 {
            return;
        }
        if let Ok(mut guard) = self.buffer.lock() {
            let buf = guard.get_or_insert_with(|| MpssBuffer::new(n));
            for _ in 0..n {
                if let Some(idx) = buf.spawn() {
                    let angle = rand::random::<f32>() * std::f32::consts::TAU;
                    let r = radius as f32 * rand::random::<f32>().sqrt();
                    buf.pos[idx] =
                        [r * angle.cos(), height as f32 * rand::random::<f32>(), r * angle.sin()];
                    buf.kind[idx] = ParticleKind::Fluid;
                    buf.mass[idx] = 0.5;
                }
            }
            self.particle_count = buf.len() as i64;
            godot_print!("[MPSS] Spawned fluid column: {} particles", buf.len());
        }
    }

    #[func]
    fn get_positions(&self) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if let Ok(guard) = self.buffer.lock() {
            if let Some(ref buf) = *guard {
                for i in 0..buf.capacity {
                    if buf.active[i] {
                        arr.push(Vector3::new(buf.pos[i][0], buf.pos[i][1], buf.pos[i][2]));
                    }
                }
            }
        }
        arr
    }

    #[func]
    fn clear(&mut self) {
        if let Ok(mut guard) = self.buffer.lock() {
            *guard = None;
        }
        self.particle_count = 0;
        godot_print!("[MPSS] Cleared all particles");
    }
}
