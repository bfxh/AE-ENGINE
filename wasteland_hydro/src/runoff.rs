use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceRunoff {
    pub water_depth: f32,
    pub flow_velocity: Vec3,
    pub sediment_load: f32,
    pub manning_n: f32,
}

impl Default for SurfaceRunoff {
    fn default() -> Self {
        Self { water_depth: 0.0, flow_velocity: Vec3::ZERO, sediment_load: 0.0, manning_n: 0.03 }
    }
}

impl SurfaceRunoff {
    pub fn flow_rate(&self) -> f32 {
        self.water_depth * self.flow_velocity.length()
    }

    pub fn manning_velocity(slope: f32, hydraulic_radius: f32, manning_n: f32) -> f32 {
        if manning_n <= 0.0 {
            return 0.0;
        }
        (1.0 / manning_n) * hydraulic_radius.powf(2.0 / 3.0) * slope.abs().sqrt()
    }

    pub fn shear_stress(&self, water_density: f32, slope: f32) -> f32 {
        const GRAVITY: f32 = 9.81;
        water_density * GRAVITY * self.water_depth * slope.abs()
    }

    pub fn transport_capacity(&self, grain_size: f32) -> f32 {
        let shear = self.shear_stress(1000.0, 0.01);
        let critical_shear = 0.03 * grain_size * 2650.0 * 9.81;
        if shear > critical_shear { (shear - critical_shear) * 0.01 } else { 0.0 }
    }

    pub fn update(&mut self, rainfall: f32, infiltration: f32, slope_gradient: Vec3, dt: f32) {
        self.water_depth += (rainfall - infiltration) * dt;
        self.water_depth = self.water_depth.max(0.0);

        let slope = slope_gradient.length();
        let h_radius = self.water_depth.max(0.001);
        let v = Self::manning_velocity(slope, h_radius, self.manning_n);

        if slope > 0.0001 {
            let dir = slope_gradient / slope;
            self.flow_velocity = dir * v;
        } else {
            self.flow_velocity = Vec3::ZERO;
        }

        let capacity = self.transport_capacity(0.001);
        self.sediment_load =
            (self.sediment_load + (capacity - self.sediment_load) * 0.1 * dt).max(0.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watershed {
    pub area: f32,
    pub slope: f32,
    pub drainage_density: f32,
    pub time_of_concentration: f32,
    pub peak_discharge: f32,
}

impl Watershed {
    pub fn peak_runoff(area: f32, rainfall_intensity: f32, runoff_coefficient: f32) -> f32 {
        area * rainfall_intensity * runoff_coefficient * 0.001
    }

    pub fn time_of_concentration(length: f32, slope: f32, manning_n: f32) -> f32 {
        if slope <= 0.0 {
            return f32::MAX;
        }
        0.007 * (manning_n * length / slope.sqrt()).powf(0.8)
    }

    pub fn hydrograph_peak(&self, rainfall_duration: f32) -> f32 {
        if rainfall_duration >= self.time_of_concentration {
            self.peak_discharge
        } else {
            self.peak_discharge * rainfall_duration / self.time_of_concentration
        }
    }
}
