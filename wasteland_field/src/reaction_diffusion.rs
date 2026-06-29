use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::scalar_field::{BoundaryCondition, FieldType, ScalarField};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionDiffusion {
    pub u_field: ScalarField,
    pub v_field: ScalarField,
    pub feed_rate: f32,
    pub kill_rate: f32,
    pub du: f32,
    pub dv: f32,
}

impl ReactionDiffusion {
    pub fn new(
        name: &str,
        resolution: [u32; 3],
        origin: Vec3,
        cell_size: f32,
        feed_rate: f32,
        kill_rate: f32,
        du: f32,
        dv: f32,
    ) -> Self {
        let mut u = ScalarField::with_initial_value(
            format!("{}_U", name),
            resolution,
            origin,
            cell_size,
            FieldType::ChemicalConcentration { compound_id: 0 },
            BoundaryCondition::Periodic,
            du,
            0.0,
            1.0,
        );
        let v = ScalarField::with_initial_value(
            format!("{}_V", name),
            resolution,
            origin,
            cell_size,
            FieldType::ChemicalConcentration { compound_id: 1 },
            BoundaryCondition::Periodic,
            dv,
            0.0,
            0.0,
        );

        let cx = resolution[0] / 2;
        let cy = resolution[1] / 2;
        let cz = resolution[2] / 2;
        let r = resolution[0].min(resolution[1]).min(resolution[2]) / 4;

        for z in cz - r..=cz + r {
            for y in cy - r..=cy + r {
                for x in cx - r..=cx + r {
                    let dx = x as f32 - cx as f32;
                    let dy = y as f32 - cy as f32;
                    let dz = z as f32 - cz as f32;
                    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                    if dist <= r as f32 {
                        u.set(x, y, z, 0.5 + rand::random::<f32>() * 0.1);
                        u.set(x, y, z, 0.5);
                    }
                }
            }
        }

        for z in cz - r / 2..=cz + r / 2 {
            for y in cy - r / 2..=cy + r / 2 {
                for x in cx - r / 2..=cx + r / 2 {
                    u.set(x, y, z, 0.25 + rand::random::<f32>() * 0.1);
                    u.set(x, y, z, 0.25);
                }
            }
        }

        Self { u_field: u, v_field: v, feed_rate, kill_rate, du, dv }
    }

    pub fn step(&mut self, dt: f32) {
        let resolution = self.u_field.resolution;
        let cell_size = self.u_field.cell_size;
        let dx2 = cell_size * cell_size;

        let mut new_u = vec![0.0f32; self.u_field.data.len()];
        let mut new_v = vec![0.0f32; self.v_field.data.len()];

        for z in 0..resolution[2] {
            for y in 0..resolution[1] {
                for x in 0..resolution[0] {
                    let idx = self.u_field.index(x, y, z);

                    let lu = self.u_field.laplacian(x, y, z) / dx2;
                    let lv = self.v_field.laplacian(x, y, z) / dx2;

                    let u = self.u_field.data[idx];
                    let v = self.v_field.data[idx];

                    let uvv = u * v * v;
                    let reaction = uvv * dt;

                    new_u[idx] = u + (self.du * lu - reaction + self.feed_rate * (1.0 - u)) * dt;
                    new_v[idx] =
                        v + (self.dv * lv + reaction - (self.feed_rate + self.kill_rate) * v) * dt;

                    new_u[idx] = new_u[idx].clamp(0.0, 1.0);
                    new_v[idx] = new_v[idx].clamp(0.0, 1.0);
                }
            }
        }

        self.u_field.data = new_u;
        self.v_field.data = new_v;
    }

    pub fn pattern_type(&self) -> RDPattern {
        let _f = self.feed_rate;
        let k = self.kill_rate;

        if k < 0.04 {
            RDPattern::Spots
        } else if k < 0.055 {
            RDPattern::Stripes
        } else if k < 0.06 {
            RDPattern::Worms
        } else if k < 0.065 {
            RDPattern::Labyrinth
        } else if k < 0.075 {
            RDPattern::Bubbles
        } else {
            RDPattern::Chaos
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RDPattern {
    Spots,
    Stripes,
    Worms,
    Labyrinth,
    Bubbles,
    Chaos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BelousovZhabotinsky {
    pub u_field: ScalarField,
    pub v_field: ScalarField,
    pub w_field: ScalarField,
    pub epsilon: f32,
    pub q: f32,
    pub f: f32,
}

impl BelousovZhabotinsky {
    pub fn new(
        name: &str,
        resolution: [u32; 3],
        origin: Vec3,
        cell_size: f32,
        epsilon: f32,
        q: f32,
        f: f32,
    ) -> Self {
        let u = ScalarField::with_initial_value(
            format!("{}_BZ_U", name),
            resolution,
            origin,
            cell_size,
            FieldType::ChemicalConcentration { compound_id: 0 },
            BoundaryCondition::Periodic,
            1.0,
            0.0,
            0.0,
        );
        let v = ScalarField::with_initial_value(
            format!("{}_BZ_V", name),
            resolution,
            origin,
            cell_size,
            FieldType::ChemicalConcentration { compound_id: 1 },
            BoundaryCondition::Periodic,
            1.0,
            0.0,
            0.0,
        );
        let w = ScalarField::with_initial_value(
            format!("{}_BZ_W", name),
            resolution,
            origin,
            cell_size,
            FieldType::ChemicalConcentration { compound_id: 2 },
            BoundaryCondition::Periodic,
            0.0,
            0.0,
            0.0,
        );

        Self { u_field: u, v_field: v, w_field: w, epsilon, q, f }
    }

    pub fn step(&mut self, dt: f32) {
        let mut new_u = vec![0.0f32; self.u_field.data.len()];
        let mut new_v = vec![0.0f32; self.v_field.data.len()];
        let mut new_w = vec![0.0f32; self.w_field.data.len()];

        for z in 0..self.u_field.resolution[2] {
            for y in 0..self.u_field.resolution[1] {
                for x in 0..self.u_field.resolution[0] {
                    let idx = self.u_field.index(x, y, z);

                    let u = self.u_field.data[idx];
                    let v = self.v_field.data[idx];
                    let w = self.w_field.data[idx];

                    let lu = self.u_field.laplacian(x, y, z);

                    let du =
                        (u - u * u - self.f * v * (u - self.q) / (u + self.q)) / self.epsilon + lu;
                    let dv = u - v;
                    let dw = 0.0;

                    new_u[idx] = (u + du * dt).clamp(0.0, 1.0);
                    new_v[idx] = (v + dv * dt).clamp(0.0, 1.0);
                    new_w[idx] = (w + dw * dt).clamp(0.0, 1.0);
                }
            }
        }

        self.u_field.data = new_u;
        self.v_field.data = new_v;
        self.w_field.data = new_w;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FungalGrowth {
    pub nutrient_field: ScalarField,
    pub mycelium_field: ScalarField,
    pub growth_rate: f32,
    pub consumption_rate: f32,
    pub branching_probability: f32,
    pub tip_extension_rate: f32,
}

impl FungalGrowth {
    pub fn new(name: &str, resolution: [u32; 3], origin: Vec3, cell_size: f32) -> Self {
        let nutrient = ScalarField::with_initial_value(
            format!("{}_nutrient", name),
            resolution,
            origin,
            cell_size,
            FieldType::NutrientLevel,
            BoundaryCondition::Dirichlet(0.0),
            0.05,
            0.001,
            1.0,
        );
        let mycelium = ScalarField::with_initial_value(
            format!("{}_mycelium", name),
            resolution,
            origin,
            cell_size,
            FieldType::BiologicalActivity,
            BoundaryCondition::Dirichlet(0.0),
            0.02,
            0.005,
            0.0,
        );

        Self {
            nutrient_field: nutrient,
            mycelium_field: mycelium,
            growth_rate: 0.3,
            consumption_rate: 0.5,
            branching_probability: 0.01,
            tip_extension_rate: 1.0,
        }
    }

    pub fn seed(&mut self, x: u32, y: u32, z: u32) {
        self.mycelium_field.set(x, y, z, 1.0);
    }

    pub fn step(&mut self, dt: f32) {
        self.nutrient_field.diffuse(dt);

        for z in 0..self.mycelium_field.resolution[2] {
            for y in 0..self.mycelium_field.resolution[1] {
                for x in 0..self.mycelium_field.resolution[0] {
                    let m = self.mycelium_field.get(x, y, z);
                    let n = self.nutrient_field.get(x, y, z);

                    if m > 0.01 && n > 0.01 {
                        let consumption = self.consumption_rate * n * m * dt;
                        self.mycelium_field.add(x, y, z, consumption * self.growth_rate);
                        self.nutrient_field.add(x, y, z, -consumption);

                        let grad = self.nutrient_field.gradient(
                            self.nutrient_field.origin
                                + Vec3::new(
                                    x as f32 * self.nutrient_field.cell_size,
                                    y as f32 * self.nutrient_field.cell_size,
                                    z as f32 * self.nutrient_field.cell_size,
                                ),
                        );

                        let growth = m * self.growth_rate * dt;
                        let dir = if grad.length() > 0.001 {
                            grad.normalize_or_zero() * self.tip_extension_rate
                        } else {
                            Vec3::new(
                                rand::random::<f32>() - 0.5,
                                rand::random::<f32>() - 0.5,
                                rand::random::<f32>() - 0.5,
                            )
                            .normalize_or_zero()
                                * self.tip_extension_rate
                        };

                        let target =
                            Vec3::new(x as f32 + dir.x, y as f32 + dir.y, z as f32 + dir.z);
                        let tx = target.x.round() as u32;
                        let ty = target.y.round() as u32;
                        let tz = target.z.round() as u32;

                        if tx < self.mycelium_field.resolution[0]
                            && ty < self.mycelium_field.resolution[1]
                            && tz < self.mycelium_field.resolution[2]
                        {
                            self.mycelium_field.add(tx, ty, tz, growth);
                        }

                        if rand::random::<f32>() < self.branching_probability * m {
                            let bx = (x as f32 + (rand::random::<f32>() - 0.5) * 2.0) as u32;
                            let by = (y as f32 + (rand::random::<f32>() - 0.5) * 2.0) as u32;
                            let bz = (z as f32 + (rand::random::<f32>() - 0.5) * 2.0) as u32;
                            if bx < self.mycelium_field.resolution[0]
                                && by < self.mycelium_field.resolution[1]
                                && bz < self.mycelium_field.resolution[2]
                            {
                                self.mycelium_field.set(bx, by, bz, m * 0.5);
                            }
                        }
                    }

                    self.mycelium_field.add(x, y, z, -m * 0.001 * dt);
                    self.mycelium_field.set(x, y, z, self.mycelium_field.get(x, y, z).max(0.0));
                }
            }
        }
    }
}
