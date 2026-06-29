use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaporation {
    pub rate: f32,
    pub net_radiation: f32,
    pub wind_speed: f32,
    pub vapor_pressure_deficit: f32,
    pub latent_heat: f32,
    pub sensible_heat: f32,
}

impl Default for Evaporation {
    fn default() -> Self {
        Self {
            rate: 0.0,
            net_radiation: 0.0,
            wind_speed: 0.0,
            vapor_pressure_deficit: 0.0,
            latent_heat: 0.0,
            sensible_heat: 0.0,
        }
    }
}

impl Evaporation {
    const PSYCHROMETRIC_CONSTANT: f32 = 66.0;
    const LATENT_HEAT_VAPORIZATION: f32 = 2.45e6;

    pub fn penman_monteith(
        net_radiation: f32,
        temperature: f32,
        wind_speed: f32,
        humidity: f32,
        atmospheric_pressure: f32,
    ) -> f32 {
        let es = 611.2 * ((17.27 * (temperature - 273.15)) / (temperature - 35.85)).exp();
        let ea = humidity * es;
        let vpd = es - ea;

        let delta = 4098.0 * es / ((temperature - 35.85 + 237.3).powi(2));

        let gamma = Self::PSYCHROMETRIC_CONSTANT * atmospheric_pressure / 101325.0;

        let wind_term = (900.0 / (temperature)) * wind_speed * vpd;

        let numerator = 0.408 * delta * net_radiation + gamma * wind_term;
        let denominator = delta + gamma * (1.0 + 0.34 * wind_speed);

        if denominator <= 0.0 {
            return 0.0;
        }
        (numerator / denominator).max(0.0)
    }

    pub fn priestley_taylor(net_radiation: f32, temperature: f32) -> f32 {
        let delta =
            4098.0 * 0.6108 * ((17.27 * (temperature - 273.15)) / (temperature - 35.85)).exp()
                / ((temperature - 35.85 + 237.3).powi(2));
        let alpha = 1.26;
        alpha * (delta / (delta + Self::PSYCHROMETRIC_CONSTANT)) * net_radiation
    }

    pub fn open_water_evaporation(
        wind_speed: f32,
        saturation_vapor_pressure: f32,
        actual_vapor_pressure: f32,
    ) -> f32 {
        let vpd = saturation_vapor_pressure - actual_vapor_pressure;
        (0.0029 + 0.00019 * wind_speed) * vpd * 1000.0
    }

    pub fn lake_evaporation(
        &self,
        water_area: f32,
        water_temperature: f32,
        air_temperature: f32,
        humidity: f32,
        wind_speed: f32,
    ) -> f32 {
        let es_water =
            611.2 * ((17.27 * (water_temperature - 273.15)) / (water_temperature - 35.85)).exp();
        let ea = humidity
            * 611.2
            * ((17.27 * (air_temperature - 273.15)) / (air_temperature - 35.85)).exp();
        let mass_transfer = 0.002 * wind_speed * (es_water - ea);
        water_area * mass_transfer
    }

    pub fn update(
        &mut self,
        solar_radiation: f32,
        temperature: f32,
        humidity: f32,
        wind_speed: f32,
        _dt: f32,
    ) {
        self.net_radiation = solar_radiation * 0.6;
        self.wind_speed = wind_speed;

        let es = 611.2 * ((17.27 * (temperature - 273.15)) / (temperature - 35.85)).exp();
        let ea = humidity * es;
        self.vapor_pressure_deficit = es - ea;

        self.rate =
            Self::penman_monteith(self.net_radiation, temperature, wind_speed, humidity, 101325.0);

        self.latent_heat = self.rate * Self::LATENT_HEAT_VAPORIZATION;
        self.sensible_heat = self.net_radiation - self.latent_heat;
    }

    pub fn evapotranspiration_mm_per_day(&self) -> f32 {
        self.rate * 86400.0 / Self::LATENT_HEAT_VAPORIZATION * 1000.0
    }
}
