use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilInfiltration {
    pub hydraulic_conductivity: f32,
    pub porosity: f32,
    pub field_capacity: f32,
    pub wilting_point: f32,
    pub current_moisture: f32,
    pub infiltration_rate: f32,
    pub percolation_rate: f32,
}

impl Default for SoilInfiltration {
    fn default() -> Self {
        Self {
            hydraulic_conductivity: 0.00001,
            porosity: 0.45,
            field_capacity: 0.3,
            wilting_point: 0.1,
            current_moisture: 0.2,
            infiltration_rate: 0.0,
            percolation_rate: 0.0,
        }
    }
}

impl SoilInfiltration {
    pub fn green_ampt_infiltration(
        &self,
        ponding_depth: f32,
        suction_head: f32,
        cumulative_infiltrated: f32,
    ) -> f32 {
        let delta_theta = self.porosity - self.current_moisture;
        if delta_theta <= 0.0 {
            return 0.0;
        }
        self.hydraulic_conductivity
            * (1.0
                + (ponding_depth + suction_head) * delta_theta / cumulative_infiltrated.max(0.001))
    }

    pub fn horton_infiltration(&self, time: f32) -> f32 {
        let f0 = self.hydraulic_conductivity * 10.0;
        let fc = self.hydraulic_conductivity;
        let k = 0.001;
        fc + (f0 - fc) * (-k * time).exp()
    }

    pub fn update(&mut self, rainfall_intensity: f32, dt: f32) -> f32 {
        let saturation_deficit = self.porosity - self.current_moisture;
        let max_infil = self.hydraulic_conductivity * saturation_deficit * 10.0;

        let actual_infil = rainfall_intensity.min(max_infil);
        self.infiltration_rate = actual_infil;

        self.current_moisture += actual_infil * dt / self.porosity;
        self.current_moisture = self.current_moisture.clamp(0.0, self.porosity);

        actual_infil
    }

    pub fn percolation(&mut self, dt: f32) -> f32 {
        if self.current_moisture <= self.field_capacity {
            return 0.0;
        }
        let excess = self.current_moisture - self.field_capacity;
        let perc = self.hydraulic_conductivity * excess * 0.5 * dt;
        self.current_moisture -= perc;
        self.percolation_rate = perc;
        perc
    }

    pub fn available_water(&self) -> f32 {
        (self.current_moisture - self.wilting_point).max(0.0)
    }

    pub fn soil_moisture_deficit(&self) -> f32 {
        self.field_capacity - self.current_moisture
    }

    pub fn saturation_ratio(&self) -> f32 {
        self.current_moisture / self.porosity
    }

    pub fn set_soil_type(&mut self, soil_type: SoilType) {
        match soil_type {
            SoilType::Sand => {
                self.hydraulic_conductivity = 0.0001;
                self.porosity = 0.35;
                self.field_capacity = 0.1;
                self.wilting_point = 0.03;
            },
            SoilType::LoamySand => {
                self.hydraulic_conductivity = 0.00005;
                self.porosity = 0.42;
                self.field_capacity = 0.15;
                self.wilting_point = 0.05;
            },
            SoilType::SandyLoam => {
                self.hydraulic_conductivity = 0.00002;
                self.porosity = 0.45;
                self.field_capacity = 0.2;
                self.wilting_point = 0.08;
            },
            SoilType::Loam => {
                self.hydraulic_conductivity = 0.00001;
                self.porosity = 0.48;
                self.field_capacity = 0.3;
                self.wilting_point = 0.12;
            },
            SoilType::SiltLoam => {
                self.hydraulic_conductivity = 0.000005;
                self.porosity = 0.5;
                self.field_capacity = 0.35;
                self.wilting_point = 0.15;
            },
            SoilType::ClayLoam => {
                self.hydraulic_conductivity = 0.000002;
                self.porosity = 0.47;
                self.field_capacity = 0.35;
                self.wilting_point = 0.2;
            },
            SoilType::Clay => {
                self.hydraulic_conductivity = 0.0000005;
                self.porosity = 0.5;
                self.field_capacity = 0.42;
                self.wilting_point = 0.28;
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoilType {
    Sand,
    LoamySand,
    SandyLoam,
    Loam,
    SiltLoam,
    ClayLoam,
    Clay,
}
