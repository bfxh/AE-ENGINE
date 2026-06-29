use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClimateZone {
    Tropical,
    Subtropical,
    Temperate,
    Boreal,
    Polar,
    Arid,
    SemiArid,
    Mediterranean,
    Alpine,
    Coastal,
}

impl ClimateZone {
    pub fn from_latitude(latitude: f32, elevation: f32, distance_to_ocean: f32) -> Self {
        let abs_lat = latitude.abs();
        if elevation > 3000.0 {
            return ClimateZone::Alpine;
        }
        if abs_lat > 66.5 {
            return ClimateZone::Polar;
        }
        if abs_lat > 50.0 {
            return ClimateZone::Boreal;
        }
        if distance_to_ocean < 100.0 && abs_lat > 30.0 && abs_lat < 45.0 {
            return ClimateZone::Mediterranean;
        }
        if distance_to_ocean < 50.0 {
            return ClimateZone::Coastal;
        }
        if abs_lat < 23.5 {
            return ClimateZone::Tropical;
        }
        if abs_lat < 35.0 {
            return ClimateZone::Subtropical;
        }
        ClimateZone::Temperate
    }

    pub fn base_temperature(&self) -> f32 {
        match self {
            ClimateZone::Tropical => 300.0,
            ClimateZone::Subtropical => 293.0,
            ClimateZone::Temperate => 283.0,
            ClimateZone::Boreal => 273.0,
            ClimateZone::Polar => 253.0,
            ClimateZone::Arid => 298.0,
            ClimateZone::SemiArid => 290.0,
            ClimateZone::Mediterranean => 289.0,
            ClimateZone::Alpine => 268.0,
            ClimateZone::Coastal => 288.0,
        }
    }

    pub fn base_humidity(&self) -> f32 {
        match self {
            ClimateZone::Tropical => 0.85,
            ClimateZone::Subtropical => 0.7,
            ClimateZone::Temperate => 0.65,
            ClimateZone::Boreal => 0.55,
            ClimateZone::Polar => 0.4,
            ClimateZone::Arid => 0.15,
            ClimateZone::SemiArid => 0.3,
            ClimateZone::Mediterranean => 0.55,
            ClimateZone::Alpine => 0.5,
            ClimateZone::Coastal => 0.75,
        }
    }

    pub fn precipitation_multiplier(&self) -> f32 {
        match self {
            ClimateZone::Tropical => 2.0,
            ClimateZone::Subtropical => 1.2,
            ClimateZone::Temperate => 1.0,
            ClimateZone::Boreal => 0.6,
            ClimateZone::Polar => 0.2,
            ClimateZone::Arid => 0.1,
            ClimateZone::SemiArid => 0.3,
            ClimateZone::Mediterranean => 0.7,
            ClimateZone::Alpine => 1.5,
            ClimateZone::Coastal => 1.3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn from_day_of_year(day: u32, latitude: f32) -> Self {
        let northern = match day {
            60..=151 => Season::Spring,
            152..=243 => Season::Summer,
            244..=334 => Season::Autumn,
            _ => Season::Winter,
        };
        if latitude >= 0.0 { northern } else { northern.opposite() }
    }

    pub fn opposite(&self) -> Self {
        match self {
            Season::Spring => Season::Autumn,
            Season::Summer => Season::Winter,
            Season::Autumn => Season::Spring,
            Season::Winter => Season::Summer,
        }
    }

    pub fn temperature_modifier(&self) -> f32 {
        match self {
            Season::Spring => 0.0,
            Season::Summer => 10.0,
            Season::Autumn => 0.0,
            Season::Winter => -10.0,
        }
    }

    pub fn daylight_factor(&self) -> f32 {
        match self {
            Season::Spring => 0.75,
            Season::Summer => 1.0,
            Season::Autumn => 0.75,
            Season::Winter => 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherPattern {
    pub zone: ClimateZone,
    pub season: Season,
    pub temperature: f32,
    pub humidity: f32,
    pub pressure: f32,
    pub cloud_cover: f32,
    pub fog_density: f32,
    pub days_since_rain: f32,
}

impl WeatherPattern {
    pub fn new(zone: ClimateZone, season: Season) -> Self {
        let base_temp = zone.base_temperature() + season.temperature_modifier();
        Self {
            zone,
            season,
            temperature: base_temp,
            humidity: zone.base_humidity(),
            pressure: 101325.0,
            cloud_cover: 0.3,
            fog_density: 0.0,
            days_since_rain: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        let target_temp = self.zone.base_temperature() + self.season.temperature_modifier();
        self.temperature += (target_temp - self.temperature) * 0.001 * dt;

        let target_humidity = self.zone.base_humidity();
        self.humidity += (target_humidity - self.humidity) * 0.0005 * dt;

        self.cloud_cover += (self.humidity - self.cloud_cover) * 0.01 * dt;
        self.cloud_cover = self.cloud_cover.clamp(0.0, 1.0);

        self.fog_density = if self.humidity > 0.9 && self.temperature < 285.0 {
            (self.fog_density + 0.01 * dt).min(1.0)
        } else {
            (self.fog_density - 0.02 * dt).max(0.0)
        };

        self.days_since_rain += dt / 86400.0;
    }

    pub fn orographic_lift(&mut self, elevation_change: f32, wind_speed: f32) {
        if wind_speed > 0.0 && elevation_change > 0.0 {
            let cooling = elevation_change * 0.0065;
            self.temperature -= cooling;
            let condensation = (self.humidity * cooling * 0.1).min(1.0);
            self.humidity = (self.humidity - condensation * 0.5).max(0.0);
            self.cloud_cover = (self.cloud_cover + condensation).min(1.0);
        }
    }

    pub fn rain_shadow(&mut self, elevation_change: f32) {
        if elevation_change < -100.0 {
            self.humidity = (self.humidity - 0.1).max(0.0);
            self.temperature += 5.0;
        }
    }
}
